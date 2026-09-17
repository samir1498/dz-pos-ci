// What separates a scanner from a cashier, checked at the boundary rather
// than near it. Every case here is one a real model produces: the two
// terminators, no terminator at all, a prefix character, and the one that
// started this work, a scanner configured for a US keyboard plugged into
// the French machine the shop actually owns.

import { describe, expect, test } from "vitest";
import {
  EMPTY,
  SCAN_MAX_GAP_MS,
  SCAN_MIN_LENGTH,
  type ScanState,
  canonicalBarcode,
  feed,
} from "./scan";

/** The physical keys of the top row, in order, as `code` reports them. */
const DIGIT_CODES = [
  "Digit0",
  "Digit1",
  "Digit2",
  "Digit3",
  "Digit4",
  "Digit5",
  "Digit6",
  "Digit7",
  "Digit8",
  "Digit9",
];

/** What a French keyboard makes of those keys unshifted, which is what the
 * page is handed when the scanner thinks it is typing digits on a US one. */
const AZERTY_UNSHIFTED = ["à", "&", "é", '"', "'", "(", "-", "è", "_", "ç"];

type Press = { key: string; code: string };

/** The presses a scanner sends for a numeric code, with `key` and `code`
 * both told the way the given layout would tell them. */
function digits(code: string, layout: "fr" | "us"): Press[] {
  return [...code].map((digit) => {
    const index = Number(digit);
    return {
      key: layout === "us" ? AZERTY_UNSHIFTED[index] : digit,
      code: DIGIT_CODES[index],
    };
  });
}

/** Feed presses `gap` milliseconds apart and return what came out. */
function burst(
  presses: Press[],
  gap: number,
  terminator?: string,
  start: ScanState = EMPTY,
): { state: ScanState; scan: string | null } {
  let state = start;
  let scan: string | null = null;
  let at = 1000;
  for (const press of presses) {
    at += gap;
    const step = feed(state, { type: "key", key: press.key, code: press.code, at });
    state = step.state;
    scan = step.scan;
  }
  at += gap;
  const end =
    terminator === undefined
      ? feed(state, { type: "idle", at })
      : feed(state, { type: "key", key: terminator, code: terminator, at });
  return { state: end.state, scan: end.scan };
}

/** Letters, the way a cashier types a product name. */
function typed(word: string): Press[] {
  return [...word].map((letter) => ({ key: letter, code: `Key${letter.toUpperCase()}` }));
}

describe("the terminator a model happens to send", () => {
  test("Enter, the common default", () => {
    expect(burst(digits("6130000000001", "fr"), 20, "Enter").scan).toBe("6130000000001");
  });

  test("Tab, which the other half of them send", () => {
    expect(burst(digits("6130000000001", "fr"), 20, "Tab").scan).toBe("6130000000001");
  });

  test("none at all: quiet ends the burst", () => {
    expect(burst(digits("6130000000001", "fr"), 20).scan).toBe("6130000000001");
  });

  test("the buffer is empty afterwards, so the next scan starts clean", () => {
    // The literal, not `EMPTY`: asserting a constant against itself holds
    // however that constant is defined, which is no assertion at all.
    expect(burst(digits("6130000000001", "fr"), 20, "Enter").state).toEqual({
      keys: "",
      codes: "",
      last: 0,
    });
  });
});

describe("a scanner set up for a US keyboard, on the French machine", () => {
  test("the digits arrive as punctuation and the physical keys are read instead", () => {
    const presses = digits("6130000000001", "us");
    expect(presses.map((p) => p.key).join("")).not.toMatch(/[0-9]/);
    expect(burst(presses, 20, "Enter").scan).toBe("6130000000001");
  });

  test("and with no terminator either, which is the worst model", () => {
    expect(burst(digits("6130000000001", "us"), 20).scan).toBe("6130000000001");
  });

  test("a French scanner on the same machine is not broken to fix it", () => {
    expect(burst(digits("6130000000001", "fr"), 20, "Enter").scan).toBe("6130000000001");
  });
});

