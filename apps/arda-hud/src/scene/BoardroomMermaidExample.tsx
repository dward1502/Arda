/**
 * BoardroomMermaidExample.tsx
 *
 * Minimal end-to-end wiring example: shows how a boardroom scene slot uses
 * MermaidMonitorSurface + useMonitorFocusWindow together. This is the proof
 * point for today's goal — click the in-scene monitor, it opens a native
 * window with the same diagram full-size, draggable to any physical display.
 *
 * Not meant to be dropped in verbatim — wire `diagramSource` to your real
 * PROMETHEUS/APOLLO plan-graph state instead of the placeholder string.
 */

import { useState } from "react";
import { MermaidMonitorSurface } from "./MermaidMonitorSurface";
import { useMonitorFocusWindow } from "./useMonitorFocusWindow";

const PLACEHOLDER_PLAN_DIAGRAM = `
graph TD
  A[Objective: refresh CoverCo lead intake] --> B{Governance gate}
  B -->|approved| C[APOLLO: draft plan]
  B -->|denied| Z[Return to operator]
  C --> D[Task: scrape current form schema]
  C --> E[Task: generate updated React form]
  D --> F[CHARON: route to local model]
  E --> F
  F --> G[Review gate: human approval]
  G --> H[Execute]
`.trim();

export function BoardroomMermaidExample() {
  const [diagramSource] = useState(PLACEHOLDER_PLAN_DIAGRAM);
  const { openFocusWindow } = useMonitorFocusWindow();

  const handleFocusRequest = (slotId: string) => {
    void openFocusWindow({
      slotId,
      contentKind: "mermaid_diagram",
      payload: {
        diagramSource: encodeURIComponent(diagramSource),
      },
    });
  };

  return (
    <MermaidMonitorSurface
      slotId="monitor_3"
      diagramSource={diagramSource}
      position={[-1.2, 1.4, -2.0]}
      onFocusRequest={handleFocusRequest}
    />
  );
}
