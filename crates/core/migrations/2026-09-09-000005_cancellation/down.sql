-- The four columns go and every document stays. A revert loses when a
-- document was annulled, by whom, why and which avoir carried the money back;
-- it must not lose the document, its number or the `status` column that says
-- it was annulled at all, and none of those is touched here.

ALTER TABLE documents DROP COLUMN cancel_avoir_document_id;
ALTER TABLE documents DROP COLUMN cancel_reason;
ALTER TABLE documents DROP COLUMN cancelled_by;
ALTER TABLE documents DROP COLUMN cancelled_at;
