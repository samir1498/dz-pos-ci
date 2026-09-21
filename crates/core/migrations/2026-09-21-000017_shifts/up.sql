-- A till opens with a float and closes with a count. One row per person per
-- stretch at the drawer: `opened_at` and the opening cash on the way in, and
-- on the way out the moment, the closer, what was counted and what the shop
-- expected. The difference is read as `counted - expected_at_close` from the
-- two stored figures, so nobody has to recompute an evening that is over.
--
-- `expected_at_close_centimes` is stored and not derived, which is the one
-- place this file departs from architecture.md rule 4's "the cash position is
-- derived on every read". The live position keeps being derived. A shift's
-- difference is a fact about one evening: a ticket annulled on Wednesday
-- changes what Monday derives, and Monday's count was signed on Monday. The
-- snapshot is what the closer saw.
--
-- No `cash_movements` and no `drawers` table. Cash handed to the owner is the
-- note at close; money paid out of the shop's own funds is an `expenses` row
-- written by a `commit_money` holder and enters no cashier's expected figure.

CREATE TABLE shifts (
    id                         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    -- RESTRICT, with `expenses` and the ledgers rather than with `sessions`:
    -- what a person counted at a drawer is a money record, and a DELETE that
    -- could empty it is not a record.
    shop_id                    INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    opened_by                  INTEGER NOT NULL REFERENCES users(id),
    -- The shop's clock (services::clock, UTC+1 all year), like `issued_at` on
    -- a document and unlike the UTC columns a delay is measured over. A shift
    -- boundary is read against sales the shop timestamps the same way, so the
    -- two hands have to be on one clock.
    opened_at                  TEXT NOT NULL,
    -- What was in the drawer when it opened. Zero is a real answer, a drawer
    -- starting empty, so the bound is `>= 0` and not `> 0`.
    opening_cash_centimes      INTEGER NOT NULL
        CHECK (typeof(opening_cash_centimes) = 'integer' AND opening_cash_centimes >= 0),

    closed_at                  TEXT,
    -- Its own column and not `opened_by` reused: a manager may close a shift
    -- its cashier walked away from, and the audit row names both.
    closed_by                  INTEGER REFERENCES users(id),
    -- What was counted in the drawer. Never negative: a count is notes and
    -- coins somebody held.
    counted_centimes           INTEGER
        CHECK (counted_centimes IS NULL
               OR (typeof(counted_centimes) = 'integer' AND counted_centimes >= 0)),
    -- What the shop expected that person to hold: their opening cash plus
    -- their own cash takings over the window. No sign bound, because a cash
    -- refund leaving the drawer subtracts from it and a drawer that paid out
    -- more than it took in is arithmetic and not an error.
    expected_at_close_centimes INTEGER
        CHECK (expected_at_close_centimes IS NULL
               OR typeof(expected_at_close_centimes) = 'integer'),
    note                       TEXT,

    -- A close is one event, so its four columns arrive together or not at
    -- all. Without this a row could carry a counted figure and no moment, and
    -- every reader downstream would have to decide for itself whether that
    -- row is open or closed.
    CONSTRAINT shifts_close_is_whole CHECK (
        (closed_at IS NULL AND closed_by IS NULL AND counted_centimes IS NULL
            AND expected_at_close_centimes IS NULL)
        OR (closed_at IS NOT NULL AND closed_by IS NOT NULL AND counted_centimes IS NOT NULL
            AND expected_at_close_centimes IS NOT NULL)
    ),

    -- A gap with no reason is the one row an owner cannot act on. The first
    -- arm lets every open shift through, the second lets a clean close
    -- through with no note, and only a close whose two figures disagree is
    -- made to say why. `counted = expected` is safe against NULL because the
    -- CHECK above has already forced the two to be non-null together.
    --
    -- `trim(note) <> ''` because a space is not a reason. The service reaches
    -- the column through `optional_field`, which trims and turns blank into
    -- NULL, so the only way a row of spaces arrives here is a writer that
    -- forgot, which is the case this line is the backstop for.
    CONSTRAINT shifts_a_difference_carries_a_reason CHECK (
        counted_centimes IS NULL
        OR counted_centimes = expected_at_close_centimes
        OR (note IS NOT NULL AND trim(note) <> '')
    )
) STRICT;

-- At most one open shift per person, and per person rather than per shop:
-- two cashiers hold overlapping shifts on purpose, because each one's cash is
-- physically their own. A partial index, so the closed rows of a person who
-- has worked every day for a year do not collide with each other.
CREATE UNIQUE INDEX idx_shifts_one_open_per_user
    ON shifts (shop_id, opened_by) WHERE closed_at IS NULL;

-- The shift list reads a shop's shifts newest first over a stretch of days.
CREATE INDEX idx_shifts_shop_opened_at ON shifts (shop_id, opened_at);

-- The expense clock, the same shift migration 000013 made for the audit log
-- and for the same reason. `expenses.created_at` took the column's own
-- default, SQLite's CURRENT_TIMESTAMP, which is UTC; the shop runs on UTC+1
-- with no daylight saving, so an expense filed at 00:30 in Algiers was stored
-- as 23:30 the day before and read back an hour early by anything that reads
-- an expense by the hour. `expense_date`, which the month list and the cash
-- position filter on, is a separate column the service has always written
-- itself, so no figure on a screen moves here.
--
-- `services::expenses::create` stamps the column from services::clock from
-- now on, which is why this shift is a one-off and has to travel in the same
-- version as that change: run against a build that already stamps, it would
-- move those rows an hour into the future.
UPDATE expenses SET created_at = datetime(created_at, '+1 hour');

-- The default stays on the column and can no longer fire: ExpenseRowWrite
-- carries created_at as a plain NaiveDateTime, so there is no insert that
-- omits it. Dropping a default in SQLite means rebuilding the table, which is
-- a bigger risk than the one it removes.
