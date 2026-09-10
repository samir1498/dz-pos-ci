-- How the app looks, per shop. Preferences, not settings.
--
-- `settings` beside this table is a dated series: a row is valid from its
-- `valid_from` until the next one, and a document issued last year still
-- reads the régime fiscal that was current then. That is the right shape for
-- a fiscal fact and the wrong shape for a colour scheme, which has no history
-- anybody reads and would append a row every time somebody tried a theme on.
--
-- So this one is written over in place: one row per (shop, key), the current
-- value and when it was last changed. Nothing here is ever read by a
-- document.
--
-- No CHECK on the theme names. `settings` carries one for `regime_fiscal`
-- because a régime the code does not know is a fiscal fact nobody can act on
-- and the file should refuse it. A theme is a stylesheet; the service parses
-- the value and a name it does not know is answered as "no preference", so
-- adding the fifth theme is a stylesheet and a string, not a migration.
CREATE TABLE preferences (
    id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    UNIQUE (shop_id, key)
) STRICT;
