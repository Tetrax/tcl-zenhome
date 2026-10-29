import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Port fixe : Tauri s'attend à http://localhost:1420 en dev.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
