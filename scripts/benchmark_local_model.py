#!/usr/bin/env python3
"""Reproducible OpenAI-compatible local-model benchmark.

Runs deterministic performance, context, native-tool, and task-correctness
checks against one already-isolated model endpoint and writes a JSON receipt.
Uses only the Python standard library so the same file runs on fleet nodes.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import statistics
import subprocess
import tempfile
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable


@dataclass
class MemorySample:
    timestamp: float
    ram_used_bytes: int
    swap_used_bytes: int
    process_rss_bytes: int | None
    gpu_used_bytes: int | None
    gpu_total_bytes: int | None


@dataclass
class Sampler:
    pid: int | None
    interval: float = 0.2
    samples: list[MemorySample] = field(default_factory=list)
    _stop: threading.Event = field(default_factory=threading.Event)
    _thread: threading.Thread | None = None

    @staticmethod
    def _meminfo() -> tuple[int, int]:
        values: dict[str, int] = {}
        for line in Path("/proc/meminfo").read_text().splitlines():
            key, value = line.split(":", 1)
            values[key] = int(value.strip().split()[0]) * 1024
        ram_used = values["MemTotal"] - values.get("MemAvailable", values["MemFree"])
        swap_used = values.get("SwapTotal", 0) - values.get("SwapFree", 0)
        return ram_used, swap_used

    @staticmethod
    def _gpu_memory() -> tuple[int | None, int | None]:
        try:
            output = subprocess.check_output(
                ["nvidia-smi", "--query-gpu=memory.used,memory.total", "--format=csv,noheader,nounits"],
                text=True,
                stderr=subprocess.DEVNULL,
                timeout=2,
            ).splitlines()[0]
            used_mib, total_mib = (int(part.strip()) for part in output.split(",")[:2])
            return used_mib * 1024**2, total_mib * 1024**2
        except (FileNotFoundError, subprocess.SubprocessError, ValueError, IndexError):
            pass

        used = total = 0
        found = False
        for card in Path("/sys/class/drm").glob("card*/device"):
            # UMA Vulkan allocations are normally exposed as GTT; include VRAM
            # as well while avoiding duplicate totals from unrelated cards.
            for kind in ("vram", "gtt"):
                try:
                    total += int((card / f"mem_info_{kind}_total").read_text())
                    used += int((card / f"mem_info_{kind}_used").read_text())
                    found = True
                except (FileNotFoundError, PermissionError, ValueError):
                    continue
        return (used, total) if found else (None, None)

    def _one(self) -> MemorySample:
        ram, swap = self._meminfo()
        rss = None
        if self.pid:
            try:
                for line in Path(f"/proc/{self.pid}/status").read_text().splitlines():
                    if line.startswith("VmRSS:"):
                        rss = int(line.split()[1]) * 1024
                        break
            except (FileNotFoundError, PermissionError, ProcessLookupError):
                pass
        gpu_used, gpu_total = self._gpu_memory()
        return MemorySample(time.time(), ram, swap, rss, gpu_used, gpu_total)

    def start(self) -> None:
        self.samples.append(self._one())
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def _run(self) -> None:
        while not self._stop.wait(self.interval):
            self.samples.append(self._one())

    def stop(self) -> None:
        self._stop.set()
        if self._thread:
            self._thread.join(timeout=2)
        self.samples.append(self._one())

    def summary(self) -> dict[str, Any]:
        def summarize(name: str) -> dict[str, int | None]:
            values = [getattr(item, name) for item in self.samples]
            actual = [value for value in values if value is not None]
            return {
                "baseline": actual[0] if actual else None,
                "peak": max(actual) if actual else None,
                "final": actual[-1] if actual else None,
                "growth": (actual[-1] - actual[0]) if actual else None,
            }

        return {
            "sample_count": len(self.samples),
            "ram_used_bytes": summarize("ram_used_bytes"),
            "swap_used_bytes": summarize("swap_used_bytes"),
            "process_rss_bytes": summarize("process_rss_bytes"),
            "gpu_used_bytes": summarize("gpu_used_bytes"),
            "gpu_total_bytes": summarize("gpu_total_bytes"),
        }


class ApiError(RuntimeError):
    pass


class Client:
    def __init__(self, base_url: str, model: str, timeout: int):
        self.base_url = base_url.rstrip("/")
        self.model = model
        self.timeout = timeout

    def _request(self, path: str, payload: dict[str, Any], stream: bool = False) -> Any:
        request = urllib.request.Request(
            self.base_url + path,
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
        )
        try:
            response = urllib.request.urlopen(request, timeout=self.timeout)
        except urllib.error.HTTPError as exc:
            detail = exc.read().decode(errors="replace")
            raise ApiError(f"HTTP {exc.code}: {detail[:2000]}") from exc
        except (urllib.error.URLError, TimeoutError) as exc:
            raise ApiError(str(exc)) from exc
        return response if stream else json.loads(response.read())

    def chat(
        self,
        messages: list[dict[str, Any]],
        *,
        max_tokens: int = 256,
        tools: list[dict[str, Any]] | None = None,
        stream: bool = False,
    ) -> Any:
        payload: dict[str, Any] = {
            "model": self.model,
            "messages": messages,
            "temperature": 0,
            "seed": 42,
            "max_tokens": max_tokens,
            "stream": stream,
            # Agent routes need final answers/tool calls without spending the
            # completion budget on visible chain-of-thought. Qwen-family chat
            # templates honor this knob; unsupported servers ignore it.
            "chat_template_kwargs": {"enable_thinking": False},
        }
        if tools:
            payload["tools"] = tools
            payload["tool_choice"] = "auto"
        return self._request("/v1/chat/completions", payload, stream=stream)

    def streamed(self, messages: list[dict[str, Any]], max_tokens: int) -> dict[str, Any]:
        start = time.monotonic()
        first_token = None
        chunks: list[str] = []
        final: dict[str, Any] = {}
        with self.chat(messages, max_tokens=max_tokens, stream=True) as response:
            for raw in response:
                line = raw.decode(errors="replace").strip()
                if not line.startswith("data:"):
                    continue
                data = line[5:].strip()
                if data == "[DONE]":
                    break
                try:
                    event = json.loads(data)
                except json.JSONDecodeError:
                    continue
                if event.get("timings"):
                    final["timings"] = event["timings"]
                choices = event.get("choices", [])
                if not choices:
                    continue
                delta = choices[0].get("delta", {})
                text = delta.get("content") or ""
                if text:
                    if first_token is None:
                        first_token = time.monotonic()
                    chunks.append(text)
                if choices[0].get("finish_reason"):
                    final["finish_reason"] = choices[0]["finish_reason"]
        end = time.monotonic()
        final.update(
            {
                "elapsed_seconds": end - start,
                "ttft_seconds": (first_token - start) if first_token else None,
                "content": "".join(chunks),
            }
        )
        return final


def content_of(response: dict[str, Any]) -> str:
    return (response.get("choices") or [{}])[0].get("message", {}).get("content") or ""


def message_of(response: dict[str, Any]) -> dict[str, Any]:
    return (response.get("choices") or [{}])[0].get("message", {})


def parse_json_text(text: str) -> Any:
    text = text.strip()
    fenced = re.search(r"```(?:json)?\s*(.*?)```", text, flags=re.DOTALL | re.IGNORECASE)
    if fenced:
        text = fenced.group(1).strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        start = min((index for index in (text.find("{"), text.find("[")) if index >= 0), default=-1)
        if start < 0:
            raise
        decoder = json.JSONDecoder()
        return decoder.raw_decode(text[start:])[0]


def normalize_tool_calls(message: dict[str, Any]) -> list[dict[str, Any]]:
    calls = message.get("tool_calls") or []
    normalized = []
    for call in calls:
        fn = call.get("function", {})
        arguments = fn.get("arguments", {})
        if isinstance(arguments, str):
            try:
                arguments = json.loads(arguments)
            except json.JSONDecodeError:
                arguments = {"__invalid_json__": arguments}
        normalized.append({"id": call.get("id"), "name": fn.get("name"), "arguments": arguments})
    return normalized


def tool(name: str, description: str, properties: dict[str, Any], required: list[str]) -> dict[str, Any]:
    return {
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": {"type": "object", "properties": properties, "required": required, "additionalProperties": False},
        },
    }


def run_check(name: str, fn: Callable[[], tuple[bool, dict[str, Any]]], category: str) -> dict[str, Any]:
    started = time.monotonic()
    try:
        passed, evidence = fn()
        return {"name": name, "category": category, "passed": bool(passed), "elapsed_seconds": time.monotonic() - started, "evidence": evidence}
    except Exception as exc:  # benchmark failures belong in the receipt
        return {"name": name, "category": category, "passed": False, "elapsed_seconds": time.monotonic() - started, "error": f"{type(exc).__name__}: {exc}"}


def filler_for_tokens(target: int, marker: str | None = None) -> str:
    # A repeated ASCII token is approximately one token across the candidate
    # tokenizers. The API-reported prompt_n remains authoritative in receipts.
    # Leave headroom for the chat template, benchmark instruction, and output.
    words = ["x"] * max(1, target - 512)
    if marker:
        words[len(words) // 2] = marker
    return " ".join(words)


def performance_suite(
    client: Client,
    contexts: list[int],
    repetitions: int,
    long_context: int,
    long_context_repetitions: int,
    long_context_max_seconds: float,
) -> list[dict[str, Any]]:
    results = []
    def prompt(target: int, nonce: str) -> str:
        return (
            f"Benchmark nonce {nonce}. "
            "This is a throughput fixture. Read all tokens, then emit an endless sequence of the word "
            "benchmark separated by spaces. Do not explain and do not stop early.\n" + filler_for_tokens(target)
        )

    # One short warm-up initializes kernels without pre-caching a measured body.
    client.chat([{"role": "user", "content": prompt(768, "warmup")}], max_tokens=8)
    for target in contexts:
        if target >= long_context:
            lower = [
                row
                for row in results
                if "error" not in row
                and row.get("target_prompt_tokens", 0) < target
                and isinstance(row.get("elapsed_seconds"), (int, float))
            ]
            if lower:
                nearest_size = max(row["target_prompt_tokens"] for row in lower)
                nearest_elapsed = statistics.median(
                    row["elapsed_seconds"]
                    for row in lower
                    if row["target_prompt_tokens"] == nearest_size
                )
                projected_seconds = nearest_elapsed * target / nearest_size
                if projected_seconds > long_context_max_seconds:
                    results.append(
                        {
                            "target_prompt_tokens": target,
                            "repetition": 1,
                            "error": (
                                f"projected duration {projected_seconds:.1f}s exceeds "
                                f"{long_context_max_seconds:.1f}s edge latency budget"
                            ),
                            "projection_source_tokens": nearest_size,
                            "projection_source_seconds": nearest_elapsed,
                        }
                    )
                    continue
        target_repetitions = long_context_repetitions if target >= long_context // 2 else repetitions
        for repetition in range(1, target_repetitions + 1):
            try:
                # The nonce precedes the repeated body so prompt caching cannot
                # turn prefill and TTFT into warm-cache measurements.
                measured = client.streamed(
                    [{"role": "user", "content": prompt(target, f"measured-{target}-{repetition}")}],
                    max_tokens=256,
                )
                timings = measured.pop("timings", {})
                content = measured.pop("content", "")
                result = {
                    "target_prompt_tokens": target,
                    "repetition": repetition,
                    **measured,
                    "output_characters": len(content),
                    "visible_reasoning": bool(re.search(r"<think>|</think>|reasoning_content", content, re.I)),
                    "timings": timings,
                }
                if target >= long_context and result["elapsed_seconds"] > long_context_max_seconds:
                    result["error"] = (
                        f"duration {result['elapsed_seconds']:.1f}s exceeds "
                        f"{long_context_max_seconds:.1f}s edge latency budget"
                    )
                results.append(result)
            except Exception as exc:
                results.append({"target_prompt_tokens": target, "repetition": repetition, "error": f"{type(exc).__name__}: {exc}"})
                for skipped in range(repetition + 1, target_repetitions + 1):
                    results.append(
                        {
                            "target_prompt_tokens": target,
                            "repetition": skipped,
                            "error": "skipped after prior repetition failed",
                        }
                    )
                break
    return results


def correctness_suite(client: Client, long_context: int, skip_long_context: bool = False) -> list[dict[str, Any]]:
    checks: list[dict[str, Any]] = []

    def exact_json() -> tuple[bool, dict[str, Any]]:
        response = client.chat([{"role": "user", "content": 'Return only this JSON object with native JSON types: {"name":"arda","count":3,"enabled":true}'}], max_tokens=96)
        text = content_of(response)
        parsed = parse_json_text(text)
        expected = {"name": "arda", "count": 3, "enabled": True}
        return parsed == expected and text.strip().startswith("{"), {"content": text, "parsed": parsed}

    checks.append(run_check("exact_json_schema", exact_json, "correctness"))

    read_tool = tool(
        "read_file",
        "Read a text file.",
        {"path": {"type": "string"}, "offset": {"type": "integer"}},
        ["path", "offset"],
    )

    def single_tool() -> tuple[bool, dict[str, Any]]:
        response = client.chat([{"role": "user", "content": "Use the tool to read Cargo.toml starting at line 17. Do not answer without calling it."}], tools=[read_tool], max_tokens=128)
        calls = normalize_tool_calls(message_of(response))
        valid = len(calls) == 1 and calls[0]["name"] == "read_file" and calls[0]["arguments"] == {"path": "Cargo.toml", "offset": 17}
        return valid, {"calls": calls, "content": content_of(response)}

    checks.append(run_check("single_native_tool", single_tool, "tools"))

    weather = tool("weather", "Get weather for a city.", {"city": {"type": "string"}}, ["city"])
    calculate = tool("calculate", "Evaluate an arithmetic expression.", {"expression": {"type": "string"}}, ["expression"])
    irrelevant = tool("delete_file", "Delete a file.", {"path": {"type": "string"}}, ["path"])

    def multi_tool() -> tuple[bool, dict[str, Any]]:
        response = client.chat(
            [{"role": "user", "content": "Select and call exactly one appropriate tool to get the weather in Oslo. Do not call unrelated tools."}],
            tools=[weather, calculate, irrelevant],
            max_tokens=192,
        )
        calls = normalize_tool_calls(message_of(response))
        valid = len(calls) == 1 and calls[0]["name"] == "weather" and calls[0]["arguments"] == {"city": "Oslo"}
        return valid, {"calls": calls, "content": content_of(response)}

    checks.append(run_check("multi_tool_selection", multi_tool, "tools"))

    typed = tool(
        "write_note",
        "Write a typed note.",
        {
            "text": {"type": "string"},
            "count": {"type": "integer"},
            "enabled": {"type": "boolean"},
        },
        ["text", "count", "enabled"],
    )

    def escaped_tool() -> tuple[bool, dict[str, Any]]:
        expected_text = 'line one "quoted"\nline two \\ path'
        response = client.chat(
            [{"role": "user", "content": f"Call write_note with text exactly {json.dumps(expected_text)}, count integer 7, and enabled boolean true."}],
            tools=[typed],
            max_tokens=160,
        )
        calls = normalize_tool_calls(message_of(response))
        args = calls[0]["arguments"] if len(calls) == 1 else {}
        valid = bool(calls) and calls[0]["name"] == "write_note" and args == {"text": expected_text, "count": 7, "enabled": True}
        return valid, {"calls": calls, "content": content_of(response)}

    checks.append(run_check("tool_argument_escaping_and_types", escaped_tool, "tools"))

    multiply = tool("multiply", "Multiply two integers.", {"a": {"type": "integer"}, "b": {"type": "integer"}}, ["a", "b"])

    def continuation() -> tuple[bool, dict[str, Any]]:
        first = client.chat([{"role": "user", "content": "Use multiply for 17 times 19, then report only RESULT=<number>."}], tools=[multiply], max_tokens=128)
        message = message_of(first)
        calls = normalize_tool_calls(message)
        if len(calls) != 1 or calls[0]["name"] != "multiply" or calls[0]["arguments"] != {"a": 17, "b": 19}:
            return False, {"initial_calls": calls, "initial_content": content_of(first)}
        call_id = calls[0]["id"] or "call_benchmark"
        assistant = dict(message)
        assistant["tool_calls"] = message.get("tool_calls")
        second = client.chat(
            [
                {"role": "user", "content": "Use multiply for 17 times 19, then report only RESULT=<number>."},
                assistant,
                {"role": "tool", "tool_call_id": call_id, "name": "multiply", "content": "323"},
            ],
            tools=[multiply],
            max_tokens=96,
        )
        final = content_of(second).strip()
        return final == "RESULT=323", {"initial_calls": calls, "final": final}

    checks.append(run_check("tool_result_continuation", continuation, "tools"))

    def instruction() -> tuple[bool, dict[str, Any]]:
        response = client.chat([{"role": "user", "content": "Output exactly ARDA_SENTINEL_7 and nothing else. Do not reveal reasoning."}], max_tokens=80)
        message = message_of(response)
        text = content_of(response).strip()
        contaminated = bool(re.search(r"<think>|</think>|reasoning_content", text, re.I)) or bool(message.get("reasoning_content"))
        return text == "ARDA_SENTINEL_7" and not contaminated, {"content": text, "reasoning_content": message.get("reasoning_content")}

    checks.append(run_check("instruction_and_no_visible_reasoning", instruction, "correctness"))

    def navigation() -> tuple[bool, dict[str, Any]]:
        fixture = """Synthetic repository tree (the only source of truth):
