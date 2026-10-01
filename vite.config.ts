import { defineConfig } from "vite";
import { resolve } from "path";

export default defineConfig({
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2022",
    rollupOptions: {
      input: {
        main: resolve(__dirname, "index.html"),
        overlay: resolve(__dirname, "overlay.html"),
        recordbar: resolve(__dirname, "recordbar.html"),
        pin: resolve(__dirname, "pin.html"),
        recframe: resolve(__dirname, "recframe.html"),
      },
    },
  },
});
