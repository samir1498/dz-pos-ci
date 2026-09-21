-- Cash handed back over the counter (features.md §1, ruling 5 of the
-- 2026-09-20 loop). One row per reversal that was settled in notes instead of
-- on an account: the document it undoes, how much left the drawer, who handed
-- it over and when.
--
-- Why a table of its own rather than a row on `debt_ledger`. That ledger's
-- `customer_id` is NOT NULL and the case this exists for is the anonymous
-- walk-in: features.md §1 allows a sale with no customer, most tickets are
-- one, and a shop that hands 3 000 DA back to somebody with no account had
-- nowhere at all to record it. The cashier wore the shortage at every close.
--
-- Additive. No existing table is rebuilt and no row is moved, so the revert
-- is a DROP and the foreign-key walk in `crates/core/tests/migration.rs`
-- stays what it was.

CREATE TABLE cash_refunds (
    id              INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    -- RESTRICT, with the ledgers and `shifts`: a record of money that left
    -- the drawer is not something a DELETE elsewhere may empty.
    shop_id         INTEGER NOT NULL REFERENCES shops(id) ON DELETE RESTRICT,
    -- What the cash was handed back against. On the avoir path it is the
    -- avoir, the numbered paper that carries the money back; on a cancelled
    -- cash ticket or facture there is no avoir to name, so it is the
    -- cancelled document itself. Either way the row names the paper a reader
    -- is holding when they ask why the drawer is lighter.
    document_id     INTEGER NOT NULL REFERENCES documents(id) ON DELETE RESTRICT,
    -- Whoever handed the notes over, which is not always whoever rang the
    -- sale. Cashier B refunding cashier A's ticket is B's drawer that is
    -- short, so this column and not the document's `user_id` is what a
    -- shift's expected figure subtracts by.
    user_id         INTEGER NOT NULL REFERENCES users(id),
    -- Strictly above zero: a refund of nothing is not an event, and the two
    -- services that write here refuse it before reaching this line so the
    -- caller gets a field and a sentence rather than a constraint.
    amount_centimes INTEGER NOT NULL
        CHECK (typeof(amount_centimes) = 'integer' AND amount_centimes > 0),
    -- The shop's clock (services::clock, UTC+1 all year), like `issued_at` on
    -- a document and `opened_at` on a shift. The day the cash was handed over
    -- and not the day the sale was rung: a ticket sold on Monday and refunded
    -- on Wednesday lowers Wednesday's drawer.
    refunded_at     TEXT NOT NULL
) STRICT;

-- One refund per document. A second call naming the same paper is a double
-- payout, and the file refuses it rather than trusting two services to
-- remember. The repo turns the violation into a validation error with the
-- field on it, so a retried request answers 400 and not 500.
CREATE UNIQUE INDEX idx_cash_refunds_one_per_document ON cash_refunds (document_id);

-- The two reads: the shop over a day or a month for the cash position, and
-- one person over one stretch of an evening for a till shift's expected
-- figure. `user_id` sits ahead of the moment so the narrower read is a range
-- scan rather than a filter over the shop's day.
CREATE INDEX idx_cash_refunds_shop_user_at ON cash_refunds (shop_id, user_id, refunded_at);
