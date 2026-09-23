// The product shots are generated, then committed (L1: the page must never
// wait on a build to show a screen). `pnpm run shots`
// (DZPOS_SHOTS_WRITE=1 vitest run tests/shots.test.ts) writes public/shots/
// and src/lib/shots.json fresh from the desktop e2e's committed
// screenshots; running this file plainly, the way `pnpm test` and the
// gates do, instead asserts the committed files still match what
// buildShots() produces right now, so a screenshot that changed without a
// rerun of the script fails here rather than shipping stale art.
//
// The same shape as @dzpos/design's tests/themeCss.test.ts.

import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { buildShots, RETINA_FACTOR, type Locale, type ShotsManifest } from "../src/lib/shots";

const PUBLIC_SHOTS = fileURLToPath(new URL("../public/shots/", import.meta.url));
const MANIFEST_PATH = fileURLToPath(new URL("../src/lib/shots.json", import.meta.url));

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
    "%s/%s's every source, and its src fallback, points at a file this run also produced",
    (key, locale) => {
      const files = new Set(built.files.map((f) => f.file));
      const entry = built.manifest[key][locale];
      expect(entry.sources.length).toBeGreaterThan(0);
      for (const source of entry.sources) {
        expect(files.has(source.url)).toBe(true);
      }
      expect(files.has(entry.src)).toBe(true);
    },
  );

  it.each(KEYS.flatMap((key) => LOCALES.map((locale) => [key, locale] as const)))(
    "%s/%s's sources are ascending by width, and src is the smallest",
    (key, locale) => {
      const entry = built.manifest[key][locale];
      const widths = entry.sources.map((source) => source.width);
      expect(widths).toEqual([...widths].sort((a, b) => a - b));
      expect(entry.src).toBe(entry.sources[0]?.url);
      // entry.width is CSS pixels (the width/height attributes); it comes
      // from one of the rendered sources (the smallest, unless a spec
      // overrides attrWidthIndex -- see shots.ts), so RETINA_FACTOR times
      // it must land on exactly one of the source widths.
      expect(entry.sources.map((source) => source.width)).toContain(entry.width * RETINA_FACTOR);
    },
  );

  it.each(KEYS.flatMap((key) => LOCALES.map((locale) => [key, locale] as const)))(
    "%s/%s's sources never get lighter as they get wider",
    (key, locale) => {
      // The hero once shipped a 684w tile at 58490 B, heavier than the
      // 1344w tile above it (42592 B) -- lossless webp storing a rescale's
      // resampling noise pixel for pixel, on a tier that was supposed to
      // save a phone bytes. A wider tier that is never lighter than a
      // narrower one is what a `w`-descriptor srcset needs to be worth
      // shipping at all: a browser choosing between two candidates should
      // never find the bigger number is also the smaller download.
      const entry = built.manifest[key][locale];
      const byFile = new Map(built.files.map((f) => [f.file, f.bytes.length]));
      const sized = entry.sources.map((source) => {
        const bytes = byFile.get(source.url);
        if (bytes === undefined) {
          throw new Error(`${source.url}: not among the files this run produced`);
        }
        return { url: source.url, width: source.width, bytes };
      });
      for (let i = 1; i < sized.length; i += 1) {
        const narrower = sized[i - 1];
        const wider = sized[i];
        if (narrower === undefined || wider === undefined) continue;
        expect(
          wider.bytes,
          `${wider.url} (${wider.width}w, ${wider.bytes} B) is lighter than ` +
            `${narrower.url} (${narrower.width}w, ${narrower.bytes} B)`,
        ).toBeGreaterThanOrEqual(narrower.bytes);
      }
    },
  );

  it.each(KEYS.flatMap((key) => LOCALES.map((locale) => [key, locale] as const)))(
    "%s/%s's sizes attribute is not empty",
    (key, locale) => {
      expect(built.manifest[key][locale].sizes.length).toBeGreaterThan(0);
    },
  );
});