describe("speed is what says scanner", () => {
  test(`a gap of exactly ${SCAN_MAX_GAP_MS} ms is still one burst`, () => {
    expect(burst(digits("61300000", "fr"), SCAN_MAX_GAP_MS, "Enter").scan).toBe("61300000");
  });

  test(`one millisecond slower is a person, and only the last key is held`, () => {
    const out = burst(digits("61300000", "fr"), SCAN_MAX_GAP_MS + 1, "Enter");
    expect(out.scan).toBeNull();
  });

  test("a pause in the middle drops what came before it", () => {
    let state = EMPTY;
    for (const [index, press] of digits("6130000000001", "fr").entries()) {
      const at = 1000 + index * 20 + (index === 6 ? 500 : 0);
      state = feed(state, { type: "key", key: press.key, code: press.code, at }).state;
    }
    // Six keys came after the pause. A code needs eight.
    expect(feed(state, { type: "key", key: "Enter", code: "Enter", at: 2000 }).scan).toBeNull();
  });
});

describe("length", () => {
  test(`${SCAN_MIN_LENGTH} characters is a code`, () => {
    expect(burst(digits("61300000", "fr"), 20, "Enter").scan).toBe("61300000");
  });

  test(`${SCAN_MIN_LENGTH - 1} is somebody typing a price`, () => {
    expect(burst(digits("6130000", "fr"), 20, "Enter").scan).toBeNull();
  });

  test(`${SCAN_MIN_LENGTH} letters and digits together, which only the keys can carry`, () => {
    // Exactly at the length, and unreadable from the physical keys, so this
    // is the case that says the length is `>=` and not `>`: the fallback
    // cannot answer for it.
    const presses = [...typed("ABCD"), ...digits("1234", "fr")];
    expect(burst(presses, 20, "Enter").scan).toBe("ABCD1234");
  });
});

describe("what the shop prints around the code", () => {
  test("a leading asterisk is the scanner's, not the article's", () => {
    const presses = [{ key: "*", code: "Digit8" }, ...digits("6130000000001", "fr")];
    expect(burst(presses, 20, "Enter").scan).toBe("6130000000001");
  });

  test("and a trailing one goes the same way", () => {
    const presses = [...digits("6130000000001", "fr"), { key: "-", code: "Minus" }];
    expect(burst(presses, 20, "Enter").scan).toBe("6130000000001");
  });

  test("two of them, because a model that sends one can send two", () => {
    const presses = [
      { key: "*", code: "Digit8" },
      { key: "]", code: "BracketRight" },
      ...digits("6130000000001", "fr"),
    ];
    expect(burst(presses, 20, "Enter").scan).toBe("6130000000001");
  });

  test("an AIM identifier is the symbology's name, not part of the code", () => {
    // `]E0` is what a scanner with "transmit AIM identifier" on sends before
    // an EAN-13. Trimming only the `]` left `E0` welded to the digits and
    // every scan from such a unit read as a code no article carries.
    const presses = [
      { key: "]", code: "BracketRight" },
      { key: "E", code: "KeyE" },
      { key: "0", code: "Digit0" },
      ...digits("6130000000001", "fr"),
    ];
    expect(burst(presses, 20, "Enter").scan).toBe("6130000000001");
  });

  test("and the identifier's own digit does not reach the physical-key form", () => {
    // The US-layout path reads `codes`, where `]E0` is `?`, `?`, `0`. The
    // two `?` trim away as punctuation and the `0` would not, so a code
    // stripped on one side only comes out with a leading zero it never had.
    const presses = [
      { key: "]", code: "BracketRight" },
      { key: "E", code: "KeyE" },
      { key: "0", code: "Digit0" },
      ...digits("6130000000001", "us"),
    ];
    expect(burst(presses, 20, "Enter").scan).toBe("6130000000001");
  });

  test("an identifier only counts at the very front of the burst", () => {
    // `A]E` then eight letters. Unanchored, the identifier pattern would
    // find `]EA` one character in and drop three from the front, and what
    // was left would read as a clean Code 128 word. The bracket is inside
    // the run, so this is somebody typing and the answer is nothing.
    const presses = [
      { key: "A", code: "KeyA" },
      { key: "]", code: "BracketRight" },
      { key: "E", code: "KeyE" },
      ...typed("ABCDEFGH"),
    ];
    expect(burst(presses, 20, "Enter").scan).toBeNull();
  });

  test("a stray closing bracket is still just punctuation", () => {
    // Three characters or none: `]` followed by something that is not a
    // symbology name is the edge trim's business, not the identifier's.
    const presses = [{ key: "]", code: "BracketRight" }, ...digits("6130000000001", "fr")];
    expect(burst(presses, 20, "Enter").scan).toBe("6130000000001");
  });

  test("punctuation in the middle is not a code, it is somebody typing", () => {
    // Only the ends are the scanner's. A character inside the run means the
    // burst was never a barcode, and Code 39's own hyphens are not something
    // an Algerian shop's EAN-13 articles carry.
    const presses = [
      ...digits("613", "fr"),
      { key: "-", code: "Minus" },
      ...digits("0009000110", "fr"),
    ];
    expect(burst(presses, 20, "Enter").scan).toBeNull();
  });
});

