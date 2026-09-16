// The shop-initiated update check and install (M5 T7, docs/architecture.md
// § Release). The only file that knows `invoke()` reaches an updater
// command at all, the same reason `api.ts` is the one file that knows
// `launch_token` does: `settings.about.tsx` and its test read a plain
// three-answer value, never Tauri's IPC shape directly.
//
// A browser preview (`pnpm dev`, `just e2e`) has no window running the
// updater plugin behind it, so `isTauri()` guards both calls the same way
// `api.ts` guards the launch token: the About screen there answers
// "unreachable" without ever calling `invoke`, which is also the honest
// answer for a machine with no update channel at all.

import { invoke, isTauri } from "@tauri-apps/api/core";

/** Mirrors `UpdateCheck` in `src-tauri/src/updater.rs`, kept in sync by hand
 * against `the_three_answers_serialize_to_the_shape_the_frontend_reads` on
 * that side; `settings.about.test.tsx`'s `UpdateCheckPanel` tests exercise
 * all three shapes through this module's mock, so a drift here shows up as
 * a failing screen test even though it mocks past the wire itself. `size`
 * is bytes, and is `null` unless the release workflow wrote one into the
 * manifest: the plugin's own format carries no byte count before a shop
 * has already agreed to download (docs/architecture.md § Release). */
export type UpdateCheck =
  | { kind: "newest" }
  | { kind: "newer"; version: string; size: number | null }
  | { kind: "unreachable" };

export async function checkForUpdate(): Promise<UpdateCheck> {
  if (!isTauri()) return { kind: "unreachable" };
  try {
    return await invoke<UpdateCheck>("check_for_update");
  } catch {
    return { kind: "unreachable" };
  }
}

/** Downloads, verifies and installs, then asks the app to restart. Resolves
 * only if something went wrong before the restart was requested; a
 * successful install is the process ending, not this promise settling. */
export async function installUpdate(): Promise<void> {
  await invoke("install_update");
}

/** One decimal place, one unit, always left-to-right: the same rule
 * `about-version`, `about-git-hash` and `about-build-date` already follow
 * in `settings.about.tsx`, because a byte count is a number and not a
 * word that changes with the shop's language. */
export function formatUpdateSize(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