Cargo.toml
crates/arda-router/Cargo.toml
crates/arda-router/src/lib.rs
crates/arda-router/src/policy.rs
apps/hud/package.json
The workspace Cargo.toml lists members only. crates/arda-router/Cargo.toml declares the dependency serde_json. Which exact file must be edited to change serde_json's version for arda-router? Reply with only the path."""
        text = content_of(client.chat([{"role": "user", "content": fixture}], max_tokens=96)).strip().strip("`")
        return text == "crates/arda-router/Cargo.toml", {"content": text}

    checks.append(run_check("repository_navigation", navigation, "correctness"))

    def rust_diagnosis() -> tuple[bool, dict[str, Any]]:
        prompt = """Diagnose this Rust compile error and reply only with the letter of the root cause.
error: future cannot be sent between threads safely; MutexGuard is not Send; value is used across an await
A) Cargo.lock is stale
B) a std::sync::MutexGuard remains alive across .await
C) tokio lacks the macros feature
D) the function needs unsafe
Code: let guard = state.lock().unwrap(); client.send(guard.value()).await?; drop(guard);"""
        text = content_of(client.chat([{"role": "user", "content": prompt}], max_tokens=64)).strip()
        return text == "B", {"content": text}

    checks.append(run_check("rust_diagnosis", rust_diagnosis, "correctness"))

    def code_edit() -> tuple[bool, dict[str, Any]]:
        prompt = """Return only JSON {"code":"..."}. The code value must be a complete Rust function with exactly this signature:
pub fn clamp_level(value: i32, low: i32, high: i32) -> i32
Requirements: return low when value < low, high when value > high, otherwise value. Do not use external crates."""
        text = content_of(client.chat([{"role": "user", "content": prompt}], max_tokens=256))
        parsed = parse_json_text(text)
        code = parsed.get("code", "") if isinstance(parsed, dict) else ""
        test_source = code + "\n#[cfg(test)] mod tests { use super::*; #[test] fn boundaries() { assert_eq!(clamp_level(-2,0,10),0); assert_eq!(clamp_level(5,0,10),5); assert_eq!(clamp_level(12,0,10),10); } }\n"
        with tempfile.TemporaryDirectory(prefix="arda-model-bench-") as directory:
            source = Path(directory) / "candidate.rs"
            binary = Path(directory) / "candidate_test"
            source.write_text(test_source)
            proc = subprocess.run(["rustc", "--edition=2021", "--test", str(source), "-o", str(binary)], capture_output=True, text=True, timeout=30)
            if proc.returncode == 0:
                proc = subprocess.run([str(binary)], capture_output=True, text=True, timeout=30)
        valid = proc.returncode == 0
        return valid, {"code": code, "exit_code": proc.returncode, "stdout": proc.stdout[-1000:], "stderr": proc.stderr[-1000:]}

    checks.append(run_check("small_rust_edit_validated", code_edit, "correctness"))

    def nonexistent() -> tuple[bool, dict[str, Any]]:
        prompt = """The complete synthetic repository contains only Cargo.toml and src/main.rs. What function is defined in src/router.rs? You may not assume unseen files. Reply exactly CANNOT_VERIFY if the requested file is absent."""
        text = content_of(client.chat([{"role": "user", "content": prompt}], max_tokens=96)).strip()
        return text == "CANNOT_VERIFY", {"content": text}

    checks.append(run_check("nonexistent_symbol_nonhallucination", nonexistent, "correctness"))

    def long_retrieval() -> tuple[bool, dict[str, Any]]:
        if skip_long_context:
            return False, {
                "target_tokens": long_context,
                "skipped": "performance request at this context failed",
            }
        marker = "ARDA_LONG_CONTEXT_KEY=mirromere-7391"
        prompt = (
            "Read this fixture. One key-value pair is embedded near the middle. At the end, reply with only its value.\n"
            + filler_for_tokens(long_context, marker)
            + "\nWhat is the exact value of ARDA_LONG_CONTEXT_KEY?"
        )
        response = client.chat([{"role": "user", "content": prompt}], max_tokens=64)
        text = content_of(response).strip()
        return text == "mirromere-7391", {"content": text, "usage": response.get("usage"), "timings": response.get("timings")}

    checks.append(run_check("long_context_retrieval", long_retrieval, "context"))
    return checks


