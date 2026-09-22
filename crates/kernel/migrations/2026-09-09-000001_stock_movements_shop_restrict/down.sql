-- The same recreation the other way, back to migration 2's CASCADE, so
-- `revert_last_migration` walks the file back to a schema migration 2 would
-- have built and the round trip in crates/core/tests/migration.rs can prove
-- each down.sql undoes its own up.sql and nothing else.
CREATE TABLE stock_movements_shop_cascade (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id            INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
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

INSERT INTO stock_movements_shop_cascade
    (id, shop_id, product_id, kind, qty_milli, unit_cost_centimes,
     document_id, user_id, created_at)
SELECT id, shop_id, product_id, kind, qty_milli, unit_cost_centimes,
       document_id, user_id, created_at
FROM stock_movements;

DROP TABLE stock_movements;

ALTER TABLE stock_movements_shop_cascade RENAME TO stock_movements;

CREATE INDEX idx_stock_movements_shop_product ON stock_movements (shop_id, product_id);
