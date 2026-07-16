// sigil: REPAIR
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: "./src/test/setup.ts",
  },
  build: {
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (!id.includes("node_modules")) {
            return;
          }
          if (id.includes("/node_modules/three/examples/")) {
            return "vendor-three-examples";
          }
          if (id.includes("/node_modules/three/src/renderers/")) {
            return "vendor-three-renderers";
          }
          if (id.includes("/node_modules/three/src/math/")) {
            return "vendor-three-math";
          }
          if (id.includes("/node_modules/three/src/geometries/")) {
            return "vendor-three-geometry";
          }
          if (id.includes("/node_modules/three/src/materials/")) {
            return "vendor-three-materials";
          }
          if (id.includes("/node_modules/three/src/core/")) {
            return "vendor-three-core-internal";
          }
          if (id.includes("/node_modules/three/")) {
            return "vendor-three-core";
          }
          if (id.includes("/node_modules/postprocessing/")) {
            return "vendor-three-post";
          }
          if (id.includes("@react-three/")) {
            return "vendor-three-react";
          }
          if (id.includes("@react-spring/")) {
            return "vendor-three-spring";
          }
          if (id.includes("@tauri-apps/")) {
            return "vendor-tauri";
          }
          if (
            id.includes("/node_modules/react/") ||
            id.includes("/node_modules/react-dom/") ||
            id.includes("/node_modules/scheduler/")
          ) {
            return "vendor-react";
          }
          if (id.includes("/node_modules/zustand/")) {
            return "vendor-store";
          }
          return;
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1421,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1422,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
