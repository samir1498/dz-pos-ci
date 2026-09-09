-- The same rebuild the other way, back to the `documents` migration 2 wrote,
-- so `revert_last_migration` walks the file back to a schema the earlier
-- migrations would have built and the round trip in
-- crates/core/tests/migration.rs can prove each down.sql undoes its own
-- up.sql and nothing else.
--
-- A document that had been given a buyer or a balance loses them here: that
-- is what reverting a migration means, and it is why a revert is a
-- development move and never something a shop's file goes through.
PRAGMA foreign_keys = OFF;

BEGIN;

-- The ledgers go first: both point at customers, and one points at
-- documents.
DROP TABLE debt_allocations;
DROP TABLE debt_ledger;

-- `documents` as migration 2 wrote it, character for character.
CREATE TABLE documents_without_customers (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id              INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    kind                 TEXT NOT NULL CHECK (kind IN
                             ('ticket', 'facture', 'proforma', 'bon_de_livraison',
                              'avoir', 'bon_de_reception')),
    series               TEXT NOT NULL,
    number               INTEGER NOT NULL
        CHECK (typeof(number) = 'integer' AND number >= 1),
    issued_at            TEXT NOT NULL,
    user_id              INTEGER NOT NULL REFERENCES users(id),
    regime               TEXT NOT NULL CHECK (regime IN ('ifu', 'reel')),
    payment_mode         TEXT NOT NULL CHECK (payment_mode IN
                             ('cash', 'card', 'credit', 'cheque', 'transfer')),
    seller_name          TEXT NOT NULL,
    seller_rc            TEXT,
    seller_nif           TEXT,
    seller_nis           TEXT,
    seller_ai            TEXT,
    seller_address       TEXT,
    seller_phone         TEXT,
    customer_id          INTEGER,
    total_ht_centimes    INTEGER NOT NULL
        CHECK (typeof(total_ht_centimes) = 'integer' AND total_ht_centimes >= 0),
    discount_centimes    INTEGER NOT NULL
        CHECK (typeof(discount_centimes) = 'integer' AND discount_centimes >= 0),
    subtotal_ht_centimes INTEGER NOT NULL
        CHECK (typeof(subtotal_ht_centimes) = 'integer' AND subtotal_ht_centimes >= 0),
    tva_centimes         INTEGER NOT NULL
        CHECK (typeof(tva_centimes) = 'integer' AND tva_centimes >= 0),
    total_ttc_centimes   INTEGER NOT NULL
        CHECK (typeof(total_ttc_centimes) = 'integer' AND total_ttc_centimes >= 0),
    stamp_centimes       INTEGER NOT NULL
        CHECK (typeof(stamp_centimes) = 'integer' AND stamp_centimes >= 0),
    net_to_pay_centimes  INTEGER NOT NULL
        CHECK (typeof(net_to_pay_centimes) = 'integer' AND net_to_pay_centimes >= 0),
    tendered_centimes    INTEGER
        CHECK (tendered_centimes IS NULL
               OR (typeof(tendered_centimes) = 'integer' AND tendered_centimes >= 0)),
    change_centimes      INTEGER
        CHECK (change_centimes IS NULL
               OR (typeof(change_centimes) = 'integer' AND change_centimes >= 0)),
    status               TEXT NOT NULL DEFAULT 'issued'
        CHECK (status IN ('issued', 'cancelled')),
    created_at           TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    UNIQUE (shop_id, series, number)
) STRICT;

-- Ids carry over, as they did on the way up, and the buyer block and the
-- balance triple are dropped with the old table: a revert is a revert.
-- The old `kind` CHECK has no `quittance` in it, so a file carrying one would
-- fail this INSERT and the revert would stop rather than lose the row.
-- Nothing issues a quittance in this milestone, so no file carries one.
INSERT INTO documents_without_customers
    (id, shop_id, kind, series, number, issued_at, user_id, regime, payment_mode,
     seller_name, seller_rc, seller_nif, seller_nis, seller_ai, seller_address,
     seller_phone, customer_id, total_ht_centimes, discount_centimes,
     subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes,
     net_to_pay_centimes, tendered_centimes, change_centimes, status, created_at)
SELECT id, shop_id, kind, series, number, issued_at, user_id, regime, payment_mode,
       seller_name, seller_rc, seller_nif, seller_nis, seller_ai, seller_address,
       seller_phone, customer_id, total_ht_centimes, discount_centimes,
       subtotal_ht_centimes, tva_centimes, total_ttc_centimes, stamp_centimes,
       net_to_pay_centimes, tendered_centimes, change_centimes, status, created_at
FROM documents;

DROP TABLE documents;

ALTER TABLE documents_without_customers RENAME TO documents;

CREATE INDEX idx_documents_shop_issued ON documents (shop_id, issued_at);

-- Last, now that nothing references it.
DROP TABLE customers;

-- Reports orphans as rows rather than failing, the same as on the way up:
-- the round trip in crates/core/tests/migration.rs is what fails.
PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
