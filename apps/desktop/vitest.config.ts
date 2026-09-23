/// <reference types="vitest/config" />
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import path from "path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["tests/**/*.test.{ts,tsx}"],
    globals: true,
    // Algeria never observes daylight saving (UTC+1 year round), and it is
    // never UTC itself: pinned so a helper that quietly reads a `Date`'s
    // UTC getters instead of its local ones (`getUTCHours` for `getHours`)
    // fails here the same way it would on a shop's own machine, rather than
    // passing by coincidence on a CI box already set to UTC.
    env: { TZ: "Africa/Algiers" },
  },
});
