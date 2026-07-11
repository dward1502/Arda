import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import process from "node:process";
const host = process.env.TAURI_DEV_HOST || "0.0.0.0";
export default defineConfig(() => ({
    plugins: [react(), tailwindcss()],
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
        host,
        hmr: {
            protocol: "ws",
            host,
            port: 1421,
        },
        watch: {
            ignored: ["**/src-tauri/**"],
        },
    },
}));
