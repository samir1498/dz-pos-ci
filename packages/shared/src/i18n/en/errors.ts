// What a refusal from the shop's core says to the person holding the phone.
// One sentence each, and none of them names a status code except the one
// that has nothing better to say.

import type { Messages } from "../message";

export const errors = {
  error_sign_in_again: "Session expired, sign in again.",
  error_pair_again: "This phone is no longer paired. Ask a manager for a new QR.",
  error_wrong_secret: "That is not the right PIN or password.",
  error_not_allowed: "You are not allowed to do that.",
  error_refused: "Refused ({status}).",
  error_no_answer: "No answer from the till computer. Check the Wi-Fi and try again.",
} as const satisfies Messages;