describe("a person at the counter", () => {
  test("a word typed fast and then left alone is not a scan", () => {
    expect(burst(typed("camembert"), 40).scan).toBeNull();
  });

  test("the same word with Enter is taken, because Code 128 carries letters", () => {
    expect(burst(typed("camembert"), 40, "Enter").scan).toBe("camembert");
  });

  test("a word that ends in digits is still a word", () => {
    const presses = [...typed("camembert"), ...digits("1234", "fr")];
    expect(burst(presses, 40).scan).toBeNull();
  });

  test("digits then letters, at scanner speed, are the digits", () => {
    // The other way round from the case above, and it comes out differently
    // on purpose. Letters cannot be part of a numeric code, so they are
    // trimmed like any other edge character and what is left is long enough
    // to be one. Sustaining eleven characters under 80 ms by hand to reach
    // this is not something that happens at a counter.
    const presses = [...digits("12345678", "fr"), ...typed("abc")];
    expect(burst(presses, 40).scan).toBe("12345678");
  });

  test("Shift and the rest of the named keys are not characters", () => {
    const state = feed(EMPTY, { type: "key", key: "Shift", code: "ShiftLeft", at: 1000 });
    expect(state.state).toEqual(EMPTY);
    expect(state.scan).toBeNull();
  });

  test("a modifier between two keys does not break the burst", () => {
    let state = EMPTY;
    let at = 1000;
    for (const press of digits("61300000", "fr")) {
      at += 20;
      state = feed(state, { type: "key", key: press.key, code: press.code, at }).state;
      state = feed(state, { type: "key", key: "Shift", code: "ShiftLeft", at: at + 1 }).state;
    }
    expect(feed(state, { type: "key", key: "Enter", code: "Enter", at: at + 20 }).scan).toBe(
      "61300000",
    );
  });

  test("an empty burst terminated by Enter is nothing at all", () => {
    expect(feed(EMPTY, { type: "key", key: "Enter", code: "Enter", at: 1000 }).scan).toBeNull();
  });
});

describe("the numeric keypad is the same keys", () => {
  test("a code that merely ends in a digit key is not one", () => {
    // `code` names a key exactly. Something that only finishes with
    // `Digit1` is a different key, and reading the digit off its tail would
    // invent numbers out of whatever the browser happened to call it.
    const presses = Array.from({ length: 8 }, () => ({ key: "&", code: "MetaDigit1" }));
    expect(burst(presses, 20, "Enter").scan).toBeNull();
  });

  test("nor one that merely starts with one", () => {
    const presses = Array.from({ length: 8 }, () => ({ key: "&", code: "Digit1Extra" }));
    expect(burst(presses, 20, "Enter").scan).toBeNull();
  });

  test("a scanner sending Numpad codes reads the same", () => {
    const presses = [...("61300000" as string)].map((digit) => ({
      key: AZERTY_UNSHIFTED[Number(digit)],
      code: `Numpad${digit}`,
    }));
    expect(burst(presses, 20, "Enter").scan).toBe("61300000");
  });
});

describe("canonicalBarcode: UPC-A and EAN-13 are the same article", () => {
  test("twelve digits gain the zero the EAN carries", () => {
    expect(canonicalBarcode("613000000001")).toBe("0613000000001");
  });

  test("thirteen are already the long form", () => {
    expect(canonicalBarcode("6130000000001")).toBe("6130000000001");
  });

  test("the two forms of one article meet", () => {
    expect(canonicalBarcode("613000000001")).toBe(canonicalBarcode("0613000000001"));
  });

  test("eight digits are an EAN-8 and stay themselves", () => {
    expect(canonicalBarcode("61300001")).toBe("61300001");
  });

  test("a code with letters is left alone", () => {
    expect(canonicalBarcode("ABC123456789")).toBe("ABC123456789");
  });

  test("surrounding space is not part of the code", () => {
    expect(canonicalBarcode("  613000000001 ")).toBe("0613000000001");
  });
});
