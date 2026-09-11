-- What a user needs before anybody can sign in as one (features.md §5).
--
-- `users` is not created here. It has existed since the first migration, with
-- its id, its shop, its name, its role and the role CHECK, and the shop's
-- owner is row 1 of it ('Propriétaire'), seeded beside the shop so that every
-- document, ledger row and audit entry has had an author since the first
-- sale. So nothing is backfilled and no existing column changes meaning: this
-- migration adds the columns signing in needs and leaves the rest alone.
--
-- Additive on purpose, five ADD COLUMNs and an index rather than the
-- twelve-step rebuild migrations 2, 6 and 8 used. Twelve tables name
-- `users(id)` in a foreign key and every one of their rows points at row 1;
-- rebuilding the table they point at, with the foreign keys off, to add a
-- column would be the largest possible way to do the smallest possible thing.

-- The password half of the pair. NULL, and only NULL, means "this user has no
-- password and signs in with a PIN": unlike `pin_hash` there is no sentinel,
-- because a NULL here is read in exactly one place and the service refuses a
-- password login against it. A till user never gets one.
ALTER TABLE users ADD COLUMN password_hash TEXT;

-- Deactivation, never deletion: documents, ledger rows and audit rows point
-- at this row for good, so a user who has left the shop is switched off and
-- the paper they wrote keeps its author. The CHECK is the products and
-- customers one, because STRICT only promises the column holds an integer.
ALTER TABLE users ADD COLUMN active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1));

-- A shop counter is a public place, so wrong PINs are counted (features.md
-- §5). Two columns on the row the check already reads rather than a table of
-- attempts: there is exactly one live value per user, it is cleared by the
-- next good PIN, and nothing reads its history. The history a person does
-- read is the audit log, which gets a row when a user locks out. A table
-- would grow a row per fat finger at a till and be pruned forever after.
ALTER TABLE users ADD COLUMN pin_failures INTEGER NOT NULL DEFAULT 0
    CHECK (pin_failures >= 0);
-- When the user may try again, UTC, NULL when they may try now. UTC and not
-- the shop's calendar: this is a delay measured in seconds and never a date
-- anybody reads off a document, and the two hands have to be on the same
-- clock or a lockout is an hour long by accident.
ALTER TABLE users ADD COLUMN locked_until TEXT;

-- Every other fiche carries one. It cannot take `DEFAULT (CURRENT_TIMESTAMP)`
-- the way `products.updated_at` and `customers.updated_at` do: SQLite refuses
-- a non-constant default on ADD COLUMN. The constant below exists only so the
-- column can be NOT NULL for the length of one statement; the UPDATE under it
-- puts every existing row on its own `created_at`, and the service stamps the
-- column on every write from there on, exactly as `services::customers` does.
-- A row can only read 1970 if something outside this app inserted it.
ALTER TABLE users ADD COLUMN updated_at TEXT NOT NULL DEFAULT '1970-01-01 00:00:00';
UPDATE users SET updated_at = created_at;

-- One name per shop. A user is picked off a list by name at a PIN pad, and
-- two 'Karim' rows on one till is a cashier signing in as the other one by
-- accident. An index rather than a table constraint because the table already
-- exists, and it is the shape `products` uses for its barcode anyway.
--
-- Exact text, not case-folded: SQLite's NOCASE only folds ASCII, so it would
-- tell 'karim' from 'Karim' and not 'Ali' from 'ALİ', and a rule that holds
-- for one alphabet of a trilingual app is worse than no rule. The service
-- trims what it is given; the rest is the owner's to look at.
CREATE UNIQUE INDEX idx_users_shop_name ON users (shop_id, name);
