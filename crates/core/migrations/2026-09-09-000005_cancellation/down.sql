-- The five columns go and every document and every line stays. A revert loses
-- when a document was annulled, by whom, why, which avoir carried the money
-- back and which facture line each avoir line credited; it must not lose the
-- document, its lines, its number or the `status` column that says it was
-- annulled at all, and none of those is touched here.

ALTER TABLE document_lines DROP COLUMN ref_line_id;
ALTER TABLE documents DROP COLUMN cancel_avoir_document_id;
ALTER TABLE documents DROP COLUMN cancel_reason;
ALTER TABLE documents DROP COLUMN cancelled_by;
ALTER TABLE documents DROP COLUMN cancelled_at;
