import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  // Tauri dev server listens on port 1420 by default.
  server: {
    port: 1420,
    strictPort: true,
  },
  // Produce a relative base so Tauri's WebView can load the built assets.
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
