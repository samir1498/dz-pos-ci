import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    // `lib/` is plain TypeScript — money, the retry queue, what a status
    // code means. `screens/` joined for C7's own wiring
    // (`the-first-clinic-module-patients-queue-appointments`): whether the
    // prop a route wrapper passes reaches the redirect it should, with
    // every hook and every `react-native` import mocked away. Screen
    // *behaviour* stays Maestro's, against a real server: a mocked till
    // proves nothing about a till, and nothing under `tests/screens/`
    // claims to.
    include: ["tests/lib/**/*.test.ts", "tests/screens/**/*.test.{ts,tsx}"],
    globals: true,
  },
});
