// The scanner, wired to a screen.
//
// Two jobs, and they are separate on purpose. `src/lib/scan.ts` decides what
// a burst of keys means and knows nothing about the DOM; this file gives it
// a clock and an element to aim at.
//
// Aiming matters. A listener that only watched would recognise a scan while
// its characters landed in whatever field had focus, so a code scanned while
// the cashier was in the amount box would add the product and leave thirteen
// digits in the amount. Instead this moves focus to the box the scan belongs
// in, before the character is inserted, and only when focus is somewhere
// that is not a text field. A scan started inside another text field is left
// alone: it is the cashier's slip, and guessing is worse than not.
//
// One listener reads every key, on `window` and in the capture phase, and
// that is not decoration. Reading them on the box instead loses the first
// character of a scan that began while a button had focus, because the key
// that moves focus is dispatched before the box exists to receive it, and a
// code one digit short finds nothing. Capture also puts this ahead of the
// screen's own handler, so a terminator that ended a burst can be marked
// as spent before anything else reads it as Enter.

import { useCallback, useEffect, useRef } from "react";

import { EMPTY, SCAN_IDLE_MS, type ScanState, feed } from "@/lib/scan";

/** A character a barcode can be made of. Anything else is not worth moving
 * focus for, and Space actively must not be: see the call site. */
const CODE_CHARACTER = /^[0-9A-Za-z]$/;

/** Fields where the cashier is typing on purpose. A scan is never taken out
 * of one of these, and never redirected away from one. */
function isTextField(element: Element | null): boolean {
  if (element === null) return false;
  const tag = element.tagName;
  if (tag === "TEXTAREA") return true;
  if (element instanceof HTMLElement && element.isContentEditable) return true;
  if (!(element instanceof HTMLInputElement)) return false;
  // A checkbox or a radio takes no characters, so a scan may land on one.
  return element.type !== "checkbox" && element.type !== "radio";
}

/**
 * Recognises a scan wherever it starts, and sends its characters to
 * `target`.
 *
 * `onScan` is called with the code once the burst ends, on Enter, on Tab, or
 * after `SCAN_IDLE_MS` of quiet for the models that terminate with nothing.
 * When it fires on a terminator, that key's default is prevented: the same
 * Enter must not also be read as "the one visible product, add it".
 *
 * `enabled` is false while the screen is locked or a dialog owns the
 * keyboard, and then nothing here touches focus.
 */
export function useScanner({
  target,
  onScan,
  enabled = true,
}: {
  target: React.RefObject<HTMLInputElement | null>;
  onScan: (code: string) => void;
  enabled?: boolean;
}): void {
  const state = useRef<ScanState>(EMPTY);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const onScanRef = useRef(onScan);
  onScanRef.current = onScan;

  const clearTimer = useCallback(() => {
    if (timer.current !== null) {
      clearTimeout(timer.current);
      timer.current = null;
    }
  }, []);

  // A burst with no terminator ends in silence, which only a timer can
  // notice. It is restarted on every character, so it fires once, after the
  // last one.
  const armIdle = useCallback(() => {
    clearTimer();
    timer.current = setTimeout(() => {
      timer.current = null;
      const step = feed(state.current, { type: "idle", at: performance.now() });
      state.current = step.state;
      if (step.scan !== null) onScanRef.current(step.scan);
    }, SCAN_IDLE_MS);
  }, [clearTimer]);

  useEffect(() => clearTimer, [clearTimer]);

  useEffect(() => {
    if (!enabled) {
      // A burst can be half-fed when the till locks or a dialog opens. The
      // listener goes, but an armed timer would still fire up to
      // SCAN_IDLE_MS later and add a product behind the lock screen, so it
      // goes too, and the half-burst with it.
      clearTimer();
      state.current = EMPTY;
      return undefined;
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.ctrlKey || event.metaKey || event.altKey) return;
      const active = document.activeElement;
      // Somebody is typing somewhere on purpose. Not ours.
      if (active !== target.current && isTextField(active)) return;

      const step = feed(state.current, {
        type: "key",
        key: event.key,
        code: event.code,
        // `performance.now()` and not `event.timeStamp`: the first is what a
        // test can move, and a burst measured in tenths of a millisecond
        // needs to be movable.
        at: performance.now(),
      });
      state.current = step.state;

      if (step.scan !== null) {
        clearTimer();
        // The terminator belongs to the scan. Left alone, this same Enter
        // would also reach the screen's own handler and add a second line.
        event.preventDefault();
        onScanRef.current(step.scan);
        return;
      }

      // A character that began somewhere harmless belongs in the box, and
      // it has to be moved there while the key is still on its way: focus
      // set now is where the browser inserts it.
      //
      // Only a character a code could be made of. Space used to qualify,
      // and moving focus on it broke every button on the screen: a button
      // fires its click on Space's *keyup*, which by then was landing on
      // the search box, so the button did nothing and the box got a space.
      if (CODE_CHARACTER.test(event.key) && !event.repeat && active !== target.current) {
        target.current?.focus();
      }
      armIdle();
    }
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [armIdle, clearTimer, enabled, target]);
}
