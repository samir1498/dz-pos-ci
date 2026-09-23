#!/usr/bin/env node
// Two bans in one gate, run beside `scripts/file-sizes.mjs`: no cargo, so it
// runs early and cheap, and `just gates` runs it right after `sizes`.
//
// (a) TypeScript: every `*.test.ts(x)` or `*.spec.ts(x)` has to sit under a
// `tests/` folder (or Playwright's `e2e/`, which already stood apart before
// this gate). Checked as "has a tests/ or e2e/ segment" rather than "has no
// src/ segment": apps/mobile has no `src/` at all, its tests mirror `lib/`,
// and a check phrased around `src/` would wave every one of those through.
// Every test moved into a package's `tests/` folder, mirroring its source
// path, so Sonar's line count reads as tests rather than as source; this
// keeps the split from drifting back one file at a time. No allow list on
// this side: after the move, the count is zero, and it stays zero.
//
// (b) Rust: no `#[cfg(test)]` followed by an inline `mod name { ... }` body
// under `crates/*/src` or `apps/desktop/src-tauri/src`. The out-of-line
// shape is `#[cfg(test)]` (optionally with `#[path = "..."]` pointing at a
// file under `tests/unit/`, for a suite that needs a crate's private items
// and so cannot be an ordinary integration test) followed by `mod name;`
// with no body — a declaration, not a definition, so the test code itself
// lives in its own file.
//
// This is a ratchet, the same shape as scripts/file-sizes.json next door:
// the Rust side still has close to fifty files with an inline module today,
// pinned by name in scripts/inline-tests.json. A file on that list may lose
// its inline module (and then has to come off the list) but a file not on
// the list may not grow one, so the list only shrinks.
//
// Run by `just gates`.

import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

function tracked(...patterns) {
  const out = execFileSync("git", ["ls-files", "-z", ...patterns], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 1 << 24,
  });
  return out.split("\0").filter(Boolean);
}

const problems = [];

// ---- (a) TypeScript tests out of src ----

// A `tests/` or `e2e/` segment somewhere in the path, not specifically
// `src/`: apps/mobile has no `src/` at all (its tests mirror `lib/`), so a
// check that only refused `src/` would wave every mobile test through.
const IN_TESTS_OR_E2E = /(^|\/)(tests|e2e)\//;

let tsExamined = 0;

for (const path of tracked("*.test.ts", "*.test.tsx", "*.spec.ts", "*.spec.tsx")) {
  tsExamined += 1;
  if (!IN_TESTS_OR_E2E.test(path)) {
    problems.push(
      `${path}: a test file outside any tests/ or e2e/ folder. Move it into ` +
        `this package's tests/ folder, mirroring its source path (src/lib/x.ts's ` +
        `test goes to tests/lib/x.test.ts; apps/mobile's lib/x.ts's goes to tests/lib/x.test.ts).`,
    );
  }
}

// ---- (b) Rust: no inline #[cfg(test)] mod body ----

const RUST_SRC_ROOTS = [/^crates\/[^/]+\/src\//, /^apps\/desktop\/src-tauri\/src\//];

const isRustSrc = (path) => RUST_SRC_ROOTS.some((re) => re.test(path)) && path.endsWith(".rs");

// #[cfg(test)], then zero or more other attributes (#[...] or #![...]),
// then an optional visibility modifier, then `mod name`, then either `{`
// (inline, banned unless allow-listed) or `;` (out-of-line, always fine).
const CFG_TEST_MOD = /#\[cfg\(test\)\]\s*(?:#!?\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*([{;])/g;

function hasInlineTestModule(source) {
  for (const match of source.matchAll(CFG_TEST_MOD)) {
    if (match[1] === "{") return true;
  }
  return false;
}

const baselinePath = join(here, "inline-tests.json");
const baseline = JSON.parse(readFileSync(baselinePath, "utf8"));
const allowed = new Set(baseline.files);

const rustPaths = tracked("*.rs").filter(isRustSrc);
const seen = new Set();

for (const path of rustPaths) {
  const source = readFileSync(join(root, path), "utf8");
  if (!hasInlineTestModule(source)) continue;
  seen.add(path);
  if (!allowed.has(path)) {
    problems.push(
      `${path}: a new #[cfg(test)] mod { ... } with a body. Give it its own ` +
        `file and a #[cfg(test)] #[path = "..."] mod name; hook instead; ` +
        `scripts/inline-tests.json is not the fix, that list only shrinks.`,
    );
  }
}

for (const path of allowed) {
  if (!seen.has(path)) {
    problems.push(
      `scripts/inline-tests.json lists ${path}, which no longer has an inline ` +
        `#[cfg(test)] mod body (or no longer exists). Remove it from the list.`,
    );
  }
}

if (problems.length > 0) {
  console.error(`no-inline-tests: ${problems.length} problem${problems.length === 1 ? "" : "s"}\n`);
  for (const problem of problems) console.error(`  ${problem}`);
  process.exit(1);
}

console.log(
  `no-inline-tests: ok. ${tsExamined} TypeScript test file(s) examined, all under tests/ or e2e/; ` +
    `${allowed.size} Rust file(s) still carrying an inline module and pinned.`,
);
