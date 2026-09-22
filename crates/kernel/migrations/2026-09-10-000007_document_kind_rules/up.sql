-- What a document's kind decides about the rest of its row (features.md §3).
--
-- The services already refuse every row this file now refuses. `sales::settle`
-- fills the tendered and change columns on a cash sale and refuses an amount
-- anywhere else; `avoir::issue` is the one writer that names a document an
-- avoir is written against, and `sales` and `proforma` name none; `proforma`
-- writes the balance triple as three zeros because a quotation moves nothing;
-- `documents::cancel` writes the day, the person and the reason together and
-- `models::document::cancellation` refuses to read back a half-written block.
-- What none of that reaches is a row that came from somewhere else: a restored
-- backup, a row repaired by hand, an import. Those are what a CHECK is for.
--
-- Every one of these rules reads two or three columns at once, so in SQLite it
-- is a table-level CHECK, and a table-level CHECK cannot be added to a table
-- that exists: the table is rebuilt, the way `debt_ledger` was rebuilt in
-- migration 6 and `documents` itself in migration 2. Nothing else about it
-- changes. The same columns in the same order, the same per-column checks, the
-- same unique key and the same index.
--
-- Additive in what it holds: every document keeps its id, because
-- `document_lines`, `document_tva`, `stock_movements`, `debt_ledger`,
-- `debt_allocations`, `documents.ref_document_id` and
-- `documents.cancel_avoir_document_id` all name those ids, and an id that
-- moved would say a facture had been settled by somebody else's payment or
-- credited by somebody else's avoir.
--
-- Nothing this app has written can fail the new rules, so the rebuild's INSERT
-- is a copy and not a repair. If it does fail, it fails loudly and the file it
-- is running against holds a row the services would have refused: the rule is
-- not the thing to relax, the row is the thing to look at.

-- Step 1 of the rebuild, and it has to be outside a transaction: a pragma
-- inside one is a no-op. `metadata.toml` tells diesel not to wrap this file.
-- With the foreign keys off, `DROP TABLE documents` below does not fail on the
-- lines, the movements and the ledger rows that point into it.
PRAGMA foreign_keys = OFF;

BEGIN;

CREATE TABLE documents_with_kind_rules (
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
    UNIQUE (shop_id, series, number),

    -- An avoir is written against the facture it credits and says which
    -- (features.md §3): a credit note naming no paper is money coming back
    -- off nothing. `avoir::issue` fills the column in on every avoir it
    -- writes.
    CHECK (kind <> 'avoir' OR ref_document_id IS NOT NULL),
    -- And the three papers the till writes stand on their own. Named rather
    -- than "every other kind": a bon de livraison and a bon de réception are
    -- kinds the column admits and no service writes, so what they would do
    -- with a reference is nobody's decision yet, and a CHECK taken here would
    -- be inventing it.
    CHECK (kind NOT IN ('ticket', 'facture', 'proforma')
           OR ref_document_id IS NULL),

    -- What the customer handed over and what went back over the counter.
    -- `sales::settle` fills both on a cash sale and refuses an amount on any
    -- other mode, so a card facture holding change is money the shop never
    -- gave back. One direction only: an avoir and a proforma copy the mode of
    -- the paper they belong to, cash included, and hold neither column.
    CHECK ((tendered_centimes IS NULL AND change_centimes IS NULL)
           OR payment_mode = 'cash'),
    -- The two are one fact about one payment, and `settle` returns them
    -- together or not at all: a row holding one of them is a write that got
    -- half way.
    CHECK ((tendered_centimes IS NULL) = (change_centimes IS NULL)),

    -- The cancellation block travels whole. A document marked annulée with
    -- nobody's name on it is exactly what the log is kept against
    -- (features.md §5), and `models::document::cancellation` already refuses
    -- to read one back.
    CHECK ((cancelled_at IS NULL) = (cancelled_by IS NULL)
           AND (cancelled_at IS NULL) = (cancel_reason IS NULL)),
    -- And `status` says the same thing the block says, both ways round: a row
    -- carrying a cancellation while it still says it stands is the same
    -- half-written write read from the other side.
    CHECK ((status = 'cancelled') = (cancelled_at IS NOT NULL)),
    -- The avoir a cancellation issued exists only where a cancellation does.
    -- `documents::cancel` is the one writer of this column and it writes it
    -- with the block.
    CHECK (cancel_avoir_document_id IS NULL OR status = 'cancelled'),
    -- And only the two papers a sale is handed over on are annulled at all.
    -- `documents::cancel` names them and refuses every other kind by not
    -- being on the list: a quotation moved nothing to put back, and an avoir
    -- is the instrument that undoes a facture rather than something undone in
    -- turn.
    CHECK (status <> 'cancelled' OR kind IN ('ticket', 'facture')),

    -- A quotation moves no goods and no money (features.md §3). `proforma.rs`
    -- writes the triple as three zeros rather than leaving it out, so the
    -- paper says what this quotation changes, which is nothing. Zero or
    -- absent, because the column was NULL on every document before customers
    -- arrived and a proforma with no triple says no more than one with three
    -- zeros; what it must never say is that a quotation put something on an
    -- account.
    CHECK (kind <> 'proforma'
           OR (COALESCE(old_balance_centimes, 0) = 0
               AND COALESCE(remaining_debt_centimes, 0) = 0
               AND COALESCE(total_debt_centimes, 0) = 0))
) STRICT;

-- Ids carried over rather than reassigned, and every column named: a copy that
-- let a column fall through to its default would be a hole this file opened.
INSERT INTO documents_with_kind_rules
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

ALTER TABLE documents_with_kind_rules RENAME TO documents;

-- The index goes with the table when it is dropped, so it is written again:
-- the document list reads one shop's papers newest first through it.
CREATE INDEX idx_documents_shop_issued ON documents (shop_id, issued_at);

-- With the keys off, nothing above was checked. The pragma reports orphans as
-- rows and never fails a statement, so it cannot stop this migration on its
-- own; it is here for a person running the file by hand. The guard is
-- crates/core/tests/migration.rs, which queries pragma_foreign_key_check after
-- migrating and fails if it returns anything.
PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
