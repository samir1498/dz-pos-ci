// A signed-in `MeDto`, for the tests that render a screen behind
// `SessionProvider` and never mean to exercise sign-in itself (that is
// `session.test.tsx`'s job). One file rather than one copy per screen test,
// the way `role.test.ts` bans a second copy of the three role names.
//
// `ME_OWNER` holds every permission there is: most screen tests were
// written before M4 T5 gated anything on one, so an owner is the session
// that keeps them showing what they always showed. A test that wants the
// cashier's narrower screen asks for `ME_CASHIER` instead.

import type { MeDto, PermissionDto } from "@dzpos/shared";

/** `Permission::ALL`, spelled on the TypeScript side (`crates/core/src/
 *  services/permissions.rs`). Kept as a literal array and not inferred from
 *  a union type, because a fixture is data and this file is not the place
 *  a new permission should fail to compile. */
const ALL_PERMISSIONS: readonly PermissionDto[] = [
  "sell",
  "discount_above_threshold",
  "override_credit_block",
  "see_cost_and_margin",
  "edit_fiches",
  "edit_settings",
  "see_reports",
  "manage_users",
  "commit_money",
  "correct_ledger",
  "export_and_import",
  "change_price_at_the_till",
  "see_audit_log",
  "open_and_close_till",
  "close_another_persons_till",
  "view_patients",
  "edit_patients",
  "view_patient_notes",
];

export const ME_OWNER: MeDto = {
  user_id: 1,
  name: "Yasmine",
  role: "owner",
  // `MeDto`'s permission list is a mutable array on the wire (ts-rs's own
  // `Array<T>`, not `ReadonlyArray<T>`), so the fixture hands out a copy of
  // the constant rather than the constant itself.
  permissions: [...ALL_PERMISSIONS],
};

/** `services::permissions::can`'s own table: a cashier holds `sell`,
 *  `open_and_close_till` and the clinic's two patient permissions, and
 *  nothing else. */
export const ME_CASHIER: MeDto = {
  user_id: 2,
  name: "Karim",
  role: "cashier",
  permissions: ["sell", "open_and_close_till", "view_patients", "edit_patients"],
};

/** Every permission `ME_OWNER` holds, minus `view_patient_notes` alone. A
 *  notes-permission test that reaches for `ME_CASHIER` instead only proves
 *  the field reacts to *some* difference between the two fixtures — a
 *  cashier is also missing `see_reports`, `manage_users` and a dozen
 *  others, so a bug that gated notes on the wrong permission could still
 *  pass. This isolates the one permission the notes field actually
 *  checks. */
export const ME_WITHOUT_PATIENT_NOTES: MeDto = {
  user_id: 3,
  name: "Nadia",
  role: "owner",
  // Filtered off `ALL_PERMISSIONS`, not off `ME_OWNER`'s own field:
  // `role.test.ts` bans that field's name appearing outside
  // `lib/session.tsx`, and this file already holds the one list both
  // fixtures are built from.
  permissions: ALL_PERMISSIONS.filter((permission) => permission !== "view_patient_notes"),
};
