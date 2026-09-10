import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["test/**/*.test.ts"],
    environment: "node",
    // Deterministic: UI tests construct their own jsdom instances.
    isolate: true,
  },
});
