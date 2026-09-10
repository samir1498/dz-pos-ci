// Headless browser run of the real stack: the axum API over a throwaway
// SQLite file, Vite serving the desktop UI, chromium driving it. Nothing
// here is mocked, so a green run means the HTTP contract in
// packages/shared still matches what crates/api answers.
//
// ObserveOne is the recorded e2e tool for this product (decision
// 2026-09-08); this config is the interim local driver.

import { defineConfig, devices } from "@playwright/test";
import { randomBytes } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

// `new URL(".", ...)` is already the directory; do not take its dirname.
const desktopDir = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = path.resolve(desktopDir, "..", "..");
const artifactsDir = path.join(desktopDir, "e2e", ".artifacts");

/** Deleted before every run, so the first screen is always the empty state. */
const tempDb = path.join(artifactsDir, "e2e.db");
/** The API puts its copies beside the database. Deleted with it: the backups
 * spec starts from "no copy at all", and a folder left by the last run would
 * make that first assertion pass or fail on history. */
const tempBackups = path.join(artifactsDir, "backups");

// Ports beside the dev ones (4317 API, 5173 Vite) so a running `just api`
// / `just dev` pair does not collide with a test run. Two checkouts on one
// box (a worktree per task) each pass their own pair.
function port(name: string, fallback: number): number {
  const raw = process.env[name];
  const value = raw === undefined || raw === "" ? fallback : Number(raw);
  if (!Number.isInteger(value) || value < 1024 || value > 65535) {
    throw new Error(`${name} must be a port between 1024 and 65535, got ${raw}`);
  }
  return value;
}
const apiPort = port("DZPOS_E2E_API_PORT", 4319);
const webPort = port("DZPOS_E2E_WEB_PORT", 5174);
const apiUrl = `http://127.0.0.1:${apiPort}`;
const baseURL = `http://127.0.0.1:${webPort}`;

// One project per UI language. Each sets `dzpos-lang` in localStorage
// through `storageState` before the app's first script runs, the way
// I18nProvider reads it (src/i18n/index.tsx, `initialLang`), and a browser
// `locale` matching it so the OS-level bits (date pickers, number input
// spinners) agree with the page. All three share the one webServer pair
// and its one SQLite file below, so `just e2e` runs one Playwright
// invocation per project rather than passing all three here: three
// projects sharing a single run would share the one database too, and the
// products suite's first assertion needs an empty table, which only the
// first project to touch it would still have.
const LANGS = ["fr", "en", "ar"] as const;
type LangCode = (typeof LANGS)[number];
const LOCALE: Record<LangCode, string> = { fr: "fr-FR", en: "en-US", ar: "ar-DZ" };

function storageStateFor(lang: LangCode) {
  return {
    cookies: [],
    origins: [{ origin: baseURL, localStorage: [{ name: "dzpos-lang", value: lang }] }],
  };
}

// The API refuses every call that does not show its launch token; the
// desktop makes one per launch, this run makes one per suite and hands it
// to both servers, the way the Tauri process hands it to its webview.
//
// It goes through the environment rather than staying a module constant
// because a worker re-imports this file in its own process: a bare
// `randomBytes` here would give each worker a different token from the one
// the API was started with, and a spec that seeds a row through `request`
// would be refused. The runner is the first to load this file, so it is the
// one that mints the token; every worker it spawns inherits the variable and
// takes that branch. `e2e/api.ts` reads the same variable.
const launchToken = process.env.DZPOS_E2E_TOKEN ?? randomBytes(32).toString("hex");
process.env.DZPOS_E2E_TOKEN = launchToken;

const home = process.env.HOME ?? "";
const cargoEnv = {
  // Non-login shells on the WSL box do not have ~/.cargo/bin on PATH.
  PATH: `${path.join(home, ".cargo", "bin")}:${process.env.PATH ?? ""}`,
  // `just e2e` exports both (one shared build folder for every checkout,
  // two jobs); a bare `pnpm desktop e2e` outside just falls back to a
  // target/ of its own, which is the per-worktree folder the disk rules
  // forbid, so run it through just.
  CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR ?? path.join(repoRoot, "target"),
  CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? "4",
};

export default defineConfig({
  testDir: path.join(desktopDir, "e2e"),
  outputDir: path.join(artifactsDir, "test-results"),
  reporter: [["list"]],
  // One browser, one worker. Every spec in the run shares one API process
  // over one SQLite file, so two of them writing at once would each see
  // rows the other seeded; and the suites are ordered by filename on
  // purpose (the settings spec leaves the shop under the réel for the till
  // specs that follow it).
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

  projects: LANGS.map((lang) => ({
    name: lang,
    use: {
      ...devices["Desktop Chrome"],
      viewport: { width: 1280, height: 800 },
      locale: LOCALE[lang],
      storageState: storageStateFor(lang),
    },
  })),

  webServer: [
    {
      // The db file is removed first, journal siblings included, so a run
      // never inherits rows from the previous one.
      // The API names its allowed origins (the dev Vite port and the Tauri
      // ones); the test Vite runs on another port, so it is passed in the
      // way the SSH case is: one extra origin on the command line.
      command: `rm -f "${tempDb}" "${tempDb}-shm" "${tempDb}-wal" "${tempDb}".before-restore-*.sqlite && rm -rf "${tempBackups}" && cargo run -p dzpos-api -- --db "${tempDb}" --port ${apiPort} --allow-origin ${baseURL}`,
      cwd: repoRoot,
      env: { ...cargoEnv, DZPOS_API_TOKEN: launchToken },
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
      env: { VITE_API_URL: apiUrl, VITE_API_TOKEN: launchToken },
      url: baseURL,
      reuseExistingServer: false,
      timeout: 120_000,
      stdout: "pipe",
      stderr: "pipe",
    },
  ],
});
