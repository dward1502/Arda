import { Terminal } from "xterm";
import { FitAddon } from "xterm-addon-fit";
import "xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api";

const terminalElement = document.getElementById("terminal");
if (!terminalElement) {
  throw new Error("missing #terminal");
}

const fitAddon = new FitAddon();
const term = new Terminal({
  fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
  fontSize: 13,
  theme: { background: "#030b11", foreground: "#cfe3ff" },
  cursorBlink: true,
  cursorStyle: "bar",
  allowTransparency: true,
  scrollback: 5000,
});
term.loadAddon(fitAddon);
term.open(terminalElement);

async function fitTerminal() {
  try {
    fitAddon.fit();
    await invoke("async_resize_pty", {
      rows: term.rows,
      cols: term.cols,
    });
  } catch {
    // ignore
  }
}

async function writeToTerminal(data: string) {
  await new Promise<void>((resolve, reject) => {
    term.write(data, () => {
      try {
        resolve();
      } catch (e) {
        reject(e);
      }
    });
  });
}

function writeToPty(data: string) {
  invoke("async_write_to_pty", { data }).catch(() => {});
}

async function initShell() {
  try {
    await invoke("async_create_shell");
  } catch {
    // linux can emit non-fatal permission errors here
  }
}

void initShell();

term.onData(writeToPty);
addEventListener("resize", fitTerminal);
fitTerminal();

async function readFromPty() {
  try {
    const data = await invoke<string>("async_read_from_pty");
    if (typeof data === "string" && data.length > 0) {
      await writeToTerminal(data);
    }
  } catch {
    // ignore read errors so polling can continue
  }

  window.requestAnimationFrame(readFromPty);
}

window.requestAnimationFrame(readFromPty);
