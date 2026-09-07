use crate::models::*;
use crate::schema::suppliers;
use diesel::prelude::*;
use diesel::SqliteConnection;

pub fn insert_supplier(conn: &mut SqliteConnection, s: &NewSupplier) -> QueryResult<Supplier> {
    diesel::insert_into(suppliers::table)
        .values(s)
        .returning(Supplier::as_returning())
        .get_result(conn)
}

pub fn find_all_suppliers(conn: &mut SqliteConnection) -> QueryResult<Vec<Supplier>> {
    suppliers::table
        .order(suppliers::name.asc())
        .select(Supplier::as_select())
        .load(conn)
}

pub fn find_supplier_by_id(
    conn: &mut SqliteConnection,
    supplier_id: i32,
) -> QueryResult<Option<Supplier>> {
    suppliers::table
        .find(supplier_id)
        .select(Supplier::as_select())
        .first(conn)
        .optional()
}

pub fn update_supplier(
    conn: &mut SqliteConnection,
    supplier_id: i32,
    s: &NewSupplier,
) -> QueryResult<()> {
    diesel::update(suppliers::table.find(supplier_id))
        .set((
            suppliers::name.eq(&s.name),
            suppliers::phone.eq(&s.phone),
            suppliers::notes.eq(&s.notes),
        ))
        .execute(conn)
        .map(|_| ())
}

pub fn delete_supplier(conn: &mut SqliteConnection, supplier_id: i32) -> QueryResult<bool> {
    let rows = diesel::delete(suppliers::table.find(supplier_id)).execute(conn)?;
    Ok(rows > 0)
}
