-- The supply side: who the shop buys from, what it ordered, what actually
-- arrived, what it owes for it, what it spends beside stock, and the marker a
-- once-a-day job leaves behind (features.md §1, Supplier, Purchase, Stock
-- movements and Expense).
--
-- Ten tables and not one line of an existing one: nothing here is rebuilt, so
-- there is no PRAGMA dance and no metadata.toml, and every table-level CHECK
-- goes straight into its CREATE TABLE.
--
-- The design choice this file writes down (plan, 2026-09-10, held after the
-- lens): suppliers get their own table and their own ledger, and `documents`
-- and `debt_ledger` stay customer-keyed. One `parties` table with a role
-- would have made every M2 query, CHECK, index and screen party-keyed for a
-- party that never buys at the till; two mirrored ledgers cost one repeated
-- service instead.
--
-- The same rules as every migration before it: STRICT everywhere, `shop_id`
-- on every table (rule 3), money in INTEGER `*_centimes` columns and
-- quantities in INTEGER `*_milli` columns (rule 6), timestamps TEXT, ids
-- AUTOINCREMENT.

-- Who the shop buys from. The same identity block a customer carries, minus
-- the credit fields: a limit and a warning threshold are what the shop grants
-- somebody, and the supplier is the one granting here.
--
-- The name is unique inside the shop, which is what features.md §1 asks for
-- ("Name (unique)"). Whole rather than partial on `active`: a deactivated
-- supplier keeps its ledger and its purchases, and a second fiche under the
-- same name would read at a counter as one party carrying two balances. The
-- UNIQUE is also the index the list and the search box read, so there is no
-- second index on (shop_id, name) beside it.
--
-- The opening debt features.md lists among the fields is not a column, for
-- the reason the customer's is not: it is the first `opening` row of the
-- ledger below, so the balance has one source and a correction to it is a
-- movement somebody can read.
--
-- shop_id RESTRICTs: the ledger and the purchases point at these rows.
CREATE TABLE suppliers (
    id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    name       TEXT NOT NULL,
    phone      TEXT,
    address    TEXT,
    rc         TEXT,
    nif        TEXT,
    nis        TEXT,
    ai         TEXT,
    notes      TEXT,
    active     INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    updated_at TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    UNIQUE (shop_id, name)
) STRICT;

