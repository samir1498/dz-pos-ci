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

import sharp from "sharp";
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

  // A real `astro build`, not an assertion: it runs in well under 4s alone,
  // but `astro build` is CPU- and IO-bound the same way `cargo build` is,
  // and this box runs both kinds side by side across worktrees with no
  // lock between them (context/processes/20260908-machines-and-heavy-jobs.md
  // -- the shared lock there is cargo's own `just claim`, which this build
  // never takes; the contention is the box's CPU and disk, not a lock this
  // build waits on). A cargo build running in another worktree at the same
  // time is enough to push this well past vitest's default hook timeout
  // (10s), which is exactly what happened: it timed out under `just test`
  // while another worktree was mid-build, and passed in 3.7s run alone. 30s
  // is comfortably over the longest contended run seen so far; trimming it
  // back to "what a build normally takes alone" reintroduces the same flake
  // on a busy box.
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
  }, 30_000);

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

  describe("the download section links all three installers (Download.astro)", () => {
    // Stable filenames from lib/site.ts: the build job renames each
    // installer before upload, so these URLs never carry a version.
    const ASSETS = ["Dinar-Setup.exe", "Dinar.dmg", "Dinar.AppImage"];
    it.each(ROUTES)("%s carries one button per OS with a stable URL", (file) => {
      const html = read(file);
      for (const asset of ASSETS) {
        expect(html).toContain(
          `https://github.com/Dinar-dz/dz-pos/releases/latest/download/${asset}`,
        );
      }
      for (const os of ["windows", "macos", "linux"]) {
        expect(html).toContain(`data-os="${os}"`);
      }
    });
  });

  describe("every <img> ships explicit width and height (L4)", () => {
    // An image with no intrinsic size reserved in the markup lets the
    // browser lay it out at zero height until the file arrives, which is
    // exactly the layout shift a Lighthouse mobile pass marks down (CLS);
    // src/lib/shots.json carries each shot's CSS pixel size (shots.ts's
    // toCssPixels) for this reason. Checked against the built HTML, not the source
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
    // viewport 390x844, DPR 3) over the built routes, after milestone 4
    // landed on main and regenerated the e2e screenshots these shots are
    // composed from (roles, PIN, permissions and the audit log -- more
    // chrome in several screens, hence heavier than the previous
    // measurement): fr 302388 B, en 302145 B, ar 400208 B -- ar includes
    // one Latin woff2 (fonts-arabic.css: the Western digits in
    // fiscal.tva) that is declared but not preloaded, so it is
    // discovered from the inline @font-face rule, not a <link>; see
    // routeWeight below for how that is still counted. DPR 3 is the
    // worst case this page has: the same Chromium capture at DPR 2 (the
    // common phone case) measured fr 210358 B, en 210115 B, ar 314578 B,
    // well under this budget already, and nothing this page serves picks
    // a heavier candidate than DPR 3 already does.
    const BUDGET_BYTES = 460_239; // ~449 KiB, ~15% over ar's 400208 B

    const VIEWPORT_WIDTH = 390;
    const DPR = 3;

    /**
     * `sizes` is a comma-separated list of `<media-condition> <length>`
     * entries with one bare `<length>` default at the end (the shapes
     * shots.ts's HERO_SIZES/PIECE_SIZES/SCREENS_SIZES actually use); this
     * evaluates it the way a browser does: first matching condition wins,
     * falling through to the default.
     */
    const evalLength = (length: string, viewportWidth: number): number => {
      const px = /^(-?\d+(?:\.\d+)?)px$/.exec(length);
      if (px?.[1] !== undefined) return Number(px[1]);
      const vwMinus = /^calc\(\s*100vw\s*-\s*(\d+(?:\.\d+)?)px\s*\)$/.exec(length);
      if (vwMinus?.[1] !== undefined) return viewportWidth - Number(vwMinus[1]);
      const halfVwMinus = /^calc\(\s*50vw\s*-\s*(\d+(?:\.\d+)?)px\s*\)$/.exec(length);
      if (halfVwMinus?.[1] !== undefined) return viewportWidth / 2 - Number(halfVwMinus[1]);
      throw new Error(`routeWeight: cannot evaluate sizes length "${length}"`);
    };

    const evalSizes = (sizes: string, viewportWidth: number): number => {
      for (const entry of sizes.split(",").map((s) => s.trim())) {
        const minWidth = /^\(min-width:\s*(\d+(?:\.\d+)?)px\)\s+(.+)$/.exec(entry);
        if (minWidth?.[1] !== undefined && minWidth[2] !== undefined) {
          if (viewportWidth >= Number(minWidth[1])) return evalLength(minWidth[2], viewportWidth);
          continue;
        }
        const maxWidth = /^\(max-width:\s*(\d+(?:\.\d+)?)px\)\s+(.+)$/.exec(entry);
        if (maxWidth?.[1] !== undefined && maxWidth[2] !== undefined) {
          if (viewportWidth <= Number(maxWidth[1])) return evalLength(maxWidth[2], viewportWidth);
          continue;
        }
        // No media condition: the trailing default entry always matches.
        return evalLength(entry, viewportWidth);
      }
      throw new Error(`routeWeight: sizes "${sizes}" matched nothing at viewport ${viewportWidth}`);
    };

    /**
     * The candidate a browser with a `sizes`-aware `srcset` (`w`
     * descriptors) actually fetches: the smallest source whose width is at
     * least the slot width times the device pixel ratio, or the largest
     * source if none is big enough. `src` is not a separate fallback here
     * -- every browser this page targets (and Lighthouse mobile emulates)
     * parses `w`-descriptor srcset and never falls back to it.
     */
    const pickCandidate = (srcset: string, sizes: string): string => {
      const slotWidth = evalSizes(sizes, VIEWPORT_WIDTH);
      const target = slotWidth * DPR;
      const candidates = srcset
        .split(",")
        .map((s) => s.trim().split(/\s+/))
        .map(([url, width]) => ({ url: url ?? "", width: Number((width ?? "").replace(/w$/, "")) }))
        .sort((a, b) => a.width - b.width);
      const picked = candidates.find((c) => c.width >= target) ?? candidates[candidates.length - 1];
      if (picked === undefined) {
        throw new Error(`routeWeight: srcset "${srcset}" has no candidates`);
      }
      return picked.url;
    };

    /**
     * The bytes a real browser downloads for this route: the document
     * itself, every stylesheet, every font a route's CSS declares via
     * @font-face whether preloaded or not (fontsource sets no
     * unicode-range, so the browser cannot rule out needing a declared
     * fallback face from the CSS alone and fetches it the moment a text run
     * falls through to it -- the ar route's fiscal digits do this for its
     * one Latin weight; see fonts-arabic.css), and for each <img> (and the
     * hero's matching <link rel=preload>) only the one srcset candidate a
     * 390 CSS px wide, DPR 3 phone actually picks, per pickCandidate above.
     * Verified against a real Chromium run over these exact routes at that
     * viewport and DPR before this budget was set: the totals matched byte
     * for byte, and each route fetched exactly one file per shot (the
     * preload and the <img> always agreed).
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

      // Every <img srcset=... sizes=...>, and the hero's <link rel=preload
      // as=image imagesrcset=... imagesizes=...>: "srcset=" and "sizes="
      // both occur as trailing substrings of "imagesrcset=" and
      // "imagesizes=", so one pattern matches the value in either
      // attribute name without needing to spell out both.
      for (const m of html.matchAll(/srcset="([^"]+)"[^>]*sizes="([^"]+)"/g)) {
        const [, srcset, sizes] = m;
        if (srcset !== undefined && sizes !== undefined) assetUrls.add(pickCandidate(srcset, sizes));
      }

      const assets = [...assetUrls].map((url) => ({
        url,
        bytes: statSync(join(outDir, url.split("?")[0] ?? url)).size,
      }));
      return { total: assets.reduce((sum, a) => sum + a.bytes, 0), assets };
    };

    it.each(ROUTES)("%s stays under the 436 KiB budget", (file) => {
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

  describe("the head carries canonical, alternate links and the card (L5)", () => {
    const ROUTE_CANONICAL: ReadonlyArray<readonly [file: string, canonical: string]> = [
      ["index.html", "https://dinar-landing.pages.dev/"],
      [join("en", "index.html"), "https://dinar-landing.pages.dev/en/"],
      [join("ar", "index.html"), "https://dinar-landing.pages.dev/ar/"],
    ];

    it.each(ROUTE_CANONICAL)("%s has one canonical link to its own URL", (file, canonical) => {
      const html = read(file);
      const links = [...html.matchAll(/<link rel="canonical" href="([^"]+)">/g)];
      expect(links).toHaveLength(1);
      expect(links[0]?.[1]).toBe(canonical);
    });

    it.each(ROUTES)("%s carries all three alternate links plus x-default", (file) => {
      const html = read(file);
      const alternates = [...html.matchAll(/<link rel="alternate" hreflang="([^"]+)" href="([^"]+)">/g)].map((m) => [
        m[1],
        m[2],
      ]);
      expect(alternates).toEqual(
        expect.arrayContaining([
          ["fr-FR", "https://dinar-landing.pages.dev/"],
          ["en-US", "https://dinar-landing.pages.dev/en/"],
          ["ar", "https://dinar-landing.pages.dev/ar/"],
          ["x-default", "https://dinar-landing.pages.dev/"],
        ]),
      );
    });

    it.each(ROUTES)("%s points og:image and twitter:image at the committed card", (file) => {
      const html = read(file);
      expect(html).toContain('<meta property="og:image" content="https://dinar-landing.pages.dev/og/card.png">');
      expect(html).toContain('<meta property="og:image:width" content="1200">');
      expect(html).toContain('<meta property="og:image:height" content="630">');
      expect(html).toContain('<meta name="twitter:card" content="summary_large_image">');
      expect(html).toContain('<meta name="twitter:image" content="https://dinar-landing.pages.dev/og/card.png">');
    });

    it("the card file og:image points at is 1200x630, on disk in the build", async () => {
      const bytes = readFileSync(join(outDir, "og", "card.png"));
      const meta = await sharp(bytes).metadata();
      expect(meta.width).toBe(1200);
      expect(meta.height).toBe(630);
    });
  });

  describe("the sitemap lists all three routes with their alternates (L5)", () => {
    it("sitemap-index.xml exists and points at sitemap-0.xml", () => {
      const index = read("sitemap-index.xml");
      expect(index).toContain("https://dinar-landing.pages.dev/sitemap-0.xml");
    });

    it("sitemap-0.xml carries the three routes, each with all three hreflang alternates", () => {
      const sitemap = read("sitemap-0.xml");
      for (const { path } of Object.values({
        fr: { path: "/" },
        en: { path: "/en/" },
        ar: { path: "/ar/" },
      })) {
        expect(sitemap).toContain(`<loc>https://dinar-landing.pages.dev${path}</loc>`);
      }
      expect((sitemap.match(/hreflang="fr-FR"/g) ?? []).length).toBe(3);
      expect((sitemap.match(/hreflang="en-US"/g) ?? []).length).toBe(3);
      expect((sitemap.match(/hreflang="ar"/g) ?? []).length).toBe(3);
    });
  });

  describe("Cloudflare Web Analytics stays off unless DZPOS_CF_BEACON_TOKEN is set (L5)", () => {
    it("the default build (this suite's own, token unset) carries no beacon script", () => {
      for (const file of ROUTES) {
        expect(read(file)).not.toContain("cloudflareinsights");
      }
    });

    // Same reasoning as the outer beforeAll's 30s: this runs its own full
    // `astro build` (a second one, with the token env var set, since the
    // token changes what gets built rather than what gets read from the
    // outer build), not an assertion, and the default 5s test timeout is
    // only enough when no other worktree is building at the same time.
    it(
      "setting the token adds exactly the Cloudflare beacon script, nothing else",
      () => {
        const tokenOutDir = mkdtempSync(join(ROOT, "node_modules", ".smoke-token-"));
        try {
          const astroBin = join(ROOT, "node_modules", ".bin", process.platform === "win32" ? "astro.cmd" : "astro");
          execFileSync(astroBin, ["build", "--outDir", tokenOutDir], {
            cwd: ROOT,
            encoding: "utf8",
            env: { ...process.env, DZPOS_CF_BEACON_TOKEN: "smoke-test-token" },
          });
          const html = readFileSync(join(tokenOutDir, "index.html"), "utf8");
          expect(html).toContain(
            '<script defer src="https://static.cloudflareinsights.com/beacon.min.js" data-cf-beacon="{&quot;token&quot;: &quot;smoke-test-token&quot;}"></script>',
          );
        } finally {
          rmSync(tokenOutDir, { recursive: true, force: true });
        }
      },
      30_000,
    );
  });
});
