-- Customers, the append-only debt ledger, the allocations a payment settles
-- documents through, and the buyer block and balance triple a document
-- carries (features.md §2 and §3).
--
-- Additive in what it holds: no earlier row is dropped and no earlier column
-- loses a value. `documents` is nevertheless rebuilt rather than ALTERed,
-- because `customer_id` has waited since migration 2 for a table to point at
-- and SQLite has no ALTER that adds a foreign key. The rebuild is the
-- documented twelve-step procedure, and every row of `documents` keeps its
-- id, its series and its number.
--
-- The same rules as the earlier migrations: STRICT everywhere, `shop_id` on
-- every table (rule 3), money in INTEGER `*_centimes` columns (rule 6),
-- timestamps TEXT, ids AUTOINCREMENT.

-- Step 1 of the rebuild, and it has to be outside a transaction: a pragma
-- inside one is a no-op. `metadata.toml` tells diesel not to wrap this file.
-- With the foreign keys off, `DROP TABLE documents` below does not cascade
-- into document_lines and document_tva, which is the whole point.
PRAGMA foreign_keys = OFF;

BEGIN;

-- The party a facture is made out to. Two customers may share a name (two
-- "Ahmed" walk into the same shop), so there is no UNIQUE on it and only an
-- index for the list and the search box.
--
-- `party_kind` is a field on the fiche, not something inferred from whether
-- an RC was typed in. Loi 04-02 art. 10 decides ticket against facture by who
-- the buyer is, and `facture_requires_party_ids` asks a different set of
-- fields of a company than of a consumer; an inference from RC would make
-- that rule flip the moment somebody clears a field.
--
-- The opening debt features.md lists as a customer field is not a column: it
-- is the first `opening` row of the ledger, so the balance has one source and
-- a correction to it is a movement somebody can read rather than an edit
-- nobody can see.
--
-- shop_id RESTRICTs like the other fiscal tables: the ledger below points at
-- these rows and a statement is printed from them.
CREATE TABLE customers (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id                 INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    -- NOT NULL: it is what the buyer block prints and what the list is
    -- ordered by. Every other identifier is optional, because a consumer has
    -- none of them (décret 05-468 art. 3-2 asks a consumer for name and
    -- address only).
    name                    TEXT NOT NULL,
    party_kind              TEXT NOT NULL CHECK (party_kind IN ('company', 'consumer')),
    phone                   TEXT,
    address                 TEXT,
    rc                      TEXT,
    nif                     TEXT,
    nis                     TEXT,
    ai                      TEXT,
    -- NULL is "no limit" and 0 is "no credit at all"; they are different
    -- answers and the column keeps them apart.
    credit_limit_centimes   INTEGER
        CHECK (credit_limit_centimes IS NULL
               OR (typeof(credit_limit_centimes) = 'integer' AND credit_limit_centimes >= 0)),
    -- NULL is "no warning".
    warn_threshold_centimes INTEGER
        CHECK (warn_threshold_centimes IS NULL
               OR (typeof(warn_threshold_centimes) = 'integer' AND warn_threshold_centimes >= 0)),
    notes                   TEXT,
    active                  INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at              TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at              TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_customers_shop_name ON customers (shop_id, name);

-- `documents` as migration 2 wrote it, plus what customers make possible.
-- Every column of migration 2 is repeated character for character apart from
-- the two changes named below; a copy that quietly relaxed a CHECK would be a
-- hole this file opened.
--
-- What changed:
--   * `customer_id` finally has a table to reference, and RESTRICTs: a
--     customer named on a facture is never deleted out from under it.
--   * `kind` admits `quittance`, the stamped receipt the comptable may ask
--     for (R8). Admitting a value costs nothing today and a rebuild
--     tomorrow; nothing issues it.
--   * `ref_document_id`, the facture an avoir is written against, RESTRICTs
--     for the same reason: the credited document outlives the credit note.
--   * the buyer block and the balance triple, both below.
CREATE TABLE documents_with_customers (
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
    -- The buyer block (features.md §3), snapshotted at issue for the same
    -- reason the seller block is: the fiche is edited in place and a reprint
    -- has to show the facture the customer was handed, not the customer as
    -- they are today. `customer_id` above still points at the live fiche;
    -- these are what the paper says.
    buyer_name           TEXT,
    buyer_party_kind     TEXT CHECK (buyer_party_kind IS NULL
                                     OR buyer_party_kind IN ('company', 'consumer')),
    buyer_rc             TEXT,
    buyer_nif            TEXT,
    buyer_nis            TEXT,
    buyer_ai             TEXT,
    buyer_address        TEXT,
    -- The facture an avoir is written against. Which factures an avoir may
    -- name (this shop's, not cancelled, not already credited whole) is a rule
    -- no foreign key states, and T4 checks it; the key is here so the row it
    -- names cannot vanish.
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
    -- The balance triple of the totals table: what the customer owed before
    -- this document, what is left after it, and the two added up. NULL on a
    -- document with no customer, which is every ticket issued so far.
    --
    -- Signed, unlike every other money column here: a customer who overpays
    -- is owed money, and the statement says so rather than clamping at zero.
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
    UNIQUE (shop_id, series, number)
) STRICT;

-- Ids are carried over rather than reassigned: document_lines, document_tva
-- and stock_movements all point at `documents.id`, and an id that moved would
-- reattach a shop's lines to somebody else's facture. The new columns take
-- their default of NULL on every existing row, which is what a ticket with no
-- customer means.
INSERT INTO documents_with_customers
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

ALTER TABLE documents_with_customers RENAME TO documents;

CREATE INDEX idx_documents_shop_issued ON documents (shop_id, issued_at);

-- The append-only debt ledger (features.md §2). A balance is the sum of this
-- table, never a stored number: a stored balance and a ledger that disagree
-- is the failure an append-only ledger exists to prevent.
--
-- Debit raises what the customer owes (an opening debt, a credit sale), and
-- credit lowers it (a payment, an avoir). One row carries one of the two and
-- the other is zero, so the direction is a fact about the row rather than the
-- sign of a number somebody has to remember to read.
--
-- The balance may go below zero: a customer who overpays is owed money, and
-- that is a real state a statement prints, so no CHECK bounds the sum.
--
-- customer_id RESTRICTs: a customer with a movement is never deleted, or the
-- documents that named them would point at nothing. document_id is nullable
-- and SET NULL, because an `opening`, a `payment` and an `adjustment` belong
-- to no single document.
CREATE TABLE debt_ledger (
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
    -- One direction per row. A row carrying both would be two movements
    -- wearing one id: the balance would still add up and the statement would
    -- print nonsense.
    CHECK (debit_centimes = 0 OR credit_centimes = 0)
) STRICT;
-- The id closes the index because a balance and a statement both read one
-- customer's rows in the order they were written, and two rows can land
-- inside the same second.
CREATE INDEX idx_debt_ledger_shop_customer ON debt_ledger (shop_id, customer_id, id);

-- What a payment settled. features.md §2: a payment settles several documents
-- oldest first, so the payment is one ledger row and the documents it covered
-- are these rows. Kept apart from the ledger so the balance stays one sum
-- over one table.
--
-- Both foreign keys RESTRICT: an allocation that outlived its payment or its
-- document would say a facture was paid by nothing.
CREATE TABLE debt_allocations (
    id                INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id           INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    payment_ledger_id INTEGER NOT NULL REFERENCES debt_ledger(id) ON DELETE RESTRICT,
    document_id       INTEGER NOT NULL REFERENCES documents(id) ON DELETE RESTRICT,
    -- Strictly above zero: an allocation of nothing settles nothing.
    amount_centimes   INTEGER NOT NULL
        CHECK (typeof(amount_centimes) = 'integer' AND amount_centimes > 0),
    created_at        TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_debt_allocations_document ON debt_allocations (shop_id, document_id);

-- Step 8 of the procedure: with the keys off, nothing above was checked. The
-- pragma reports orphans as rows and never fails a statement, so it cannot
-- stop this migration on its own; it is here for a person running the file by
-- hand. The guard is crates/core/tests/migration.rs, which queries
-- pragma_foreign_key_check after migrating and fails if it returns anything.
PRAGMA foreign_key_check;

COMMIT;

PRAGMA foreign_keys = ON;
