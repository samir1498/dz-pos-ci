// The shape of a PIN, mirrored from `services::users::PIN_DIGITS` so the
// desktop dialog, the sign-in pad and the phone can draw the boxes and refuse
// a short one before the round trip. The server still decides: a run (1234)
// or a repeat (1111) is refused there and is not this file's business.

/** How many digits a PIN has. One length, not a range: a range has to be
 *  explained on every screen that takes a PIN, and a fixed one is what lets
 *  the pad draw four boxes. */
export const PIN_DIGITS = 4;

/** Exactly `PIN_DIGITS` digits, nothing else. */
export const PIN_SHAPE = /^\d{4}$/;

/**
 * What is wrong with a PIN before it is sent, or `null`. Mirrors
 * `services::users::validate_pin`: `"shape"` is not four digits, `"weak"`
 * is one digit repeated (1111) or a count up or down (1234, 4321), the two
 * PINs anybody tries first. Mirrored so the dialog can say which before the
 * round trip; the server still decides and refuses the same two.
 */
export function pinProblem(pin: string): "shape" | "weak" | null {
  if (!PIN_SHAPE.test(pin)) return "shape";
  const digits = Array.from(pin, (c) => Number(c));
  const steps = digits.slice(1).map((d, i) => d - (digits[i] ?? 0));
  if (steps.every((step) => step === 0)) return "weak";
  if (steps.every((step) => step === 1) || steps.every((step) => step === -1)) return "weak";
  return null;
}
