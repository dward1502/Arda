/**
 * MermaidMonitorSurface.tsx
 *
 * R3F component for a single boardroom monitor slot showing a live mermaid
 * diagram. Handles:
 *   - rendering the diagram to an offscreen canvas
 *   - uploading that canvas as a Three.js CanvasTexture on the monitor mesh
 *   - click-to-expand: emits a focus request that the parent shell turns into
 *     either an in-app focused panel or a popped-out native Tauri window
 *
 * This component does NOT decide where geometry comes from — pass in the
 * mesh's `width`/`height` (matching your monitor GLB's screen-face UVs) and
 * the `slotId` from ARDA_BOARDROOM_SLOT_ASSIGNMENT_CONTRACT.md.
 *
 * Drop location (suggested): apps/arda-hud/src/scene/monitors/MermaidMonitorSurface.tsx
 */

import { useEffect, useMemo, useRef, useState } from "react";
import { useFrame } from "@react-three/fiber";
import * as THREE from "three";
import {
  MermaidCanvasRenderer,
  ARDA_BOARDROOM_MERMAID_THEME,
  type MermaidCanvasTheme,
} from "./mermaidCanvasRenderer";

export interface MermaidMonitorSurfaceProps {
  /** Stable scene slot id, e.g. "monitor_left_3" — see boardroom slot contract */
  slotId: string;
  /** Mermaid diagram source. Update this prop as the agent's plan/graph changes. */
  diagramSource: string;
  /** Physical plane size in scene units, matched to the monitor mesh screen face */
  width?: number;
  height?: number;
  /** Backing canvas resolution — higher = crisper text, more GPU upload cost */
  resolutionX?: number;
  resolutionY?: number;
  theme?: MermaidCanvasTheme;
  /** Called when the user clicks the monitor. Parent decides: in-scene focus
   *  panel vs. popped-out native Tauri window. This component only reports intent. */
  onFocusRequest?: (slotId: string) => void;
  /** Position/rotation passed straight through to the underlying mesh */
  position?: [number, number, number];
  rotation?: [number, number, number];
}

export function MermaidMonitorSurface({
  slotId,
  diagramSource,
  width = 1.6,
  height = 0.9,
  resolutionX = 1024,
  resolutionY = 576,
  theme = ARDA_BOARDROOM_MERMAID_THEME,
  onFocusRequest,
  position = [0, 0, 0],
  rotation = [0, 0, 0],
}: MermaidMonitorSurfaceProps) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const rendererRef = useRef<MermaidCanvasRenderer | null>(null);
  const textureRef = useRef<THREE.CanvasTexture | null>(null);
  const [hovered, setHovered] = useState(false);
  const [renderError, setRenderError] = useState<string | null>(null);
  const meshRef = useRef<THREE.Mesh>(null);

  // Offscreen canvas + texture, created once per mount.
  useEffect(() => {
    const canvas = document.createElement("canvas");
    canvas.width = resolutionX;
    canvas.height = resolutionY;
    canvasRef.current = canvas;

    const renderer = new MermaidCanvasRenderer(canvas, theme);
    rendererRef.current = renderer;

    const texture = new THREE.CanvasTexture(canvas);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    textureRef.current = texture;

    return () => {
      renderer.dispose();
      texture.dispose();
      rendererRef.current = null;
      textureRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [resolutionX, resolutionY]);

  // Re-render whenever the diagram source changes.
  useEffect(() => {
    let cancelled = false;
    const renderer = rendererRef.current;
    if (!renderer) return;

    renderer.render(diagramSource).then((result) => {
      if (cancelled) return;
      if (!result.success) {
        setRenderError(result.error ?? "unknown mermaid render error");
      } else {
        setRenderError(null);
      }
      if (textureRef.current) {
        textureRef.current.needsUpdate = true;
      }
    });

    return () => {
      cancelled = true;
    };
  }, [diagramSource]);

  // Slight emissive pulse on hover so the monitor reads as clickable in-scene.
  useFrame(() => {
    const mesh = meshRef.current;
    if (!mesh) return;
    const mat = mesh.material as THREE.MeshBasicMaterial;
    const targetIntensity = hovered ? 1.0 : 0.85;
    if (mat && "opacity" in mat) {
      mat.opacity = THREE.MathUtils.lerp(mat.opacity, targetIntensity, 0.15);
    }
  });

  const geometryArgs = useMemo<[number, number]>(() => [width, height], [width, height]);

  return (
    <group position={position} rotation={rotation}>
      <mesh
        ref={meshRef}
        onPointerOver={(e) => {
          e.stopPropagation();
          setHovered(true);
          document.body.style.cursor = "pointer";
        }}
        onPointerOut={(e) => {
          e.stopPropagation();
          setHovered(false);
          document.body.style.cursor = "auto";
        }}
        onClick={(e) => {
          e.stopPropagation();
          onFocusRequest?.(slotId);
        }}
        userData={{ slotId, adapterType: "component_grid", contentKind: "mermaid_diagram" }}
      >
        <planeGeometry args={geometryArgs} />
        <meshBasicMaterial
          map={textureRef.current ?? undefined}
          transparent
          opacity={0.85}
          toneMapped={false}
        />
      </mesh>

      {/* Thin reactive bezel edge — cyan normally, amber if the last render errored */}
      <lineSegments position={[0, 0, 0.001]}>
        <edgesGeometry args={[new THREE.PlaneGeometry(width, height)]} />
        <lineBasicMaterial color={renderError ? "#e0a83d" : "#3ddcd2"} />
      </lineSegments>
    </group>
  );
}
