import type { RoleDto } from "@dzpos/shared";

import type { Key } from "@/i18n";

/**
 * The three roles, spelled once for the whole app.
 *
 * `RoleDto` is a generated union with no runtime value behind it, the way
 * `@dzpos/design` exports `THEMES` for themes, so somebody has to write the
 * names down. This is that somebody. Before it existed they were written in
 * the topbar, again in the staff screen and again in `role.test.ts`, and a
 * fourth role added to the wire would have reached the topbar's label and
 * quietly not reached the staff screen's picker.
 *
 * Neither of these is a comparison: a screen looks a role up here and never
 * asks which one it is. `role.test.ts` holds every other file to that.
 */
const LABEL: Readonly<Record<RoleDto, Key>> = {
  owner: "role_owner",
  manager: "role_manager",
  cashier: "role_cashier",
};

/**
 * The keys of a record, still typed as its keys. `Object.keys` widens to
 * `string[]`, and the workspace bans an `as` assertion to narrow it back, so
 * the narrowing is a predicate over a real runtime check instead.
 */
const keysOf = <K extends string>(record: Readonly<Record<K, unknown>>): K[] =>
  Object.keys(record).filter((key): key is K => key in record);

/** A role's translation key. The compiler refuses a role with no label. */
export const ROLE_LABEL = LABEL;

/**
 * The roles a screen may offer. Read off the labels rather than written a
 * second time, so a role that has a label is a role the staff screen offers.
 */
export const ROLES: readonly RoleDto[] = keysOf(LABEL);

/**
 * What a person is when the staff screen opens its "add someone" form.
 *
 * The least a person can hold, deliberately: whoever is adding staff can
 * raise it in the same dialog, and the cost of forgetting is a cashier who
 * has to ask rather than a cashier who can read the shop's margins.
 */
export const DEFAULT_ROLE: RoleDto = "cashier";
