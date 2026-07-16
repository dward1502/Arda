/**
 * useMonitorFocusWindow.ts
 *
 * Handles the "click monitor -> pop to native OS window -> drag to another
 * physical display -> close to return in-scene" interaction described in the
 * boardroom slot contract (`focus.mode: native_window`).
 *
 * A Tauri WebviewWindow IS a real OS window once created, so "drag it to a
 * different physical monitor" is just normal OS window management — nothing
 * special required on our end for that part. This hook only owns the
 * create/focus/close lifecycle and keeps exactly one window per slot.
 *
 * Drop location (suggested): apps/arda-hud/src/components/arda/hooks/useMonitorFocusWindow.ts
 *
 * Requires (Cargo.toml / tauri.conf.json):
 *   - Tauri v2 multiwindow capability enabled for the relevant window labels,
 *     or use `create` permission scoped to a "monitor-focus-*" label prefix.
 */

import { useCallback, useRef } from "react";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";

export interface MonitorFocusContent {
  slotId: string;
  contentKind: "mermaid_diagram" | "hermes_terminal" | "diff_viewer" | "markdown_doc";
  /** Arbitrary payload the focused route will read on mount, e.g. diagram source,
   *  PTY session id, or file path. Kept as a flat record for URL/query encoding. */
  payload: Record<string, string>;
}

const WINDOW_LABEL_PREFIX = "arda-monitor-focus";

function windowLabelFor(slotId: string): string {
  // Tauri window labels must be simple identifiers — sanitize the slot id.
  const safe = slotId.replace(/[^a-zA-Z0-9_-]/g, "_");
  return `${WINDOW_LABEL_PREFIX}-${safe}`;
}

export function useMonitorFocusWindow() {
  // Track which slots currently have an open focus window, to avoid duplicates.
  const openWindows = useRef<Map<string, WebviewWindow>>(new Map());

  const openFocusWindow = useCallback(async (content: MonitorFocusContent) => {
    const label = windowLabelFor(content.slotId);
    const existing = openWindows.current.get(content.slotId);

    if (existing) {
      // Already open — bring it to front instead of creating a duplicate.
      await existing.setFocus();
      return existing;
    }

    const query = new URLSearchParams({
      slotId: content.slotId,
      contentKind: content.contentKind,
      ...content.payload,
    }).toString();

    const win = new WebviewWindow(label, {
      url: `/monitor-focus?${query}`,
      title: `ARDA — ${content.slotId}`,
      width: 1100,
      height: 720,
      minWidth: 480,
      minHeight: 320,
      resizable: true,
      // Deliberately NOT setting `alwaysOnTop` — the user may want it behind
      // their editor. Deliberately NOT setting fixed `x`/`y` — let the OS
      // place it, since the user is free to drag it to any physical monitor.
      decorations: true,
    });

    win.once("tauri://created", () => {
      openWindows.current.set(content.slotId, win);
    });

    win.once("tauri://error", (e) => {
      console.error(`ARDA monitor focus window failed for ${content.slotId}:`, e);
      openWindows.current.delete(content.slotId);
    });

    win.once("tauri://destroyed", () => {
      openWindows.current.delete(content.slotId);
    });

    return win;
  }, []);

  const closeFocusWindow = useCallback(async (slotId: string) => {
    const win = openWindows.current.get(slotId);
    if (win) {
      await win.close();
      openWindows.current.delete(slotId);
    }
  }, []);

  const isFocusWindowOpen = useCallback((slotId: string) => {
    return openWindows.current.has(slotId);
  }, []);

  return { openFocusWindow, closeFocusWindow, isFocusWindowOpen };
}
