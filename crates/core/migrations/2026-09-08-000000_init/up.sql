-- First migration. Rule 3 in docs/architecture.md: shop_id on every table
-- from day one. Money is INTEGER centimes in *_centimes columns (rule 6).
-- Quantities are INTEGER thousandths of the unit in *_milli columns, so a
-- kilo product never needs a float.

CREATE TABLE shops (
    id         INTEGER PRIMARY KEY NOT NULL,
    name       TEXT NOT NULL,
    rc         TEXT,
    nif        TEXT,
    nis        TEXT,
    ai         TEXT,
    address    TEXT,
    phone      TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);

-- Shop-level settings kept as a dated series: a row is valid from
-- valid_from until the next row for the same key. A document reads the row
-- that was current when it was issued, so the régime fiscal it printed
-- under stays readable after the shop changes régime.
CREATE TABLE settings (
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL,
    valid_from TIMESTAMP NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    PRIMARY KEY (shop_id, key, valid_from),
    CHECK (key <> 'regime_fiscal' OR value IN ('ifu', 'reel'))
);

CREATE TABLE users (
    id         INTEGER PRIMARY KEY NOT NULL,
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    role       TEXT NOT NULL CHECK (role IN ('owner', 'manager', 'cashier')),
    pin_hash   TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);
CREATE INDEX idx_users_shop ON users (shop_id);

CREATE TABLE categories (
    id                INTEGER PRIMARY KEY NOT NULL,
    shop_id           INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    -- TVA rate in basis points a product inherits when it names no rate of
    -- its own. features.md, TVA rates row: 19 % standard, 9 % reduced.
    default_rate_bps  INTEGER NOT NULL,
    UNIQUE (shop_id, name)
);

CREATE TABLE products (
    id                 INTEGER PRIMARY KEY NOT NULL,
    shop_id            INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name               TEXT NOT NULL,
    -- NULL until one is set; SQLite allows many NULLs under a UNIQUE index,
    -- which is what "optional, auto-generated if blank" needs.
    barcode            TEXT,
    category_id        INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    unit               TEXT NOT NULL CHECK (unit IN ('piece', 'kg', 'litre', 'box')),
    cost_centimes      INTEGER NOT NULL,
    -- Under the IFU régime this is the single price the customer pays and
    -- rate_bps is ignored: no HT/TTC split and no TVA line on any document
    -- (features.md, Régime fiscal row; CTCA 2026 art. 64).
    selling_centimes   INTEGER NOT NULL,
    wholesale_centimes INTEGER,
    qty_on_hand_milli  INTEGER NOT NULL DEFAULT 0,
    low_stock_at_milli INTEGER NOT NULL DEFAULT 0,
    -- Ignored under IFU; see selling_centimes above.
    rate_bps           INTEGER NOT NULL,
    active             BOOLEAN NOT NULL DEFAULT 1,
    created_at         TIMESTAMP NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at         TIMESTAMP NOT NULL DEFAULT (CURRENT_TIMESTAMP)
);
CREATE UNIQUE INDEX idx_products_shop_barcode ON products (shop_id, barcode);
CREATE INDEX idx_products_shop_name ON products (shop_id, name);

-- v1 is one shop, so it exists from the first open. The owner is seeded
-- with it so every later document and ledger row has an author before M4
-- brings real users.
INSERT INTO shops (id, name) VALUES (1, 'Mon magasin');
INSERT INTO users (shop_id, name, role) VALUES (1, 'Propriétaire', 'owner');
INSERT INTO settings (shop_id, key, value, valid_from)
    VALUES (1, 'regime_fiscal', 'reel', '2026-01-01 00:00:00');
INSERT INTO categories (shop_id, name, default_rate_bps) VALUES (1, 'Général', 1900);
