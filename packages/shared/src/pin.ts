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
