// The first day of a shop, as one test (plan automated-qa-rounds-for-the-shop,
// round 1): Samir's 2026-09-24 hand walk replayed from an empty database,
// first setup included. It cannot live in playwright.config.ts: that run's
// globalSetup writes the owner's credential before any spec, which shuts
// the first-setup door this test has to walk through, and its one shared
// SQLite file expects an empty catalogue in products.spec.ts, which three
// products seeded here would break. So this config has its own database,
// no globalSetup, and its own pair of ports beside the main run's.
//
// `just e2e-first-day` runs it in fr then ar. DZPOS_E2E_API_BIN, when set,
// runs that binary instead of building one, so a session that must not run
// cargo (the automation loop) can drive a copied build.

import { defineConfig, devices } from "@playwright/test";
import { randomBytes } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

const desktopDir = fileURLToPath(new URL(".", import.meta.url));
const repoRoot = path.resolve(desktopDir, "..", "..");
const artifactsDir = path.join(desktopDir, "e2e", ".artifacts", "first-day");
const tempDb = path.join(artifactsDir, "first-day.db");
const tempBackups = path.join(artifactsDir, "backups");

function port(name: string, fallback: number): number {
  const raw = process.env[name];
  const value = raw === undefined || raw === "" ? fallback : Number(raw);
  if (!Number.isInteger(value) || value < 1024 || value > 65535) {
    throw new Error(`${name} must be a port between 1024 and 65535, got ${raw}`);
  }
  return value;
}
// e2e/api.ts reads DZPOS_E2E_API_PORT to reach the API from `request`, so
// the fallback is written back into the environment the workers inherit.
const apiPort = port("DZPOS_E2E_API_PORT", 4321);
process.env.DZPOS_E2E_API_PORT = String(apiPort);
const webPort = port("DZPOS_E2E_WEB_PORT", 5176);
const apiUrl = `http://127.0.0.1:${apiPort}`;
const baseURL = `http://127.0.0.1:${webPort}`;

// Minted once by the runner and inherited by every worker, as in
// playwright.config.ts.
const launchToken = process.env.DZPOS_E2E_TOKEN ?? randomBytes(32).toString("hex");
process.env.DZPOS_E2E_TOKEN = launchToken;

const home = process.env.HOME ?? "";
const cargoEnv = {
  PATH: `${path.join(home, ".cargo", "bin")}:${process.env.PATH ?? ""}`,
  CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR ?? path.join(repoRoot, "target"),
  CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS ?? "4",
};

const apiArgs = `--db "${tempDb}" --port ${apiPort} --allow-origin ${baseURL}`;
const prebuilt = process.env.DZPOS_E2E_API_BIN;
const startApi =
  prebuilt === undefined || prebuilt === ""
    ? `mkdir -p "$CARGO_TARGET_DIR" && flock "$CARGO_TARGET_DIR/.lock" cargo build -p dzpos-api && exec "$CARGO_TARGET_DIR/debug/dzpos-api" ${apiArgs}`
    : `exec "${prebuilt}" ${apiArgs}`;

const LOCALE = { fr: "fr-FR", ar: "ar-DZ" } as const;

function storageStateFor(lang: keyof typeof LOCALE) {
  return {
    cookies: [],
    origins: [{ origin: baseURL, localStorage: [{ name: "dzpos-lang", value: lang }] }],
  };
}

export default defineConfig({
  testDir: path.join(desktopDir, "e2e", "first-day"),
  outputDir: path.join(artifactsDir, "test-results"),
  reporter: [["list"]],
  fullyParallel: false,
  workers: 1,
  retries: 0,
  forbidOnly: !!process.env.CI,
  // One test walks a whole day; each step is its own test.step.
  timeout: 300_000,
  expect: { timeout: 10_000 },
  use: {
    baseURL,
    headless: true,
    viewport: { width: 1280, height: 800 },
    trace: "retain-on-failure",
  },
  // One project per invocation (`just e2e-first-day` loops), since both
  // would share the one database and the first-setup door opens once.
  projects: (["fr", "ar"] as const).map((lang) => ({
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
      command: `rm -rf "${artifactsDir}/first-day.db"* "${tempBackups}" && mkdir -p "${artifactsDir}" && ${startApi}`,
      cwd: repoRoot,
      env: { ...cargoEnv, DZPOS_API_TOKEN: launchToken },
      url: `${apiUrl}/health`,
      reuseExistingServer: false,
      timeout: 600_000,
      stdout: "pipe",
      stderr: "pipe",
    },
    {
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