def aggregate(performance: list[dict[str, Any]], checks: list[dict[str, Any]], memory: dict[str, Any], gates: dict[str, float]) -> dict[str, Any]:
    valid_perf = [row for row in performance if "error" not in row]
    prompt_8k = [row.get("timings", {}).get("prompt_per_second") for row in valid_perf if row["target_prompt_tokens"] == 8192]
    gen_all = [row.get("timings", {}).get("predicted_per_second") for row in valid_perf]
    ttft_all = [row.get("ttft_seconds") for row in valid_perf]
    prompt_8k = [value for value in prompt_8k if isinstance(value, (int, float))]
    gen_all = [value for value in gen_all if isinstance(value, (int, float))]
    ttft_all = [value for value in ttft_all if isinstance(value, (int, float))]

    groups: dict[str, list[dict[str, Any]]] = {}
    for check in checks:
        groups.setdefault(check["category"], []).append(check)
    rates = {name: sum(bool(item["passed"]) for item in items) / len(items) for name, items in groups.items()}
    correctness = rates.get("correctness", 0.0)
    tools = rates.get("tools", 0.0)
    context = rates.get("context", 0.0)

    median_gen = statistics.median(gen_all) if gen_all else 0.0
    median_prompt_8k = statistics.median(prompt_8k) if prompt_8k else 0.0
    latency_score = min(1.0, median_gen / gates["generation_tps"]) * 0.6 + min(1.0, median_prompt_8k / gates["prompt_8k_tps"]) * 0.4
    gpu = memory.get("gpu_used_bytes", {})
    total = memory.get("gpu_total_bytes", {}).get("baseline")
    peak = gpu.get("peak")
    margin = (total - peak) if isinstance(total, int) and isinstance(peak, int) else None
    memory_threshold = gates["memory_margin_bytes"]
    if memory_threshold <= 0:
        memory_score = 1.0
    else:
        memory_score = min(1.0, max(0.0, margin / memory_threshold)) if margin is not None else 0.0
    weighted = 100 * (0.40 * correctness + 0.25 * tools + 0.20 * latency_score + 0.10 * context + 0.05 * memory_score)
    swap_growth = memory.get("swap_used_bytes", {}).get("growth")
    gate_results = {
        "generation_tps": median_gen >= gates["generation_tps"],
        "prompt_8k_tps": median_prompt_8k >= gates["prompt_8k_tps"],
        "tool_validity": tools >= gates["tool_validity"],
        "task_correctness": correctness >= gates["task_correctness"],
        "context_stability": context == 1.0,
        "no_swap_growth": isinstance(swap_growth, int) and swap_growth <= 64 * 1024**2,
        "memory_margin": memory_threshold <= 0 or (margin is not None and margin >= memory_threshold),
        "no_performance_errors": len(valid_perf) == len(performance),
    }
    return {
        "weighted_score": round(weighted, 3),
        "category_rates": rates,
        "median_generation_tokens_per_second": median_gen,
        "median_prompt_8k_tokens_per_second": median_prompt_8k,
        "median_ttft_seconds": statistics.median(ttft_all) if ttft_all else None,
        "practical_gpu_margin_bytes": margin,
        "gates": gate_results,
        "all_gates_passed": all(gate_results.values()),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--url", required=True, help="OpenAI-compatible base URL, normally http://127.0.0.1:PORT")
    parser.add_argument("--model", required=True)
    parser.add_argument("--host-label", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--pid", type=int)
    parser.add_argument("--load-seconds", type=float)
    parser.add_argument("--contexts", default="1024,8192,32768")
    parser.add_argument("--long-context", type=int, default=32768)
    parser.add_argument("--repetitions", type=int, default=3)
    parser.add_argument("--long-context-repetitions", type=int, default=1)
    parser.add_argument("--long-context-max-seconds", type=float, default=300.0)
    parser.add_argument("--timeout", type=int, default=900)
    parser.add_argument("--generation-gate", type=float, required=True)
    parser.add_argument("--prompt-8k-gate", type=float, required=True)
    parser.add_argument("--tool-gate", type=float, required=True)
    parser.add_argument("--correctness-gate", type=float, default=0.85)
    parser.add_argument("--memory-margin-gib", type=float, default=1.0)
    args = parser.parse_args()

    contexts = [int(item) for item in args.contexts.split(",")]
    client = Client(args.url, args.model, args.timeout)
    # Fail before sampling a dead/wrong endpoint.
    models = client._request("/v1/models", {}) if False else json.loads(urllib.request.urlopen(args.url.rstrip("/") + "/v1/models", timeout=10).read())

    sampler = Sampler(args.pid)
    sampler.start()
    started = time.monotonic()
    performance = performance_suite(
        client,
        contexts,
        args.repetitions,
        args.long_context,
        args.long_context_repetitions,
        args.long_context_max_seconds,
    )
    long_context_failed = any(
        row.get("target_prompt_tokens", 0) >= args.long_context
        and (row.get("error") or row.get("elapsed_seconds", 0) > args.long_context_max_seconds)
        for row in performance
    )
    checks = correctness_suite(client, args.long_context, skip_long_context=long_context_failed)
    # Recovery window: detect memory/swap that remains after request completion.
    time.sleep(3)
    sampler.stop()

    gates = {
        "generation_tps": args.generation_gate,
        "prompt_8k_tps": args.prompt_8k_gate,
        "tool_validity": args.tool_gate,
        "task_correctness": args.correctness_gate,
        "memory_margin_bytes": args.memory_margin_gib * 1024**3,
    }
    memory = sampler.summary()
    receipt = {
        "schema_version": 1,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "host": args.host_label,
        "model": args.model,
        "endpoint": args.url,
        "models_response": models,
        "process_pid": args.pid,
        "model_load_seconds": args.load_seconds,
        "benchmark_elapsed_seconds": time.monotonic() - started,
        "configuration": {
            "contexts": contexts,
            "long_context": args.long_context,
            "repetitions": args.repetitions,
            "long_context_repetitions": args.long_context_repetitions,
            "long_context_max_seconds": args.long_context_max_seconds,
            "gates": gates,
        },
        "memory": memory,
        "performance": performance,
        "checks": checks,
    }
    receipt["summary"] = aggregate(performance, checks, memory, gates)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(json.dumps({"receipt": str(args.output), **receipt["summary"]}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
