-- `payment_mode` belongs to a payment and to nothing else (features.md §2).
--
-- Migration 4 added the column and wrote the rule in a comment, because the
-- only CHECK an `ALTER TABLE ... ADD COLUMN` can carry is one that reads the
-- new column alone: it could say the mode is cash or card or nothing, and it
-- could not say which rows are allowed to have one. That left the row the
-- comment forbids and the table accepted: an `opening`, a `sale`, an `avoir`
-- or an `adjustment` stamped 'cash', which reads as money that was handed
-- over and never was.
--
-- The rule reads two columns at once, so in SQLite it is a table-level CHECK,
-- and a table-level CHECK cannot be added to a table that exists: the table
-- is rebuilt. Nothing else about it changes. Same columns in the same order,
-- same per-column checks, same index.
--
-- Not the stronger rule that every payment carries a mode. A payment is
-- money handed over in something today, and `pay` always says which; but a
-- settlement out of credit the customer was already holding is money nobody
-- handed over, and a table that demanded a mode there would be asking for a
-- 'cash' nobody counted.
--
-- Additive in what it holds: every movement keeps its id, because
-- `debt_allocations.payment_ledger_id` points at those ids and an id that
-- moved would say a facture was settled by somebody else's payment.
--
-- Nothing this app has written can fail the new rule. `services::debt::pay`
-- is the one writer that fills the mode in and `append_at` writes NULL on
-- every other kind, so the rebuild's INSERT is a copy and not a repair. What
-- the CHECK is for is the file that came from somewhere else: a restored
-- backup, a row repaired by hand, an import.

-- Step 1 of the rebuild, and it has to be outside a transaction: a pragma
-- inside one is a no-op. `metadata.toml` tells diesel not to wrap this file.
-- With the foreign keys off, `DROP TABLE debt_ledger` below does not fail on
-- the allocations that point into it.
PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE debt_ledger_checked (
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id         INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    customer_id     INTEGER NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
    document_id     INTEGER REFERENCES documents(id) ON DELETE SET NULL,
    kind            TEXT NOT NULL CHECK (kind IN
                        ('opening', 'sale', 'payment', 'avoir', 'adjustment')),
    debit_centimes  INTEGER NOT NULL
        CHECK (typeof(debit_centimes) = 'integer' AND debit_centimes >= 0),
    credit_centimes INTEGER NOT NULL
        CHECK (typeof(credit_centimes) = 'integer' AND credit_centimes >= 0),
    user_id         INTEGER NOT NULL REFERENCES users(id),
    note            TEXT,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    payment_mode    TEXT
        CHECK (payment_mode IS NULL OR payment_mode IN ('cash', 'card')),
    -- One direction per row, as migration 2 wrote it.
    CHECK (debit_centimes = 0 OR credit_centimes = 0),
    -- And the mode belongs to a payment and to nothing else, which is what
    -- migration 4's own comment says the column means and what an ALTER
    -- TABLE could not say.
    CHECK (payment_mode IS NULL OR kind = 'payment')
) STRICT;

-- Ids carried over rather than reassigned, the way the documents rebuild
-- carried its own: `debt_allocations.payment_ledger_id` names these rows.
INSERT INTO debt_ledger_checked
    (id, shop_id, customer_id, document_id, kind, debit_centimes,
     credit_centimes, user_id, note, created_at, payment_mode)
SELECT id, shop_id, customer_id, document_id, kind, debit_centimes,
       credit_centimes, user_id, note, created_at, payment_mode
FROM debt_ledger;

DROP TABLE debt_ledger;

ALTER TABLE debt_ledger_checked RENAME TO debt_ledger;

-- The index goes with the table, so it is written again. The id closes it
-- because a balance and a statement both read one customer's rows in the
-- order they were written, and two rows can land inside the same second.
CREATE INDEX idx_debt_ledger_shop_customer ON debt_ledger (shop_id, customer_id, id);

-- With the keys off, nothing above was checked. The pragma reports orphans as
-- rows and never fails a statement, so it cannot stop this migration on its
-- own; it is here for a person running the file by hand. The guard is
-- crates/core/tests/migration.rs, which queries pragma_foreign_key_check
-- after migrating and fails if it returns anything.
PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
