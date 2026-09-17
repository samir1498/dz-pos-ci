#!/usr/bin/env node
// No huge files. A file that keeps growing is one nobody reads before
// changing it, and it is where two branches collide: crates/api/src/dto.rs
// reached 3071 lines holding every domain at once before it was split.
//
// This is a ratchet, not a big-bang rule, and it is the same shape as
// apps/desktop/src/lint/allowlist.json next door: the files that are already
// over the limit are listed in file-sizes.json with the length they have
// today, they may not grow, and an entry that has come back under the limit
// has to be deleted. So the list only shrinks, and nothing new joins it.
//
// Run by `just sizes`, which `just gates` runs.

import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");

/** A screen or a service past this is doing more than one thing. */
const SRC_LIMIT = 600;

/** A test file is a list of independent cases, so it earns more room. Past
 * this it is several suites that have not been named separately yet. */
const TEST_LIMIT = 1200;

/** Written by ts-rs from the Rust wire types; not ours to shorten. */
const IGNORED = [/^packages\/shared\/src\/generated\//];

const isTest = (path) =>
  /(^|\/)tests\//.test(path) ||
  /(^|\/)e2e\//.test(path) ||
  /\.(test|spec)\.tsx?$/.test(path);

const limitFor = (path) => (isTest(path) ? TEST_LIMIT : SRC_LIMIT);

function tracked() {
  const out = execFileSync("git", ["ls-files", "-z", "*.rs", "*.ts", "*.tsx"], {
    cwd: root,
    encoding: "utf8",
    maxBuffer: 1 << 24,
  });
  return out
    .split("\0")
    .filter(Boolean)
    .filter((path) => !IGNORED.some((re) => re.test(path)));
}

const lines = (path) => readFileSync(join(root, path), "utf8").split("\n").length - 1;

const baseline = JSON.parse(readFileSync(join(here, "file-sizes.json"), "utf8"));
const allowed = baseline.over_the_limit;
const problems = [];

const paths = tracked();
const seen = new Set(paths);

for (const path of paths) {
  const limit = limitFor(path);
  const count = lines(path);
  const cap = allowed[path];
  if (cap === undefined) {
    if (count > limit) {
      problems.push(
        `${path} is ${count} lines, over the ${limit} line limit. Split it. ` +
          `Adding it to scripts/file-sizes.json is not the fix; that list only shrinks.`,
      );
    }
    continue;
  }
  if (count > cap) {
    problems.push(
      `${path} is ${count} lines and was allowed ${cap}. It may not grow. ` +
        `Put the new code somewhere else, or split the file and lower the entry.`,
    );
  } else if (count <= limit) {
    problems.push(
      `${path} is ${count} lines, back under the ${limit} line limit. ` +
        `Delete its entry from scripts/file-sizes.json so it cannot grow again.`,
    );
  } else if (count < cap) {
    problems.push(
      `${path} is ${count} lines and its entry still says ${cap}. ` +
        `Lower the entry to ${count} so the ground it gained is kept.`,
    );
  }
}

for (const path of Object.keys(allowed)) {
  if (!seen.has(path)) {
    problems.push(`scripts/file-sizes.json lists ${path}, which no longer exists. Delete the entry.`);
  }
}

if (problems.length > 0) {
  console.error(`file sizes: ${problems.length} problem${problems.length === 1 ? "" : "s"}\n`);
  for (const problem of problems) console.error(`  ${problem}`);
  process.exit(1);
}

console.log(
  `file sizes: ok. ${paths.length} files, ${Object.keys(allowed).length} still over the limit and pinned.`,
);
