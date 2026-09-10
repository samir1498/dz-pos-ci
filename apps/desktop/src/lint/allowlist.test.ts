// The allowlist is a to-do list with a deadline, and this is what keeps it
// one. Two failures it exists to cause:
//
// - An entry naming a file that is gone. Someone deleted or renamed a screen
//   and the exemption stayed behind, quietly covering nothing, or worse,
//   covering the next file to take that path.
// - An entry whose file has nothing left to fix. That is the important one:
//   without it a screen could be rewritten on the kit and keep its
//   exemption, the count would never move, and the number that is supposed to
//   reach zero would sit at eighteen forever.
//
// So a screen leaving the list is not optional. Rewrite it and the gates tell
// you to delete its line in the same commit.
//
// The check for "has something left to fix" is a text search rather than a
// second run of eslint. eslint is the gate that decides what is allowed; this
// only has to answer whether the exemption is still earning its place, and a
// search for the five element names in the file answers that without starting
// a linter inside a test run.

import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";

import { describe, expect, test } from "vitest";

import allowlist from "./allowlist.json";

// process.cwd(), not import.meta.url: these run under jsdom, where
// import.meta.url is an http:// URL and fileURLToPath refuses it.
const ROOT = process.cwd();

const raw = (element: string) => new RegExp(`<${element}[\\s/>]`);

describe("the raw-element allowlist", () => {
  test("names the five elements the kit replaces", () => {
    expect(allowlist.elements).toEqual(["input", "button", "select", "textarea", "table"]);
  });

  test("every entry says which screen it is, so the list reads as work left", () => {
    for (const entry of allowlist.screens) {
      expect(entry.screen.trim()).not.toBe("");
    }
  });

  test("names no file twice", () => {
    const files = allowlist.screens.map((entry) => entry.file);
    expect(new Set(files).size).toBe(files.length);
  });

  test.each(allowlist.screens)("$screen ($file) is still there", ({ file }) => {
    expect(existsSync(join(ROOT, file))).toBe(true);
  });

  /**
   * The one that makes the count fall. An exemption for a file with nothing
   * left in it is an exemption nobody will ever remove on their own.
   */
  test.each(allowlist.screens)("$screen ($file) still has something to fix", ({ file }) => {
    const source = readFileSync(join(ROOT, file), "utf8");
    const found = allowlist.elements.filter((element) => raw(element).test(source));
    expect({ file, found }).not.toEqual({ file, found: [] });
  });

  /**
   * Not an assertion about the number, which falls as the screens are
   * rewritten. It is here so the count shows in the run's output and a
   * reviewer can read the progress off a green test.
   */
  test("reports how many screens are still on the old controls", () => {
    expect(allowlist.screens.length).toBeLessThanOrEqual(18);
  });
});
