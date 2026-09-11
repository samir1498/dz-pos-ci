// Applies the one thing missing from a fresh e2e database before any spec
// can sign in: a real credential on the seeded owner row. Runs after
// `webServer` in playwright.config.ts is confirmed healthy (Playwright's
// own order, the canonical place for post-boot auth setup), so the API has
// already migrated `tempDb` into an empty shop with that row in it —
// `crates/core/migrations/2026-09-08-000000_init` seeds user id 1 with the
// sentinel `pin_hash = '!unset'` and a `NULL` `password_hash`, which is
// refused by every login attempt, PIN or password, until this runs.
//
// The two hashes below are not computed here, and this does not run
// `dzpos-seed`: that binary is the only thing in the workspace that can
// turn a PIN or a password into an argon2id hash without an authenticated
// session, and it also fills a month of trading data, which the products
// suite's first assertion needs the shop to not have. So the hashes were
// taken once, by hand, off a throwaway `.dev/` database (`just seed`, then
// `just seed-clean` to discard it, never touching the real e2e database),
// and pasted here the way a developer reads `dzpos-seed`'s own printed PIN
// and password off its terminal. Verifying an argon2id hash reads its cost
// and salt back out of the string itself, so a hash made once verifies
// "1379" and "developpement" forever, on any machine, with dzpos-seed never
// running against tempDb.
//
// `node:sqlite` (Node 22.5+, no external dependency) rather than the
// `sqlite3` CLI or a new npm package: it is already on this box and reads
// and writes the same file format the API's WAL-mode connection does,
// without a second connection ever holding the file exclusively (`crates/
// core/src/db.rs::open` sets `journal_mode=WAL`, not `locking_mode=
// EXCLUSIVE`), which is what makes writing into a file the API already has
// open safe to begin with.

import { DatabaseSync } from "node:sqlite";

import { tempDb } from "../playwright.config";
import { OWNER_NAME } from "./auth";

/** `crates/core/src/services/seed.rs::OWNER_PIN` ("1379"), hashed once
 * against a throwaway `.dev/dev.db`. */
const PIN_HASH =
  "$argon2id$v=19$m=19456,t=2,p=1$IKWBOzpEx3OYWOJH3lio9A$dBsP62MmpI6DuHR4C7Jlhofet+Hh+mq4jc8sfIcOrCI";
/** `crates/core/src/services/seed.rs::OWNER_PASSWORD` ("developpement"),
 * hashed the same way. */
const PASSWORD_HASH =
  "$argon2id$v=19$m=19456,t=2,p=1$ewVtC07MFQKo36KiqheHpA$5QSrgQXgO94Qoiw9HaU9zgSsXfgkThgi1Q46VukOcUU";

export default function globalSetup(): void {
  const db = new DatabaseSync(tempDb);
  try {
    // A brief wait rather than an immediate refusal if the API's own
    // connection is mid-write when this opens: WAL lets a reader and a
    // writer overlap, but two writers still take turns.
    db.exec("PRAGMA busy_timeout = 5000;");
    const result = db
      .prepare(
        "UPDATE users SET pin_hash = ?, password_hash = ? WHERE shop_id = 1 AND id = 1 AND name = ?",
      )
      .run(PIN_HASH, PASSWORD_HASH, OWNER_NAME);
    if (result.changes !== 1) {
      throw new Error(
        `e2e/globalSetup.ts: expected to update exactly one row (shop 1, user 1, "${OWNER_NAME}") in ${tempDb}, updated ${result.changes}. Did the seeded owner row's name or id change?`,
      );
    }
  } finally {
    db.close();
  }
}
