-- The same table without the kind rules: `documents` as migration 2 wrote it
-- plus the four columns migration 5 added to it.
--
-- Nothing is lost on the way down. Every rule this drops is one the file could
-- only have met, so there is no row here the older table would refuse. Every
-- row goes back with its id, so the lines, the movements, the ledger and the
-- avoirs that name them still name the same papers.

PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE documents_without_kind_rules (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id              INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    kind                 TEXT NOT NULL CHECK (kind IN
                             ('ticket', 'facture', 'proforma', 'bon_de_livraison',
                              'avoir', 'bon_de_reception', 'quittance')),
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
    customer_id          INTEGER REFERENCES customers(id) ON DELETE RESTRICT,
    buyer_name           TEXT,
    buyer_party_kind     TEXT CHECK (buyer_party_kind IS NULL
                                     OR buyer_party_kind IN ('company', 'consumer')),
    buyer_rc             TEXT,
    buyer_nif            TEXT,
    buyer_nis            TEXT,
    buyer_ai             TEXT,
    buyer_address        TEXT,
    ref_document_id      INTEGER REFERENCES documents(id) ON DELETE RESTRICT,
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
    old_balance_centimes    INTEGER
        CHECK (old_balance_centimes IS NULL OR typeof(old_balance_centimes) = 'integer'),
    remaining_debt_centimes INTEGER
        CHECK (remaining_debt_centimes IS NULL
               OR typeof(remaining_debt_centimes) = 'integer'),
    total_debt_centimes     INTEGER
        CHECK (total_debt_centimes IS NULL OR typeof(total_debt_centimes) = 'integer'),
    status               TEXT NOT NULL DEFAULT 'issued'
        CHECK (status IN ('issued', 'cancelled')),
    created_at           TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    cancelled_at         TEXT,
    cancelled_by         INTEGER REFERENCES users(id),
    cancel_reason        TEXT,
    cancel_avoir_document_id INTEGER REFERENCES documents(id),
    UNIQUE (shop_id, series, number)
) STRICT;

INSERT INTO documents_without_kind_rules
    (id, shop_id, kind, series, number, issued_at, user_id, regime, payment_mode,
     seller_name, seller_rc, seller_nif, seller_nis, seller_ai, seller_address,
     seller_phone, customer_id, buyer_name, buyer_party_kind, buyer_rc, buyer_nif,
     buyer_nis, buyer_ai, buyer_address, ref_document_id, total_ht_centimes,
     discount_centimes, subtotal_ht_centimes, tva_centimes, total_ttc_centimes,
     stamp_centimes, net_to_pay_centimes, tendered_centimes, change_centimes,
     old_balance_centimes, remaining_debt_centimes, total_debt_centimes, status,
     created_at, cancelled_at, cancelled_by, cancel_reason, cancel_avoir_document_id)
SELECT id, shop_id, kind, series, number, issued_at, user_id, regime, payment_mode,
       seller_name, seller_rc, seller_nif, seller_nis, seller_ai, seller_address,
       seller_phone, customer_id, buyer_name, buyer_party_kind, buyer_rc, buyer_nif,
       buyer_nis, buyer_ai, buyer_address, ref_document_id, total_ht_centimes,
       discount_centimes, subtotal_ht_centimes, tva_centimes, total_ttc_centimes,
       stamp_centimes, net_to_pay_centimes, tendered_centimes, change_centimes,
       old_balance_centimes, remaining_debt_centimes, total_debt_centimes, status,
       created_at, cancelled_at, cancelled_by, cancel_reason, cancel_avoir_document_id
FROM documents;

DROP TABLE documents;

ALTER TABLE documents_without_kind_rules RENAME TO documents;

CREATE INDEX idx_documents_shop_issued ON documents (shop_id, issued_at);

-- Reports orphans as rows rather than failing, the same as on the way up: the
-- round trip in crates/core/tests/migration.rs is what fails.
PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
