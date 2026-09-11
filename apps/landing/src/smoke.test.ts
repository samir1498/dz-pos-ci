// The pattern this borrows from apps/desktop/src/theme.test.ts: scan the
// source tree for a banned pattern instead of trusting a reviewer to catch
// it by eye. Two things are asserted here that theme.test.ts does not need
// to, because this package ships no theme switcher and no committed
// generated file: the three routes actually render, and the Arabic one is
// RTL (context/plans/20260911-landing-page.md, L0).

import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
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
});
