/// <reference types="vitest/config" />
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// The app is served under http://<pi>/flashcards/, not /, so built asset
// URLs must carry that prefix.
export default defineConfig({
  plugins: [react()],
  base: "/flashcards/",
  server: {
    // Dev only: proxy API calls to the local backend so the client uses
    // the same "/flashcards/api/..." paths it will use in production.
    proxy: {
      "/flashcards/api": {
        target: "http://127.0.0.1:8081",
        changeOrigin: true,
        rewrite: (p) => p.replace(/^\/flashcards\/api/, ""),
      },
    },
  },
  test: {
    environment: "jsdom",
    globals: false,
    setupFiles: ["./src/test/setup.ts"],
  },
});
