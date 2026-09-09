-- The column goes and every movement stays: a revert loses how a payment was
-- taken, which is what going backwards costs, and it must not lose the
-- payment. The ids the allocations name and the index over them survive.
ALTER TABLE debt_ledger DROP COLUMN payment_mode;
