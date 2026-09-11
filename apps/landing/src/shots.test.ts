// The product shots are generated, then committed (L1: the page must never
// wait on a build to show a screen). `pnpm run shots`
// (DZPOS_SHOTS_WRITE=1 vitest run src/shots.test.ts) writes public/shots/
// and src/lib/shots.json fresh from the desktop e2e's committed
// screenshots; running this file plainly, the way `pnpm test` and the
// gates do, instead asserts the committed files still match what
// buildShots() produces right now, so a screenshot that changed without a
// rerun of the script fails here rather than shipping stale art.
//
// The same shape as @dzpos/design's src/themeCss.test.ts.

import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { buildShots, type Locale, type ShotsManifest } from "./lib/shots";

const PUBLIC_SHOTS = fileURLToPath(new URL("../public/shots/", import.meta.url));
const MANIFEST_PATH = fileURLToPath(new URL("./lib/shots.json", import.meta.url));

const built = await buildShots();
const manifestJson = `${JSON.stringify(built.manifest, null, 2)}\n`;

if (process.env.DZPOS_SHOTS_WRITE === "1") {
  mkdirSync(PUBLIC_SHOTS, { recursive: true });
  // Clear stale files first: a shot renamed or dropped by lib/shots.ts
  // should not leave its old bytes behind for the page to still find.
  for (const entry of readdirSync(PUBLIC_SHOTS, { withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith(".webp")) {
      rmSync(`${PUBLIC_SHOTS}${entry.name}`);
    }
  }
  for (const file of built.files) {
    writeFileSync(`${PUBLIC_SHOTS}${file.file}`, file.bytes);
  }
  writeFileSync(MANIFEST_PATH, manifestJson, "utf8");
}

describe("the committed shots", () => {
  it("is the manifest buildShots() produces right now", () => {
    expect(readFileSync(MANIFEST_PATH, "utf8")).toBe(manifestJson);
  });

  it.each(built.files)("$file matches the committed bytes", (file) => {
    const committed = readFileSync(`${PUBLIC_SHOTS}${file.file}`);
    expect(committed.equals(file.bytes), `${file.file} is stale; run pnpm run shots`).toBe(true);
  });

  it("commits no file buildShots() no longer produces", () => {
    const expected = new Set(built.files.map((f) => f.file));
    const onDisk = readdirSync(PUBLIC_SHOTS).filter((name) => name.endsWith(".webp"));
    expect(onDisk.filter((name) => !expected.has(name)).sort()).toEqual([]);
  });

  const KEYS: readonly (keyof ShotsManifest)[] = [
    "hero",
    "selling",
    "invoicing",
    "stock",
    "knowing",
    "collage",
  ];
  const LOCALES: readonly Locale[] = ["fr", "en", "ar"];

  it.each(KEYS.flatMap((key) => LOCALES.map((locale) => [key, locale] as const)))(
    "%s/%s points at a file this run also produced",
    (key, locale) => {
      const files = new Set(built.files.map((f) => f.file));
      const entry = built.manifest[key][locale];
      expect(files.has(entry.src1x)).toBe(true);
      expect(files.has(entry.src2x)).toBe(true);
    },
  );
});
