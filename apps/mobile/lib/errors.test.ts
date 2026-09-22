// The table in errors.ts is only worth having if it is complete, and it can
// only be complete against the server's own lists. Both are a Rust `match`
// returning a string literal per arm, so they can be read: `CoreError::code`
// in crates/kernel/src/error.rs (moved from crates/core/src/error.rs in the
// kernel crate split, S3 of a-kernel-crate-and-retail-as-the-first-module)
// and `ApiError::parts` in crates/api/src/error.rs.
//
// A walk, not a copy. A copy of the codes here would go green the day the
// server grows a new one, which is the day a cashier starts being told a
// status number again.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { ERROR_KEY, errorKey } from "./errors";

const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");

/** The body of one Rust function, from `marker` to its closing brace, found
 *  by counting braces so a later function in the same file is not swept in. */
function bodyOf(file: string, marker: string): string {
  const source = readFileSync(join(repo, file), "utf8");
  const start = source.indexOf(marker);
  expect(start, `${marker} in ${file}`).toBeGreaterThan(-1);
  let depth = 0;
  for (let i = source.indexOf("{", start); i < source.length; i++) {
    if (source[i] === "{") depth++;
    else if (source[i] === "}" && --depth === 0) return source.slice(start, i);
  }
  throw new Error(`${marker} in ${file} never closes`);
}

function codesIn(body: string): string[] {
  return [...new Set([...body.matchAll(/"([a-z_]+)"/g)].map((m) => m[1]))];
}

const CORE = bodyOf("crates/kernel/src/error.rs", "pub const fn code(&self)");
const API = bodyOf("crates/api/src/error.rs", "fn parts(&self)");
const SERVER_CODES = [...new Set([...codesIn(CORE), ...codesIn(API)])].sort();

/** The refusals a cashier can do something about. Each one earns a sentence
 *  of its own; the rest share `error_till_problem`, because a workbook that
 *  will not write and a route that does not take this method are the same
 *  thing at a counter. */
const ACTIONABLE = [
  "validation", "credit_limit", "party_ids", "not_found",
  "duplicate_barcode", "conflict", "exhausted", "locked_out",
];

/** The credential refusals. Their own group because two of them share a
 *  sentence on purpose: `unauthorized` and `session_required` are the same
 *  thing to the person holding the phone, sign in again, and only the
 *  server cares which of the two it was. */
const CREDENTIAL = ["auth_refused", "device_refused", "session_required", "unauthorized", "forbidden"];

describe("every code the server can send", () => {
  it("is a list this walk actually found", () => {
    // The walk failing open is the one way this file could pass while
    // proving nothing, and it has two shapes. A renamed function is caught
    // in `bodyOf`, which asserts it found the marker at all; proved by
    // renaming `code` and watching the file refuse to load. This row is for
    // the other shape: the marker still there and the arms no longer string
    // literals, which would leave the checks below running over nothing and
    // agreeing with themselves. Proved by narrowing the pattern until it
    // matched no arm.
    expect(SERVER_CODES.length).toBeGreaterThan(15);
    expect(SERVER_CODES).toContain("credit_limit");
    expect(SERVER_CODES).toContain("device_refused");
  });

  it("names its code in the arm, or delegates to a list this walk also reads", () => {
    // The third shape, and the one the walk cannot see by itself: an arm
    // whose code is a call rather than a literal. Two exist today, both
    // `e.code()`, both landing in `CoreError::code`, which is the other
    // body walked here. A future arm delegating anywhere else would add a
    // code with no sentence and nothing would go red, so the shape is
    // checked rather than assumed.
    const arms = API.split("=>").slice(1);
    const undelegated = arms.filter((arm) => {
      const head = arm.slice(0, arm.indexOf("\n", arm.indexOf(")")) + 1);
      return !/"[a-z_]+"/.test(head) && !/\.code\(\)/.test(head);
    });
    expect(undelegated).toEqual([]);
  });

  it("has a sentence on the phone", () => {
    expect(SERVER_CODES.filter((code) => errorKey(code) === null)).toEqual([]);
  });

  it("has no line here for a code it cannot send", () => {
    // The other direction. A key left behind by a code the server dropped
    // is dead weight that reads as coverage.
    const server = new Set(SERVER_CODES);
    expect(Object.keys(ERROR_KEY).filter((code) => !server.has(code))).toEqual([]);
  });
});

describe("what the cashier is told", () => {
  it("is its own sentence for every refusal they can act on", () => {
    // Not a second copy of the table: what is asserted is that no two of
    // these share a sentence. Pointing `duplicate_barcode` at the credit
    // limit's key passes every other check in this file, and a barcode
    // already taken would read as a customer over their limit.
    const keys = ACTIONABLE.map((code) => errorKey(code));
    expect(keys).not.toContain(null);
    expect(new Set(keys).size).toBe(ACTIONABLE.length);
  });

  it("is one sentence for every fault they cannot", () => {
    const known = new Set([...ACTIONABLE, ...CREDENTIAL]);
    const faults = SERVER_CODES.filter((code) => !known.has(code));
    expect(faults.length).toBeGreaterThan(4);
    expect(new Set(faults.map((code) => errorKey(code)))).toEqual(new Set(["error_till_problem"]));
  });

  it("sends each dead credential to the thing that revives it", () => {
    // Four sentences for five codes, and which code shares which is the
    // point: a revoked phone needs a manager with a QR, an idled-out
    // session needs a PIN, and reading one as the other is what sent a
    // phone back to the pairing screen over a mistyped digit.
    expect(CREDENTIAL.map((code) => errorKey(code))).toEqual([
      "error_wrong_secret",
      "error_pair_again",
      "error_sign_in_again",
      "error_sign_in_again",
      "error_not_allowed",
    ]);
  });
});
