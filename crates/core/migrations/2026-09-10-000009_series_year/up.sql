-- The year a document's series counts in (features.md §4, Numbering).
--
-- Every series restarts at 1 each year and the printed number carries the
-- year: FA-2026-000001. That is the common practice in Algeria and not a rule
-- of décret 05-468, which asks only that a series be uninterrupted; Samir took
-- the decision on 2026-09-10 and the comptable (R8) is being asked to confirm
-- the practice, not the choice.
--
-- The year is the shop's, never the machine's. `issued_at` is already written
-- on the shop's calendar (`services::clock::now` reads UTC+1 before storing),
-- so `strftime('%Y', issued_at)` is the shop's year with no offset to apply
-- here. `created_at` is the column default's UTC and would put a document
-- issued at 00:30 on 1 January into the year before.
--
-- Three statements and not one, in this order, because two of them read what
-- the one before them left:
--
-- 1. The column, additive with a DEFAULT so every seeder and every INSERT
--    written before this migration still compiles and still runs.
-- 2. The backfill, off `issued_at`.
-- 3. The counter rows, renamed to the key the code now asks for
--    (`doc_facture:2026`), while `documents.series` still spells the old key
--    they are matched by.
-- 4. `documents.series`, rewritten to the same key.
--
-- Steps 3 and 4 are what keeps this file from printing one number twice. The
-- code takes its numbers from `doc_facture:<year>`; a counter left named
-- `doc_facture` would be a series nobody reads any more, and the new one would
-- start at 1 beside a facture already holding number 1 with the same printed
-- form. Renaming the counter to the year of the newest document of that series
-- carries the count on instead: the first facture after this migration is
-- N + 1 of its year, and the UNIQUE (shop_id, series, number) that migration 7
-- rebuilt is the thing proving no number is handed out twice.
--
-- What the carry-over does not do is renumber a document that has been handed
-- to a customer. A shop that issued tickets 1 to 40 across 2025 and 2026 comes
-- out of this file with a 2025 series holding the ones it holds and a 2026
-- series starting wherever the year turned, so the year this migration runs in
-- is the one year whose series may not begin at 1. Every year after it does.
--
-- A counter of a series no document has used yet is left alone: there is no
-- year to name it after, and the code creates the row it needs on first use.
-- `in_store_barcode` is left alone for the same reason it always is: it is not
-- a document series and it does not restart (features.md §1).
--
-- Four added or rewritten values and no rebuilt table, so this migration runs
-- inside the transaction diesel opens for it and opens none of its own.
--
-- What going down costs. `down.sql` puts the series strings and the counter
-- names back the way it found them and drops the column, and a file this
-- migration converted goes down and comes back up unchanged: the years it
-- reads off `issued_at` the second time are the ones it read the first.
--
-- What it cannot do is come down over a January. Once a series has restarted,
-- the file holds two counters (`doc_ticket:2026` and `doc_ticket:2027`) that
-- the down would fold onto the one name `doc_ticket`, and two documents both
-- numbered 1 whose series strings it would fold to the same string. The first
-- collides with `counters`' PRIMARY KEY (shop_id, name) and the second with
-- `documents`' UNIQUE (shop_id, series, number), so the revert fails loudly
-- and writes nothing rather than losing one of the two. That is the honest
-- outcome: the old scheme has no way to hold two years, and a migration is
-- reverted by hand, on purpose, by somebody who then has this file open.
--
-- Where the CHECK goes. The series string and `series_year` are one fact
-- stored twice and nothing at this level stops them drifting: a table-level
-- CHECK cannot be added to a table SQLite has already created, so
-- `repos::documents::insert` refuses a row whose series does not end in its
-- year. The next migration that rebuilds `documents` (the way migration 7
-- rebuilt it) should carry
--   CHECK (series = kind_series_stem || ':' || series_year)
-- spelled against whatever the stem column is by then, and the guard in the
-- repo becomes the belt beside that brace.

-- 1. The year, on every document. Zero on a row nothing has backfilled yet,
-- which after the UPDATE below is no row at all; the DEFAULT is there for the
-- INSERT statements written before this file existed.
ALTER TABLE documents ADD COLUMN series_year INTEGER NOT NULL DEFAULT 0;

-- 2. The shop's year of issue.
UPDATE documents SET series_year = CAST(strftime('%Y', issued_at) AS INTEGER);

-- 3. The counters, before the series strings they are matched by change.
UPDATE counters
SET name = name || ':' || (
        SELECT MAX(d.series_year)
        FROM documents d
        WHERE d.shop_id = counters.shop_id AND d.series = counters.name
    )
WHERE name LIKE 'doc\_%' ESCAPE '\'
  AND EXISTS (
        SELECT 1
        FROM documents d
        WHERE d.shop_id = counters.shop_id AND d.series = counters.name
    );

-- 4. The series each document was numbered in, now carrying its year.
UPDATE documents SET series = series || ':' || series_year;
