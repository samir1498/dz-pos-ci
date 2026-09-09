// Headless browser run of the real stack: the axum API over a throwaway
// SQLite file, Vite serving the desktop UI, chromium driving it. Nothing
// here is mocked, so a green run means the HTTP contract in
// packages/shared still matches what crates/api answers.
//
// ObserveOne is the recorded e2e tool for this product (decision
// 2026-09-08); this config is the interim local driver.

import { defineConfig, devices } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";

// `new URL(".", ...)` is already the directory; do not take its dirname.
const desktopDir = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = path.resolve(desktopDir, "..", "..");
const artifactsDir = path.join(desktopDir, "e2e", ".artifacts");

/** Deleted before every run, so the first screen is always the empty state. */
const tempDb = path.join(artifactsDir, "e2e.db");

// Fixed ports, deliberately beside the dev ones (4317 API, 5173 Vite) so a
// running `just api` / `just dev` pair does not collide with a test run.
const apiPort = 4319;
const webPort = 5174;
const apiUrl = `http://127.0.0.1:${apiPort}`;
const baseURL = `http://127.0.0.1:${webPort}`;

const home = process.env.HOME ?? "";
const cargoEnv = {
  // Non-login shells on the WSL box do not have ~/.cargo/bin on PATH.
  PATH: `${path.join(home, ".cargo", "bin")}:${process.env.PATH ?? ""}`,
  CARGO_TARGET_DIR: path.join(repoRoot, "target"),
  CARGO_BUILD_JOBS: "4",
};

export default defineConfig({
  testDir: path.join(desktopDir, "e2e"),
  outputDir: path.join(artifactsDir, "test-results"),
  reporter: [["list"]],
  // One browser, one worker: the two tests share one SQLite file.
  fullyParallel: false,
  workers: 1,
  retries: 0,
  forbidOnly: !!process.env.CI,
  timeout: 30_000,
  expect: { timeout: 10_000 },

  use: {
    baseURL,
    headless: true,
    viewport: { width: 1280, height: 800 },
    trace: "retain-on-failure",
  },

  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 800 } },
    },
  ],

  webServer: [
    {
      // The db file is removed first, journal siblings included, so a run
      // never inherits rows from the previous one.
      // The API names its allowed origins (the dev Vite port and the Tauri
      // ones); the test Vite runs on another port, so it is passed in the
      // way the SSH case is: one extra origin on the command line.
      command: `rm -f "${tempDb}" "${tempDb}-shm" "${tempDb}-wal" && cargo run -p dzpos-api -- --db "${tempDb}" --port ${apiPort} --allow-origin ${baseURL}`,
      cwd: repoRoot,
      env: cargoEnv,
      url: `${apiUrl}/health`,
      reuseExistingServer: false,
      // A cold cargo build takes minutes on this box.
      timeout: 600_000,
      stdout: "pipe",
      stderr: "pipe",
    },
    {
      // The app reads VITE_API_URL (apps/desktop/src/api.ts); passing it
      // here keeps the test API out of the app's source.
      command: `pnpm exec vite --port ${webPort} --strictPort`,
      cwd: desktopDir,
      env: { VITE_API_URL: apiUrl },
      url: baseURL,
      reuseExistingServer: false,
      timeout: 120_000,
      stdout: "pipe",
      stderr: "pipe",
    },
  ],
});
