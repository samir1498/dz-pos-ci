-- One form for a barcode, so a scan matches whatever the shop typed in.
--
-- A UPC-A is twelve digits and the same article's EAN-13 is those twelve
-- with a zero in front. Until now the till canonicalised both sides of the
-- comparison while the column kept whatever was typed, which meant the same
-- article could sit in the file twice under its two forms, past a UNIQUE
-- index that only ever saw two different strings, and a scan resolved to
-- whichever of them sorted first by name.
--
-- The twelve-digit form is folded into the thirteen it is short for. A row
-- whose canonical form is already taken in the same shop is left alone: it
-- is the collision this migration exists to stop happening again, and the
-- two rows have to be merged by somebody who knows which is the real one.
UPDATE products
SET barcode = '0' || barcode
WHERE barcode IS NOT NULL
  AND length(barcode) = 12
  AND barcode NOT GLOB '*[^0-9]*'
  AND NOT EXISTS (
      SELECT 1
      FROM products AS taken
      WHERE taken.shop_id = products.shop_id
        AND taken.barcode = '0' || products.barcode
  );
