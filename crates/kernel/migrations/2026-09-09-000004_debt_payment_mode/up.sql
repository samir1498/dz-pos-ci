-- How a payment against a debt reached the till (features.md §2). A payment
-- is taken in cash or on a card, and which of the two is a fact about the
-- movement rather than about the fiche: the till is reconciled by what came
-- through the drawer, and the stamped receipt the comptable has still to rule
-- on (R8) is a cash question before it is anything else.
--
-- Nullable, and null on every movement that is not a payment: an opening
-- balance, a sale on credit, an avoir and a correction are not paid in
-- anything, and a made-up 'cash' on them would read as money that moved.
--
-- One added column and no rebuild, so this migration runs inside the
-- transaction diesel opens for it and opens none of its own.
ALTER TABLE debt_ledger ADD COLUMN payment_mode TEXT
    CHECK (payment_mode IS NULL OR payment_mode IN ('cash', 'card'));
