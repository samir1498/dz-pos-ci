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
  error_validation: "The till computer would not accept that. Check what you typed.",
  error_not_found: "That is not there any more. Ask a manager to refresh the till.",
  error_duplicate_barcode: "Another product already has that barcode.",
  error_sale_already_rung: "That sale already went through. Do not ring it again, check the till computer.",
  error_exhausted: "The numbers for this document have run out. Tell a manager.",
  error_credit_limit: "This sale would take the customer past their credit limit.",
  error_party_ids: "This customer is missing an identifier a facture needs.",
  error_locked_out: "Too many wrong tries. Wait a moment and try again.",
  error_till_problem: "Something went wrong on the till computer. Tell a manager.",
} as const satisfies Messages;
