/**
 * mermaidCanvasRenderer.ts
 *
 * Renders a Mermaid diagram source string to an offscreen HTMLCanvasElement.
 * This is the content-adapter layer for the `agent_activity` / `component_grid`
 * monitor slot — it has no knowledge of Three.js or React. It just turns a
 * mermaid string into pixels on a canvas you already own.
 *
 * Drop location (suggested): apps/arda-hud/src/scene/monitors/mermaidCanvasRenderer.ts
 *
 * Usage pattern:
 *   const renderer = new MermaidCanvasRenderer(canvas);
 *   await renderer.render(mermaidSource);
 *   // canvas now has pixels; caller marks their CanvasTexture needsUpdate = true
 */

import mermaid from "mermaid";

export interface MermaidRenderResult {
  success: boolean;
  error?: string;
  /** natural SVG size before scaling to canvas */
  svgWidth: number;
  svgHeight: number;
}

export interface MermaidCanvasTheme {
  background: string;
  lineColor: string;
  primaryColor: string;
  primaryTextColor: string;
  primaryBorderColor: string;
  fontFamily: string;
}

/** Matches the boardroom cyan/magenta/teal reactive palette by default. */
export const ARDA_BOARDROOM_MERMAID_THEME: MermaidCanvasTheme = {
  background: "#06090c",
  lineColor: "#3ddcd2",
  primaryColor: "#0e1b1f",
  primaryTextColor: "#bdf7f1",
  primaryBorderColor: "#3ddcd2",
  fontFamily: "ui-monospace, 'JetBrains Mono', 'Fira Code', monospace",
};

let mermaidInitialized = false;

function ensureMermaidInitialized(theme: MermaidCanvasTheme) {
  if (mermaidInitialized) return;
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: "strict",
    theme: "base",
    themeVariables: {
      background: theme.background,
      lineColor: theme.lineColor,
      primaryColor: theme.primaryColor,
      primaryTextColor: theme.primaryTextColor,
      primaryBorderColor: theme.primaryBorderColor,
      fontFamily: theme.fontFamily,
    },
  });
  mermaidInitialized = true;
}

export class MermaidCanvasRenderer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private theme: MermaidCanvasTheme;
  private renderIdCounter = 0;
  private lastSource: string | null = null;

  constructor(canvas: HTMLCanvasElement, theme: MermaidCanvasTheme = ARDA_BOARDROOM_MERMAID_THEME) {
    this.canvas = canvas;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("MermaidCanvasRenderer: canvas 2d context unavailable");
    }
    this.ctx = ctx;
    this.theme = theme;
    ensureMermaidInitialized(theme);
  }

  /**
   * Renders mermaid source to the canvas. Safe to call repeatedly with a new
   * source string (e.g. as an agent's plan graph grows). Skips re-render if
   * the source is unchanged from the last call.
   */
  async render(source: string): Promise<MermaidRenderResult> {
    if (source === this.lastSource) {
      return { success: true, svgWidth: this.canvas.width, svgHeight: this.canvas.height };
    }

    const renderId = `arda-mermaid-${this.renderIdCounter++}`;

    try {
      const { svg } = await mermaid.render(renderId, source);
      const result = await this.drawSvgStringToCanvas(svg);
      this.lastSource = source;
      return result;
    } catch (err) {
      this.drawErrorState(err instanceof Error ? err.message : String(err));
      return {
        success: false,
        error: err instanceof Error ? err.message : String(err),
        svgWidth: 0,
        svgHeight: 0,
      };
    }
  }

  private drawSvgStringToCanvas(svgString: string): Promise<MermaidRenderResult> {
    return new Promise((resolve, reject) => {
      const svgBlob = new Blob([svgString], { type: "image/svg+xml;charset=utf-8" });
      const url = URL.createObjectURL(svgBlob);
      const img = new Image();

      img.onload = () => {
        const { width: cw, height: ch } = this.canvas;
        this.ctx.fillStyle = this.theme.background;
        this.ctx.fillRect(0, 0, cw, ch);

        // Fit the diagram within the canvas, preserving aspect ratio, centered.
        const padding = 24;
        const availW = cw - padding * 2;
        const availH = ch - padding * 2;
        const scale = Math.min(availW / img.width, availH / img.height, 1);
        const drawW = img.width * scale;
        const drawH = img.height * scale;
        const dx = (cw - drawW) / 2;
        const dy = (ch - drawH) / 2;

        this.ctx.drawImage(img, dx, dy, drawW, drawH);
        URL.revokeObjectURL(url);
        resolve({ success: true, svgWidth: img.width, svgHeight: img.height });
      };

      img.onerror = () => {
        URL.revokeObjectURL(url);
        reject(new Error("Failed to rasterize mermaid SVG to canvas"));
      };

      img.src = url;
    });
  }

  private drawErrorState(message: string) {
    const { width: cw, height: ch } = this.canvas;
    this.ctx.fillStyle = this.theme.background;
    this.ctx.fillRect(0, 0, cw, ch);

    this.ctx.fillStyle = "#ff5d6c";
    this.ctx.font = `14px ${this.theme.fontFamily}`;
    this.ctx.textAlign = "center";
    this.ctx.textBaseline = "middle";

    const lines = this.wrapText(`diagram error: ${message}`, cw - 48);
    const lineHeight = 20;
    const startY = ch / 2 - (lines.length * lineHeight) / 2;
    lines.forEach((line, i) => {
      this.ctx.fillText(line, cw / 2, startY + i * lineHeight);
    });
  }

  private wrapText(text: string, maxWidth: number): string[] {
    const words = text.split(" ");
    const lines: string[] = [];
    let current = "";
    for (const word of words) {
      const test = current ? `${current} ${word}` : word;
      if (this.ctx.measureText(test).width > maxWidth && current) {
        lines.push(current);
        current = word;
      } else {
        current = test;
      }
    }
    if (current) lines.push(current);
    return lines;
  }

  dispose() {
    this.lastSource = null;
  }
}
