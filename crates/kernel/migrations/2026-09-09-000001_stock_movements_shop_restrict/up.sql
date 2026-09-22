-- `stock_movements.shop_id` RESTRICTs, like every other ledger table.
--
-- Migration 2 shipped it as ON DELETE CASCADE while `documents` and
-- `audit_log` both RESTRICT, so a shop that had keyed its opening stock in and
-- not yet sold anything could be deleted and take its whole ledger with it.
-- The ledger is the truth about what is on the shelf (features.md §1) and
-- products.qty_on_hand_milli is only a cache of it, so that DELETE loses the
-- stock and leaves nothing that says it ever existed.
--
-- Migrations are forward-only: migration 2 is not edited, this one recreates
-- the table. SQLite has no ALTER for a foreign key, so it is the documented
-- twelve-step dance in short: create the new table, copy every row with its
-- id, drop the old one, rename, put the index back. Nothing references
-- stock_movements, so no other table's definition has to be rewritten and
-- `PRAGMA foreign_keys` can stay on for the whole thing.
--
-- The column list, the CHECKs and the index are migration 2's, character for
-- character apart from the one word; a copy that quietly relaxed a CHECK
-- would be a hole this file opened.
CREATE TABLE stock_movements_shop_restrict (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id            INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    product_id         INTEGER NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    kind               TEXT NOT NULL CHECK (kind IN
                           ('opening', 'purchase', 'sale', 'adjustment', 'return')),
    qty_milli          INTEGER NOT NULL CHECK (typeof(qty_milli) = 'integer'),
    unit_cost_centimes INTEGER NOT NULL
        CHECK (typeof(unit_cost_centimes) = 'integer' AND unit_cost_centimes >= 0),
    document_id        INTEGER REFERENCES documents(id) ON DELETE SET NULL,
    user_id            INTEGER NOT NULL REFERENCES users(id),
    created_at         TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;

-- Ids are carried over rather than reassigned: a movement is pointed at by
-- nothing today, but an id that moved would make every log line and every
-- exported ledger written before this migration name a different row.
INSERT INTO stock_movements_shop_restrict
    (id, shop_id, product_id, kind, qty_milli, unit_cost_centimes,
     document_id, user_id, created_at)
SELECT id, shop_id, product_id, kind, qty_milli, unit_cost_centimes,
       document_id, user_id, created_at
FROM stock_movements;

DROP TABLE stock_movements;

ALTER TABLE stock_movements_shop_restrict RENAME TO stock_movements;

CREATE INDEX idx_stock_movements_shop_product ON stock_movements (shop_id, product_id);
