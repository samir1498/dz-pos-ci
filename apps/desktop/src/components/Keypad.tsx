// The pad a cashier hits with a thumb. Twelve keys, the same twelve every
// counter terminal in the country carries: the nine digits, a double zero
// because prices here are whole dinars in the hundreds, a zero, a backspace,
// and one wide key that takes the sale.
//
// The component holds no amount. The key that was pressed goes out as a token
// and the screen decides what it means, so the same pad can drive the cash box
// today and a quantity box tomorrow without learning about either. The one
// rule that would otherwise be written twice, what a press does to an amount,
// is `keyedAmount` at the bottom of this file.
//
// The keyboard mirrors the pad rather than competing with it. A digit types
// that digit, Backspace is the backspace key and Enter is the wide one. On
// the till the pad only listens while focus is inside it, so a search box
// can still take digits. On a screen that is only the pad (sign-in, the
// lock), `captureWindow` listens on the window so a cashier does not have
// to tap a key first. The one trap that costs a double count: a focused
// button already fires its own click on Enter, so an Enter that came from
// a key of this pad is left to the browser.
//
// The keys are set in the figure face and read left to right in Arabic too,
// the same decision `Money` takes: a keypad whose 7 and 8 swapped places in
// one language is a pad nobody can use without looking. That is the grid's
// job and not the glyph's: a `dir` on each key keeps "00" reading as two
// zeros but leaves the grid flowing from the page's own side, which is how
// an Arabic till drew the 1 where a thumb reaches for the 3 until
// 2026-09-12. The twelve keys sit in a box of their own that is `ltr` in
// every language; the wide key stays outside it and reads with the page.

import { CornerDownLeft, Delete } from "lucide-react";
import { useEffect, useRef, type KeyboardEvent } from "react";

import { Icon } from "@/components/Icon";
import { Button } from "@/components/ui/button";
import { useTranslation } from "@/i18n";
import { cn } from "@/lib/utils";

/** What a press means. The screen turns it into an amount. */
export type KeypadKey =
  | "0"
  | "1"
  | "2"
  | "3"
  | "4"
  | "5"
  | "6"
  | "7"
  | "8"
  | "9"
  | "00"
  | "backspace"
  | "enter";

/** The face of the pad, in reading order. */
const FACE: readonly KeypadKey[] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "00", "0"];

const DIGITS: readonly KeypadKey[] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

/** The keyboard key this pad answers to, or nothing. `find` rather than a
 * cast: the union is the source and a key the pad has no face for is null. */
function typedKey(key: string): KeypadKey | null {
  if (key === "Backspace") return "backspace";
  if (key === "Enter") return "enter";
  return DIGITS.find((digit) => digit === key) ?? null;
}

function isTypingInAField(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.closest("input, textarea, select") !== null || target.isContentEditable;
}

