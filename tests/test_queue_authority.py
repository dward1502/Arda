import os
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GUARD = ROOT / "scripts" / "check_task_queue_append_only.sh"


def _git(repo: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=repo,
        check=True,
        text=True,
        capture_output=True,
    )


def _queue_repo(tmp_path: Path) -> Path:
    queue = tmp_path / "core" / "projects" / "tasks" / "queue.jsonl"
    queue.parent.mkdir(parents=True)
    queue.write_text('{"id":"one","status":"queued"}\n')
    _git(tmp_path, "init", "-q")
    _git(tmp_path, "config", "user.email", "queue-test@example.invalid")
    _git(tmp_path, "config", "user.name", "Queue Test")
    _git(tmp_path, "add", "core/projects/tasks/queue.jsonl")
    _git(tmp_path, "commit", "-qm", "queue baseline")
    return queue


def _run_guard(repo: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["ARDA_ROOT"] = str(repo)
    return subprocess.run(
        [str(GUARD)],
        cwd=repo,
        env=env,
        text=True,
        capture_output=True,
    )


def test_append_only_guard_checks_canonical_project_queue(tmp_path: Path) -> None:
    queue = _queue_repo(tmp_path)
    with queue.open("a") as handle:
        handle.write('{"id":"two","status":"queued"}\n')

    result = _run_guard(tmp_path)

    assert result.returncode == 0
    assert "ok append-only" in result.stdout
    assert "core/projects/tasks/queue.jsonl" in result.stdout


def test_append_only_guard_allows_first_rows_after_empty_baseline(tmp_path: Path) -> None:
    queue = _queue_repo(tmp_path)
    queue.write_text("")
    _git(tmp_path, "add", "core/projects/tasks/queue.jsonl")
    _git(tmp_path, "commit", "-qm", "empty queue baseline")
    queue.write_text('{"id":"operator-objective","status":"pending"}\n')

    result = _run_guard(tmp_path)

    assert result.returncode == 0
    assert "ok append-only from empty baseline" in result.stdout


def test_append_only_guard_blocks_rewritten_canonical_project_queue(tmp_path: Path) -> None:
    queue = _queue_repo(tmp_path)
    queue.write_text('{"id":"replacement","status":"queued"}\n')

    result = _run_guard(tmp_path)

    assert result.returncode == 1
    assert "blocked non-append edit" in result.stderr
    assert "core/projects/tasks/queue.jsonl" in result.stderr


def test_append_only_guard_uses_last_historical_baseline_after_queue_restoration(
    tmp_path: Path,
) -> None:
    queue = _queue_repo(tmp_path)
    queue.unlink()
    _git(tmp_path, "add", "core/projects/tasks/queue.jsonl")
    _git(tmp_path, "commit", "-qm", "remove queue")
    queue.write_text('{"id":"replacement","status":"queued"}\n')

    result = _run_guard(tmp_path)

    assert result.returncode == 1
    assert "blocked non-append edit" in result.stderr
