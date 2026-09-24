-- A product's pack size, optional (plan shop-manual-test-findings T13): 1,5 L
-- of oil, 250 g of coffee. Shops type it into the name today; this gives it
-- a place of its own that the screens append to the name, and that a shelf
-- label can later read a price per litre or per kilo off.
--
-- The quantity is thousandths of the unit, the way every quantity on the
-- file is (1,5 L is 1500 with the unit 'l'), so no float is ever made from
-- it. The two are set together or not at all.
--
-- Two columns added, no row moved, so the revert drops them.
ALTER TABLE products ADD COLUMN contenance_milli INTEGER
    CHECK (contenance_milli IS NULL
           OR (typeof(contenance_milli) = 'integer' AND contenance_milli > 0));

ALTER TABLE products ADD COLUMN contenance_unit TEXT
    CHECK ((contenance_unit IS NULL AND contenance_milli IS NULL)
           OR (contenance_unit IN ('g', 'kg', 'ml', 'l') AND contenance_milli IS NOT NULL));
