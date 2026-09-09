//! The only place documents touch diesel. Every query is scoped by `shop_id`
//! (rule 3).

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::document::{
    assemble, Document, DocumentKind, DocumentLineRow, DocumentLineRowWrite, DocumentRow,
    DocumentRowWrite, DocumentTvaRow, DocumentTvaRowWrite,
};
use crate::schema::{document_lines, document_tva, documents, products};

pub fn insert(conn: &mut SqliteConnection, write: &DocumentRowWrite) -> Result<i32, CoreError> {
    let id: i32 = diesel::insert_into(documents::table)
        .values(write)
        .returning(documents::id)
        .get_result(conn)?;
    Ok(id)
}

pub fn insert_line(
    conn: &mut SqliteConnection,
    write: &DocumentLineRowWrite,
) -> Result<(), CoreError> {
    diesel::insert_into(document_lines::table)
        .values(write)
        .execute(conn)?;
    Ok(())
}

pub fn insert_tva(
    conn: &mut SqliteConnection,
    write: &DocumentTvaRowWrite,
) -> Result<(), CoreError> {
    diesel::insert_into(document_tva::table)
        .values(write)
        .execute(conn)?;
    Ok(())
}

/// Whether the product is one of this shop's. A line points at a product by
/// id, and the foreign key alone would let another shop's row through.
pub fn product_belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    product_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = products::table
        .filter(products::shop_id.eq(shop_id))
        .filter(products::id.eq(product_id))
        .select(products::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Document, CoreError> {
    let row: DocumentRow = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .filter(documents::id.eq(id))
        .select(DocumentRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "document",
            id,
        })?;
    with_children(conn, shop_id, row)
}

/// Newest first. `issued_at` is whole seconds and a busy till issues two
/// tickets inside one, so the id breaks the tie: the later insert is the
/// later sale.
pub fn list(
    conn: &mut SqliteConnection,
    shop_id: i32,
    kind: Option<DocumentKind>,
) -> Result<Vec<Document>, CoreError> {
    let mut query = documents::table
        .filter(documents::shop_id.eq(shop_id))
        .into_boxed();
    if let Some(kind) = kind {
        query = query.filter(documents::kind.eq(kind));
    }
    let rows: Vec<DocumentRow> = query
        .order((documents::issued_at.desc(), documents::id.desc()))
        .select(DocumentRow::as_select())
        .load(conn)?;
    rows.into_iter()
        .map(|row| with_children(conn, shop_id, row))
        .collect()
}

fn with_children(
    conn: &mut SqliteConnection,
    shop_id: i32,
    row: DocumentRow,
) -> Result<Document, CoreError> {
    let lines: Vec<DocumentLineRow> = document_lines::table
        .filter(document_lines::shop_id.eq(shop_id))
        .filter(document_lines::document_id.eq(row.id))
        .order(document_lines::position.asc())
        .select(DocumentLineRow::as_select())
        .load(conn)?;
    let tva: Vec<DocumentTvaRow> = document_tva::table
        .filter(document_tva::shop_id.eq(shop_id))
        .filter(document_tva::document_id.eq(row.id))
        .order(document_tva::rate_bps.asc())
        .select(DocumentTvaRow::as_select())
        .load(conn)?;
    assemble(row, lines, tva)
}
