import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    // Only `lib/` is tested here: it is the part of the phone that is plain
    // TypeScript — money, the retry queue, what a status code means. The
    // screens are driven by Maestro against a real server instead, because
    // a mocked till proves nothing about a till.
    include: ["tests/lib/**/*.test.ts"],
    globals: true,
  },
});
