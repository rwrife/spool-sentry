import { defineConfig } from "vite";

// Device-hosted static bundle. Keep the output small and dependency-free;
// firmware serves dist/ verbatim over local HTTP.
export default defineConfig({
  build: {
    target: "es2021",
    outDir: "dist",
    emptyOutDir: true,
    cssCodeSplit: false,
    modulePreload: false,
    rollupOptions: {
      output: {
        entryFileNames: "assets/app.js",
        assetFileNames: "assets/app.[ext]",
      },
    },
  },
});
