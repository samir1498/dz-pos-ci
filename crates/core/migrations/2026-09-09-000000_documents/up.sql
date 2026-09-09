-- Documents, their lines and TVA recap, the stock ledger and the audit log.
-- Additive: nothing in the first migration is altered or dropped.
--
-- The same rules as migration 1 apply: STRICT everywhere, shop_id on every
-- table (rule 3), money in INTEGER *_centimes columns (rule 6), quantities in
-- INTEGER *_milli columns, timestamps TEXT, ids AUTOINCREMENT.

-- One row per fiscal document. features.md §3: every kind shares these
-- lines and totals; only numbering, legal blocks and stock effect differ.
-- M1 issues `ticket` only, and the other kinds are here because retrofitting
-- a kind column after there are documents rewrites history.
--
-- The seller block and the régime are snapshotted, not joined: `shops` is
-- replaced in place by the settings screen and a régime row can be dated into
-- the past, so a reprint that read them live would print a document the shop
-- never issued.
--
-- Amounts are at or above zero on every kind. An avoir carries positive
-- amounts and its kind is what says the money goes the other way.
CREATE TABLE documents (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id              INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    kind                 TEXT NOT NULL CHECK (kind IN
                             ('ticket', 'facture', 'proforma', 'bon_de_livraison',
                              'avoir', 'bon_de_reception')),
    -- The number and the series it belongs to. One uninterrupted series per
    -- kind (features.md, Numbering row; décret 05-468 art. 10), so the
    -- uniqueness is on the series, never on the kind: a series rename would
    -- otherwise let a number come round twice.
    series               TEXT NOT NULL,
    number               INTEGER NOT NULL
        CHECK (typeof(number) = 'integer' AND number >= 1),
    issued_at            TEXT NOT NULL,
    user_id              INTEGER NOT NULL REFERENCES users(id),
    regime               TEXT NOT NULL CHECK (regime IN ('ifu', 'reel')),
    -- cheque and transfer are parked as payment modes (features.md, Later),
    -- and the column admits them so adding one is not a migration.
    payment_mode         TEXT NOT NULL CHECK (payment_mode IN
                             ('cash', 'card', 'credit', 'cheque', 'transfer')),
    seller_name          TEXT NOT NULL,
    seller_rc            TEXT,
    seller_nif           TEXT,
    seller_nis           TEXT,
    seller_ai            TEXT,
    seller_address       TEXT,
    seller_phone         TEXT,
    -- No REFERENCES yet: customers arrive in M2 with their table. Until then
    -- a ticket carries no buyer at all (features.md §3, buyer block).
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
    -- Cash only: a card sale tenders nothing and gives no change.
    tendered_centimes    INTEGER
        CHECK (tendered_centimes IS NULL
               OR (typeof(tendered_centimes) = 'integer' AND tendered_centimes >= 0)),
    change_centimes      INTEGER
        CHECK (change_centimes IS NULL
               OR (typeof(change_centimes) = 'integer' AND change_centimes >= 0)),
    -- A cancelled document keeps its number and its row; it is never deleted,
    -- or the series would gap (features.md, Numbering row).
    status               TEXT NOT NULL DEFAULT 'issued'
        CHECK (status IN ('issued', 'cancelled')),
    created_at           TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    UNIQUE (shop_id, series, number)
) STRICT;
CREATE INDEX idx_documents_shop_issued ON documents (shop_id, issued_at);

-- The line as it was sold. name and barcode are snapshots: the product may be
-- renamed or deleted, and a reprint has to show what the customer was handed.
-- product_id is nullable and ON DELETE SET NULL for the same reason.
CREATE TABLE document_lines (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id               INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    document_id           INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    position              INTEGER NOT NULL
        CHECK (typeof(position) = 'integer' AND position >= 0),
    product_id            INTEGER REFERENCES products(id) ON DELETE SET NULL,
    name                  TEXT NOT NULL,
    barcode               TEXT,
    -- Thousandths of the unit, so 1,5 kg is 1500 and no float reaches a total
    -- (features.md, Line quantity and line total row).
    qty_milli             INTEGER NOT NULL
        CHECK (typeof(qty_milli) = 'integer' AND qty_milli > 0),
    unit_price_centimes   INTEGER NOT NULL
        CHECK (typeof(unit_price_centimes) = 'integer' AND unit_price_centimes >= 0),
    line_discount_centimes INTEGER NOT NULL
        CHECK (typeof(line_discount_centimes) = 'integer' AND line_discount_centimes >= 0),
    rate_bps              INTEGER NOT NULL CHECK (rate_bps BETWEEN 0 AND 10000),
    -- The rounded gross minus the line discount, stored so a reprint never
    -- recomputes and never disagrees with the paper.
    line_total_centimes   INTEGER NOT NULL
        CHECK (typeof(line_total_centimes) = 'integer' AND line_total_centimes >= 0),
    UNIQUE (document_id, position)
) STRICT;
CREATE INDEX idx_document_lines_document ON document_lines (shop_id, document_id);

-- The TVA recap, one row per rate present on the document. Stored rather than
-- derived: the rounding is once per rate group on the discounted base, and a
-- reprint that recomputed it could differ from the paper after a rule change.
CREATE TABLE document_tva (
    id             INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id        INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    document_id    INTEGER NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    rate_bps       INTEGER NOT NULL CHECK (rate_bps BETWEEN 0 AND 10000),
    base_centimes  INTEGER NOT NULL
        CHECK (typeof(base_centimes) = 'integer' AND base_centimes >= 0),
    amount_centimes INTEGER NOT NULL
        CHECK (typeof(amount_centimes) = 'integer' AND amount_centimes >= 0),
    UNIQUE (document_id, rate_bps)
) STRICT;

-- The append-only stock ledger (features.md §1, Stock movements). It is the
-- truth; products.qty_on_hand_milli is a cache re-derived from it.
-- qty_milli is signed: a sale is negative, a purchase positive.
CREATE TABLE stock_movements (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id            INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    product_id         INTEGER NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    kind               TEXT NOT NULL CHECK (kind IN
                           ('opening', 'purchase', 'sale', 'adjustment', 'return')),
    qty_milli          INTEGER NOT NULL CHECK (typeof(qty_milli) = 'integer'),
    unit_cost_centimes INTEGER NOT NULL
        CHECK (typeof(unit_cost_centimes) = 'integer' AND unit_cost_centimes >= 0),
    document_id        INTEGER REFERENCES documents(id) ON DELETE SET NULL,
    user_id            INTEGER NOT NULL REFERENCES users(id),
    created_at         TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_stock_movements_shop_product ON stock_movements (shop_id, product_id);

-- Sensitive actions, an ISO 27001 control we get nearly free by writing it
-- now (features.md §5). "before" and "after" hold JSON documents; they are
-- quoted because BEFORE is a keyword to SQLite's trigger syntax.
CREATE TABLE audit_log (
    id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    user_id    INTEGER NOT NULL REFERENCES users(id),
    action     TEXT NOT NULL,
    entity     TEXT NOT NULL,
    entity_id  INTEGER CHECK (entity_id IS NULL OR typeof(entity_id) = 'integer'),
    "before"   TEXT,
    "after"    TEXT,
    created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_audit_log_shop_created ON audit_log (shop_id, created_at);