export function Keypad({
  onKey,
  disabled = false,
  captureWindow = false,
  className,
  "data-testid": testId = "keypad",
}: {
  onKey: (key: KeypadKey) => void;
  disabled?: boolean;
  /** Listen on the window, not only while a key of this pad is focused. */
  captureWindow?: boolean;
  className?: string;
  "data-testid"?: string;
}) {
  const { t } = useTranslation();
  const onKeyRef = useRef(onKey);
  onKeyRef.current = onKey;

  function take(event: { key: string; target: EventTarget | null; preventDefault: () => void }) {
    if (disabled) return;
    const key = typedKey(event.key);
    if (key === null) return;
    // Enter on a key of the pad is that key: the browser is about to click
    // the focused button, and counting the press here as well would send two.
    // A digit or a backspace fires no click, so those still mirror.
    const onAKey = event.target instanceof HTMLElement && event.target.closest("button") !== null;
    if (key === "enter" && onAKey) return;
    event.preventDefault();
    onKeyRef.current(key);
  }

  useEffect(() => {
    if (!captureWindow) return;
    function onWindowKey(event: globalThis.KeyboardEvent) {
      if (isTypingInAField(event.target)) return;
      take(event);
    }
    window.addEventListener("keydown", onWindowKey);
    return () => window.removeEventListener("keydown", onWindowKey);
  }, [captureWindow, disabled]);

  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (captureWindow) return;
    take(event);
  }

  return (
    <div
      role="group"
      aria-label={t("keypad_label")}
      data-testid={testId}
      // A cashier with focus in here is typing an amount, digit by digit, and
      // the digits mean this pad rather than a barcode. `useScanner` reads
      // this to leave the pad alone, the same way it leaves a text field
      // alone: without it the first digit reached the amount and every digit
      // after it landed in the search box, because the scanner had moved
      // focus there on the first one.
      data-keypad=""
      className={cn("grid gap-2", className)}
      onKeyDown={onKeyDown}
    >
      <div dir="ltr" className="grid grid-cols-3 gap-2">
        {FACE.map((key) => (
          <Button
            key={key}
            type="button"
            variant="secondary"
            disabled={disabled}
            className="h-(--control-h-lg) font-numeric text-xl font-medium tabular-nums"
            onClick={() => onKey(key)}
          >
            {key}
          </Button>
        ))}
        {/* The arrow points at the digit it takes off, and the digits run
            left to right in every language, so this one does not mirror. */}
        <Button
          type="button"
          variant="secondary"
          disabled={disabled}
          aria-label={t("keypad_backspace")}
          className="h-(--control-h-lg)"
          onClick={() => onKey("backspace")}
        >
          <Icon as={Delete} size={20} />
        </Button>
      </div>
      <Button
        type="button"
        variant="outline"
        disabled={disabled}
        className="h-(--control-h-lg) text-md font-semibold"
        onClick={() => onKey("enter")}
      >
        <Icon as={CornerDownLeft} size={20} flip />
        {t("keypad_enter")}
      </Button>
    </div>
  );
}

/** How many dinars a pad can type before it stops taking digits. Nine is a
 * basket of a billion dinars, which no counter has ever rung up, and it keeps
 * the centimes inside the safe integers with room to spare. */
const MAX_DIGITS = 9;

/**
 * What a press does to the amount in a box, in centimes.
 *
 * A key types whole dinars, the way the counter mockup's pad does and the way
 * a shop says an amount out loud: 1, 5, 0, 0 is 1 500 DA and not 15,00 DA.
 * Centimes are typed in the box itself; a press after one has been typed
 * drops it rather than shifting it up, because a pad that turned 12,50 into
 * 125,05 on one key would be worse than one that starts from the dinars.
 *
 * An empty box is `null` and not zero, the same distinction `MoneyInput`
 * makes: backspacing the last digit leaves a cashier who has not counted the
 * notes yet, not one who was handed nothing.
 *
 * `enter` is not an amount; it gives the value back untouched and the screen
 * decides what validating means.
 */
export function keyedAmount(current: number | null, key: KeypadKey): number | null {
  if (key === "enter") return current;
  const dinars = current === null ? "" : String(Math.trunc(current / 100));
  if (key === "backspace") {
    const left = dinars.slice(0, -1);
    return left === "" || left === "-" ? null : Number(left) * 100;
  }
  const next = (dinars === "0" ? "" : dinars) + key;
  if (next.length > MAX_DIGITS) return current;
  return Number(next) * 100;
}

/**
 * What a press does to a string of digits typed on this pad: a user id, a
 * PIN. Not `keyedAmount`, on purpose. That one reads dinars and turns
 * leading zeros into nothing, which is exactly wrong for a PIN, where
 * `0512` is a different PIN from `512`; this one keeps every digit typed,
 * in order, up to `maxDigits`, and never turns the string into a number.
 *
 * `00` types two zeros, the same face as `0` pressed twice, rather than
 * being refused: the pad has no other way to type a second zero in a row
 * quickly and a PIN of `1200` is an ordinary one. `enter` and `backspace`
 * past empty behave as `keyedAmount`'s do, for the same reason: the wide
 * key is never a digit and there is nothing to erase from nothing.
 */
export function keyedDigits(current: string, key: KeypadKey, maxDigits: number): string {
  if (key === "enter") return current;
  if (key === "backspace") return current.slice(0, -1);
  const next = current + (key === "00" ? "00" : key);
  return next.length > maxDigits ? current : next;
}
