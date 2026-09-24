-- A purchase gets a number of its own (plan shop-manual-test-findings T45):
-- BA-2026-000001, gapless inside the shop's year, the way every document
-- series counts (features.md §1, Purchase; §3, Numbering). Until now an order
-- had only its id, and the supplier's own delivery-note number was the only
-- number on the screen, which read like ours.
--
-- The number comes out of the counters table under `purchase:<year>`, the
-- same `take_next` a ticket's comes out of, inside the transaction that
-- writes the order: a save that fails rolls the counter back with it, so a
-- refused order burns no number.
--
-- The year is the year of `purchase_date`, the day on the shop's calendar
-- the order is dated, and never `created_at`, which is UTC and would put an
-- order written at 00:30 on 1 January into the year before.
--
-- Two columns and not three: the series key is `purchase:` and the year, so
-- storing it beside `series_year` would be one fact written twice.
--
-- The DEFAULT 0 is for the INSERT statements written before this file, and
-- zero means "no number": the unique index below skips it, and the repo
-- refuses to write a purchase whose number is below one. A CHECK of
-- `number >= 1` cannot be added here: SQLite tests an added column's CHECK
-- against the rows already there, which all hold the default at that point.
ALTER TABLE purchases ADD COLUMN series_year INTEGER NOT NULL DEFAULT 0
    CHECK (typeof(series_year) = 'integer' AND series_year >= 0);
ALTER TABLE purchases ADD COLUMN number INTEGER NOT NULL DEFAULT 0
    CHECK (typeof(number) = 'integer' AND number >= 0);

-- The orders already on the file, numbered in the order they were written
-- (their id) inside the year they are dated.
UPDATE purchases SET series_year = CAST(substr(purchase_date, 1, 4) AS INTEGER);
UPDATE purchases
SET number = (
    SELECT COUNT(*)
    FROM purchases p
    WHERE p.shop_id = purchases.shop_id
      AND p.series_year = purchases.series_year
      AND p.id <= purchases.id
);

-- The counters carry on from the backfill, so the next order of a year the
-- file already has is the one after its last, and not a second number 1.
INSERT INTO counters (shop_id, name, next_value)
SELECT shop_id, 'purchase:' || series_year, MAX(number) + 1
FROM purchases
GROUP BY shop_id, series_year;

CREATE UNIQUE INDEX idx_purchases_series_number
    ON purchases (shop_id, series_year, number)
    WHERE number > 0;
