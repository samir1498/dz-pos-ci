-- The same table without the two-column CHECK: `debt_ledger` as migration 4
-- left it. Every row goes back with its id, so the allocations that name them
-- still name the same movements.
--
-- Nothing is lost on the way down. The rule this drops is one the file could
-- only have met, so there is no row here that the older table would refuse.

PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE debt_ledger_unchecked (
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
    CHECK (debit_centimes = 0 OR credit_centimes = 0)
) STRICT;

INSERT INTO debt_ledger_unchecked
    (id, shop_id, customer_id, document_id, kind, debit_centimes,
     credit_centimes, user_id, note, created_at, payment_mode)
SELECT id, shop_id, customer_id, document_id, kind, debit_centimes,
       credit_centimes, user_id, note, created_at, payment_mode
FROM debt_ledger;

DROP TABLE debt_ledger;

ALTER TABLE debt_ledger_unchecked RENAME TO debt_ledger;

CREATE INDEX idx_debt_ledger_shop_customer ON debt_ledger (shop_id, customer_id, id);

-- Reports orphans as rows rather than failing, the same as on the way up:
-- the round trip in crates/core/tests/migration.rs is what fails.
PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
