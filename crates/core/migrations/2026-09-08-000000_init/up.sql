-- First migration. Rule 3 in docs/architecture.md: shop_id on every table
-- from day one. Money is INTEGER centimes in *_centimes columns (rule 6).
-- Quantities are INTEGER thousandths of the unit in *_milli columns, so a
-- kilo product never needs a float.
--
-- Every table is STRICT (SQLite 3.37 and above; libsqlite3-sys is pinned
-- bundled, so the version travels with the binary). STRICT refuses text or
-- a real where an integer is declared, which is what keeps 19.99 out of a
-- *_centimes column. STRICT accepts only INT, INTEGER, REAL, TEXT, BLOB and
-- ANY as declared types, so timestamps are TEXT (SQLite stores them that
-- way already) and the boolean is an INTEGER with a CHECK.
--
-- Every id is AUTOINCREMENT. Plain INTEGER PRIMARY KEY hands the highest
-- deleted rowid out again, and a reissued product id would reissue the
-- in-store barcode printed on a label that is still on a shelf.

CREATE TABLE shops (
    id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name       TEXT NOT NULL,
    rc         TEXT,
    nif        TEXT,
    nis        TEXT,
    ai         TEXT,
    address    TEXT,
    phone      TEXT,
    created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;

-- Shop-level settings kept as a dated series: a row is valid from
-- valid_from until the next row for the same key. A document reads the row
-- that was current when it was issued, so the régime fiscal it printed
-- under stays readable after the shop changes régime.
CREATE TABLE settings (
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    value      TEXT NOT NULL,
    valid_from TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    PRIMARY KEY (shop_id, key, valid_from),
    CHECK (key <> 'regime_fiscal' OR value IN ('ifu', 'reel'))
) STRICT;

CREATE TABLE users (
    id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    role       TEXT NOT NULL CHECK (role IN ('owner', 'manager', 'cashier')),
    -- Never NULL: a NULL hash reads as "no PIN set", which is one careless
    -- check away from "anyone may log in". '!unset' is not a hash any
    -- verifier accepts, so it fails closed. M4 brings login and must write a
    -- real hash over it before the first sign-in.
    pin_hash   TEXT NOT NULL DEFAULT '!unset',
    created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_users_shop ON users (shop_id);

-- Numbers a shop hands out for itself, one series per name. A number is
-- taken inside the transaction that writes the row using it, so it is never
-- derived from a rowid: a rolled back insert leaves the counter advanced
-- rather than pointing at a number that is already taken for ever.
CREATE TABLE counters (
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    next_value INTEGER NOT NULL
        CHECK (typeof(next_value) = 'integer' AND next_value >= 1),
    PRIMARY KEY (shop_id, name)
) STRICT;

CREATE TABLE categories (
    id                INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id           INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name              TEXT NOT NULL,
    -- TVA rate in basis points a product inherits when it names no rate of
    -- its own. features.md, TVA rates row: 19 % standard, 9 % reduced.
    default_rate_bps  INTEGER NOT NULL CHECK (default_rate_bps BETWEEN 0 AND 10000),
    UNIQUE (shop_id, name)
) STRICT;

CREATE TABLE products (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id            INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name               TEXT NOT NULL,
    -- NULL until one is set; SQLite allows many NULLs under a UNIQUE index,
    -- which is what "optional, auto-generated if blank" needs.
    barcode            TEXT,
    category_id        INTEGER REFERENCES categories(id) ON DELETE SET NULL,
    unit               TEXT NOT NULL CHECK (unit IN ('piece', 'kg', 'litre', 'box')),
    -- typeof() alongside STRICT because STRICT converts a lossless real
    -- (19.0) to an integer, and a price that arrived as a float is a bug
    -- upstream even when it converts cleanly.
    cost_centimes      INTEGER NOT NULL
        CHECK (typeof(cost_centimes) = 'integer' AND cost_centimes >= 0),
    -- Under the IFU régime this is the single price the customer pays and
    -- rate_bps is ignored: no HT/TTC split and no TVA line on any document
    -- (features.md, Régime fiscal row; CTCA 2026 art. 64).
    selling_centimes   INTEGER NOT NULL
        CHECK (typeof(selling_centimes) = 'integer' AND selling_centimes >= 0),
    wholesale_centimes INTEGER
        CHECK (wholesale_centimes IS NULL
               OR (typeof(wholesale_centimes) = 'integer' AND wholesale_centimes >= 0)),
    qty_on_hand_milli  INTEGER NOT NULL DEFAULT 0
        CHECK (typeof(qty_on_hand_milli) = 'integer'),
    low_stock_at_milli INTEGER NOT NULL DEFAULT 0
        CHECK (typeof(low_stock_at_milli) = 'integer' AND low_stock_at_milli >= 0),
    -- Ignored under IFU; see selling_centimes above.
    rate_bps           INTEGER NOT NULL CHECK (rate_bps BETWEEN 0 AND 10000),
    active             INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at         TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at         TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE UNIQUE INDEX idx_products_shop_barcode ON products (shop_id, barcode);
CREATE INDEX idx_products_shop_name ON products (shop_id, name);

-- v1 is one shop, so it exists from the first open. The owner is seeded
-- with it so every later document and ledger row has an author before M4
-- brings real users.
INSERT INTO shops (id, name) VALUES (1, 'Mon magasin');
INSERT INTO users (shop_id, name, role, pin_hash)
    VALUES (1, 'Propriétaire', 'owner', '!unset');
INSERT INTO settings (shop_id, key, value, valid_from)
    VALUES (1, 'regime_fiscal', 'reel', '2026-01-01 00:00:00');
INSERT INTO categories (shop_id, name, default_rate_bps) VALUES (1, 'Général', 1900);
INSERT INTO counters (shop_id, name, next_value) VALUES (1, 'in_store_barcode', 1);
