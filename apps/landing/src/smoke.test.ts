// The pattern this borrows from apps/desktop/src/theme.test.ts: scan the
// source tree for a banned pattern instead of trusting a reviewer to catch
// it by eye. Two things are asserted here that theme.test.ts does not need
// to, because this package ships no theme switcher and no committed
// generated file: the three routes actually render, and the Arabic one is
// RTL (context/plans/20260911-landing-page.md, L0).

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const SRC = join(ROOT, "src");

const sourceFiles = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(full);
    return /\.(ts|tsx|astro|css)$/.test(entry.name) ? [full] : [];
  });

const grep = (files: readonly string[], pattern: RegExp): string[] =>
  files.flatMap((file) =>
    readFileSync(file, "utf8")
      .split("\n")
      .map((text, index) => ({ file, line: index + 1, text }))
      .filter((row) => pattern.test(row.text))
      .map((row) => `${row.file}:${row.line}: ${row.text.trim()}`),
  );

describe("no hardcoded hex or theme-switch outside the token import", () => {
  // src/lib/tokens.ts is the generator; it never spells a hex literal
  // itself, only reading @dzpos/design's already-resolved values at
  // runtime, so it needs no exemption here. src/styles/tokens.css is the
  // generated file the rule is about, is gitignored, and may or may not
  // exist on disk depending on whether `astro dev`/`build` already ran.
  // This file is excluded too: it has to spell the pattern it bans, the
  // same reason apps/desktop/src/theme.test.ts exempts itself.
  const files = sourceFiles(SRC).filter(
    (file) => !file.endsWith(join("styles", "tokens.css")) && file !== fileURLToPath(import.meta.url),
  );

  it("has no literal hex colour", () => {
    expect(grep(files, /#[0-9a-fA-F]{3,8}\b/)).toEqual([]);
  });

  it("sets data-theme nowhere: one theme, no switcher, nothing to select", () => {
    expect(grep(files, /data-theme=|dataset\.theme/)).toEqual([]);
  });
});

describe("the three routes render", () => {
  let outDir: string;
  let build: { code: number; output: string };

  beforeAll(() => {
    // Under node_modules (gitignored, same filesystem as the project) rather
    // than os.tmpdir(): Astro moves prerendered assets into outDir with a
    // bare fs.rename, and the WSL box mounts /tmp on a different device from
    // this worktree, which turns that rename into EXDEV.
    outDir = mkdtempSync(join(ROOT, "node_modules", ".smoke-"));
    const astroBin = join(ROOT, "node_modules", ".bin", process.platform === "win32" ? "astro.cmd" : "astro");
    try {
      const output = execFileSync(astroBin, ["build", "--outDir", outDir], {
        cwd: ROOT,
        encoding: "utf8",
      });
      build = { code: 0, output };
    } catch (error) {
      const output = error instanceof Error ? error.message : String(error);
      build = { code: 1, output };
    }
  });

  afterAll(() => {
    rmSync(outDir, { recursive: true, force: true });
  });

  const read = (path: string): string => readFileSync(join(outDir, path), "utf8");

  it("builds without error", () => {
    expect(build.code, build.output).toBe(0);
  });

  it("French at / is ltr", () => {
    const html = read("index.html");
    expect(html).toContain('lang="fr"');
    expect(html).not.toContain('dir="rtl"');
  });

  it("English at /en", () => {
    const html = read(join("en", "index.html"));
    expect(html).toContain('lang="en"');
  });

  it("Arabic at /ar is rtl", () => {
    const html = read(join("ar", "index.html"));
    expect(html).toContain('lang="ar"');
    expect(html).toContain('dir="rtl"');
  });

  const ROUTES = ["index.html", join("en", "index.html"), join("ar", "index.html")];

  describe("every <img> ships explicit width and height (L4)", () => {
    // An image with no intrinsic size reserved in the markup lets the
    // browser lay it out at zero height until the file arrives, which is
    // exactly the layout shift a Lighthouse mobile pass marks down (CLS);
    // src/lib/shots.json carries the real pixel size for every shot for
    // this reason. Checked against the built HTML, not the source
    // components, so this catches a width/height dropped anywhere in the
    // chain, not only in a .astro file this test happens to read.
    it.each(ROUTES)("%s has no <img> missing width or height", (file) => {
      const html = read(file);
      const imgs = [...html.matchAll(/<img\b[^>]*>/g)].map((m) => m[0]);
      const missing = imgs.filter((tag) => !/\bwidth="\d+"/.test(tag) || !/\bheight="\d+"/.test(tag));
      expect(
        missing,
        `${file} ships ${missing.length} <img> tag(s) with no width/height attribute:\n${missing.join("\n")}`,
      ).toEqual([]);
    });
  });

  describe("page weight stays under budget (L4)", () => {
    // Today's heaviest route is /ar (real weight below); this budget is
    // that figure plus about 15% headroom, not a generic web-performance
    // number, so a real regression (an unshrunk screenshot, a font weight
    // nobody trims, a whole extra family) trips it well before it reaches
    // a visitor, while normal copy or shot changes still fit.
    // Measured 2026-09-11 with a real Chromium network capture (mobile
    // viewport, DPR 3) over the built routes: fr 532840 B, en 532567 B,
    // ar 602830 B -- ar includes one Latin woff2 (fonts-arabic.css: the
    // Western digits in fiscal.tva) that is declared but not preloaded, so
    // it is discovered from the inline @font-face rule, not a <link>; see
    // routeWeight below for how that is still counted.
    const BUDGET_BYTES = 694_272; // 678 KiB, ~15% over ar's 602830 B

    /**
     * The bytes a real browser downloads for this route: the document
     * itself, every stylesheet, every font a route's CSS declares via
     * @font-face whether preloaded or not (fontsource sets no
     * unicode-range, so the browser cannot rule out needing a declared
     * fallback face from the CSS alone and fetches it the moment a text run
     * falls through to it -- the ar route's fiscal digits do this for its
     * one Latin weight; see fonts-arabic.css), and for each <img> only the
     * heaviest srcset candidate (the 2x file a high-DPR phone actually
     * fetches; the plain `src=` on the same tag is the same 1x file
     * srcset's 1x descriptor already lists, so it is not double-counted).
     * Verified against a real Chromium run over these exact routes before
     * this budget was set (mobile viewport, DPR 3): the totals matched byte
     * for byte.
     */
    const routeWeight = (file: string): { total: number; assets: readonly { url: string; bytes: number }[] } => {
      const html = read(file);
      const assetUrls = new Set<string>([file]);

      for (const m of html.matchAll(/href="([^"]+\.(?:css|woff2?))"/g)) assetUrls.add(m[1]);
      // @font-face src urls: today Astro inlines every route's small CSS
      // into a <style> block in <head>, so this all lives in `html`, but
      // that is a build detail (astro.config.ts's inlineStylesheets
      // threshold, or just more/heavier fonts) that could push a face's
      // rule into the external `fonts.*.css` chunk instead -- so this scans
      // that file's text too, not only the inline markup. Only .woff2:
      // every @font-face here lists a .woff fallback after it in the same
      // src list, and no browser this page targets (or Lighthouse mobile
      // emulates) ever needs that second, unsupported-format entry.
      const cssHrefs = [...html.matchAll(/href="([^"]+\.css)"/g)].map((m) => m[1]);
      const cssText = cssHrefs.map((href) => read(href)).join("\n");
      for (const m of (html + cssText).matchAll(/url\(([^)"]+\.woff2)\)/g)) assetUrls.add(m[1]);

      const dropped1x = new Set<string>();
      for (const m of html.matchAll(/srcset="([^"]+)"/g)) {
        const candidates = m[1].split(",").map((s) => s.trim().split(" "));
        const has2x = candidates.some(([, density]) => density === "2x");
        for (const [url, density] of candidates) {
          if (has2x && density === "1x") {
            dropped1x.add(url);
            continue;
          }
          assetUrls.add(url);
        }
      }
      for (const m of html.matchAll(/\ssrc="(\/shots\/[^"]+)"/g)) {
        if (!dropped1x.has(m[1])) assetUrls.add(m[1]);
      }

      const assets = [...assetUrls].map((url) => ({
        url,
        bytes: statSync(join(outDir, url.split("?")[0] ?? url)).size,
      }));
      return { total: assets.reduce((sum, a) => sum + a.bytes, 0), assets };
    };

    it.each(ROUTES)("%s stays under the 678 KiB budget", (file) => {
      const { total, assets } = routeWeight(file);
      const heaviest = [...assets]
        .sort((a, b) => b.bytes - a.bytes)
        .slice(0, 5)
        .map((a) => `${a.bytes}B ${a.url}`)
        .join("\n  ");
      expect(
        total,
        `${file} weighs ${total} bytes, over the ${BUDGET_BYTES} byte budget. Heaviest assets:\n  ${heaviest}`,
      ).toBeLessThanOrEqual(BUDGET_BYTES);
    });
  });
});