-- An order placed with a supplier. It is not a row of `documents` and never
-- will be: that table's NOT NULL regime, its payment mode and its customer
-- key have no honest value for something the shop buys, and its series is a
-- numbering the tax code hands out for what the shop sells.
--
-- `supplier_document_number` is the number written on the paper the supplier
-- sent. Unique under one supplier where it is set, by the partial index
-- below, and absent as often as the shop pleases: a delivery note with no
-- number on it is a thing suppliers hand over.
--
-- `transport_centimes` and `extra_costs_centimes` are what the goods cost to
-- get here. They are held apart from the lines because they are agreed once
-- for the whole order; T3 spreads them over the ordered lines by value and
-- writes the answer into each line's `landed_unit_cost_centimes`.
--
-- The five states: `ordered` until something arrives, `partially_received`
-- and `received` as it does, `cancelled` when nothing ever did, `closed_short`
-- when the rest never will. The status is `ordered` by default the way a
-- document's is `issued`, so a row written without one is in the state a
-- purchase starts in rather than in no state at all.
--
-- `purchase_date` and `due_date` are days on the shop's calendar (`YYYY-MM-DD`)
-- and `created_at` is the moment the row was written, which is UTC. They are
-- different questions and the two columns keep them apart.
CREATE TABLE purchases (
    id                       INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id                  INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    supplier_id              INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    supplier_document_number TEXT,
    purchase_date            TEXT NOT NULL,
    due_date                 TEXT,
    transport_centimes       INTEGER NOT NULL DEFAULT 0
        CHECK (typeof(transport_centimes) = 'integer' AND transport_centimes >= 0),
    extra_costs_centimes     INTEGER NOT NULL DEFAULT 0
        CHECK (typeof(extra_costs_centimes) = 'integer' AND extra_costs_centimes >= 0),
    status                   TEXT NOT NULL DEFAULT 'ordered' CHECK (status IN
                                 ('ordered', 'partially_received', 'received',
                                  'cancelled', 'closed_short')),
    user_id                  INTEGER NOT NULL REFERENCES users(id),
    note                     TEXT,
    created_at               TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_purchases_shop_date ON purchases (shop_id, purchase_date);
CREATE INDEX idx_purchases_shop_supplier ON purchases (shop_id, supplier_id, id);
-- Partial, because two purchases whose paper carried no number are not a
-- duplicate of each other and a plain UNIQUE would say they were on the
-- second one. SQLite compares NULLs as distinct in a UNIQUE index, so the
-- WHERE is belt and braces; it also keeps the index off the rows that have
-- nothing to say.
CREATE UNIQUE INDEX idx_purchases_supplier_document
    ON purchases (shop_id, supplier_id, supplier_document_number)
    WHERE supplier_document_number IS NOT NULL;

-- What was ordered, and what has arrived of it so far.
--
-- `unit_cost_centimes` is what the supplier charges for the unit.
-- `landed_unit_cost_centimes` is that plus this line's share of the transport
-- and the extra costs, fixed once when the purchase is saved: the margin a
-- report reads is against the cost the goods actually landed at, and a share
-- recomputed later would move a cost a sale has already been measured
-- against.
--
-- `qty_received_milli` is a running total the receipts add up to. The CHECK
-- keeps it at or under what was ordered, because a line saying more arrived
-- than was asked for is a purchase whose stock and whose supplier debt
-- disagree with the paper they came from. How much one receipt may add is a
-- rule the service holds (T3); this is the state the file refuses to hold.
--
-- `purchase_id` CASCADEs so a purchase deleted before anything arrived takes
-- its own lines; `product_id` RESTRICTs because a product named on a purchase
-- outlives the fiche, the way a movement's does.
CREATE TABLE purchase_lines (
    id                        INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id                   INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    purchase_id               INTEGER NOT NULL REFERENCES purchases(id) ON DELETE CASCADE,
    product_id                INTEGER NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    -- Strictly above zero: a line ordering nothing orders nothing.
    qty_ordered_milli         INTEGER NOT NULL
        CHECK (typeof(qty_ordered_milli) = 'integer' AND qty_ordered_milli > 0),
    unit_cost_centimes        INTEGER NOT NULL
        CHECK (typeof(unit_cost_centimes) = 'integer' AND unit_cost_centimes >= 0),
    landed_unit_cost_centimes INTEGER NOT NULL
        CHECK (typeof(landed_unit_cost_centimes) = 'integer'
               AND landed_unit_cost_centimes >= 0),
    qty_received_milli        INTEGER NOT NULL DEFAULT 0
        CHECK (typeof(qty_received_milli) = 'integer' AND qty_received_milli >= 0),
    CHECK (qty_received_milli <= qty_ordered_milli)
) STRICT;
CREATE INDEX idx_purchase_lines_purchase ON purchase_lines (shop_id, purchase_id, id);

-- The bon de réception: one delivery against one purchase, kept and listed
-- like the paper it stands for. It takes its number from the counters table
-- under the series `reception:<year>`, so the yearly reset T0 gives every
-- other series is already the shape this one has, and the number is unique
-- inside the shop and the series exactly as a document's is.
--
-- `purchase_id` RESTRICTs: the goods came in on this order and the movements
-- of stock say so, so the order outlives the delivery.
CREATE TABLE purchase_receipts (
    id          INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id     INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    purchase_id INTEGER NOT NULL REFERENCES purchases(id) ON DELETE RESTRICT,
    series      TEXT NOT NULL,
    number      INTEGER NOT NULL
        CHECK (typeof(number) = 'integer' AND number >= 1),
    received_at TEXT NOT NULL,
    user_id     INTEGER NOT NULL REFERENCES users(id),
    note        TEXT,
    created_at  TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    UNIQUE (shop_id, series, number)
) STRICT;
CREATE INDEX idx_purchase_receipts_purchase ON purchase_receipts (shop_id, purchase_id, id);

-- What arrived on one delivery, line by line. A quantity of nothing is
-- refused: a receipt line saying nothing came in says nothing at all, and the
-- lines that were not delivered this time are simply absent.
CREATE TABLE purchase_receipt_lines (
    id               INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id          INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    receipt_id       INTEGER NOT NULL REFERENCES purchase_receipts(id) ON DELETE CASCADE,
    purchase_line_id INTEGER NOT NULL REFERENCES purchase_lines(id) ON DELETE RESTRICT,
    qty_milli        INTEGER NOT NULL
        CHECK (typeof(qty_milli) = 'integer' AND qty_milli > 0)
) STRICT;
CREATE INDEX idx_purchase_receipt_lines_receipt
    ON purchase_receipt_lines (shop_id, receipt_id, id);

-- What the shop owes its suppliers, the mirror of `debt_ledger` as migration
-- 6 left it. A balance is the sum of this table and never a stored number.
--
-- Debit raises what the shop owes (an opening balance, goods received) and
-- credit lowers it (a payment, a return to the supplier). One row carries one
-- of the two and the other is zero, so the direction is a fact about the row
-- rather than the sign of a number somebody has to remember to read.
--
-- The balance may go below zero: a shop that has paid in advance is owed
-- goods or money, and a statement prints that rather than clamping it.
--
-- The kinds differ from the customer side by exactly what the two sides do
-- differently: `purchase` where a customer has a `sale`, and `return` where a
-- customer has an `avoir`. A `sale` on this ledger would be a supplier buying
-- at the till, which is not a thing that happens.
--
-- Debt rises on receipt and not on save (plan lens, 2026-09-10): the row is
-- written for the value that actually arrived, so a purchase closed short
-- owes nothing for what never came. T3 writes it.
--
-- `purchase_id` is nullable and SET NULL, the way `document_id` is on the
-- customer side: an `opening`, a `payment` and an `adjustment` belong to no
-- single purchase.
--
-- `created_at` keeps the column default. The service stamps it from the shop
-- clock the way `debt::pay` does, and the default is what a row written
-- without one gets, not a second opinion about the calendar.
CREATE TABLE supplier_ledger (
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id         INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    supplier_id     INTEGER NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    purchase_id     INTEGER REFERENCES purchases(id) ON DELETE SET NULL,
    kind            TEXT NOT NULL CHECK (kind IN
                        ('opening', 'purchase', 'payment', 'return', 'adjustment')),
    debit_centimes  INTEGER NOT NULL
        CHECK (typeof(debit_centimes) = 'integer' AND debit_centimes >= 0),
    credit_centimes INTEGER NOT NULL
        CHECK (typeof(credit_centimes) = 'integer' AND credit_centimes >= 0),
    user_id         INTEGER NOT NULL REFERENCES users(id),
    note            TEXT,
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    payment_mode    TEXT
        CHECK (payment_mode IS NULL OR payment_mode IN ('cash', 'card')),
    -- One direction per row. A row carrying both would be two movements
    -- wearing one id: the balance would still add up and the statement would
    -- print nonsense.
    CHECK (debit_centimes = 0 OR credit_centimes = 0),
    -- The mode is on a payment and on nothing else, written as an equality so
    -- it holds both ways round: a made-up 'cash' on an opening balance reads
    -- as money that moved and never did, and a payment saying nothing is
    -- money whose road through the drawer nobody can retrace.
    CHECK ((payment_mode IS NULL) = (kind <> 'payment'))
) STRICT;
-- The id closes the index because a balance and a statement both read one
-- supplier's rows in the order they were written, and two rows can land
-- inside the same second.
CREATE INDEX idx_supplier_ledger_shop_supplier ON supplier_ledger (shop_id, supplier_id, id);

-- What a payment to a supplier settled, the mirror of `debt_allocations`: a
-- payment settles several purchases oldest first, so the payment is one
-- ledger row and the purchases it covered are these rows. Kept apart from the
-- ledger so the balance stays one sum over one table, and so what is still
-- owed on a single purchase is a question with an answer.
CREATE TABLE supplier_allocations (
    id                INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id           INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    payment_ledger_id INTEGER NOT NULL REFERENCES supplier_ledger(id) ON DELETE RESTRICT,
    purchase_id       INTEGER NOT NULL REFERENCES purchases(id) ON DELETE RESTRICT,
    -- Strictly above zero: an allocation of nothing settles nothing.
    amount_centimes   INTEGER NOT NULL
        CHECK (typeof(amount_centimes) = 'integer' AND amount_centimes > 0),
    created_at        TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_supplier_allocations_purchase ON supplier_allocations (shop_id, purchase_id);

-- What an expense is filed under. The row carries a key and not a label: the
-- desktop reads the label out of its i18n files by that key in the three
-- languages, so a shop switching language does not have to rewrite its rows
-- and a category cannot exist in French and be missing in Arabic.
--
-- `sort_order` is the order the screen lists them in, which is the order the
-- seven below are seeded in and not alphabetical in any one language.
-- `active` retires a category the shop stopped using without taking the
-- expenses filed under it.
CREATE TABLE expense_categories (
    id         INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id    INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    key        TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0
        CHECK (typeof(sort_order) = 'integer' AND sort_order >= 0),
    active     INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    UNIQUE (shop_id, key)
) STRICT;

-- Money out that is not stock (features.md §1, Expense). An expense of
-- nothing is refused: it is a payment somebody made, and nobody pays nothing.
-- `category_id` RESTRICTs, so a category with expenses behind it is retired
-- rather than deleted.
CREATE TABLE expenses (
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id         INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    category_id     INTEGER NOT NULL REFERENCES expense_categories(id) ON DELETE RESTRICT,
    amount_centimes INTEGER NOT NULL
        CHECK (typeof(amount_centimes) = 'integer' AND amount_centimes > 0),
    expense_date    TEXT NOT NULL,
    note            TEXT,
    user_id         INTEGER NOT NULL REFERENCES users(id),
    created_at      TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP)
) STRICT;
CREATE INDEX idx_expenses_shop_date ON expenses (shop_id, expense_date);

-- The day a once-a-day job last ran, per shop. The stock re-derive (T5) runs
-- at app start when this says it has not run today, so the marker has to
-- survive a restart, which a value held in the process does not.
--
-- `last_run_day` is nullable and null means never: a job named here for the
-- first time has not run, and a date standing in for that would say it had.
CREATE TABLE jobs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    shop_id      INTEGER NOT NULL REFERENCES shops(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    last_run_day TEXT,
    updated_at   TEXT NOT NULL DEFAULT (CURRENT_TIMESTAMP),
    UNIQUE (shop_id, name)
) STRICT;

-- The seven categories features.md §1 names, once per shop the file already
-- carries. Written as a join against `shops` rather than seven inserts under
-- shop 1, because a file restored from a second till (M6) can hold more than
-- one shop and each of them needs its own rows: the keys are unique per shop.
-- A shop created after this migration is seeded by the service that creates
-- it, which is where a new shop's categories belong.
INSERT INTO expense_categories (shop_id, key, sort_order)
SELECT s.id, k.key, k.sort_order
FROM shops s,
     (SELECT 'rent' AS key, 1 AS sort_order
      UNION ALL SELECT 'electricity', 2
      UNION ALL SELECT 'water', 3
      UNION ALL SELECT 'salaries', 4
      UNION ALL SELECT 'transport', 5
      UNION ALL SELECT 'maintenance', 6
      UNION ALL SELECT 'other', 7) k;
