-- The indexes go with the table they are on; naming them is what keeps the
-- reverse readable beside the up. Nothing else moves: the up added a table
-- and touched no row of any other, so this is the whole of the reverse and a
-- file that goes down and up again holds what it held.
DROP INDEX idx_cash_refunds_shop_user_at;
DROP INDEX idx_cash_refunds_one_per_document;
DROP TABLE cash_refunds;
