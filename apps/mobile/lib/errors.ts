// What the phone says for a code the shop's core sent.
//
// The server sends a code, never a sentence: its `message` is a Rust
// `Display` string and it is English, always. The desktop settled this shape
// at `apps/desktop/src/lib/fields.tsx`, where a code picks a key and the
// server's own prose never reaches a screen. This is the same table for the
// phone, which until now had only the four credential cases and a sentence
// naming a status number for everything else.
//
// The table is complete on purpose, against the server's own two lists
// rather than against a guess at what a till screen can provoke. Any core
// code can arrive with a 422, because `ApiError::Request` maps whatever it
// wraps to that status and keeps the wrapped code, so a subset would have
// been a subset of the wrong thing. `errors.test.ts` walks the two Rust
// files and fails on a code that has no line here.

import type { Key } from "@dzpos/shared";

/** The sentence for a code, or nothing when the code is not one of ours.
 *
 *  Several codes share `error_till_problem`. Those are the ones where the
 *  server is reporting its own fault or a request this app built wrong: a
 *  workbook that will not write, a template that will not render, a route
 *  that does not take this method. A cashier can do nothing about any of
 *  them and telling them apart on a phone would be nine sentences nobody
 *  reads. The code is still in the table, so the walk cannot go green on a
 *  code nobody thought about. */
export const ERROR_KEY = {
  // The credential cases. `outcomeOf` decides these in its own branches,
  // because each one also decides what the phone throws away, but it reads
  // the sentence off this table rather than writing it out again: two
  // tables that have to agree are what the desktop spent eleven files
  // learning not to keep.
  auth_refused: "error_wrong_secret",
  device_refused: "error_pair_again",
  session_required: "error_sign_in_again",
  unauthorized: "error_sign_in_again",
  forbidden: "error_not_allowed",

  // The ones a cashier can act on.
  validation: "error_validation",
  // These two carry figures on the wire that the phone drops: the server
  // sends the limit and the balance after with `credit_limit`, and the list
  // of missing identifiers with `party_ids` (docs/features.md, the credit
  // and facture rows), while `errorIn` in `lib/api.ts` reads only `code` and
  // `message` and `Say` has no slot to put them in. No shop sees this today
  // because the till rings cash and `useRing` sends no `customer_id`, so
  // neither code can be reached from this app; the sentence is here so the
  // walk stays complete, and the figures are the first thing to add the day
  // the phone sells on credit.
  credit_limit: "error_credit_limit",
  party_ids: "error_party_ids",
  not_found: "error_not_found",
  duplicate_barcode: "error_duplicate_barcode",
  // The only `conflict` this app can provoke is the retry key: both raisers
  // in `services/sales.rs` are the idempotency table, and the other two in
  // the core are a supplier name and a user name, neither of which the phone
  // creates. So the sentence says the sale is already rung rather than "try
  // again", which is what the generic wording said and what would have had a
  // cashier press Ring a second time. `useRing` mints a fresh key on every
  // press, so that second press is a second sale, not a retry. The key is
  // named for the sale rather than for the code, because the sentence under
  // it is about a sale and this dictionary is in a shared package; the
  // desktop keeps its own generic `error_conflict` in `src/i18n`, where a
  // duplicate supplier name still lands.
  conflict: "error_sale_already_rung",
  exhausted: "error_exhausted",
  locked_out: "error_locked_out",

  // The server's own faults, and requests this app built wrong.
  money: "error_till_problem",
  storage: "error_till_problem",
  print: "error_till_problem",
  workbook: "error_till_problem",
  bad_request: "error_till_problem",
  ungated_write: "error_till_problem",
  method_not_allowed: "error_till_problem",
  restart_needed: "error_till_problem",
  restore_failed_restart_needed: "error_till_problem",
} as const satisfies Record<string, Key>;

/** The same table, seen as a plain lookup. `ERROR_KEY` keeps its literal
 *  types so `outcome.ts` can name a credential's key straight off it and
 *  there is one table rather than two that have to agree; this widening is
 *  what lets a code that arrived over the wire be looked up at all. */
const BY_CODE: Readonly<Record<string, Key>> = ERROR_KEY;

/** The key for a code the server sent, or null when there is none.
 *
 *  Null rather than a fallback key: the caller knows the status and can say
 *  something with it, and swallowing an unknown code into a general sentence
 *  is how a real refusal came back as "something went wrong" on the desktop
 *  for as long as eleven files each kept their own half of this table. */
export function errorKey(code: string | null | undefined): Key | null {
  if (typeof code !== "string") return null;
  return BY_CODE[code] ?? null;
}
