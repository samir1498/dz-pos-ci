// Telling a barcode scanner apart from a person typing.
//
// A scanner is a keyboard. It types the code into whatever has focus and
// sends a key at the end, and the models differ in ways that break a till
// that waits for Enter: some send Tab, some send nothing, some are
// configured for a US keyboard while the shop's machine is French, in which
// case the digits arrive as `&é"'(` because that is what the AZERTY map
// makes of those physical keys.
//
// What a scanner cannot hide is its speed. Eight or more characters with
// gaps of tens of milliseconds and no thinking pause is a signature nobody
// produces at a counter, so that is what this file recognises.
//
// Everything here is pure and the clock comes in as a number. A test says
// what time it is; nothing sleeps, and the boundary is checked at the
// boundary rather than near it.

/** Fewer characters than this is a person, whatever the gaps look like.
 * EAN-13, UPC-A and the short EAN-8 all clear it. */
export const SCAN_MIN_LENGTH = 8;

/** The longest gap between two characters that still counts as one burst.
 * A Bluetooth wedge sits at 10 to 30 ms and a slow USB one higher, so this
 * is generous; sustaining it over eight characters by hand is not. */
export const SCAN_MAX_GAP_MS = 80;

/** Quiet for this long ends a burst, for the scanners that send no
 * terminator at all. */
export const SCAN_IDLE_MS = 100;

/** A key press as the DOM reports it, plus when it happened.
 *
 * `key` is what the OS decided the key means, `code` is which key was
 * pressed. They disagree exactly when the scanner's layout and the
 * machine's do, which is the case worth surviving. */
export type ScanEvent =
  | { type: "key"; key: string; code: string; at: number }
  | { type: "idle"; at: number };

export type ScanState = {
  /** What `key` said, one character per press. */
  keys: string;
  /** What `code` said, in step with `keys` so the two can be compared
   * position by position: a digit for a digit key, `?` for anything else. */
  codes: string;
  /** When the last character arrived, so the next one can be timed. */
  last: number;
};

export const EMPTY: ScanState = { keys: "", codes: "", last: 0 };

/** Terminators a scanner sends after the code. Enter is the common
 * default, Tab the other one. */
const TERMINATORS = new Set(["Enter", "Tab"]);

/** The digit a physical key stands for, whatever the keyboard map made of
 * it. `?` for every other key, so `codes` and `keys` stay the same length
 * and a prefix character does not shift the digits out of line. */
function codeChar(code: string): string {
  const digit = /^(?:Digit|Numpad)([0-9])$/.exec(code);
  return digit === null ? "?" : digit[1];
}

/** A scanner's own prefix or suffix is punctuation around the code: an
 * asterisk, a stray carriage return. The code is what is left after both
 * ends are cleaned. An AIM identifier is not punctuation and is not handled
 * here; see `aimLength`. */
function trimEdges(text: string): string {
  return text.replace(/^[^0-9A-Za-z]+/, "").replace(/[^0-9A-Za-z]+$/, "");
}

/**
 * How many characters of a leading AIM symbology identifier to drop.
 *
 * A scanner with "transmit AIM identifier" switched on prefixes every code
 * with `]` and two characters naming the symbology it read: `]E0` for an
 * EAN-13, `]C0` for Code 128, `]d2` for a Data Matrix (ISO/IEC 15424). Only
 * the `]` is punctuation, so trimming the edges leaves `E0` welded to the
 * front of the digits and the whole scan reads as a code no article
 * carries. Three characters or none, because a `]` that is not followed by
 * the two an identifier has is somebody's stray keystroke and the edge trim
 * already owns that case.
 */
const AIM = /^\][A-Za-z][A-Za-z0-9]/;
const aimLength = (text: string): number => (AIM.test(text) ? 3 : 0);

const ALPHANUMERIC = /^[0-9A-Za-z]+$/;
const DIGITS = /^[0-9]+$/;

/**
 * The code a finished burst carries, or `null` when the burst was a person.
 *
 * `keys` is believed first. A French scanner on a French machine sends
 * digits and they arrive as digits, and reading the physical keys instead
 * would break that perfectly good setup in the name of fixing the broken
 * one. Only when what the OS produced cannot be a barcode do the physical
 * keys get their turn, and then only if they spell a number.
 *
 * `strict` is the idle path: a burst that nobody terminated has to look
 * like a number before it is taken, or a fast typist's word followed by a
 * pause becomes a scan. A terminator is a statement of intent, so that path
 * accepts letters too, which is what Code 128 needs.
 */
export function burstCode(state: ScanState, strict: boolean): string | null {
  // `keys` and `codes` are kept in step, one character per press, so an
  // identifier dropped from the front of one is dropped from the front of
  // the other at the same offset. Its own last character is a digit often
  // enough (`]E0`) that leaving it on `codes` would invent a leading zero.
  const aim = aimLength(state.keys);
  const keys = trimEdges(state.keys.slice(aim));
  if (keys.length >= SCAN_MIN_LENGTH && ALPHANUMERIC.test(keys)) {
    if (!strict || DIGITS.test(keys)) return keys;
  }
  const codes = trimEdges(state.codes.slice(aim));
  if (codes.length >= SCAN_MIN_LENGTH && DIGITS.test(codes)) return codes;
  return null;
}

/**
 * One event against the running state. Returns the state to keep and, when
 * the burst ended, the code it carried.
 *
 * A character that arrives later than `SCAN_MAX_GAP_MS` after the last one
 * is not part of that burst: it starts a new one. That is what keeps a
 * cashier typing a word letter by letter from ever accumulating into
 * something long enough to be taken for a code.
 */
export function feed(state: ScanState, event: ScanEvent): { state: ScanState; scan: string | null } {
  if (event.type === "idle") {
    const scan = burstCode(state, true);
    return { state: EMPTY, scan };
  }
  if (TERMINATORS.has(event.key)) {
    const scan = burstCode(state, false);
    return { state: EMPTY, scan };
  }
  // Modifiers, arrows, F-keys and the dead keys of an accented layout all
  // report a name rather than a character. None of them is part of a code.
  if (event.key.length !== 1) return { state, scan: null };

  // No test of "is there a buffer" here: appending to an empty one is the
  // same as starting one, character for character, so the question does not
  // change an answer. Mutation testing is what said so, 2026-09-17.
  const continues = event.at - state.last <= SCAN_MAX_GAP_MS;
  const next: ScanState = continues
    ? { keys: state.keys + event.key, codes: state.codes + codeChar(event.code), last: event.at }
    : { keys: event.key, codes: codeChar(event.code), last: event.at };
  return { state: next, scan: null };
}

/**
 * The one form a barcode is compared in.
 *
 * A UPC-A code is twelve digits and the same article's EAN-13 is those
 * twelve with a zero in front. A scanner reads whichever is printed on the
 * box, the shop file holds whichever was typed in, and a till that compares
 * the strings as they come misses half of those. Both sides go through
 * here, so both are thirteen digits when they meet.
 */
export function canonicalBarcode(code: string): string {
  const trimmed = code.trim();
  return /^[0-9]{12}$/.test(trimmed) ? `0${trimmed}` : trimmed;
}
