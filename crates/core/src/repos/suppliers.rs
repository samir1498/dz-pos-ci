// The callers are T2 (suppliers), T3 (purchases), T4 (expenses) and T5 (the
// re-derive job): the tables land here before the services that read them, and
// `repos` is crate-internal on purpose (architecture.md: nothing outside this
// crate touches diesel), so a plain build sees no use of these yet.
#![allow(dead_code)]

//! The only place suppliers touch diesel. Every query is scoped by `shop_id`
//! (rule 3); a function that forgets it is the bug this layer exists to make
//! visible.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::supplier::{Supplier, SupplierRow, SupplierRowWrite};
use crate::schema::suppliers;

/// The ones the shop still buys from first, then alphabetical inside each
/// group: the list is read by somebody looking for a supplier to order from,
/// and a deactivated fiche is kept for its ledger rather than for that. The
/// name is unique inside the shop, so it settles the order on its own; the id
/// closes it anyway, the way the customers list does.
pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Supplier>, CoreError> {
    let rows: Vec<SupplierRow> = suppliers::table
        .filter(suppliers::shop_id.eq(shop_id))
        .order((
            suppliers::active.desc(),
            suppliers::name.asc(),
            suppliers::id.asc(),
        ))
        .select(SupplierRow::as_select())
        .load(conn)?;
    Ok(rows.into_iter().map(Supplier::from).collect())
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Supplier, CoreError> {
    let row: SupplierRow = suppliers::table
        .filter(suppliers::shop_id.eq(shop_id))
        .filter(suppliers::id.eq(id))
        .select(SupplierRow::as_select())
        .first(conn)
        .optional()?
        .ok_or(CoreError::NotFound {
            entity: "supplier",
            id,
        })?;
    Ok(Supplier::from(row))
}

pub fn insert(
    conn: &mut SqliteConnection,
    write: &SupplierRowWrite,
) -> Result<Supplier, CoreError> {
    let row: SupplierRow = diesel::insert_into(suppliers::table)
        .values(write)
        .returning(SupplierRow::as_returning())
        .get_result(conn)?;
    Ok(Supplier::from(row))
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    write: &SupplierRowWrite,
) -> Result<Supplier, CoreError> {
    let changed = diesel::update(
        suppliers::table
            .filter(suppliers::shop_id.eq(shop_id))
            .filter(suppliers::id.eq(id)),
    )
    .set(write)
    .execute(conn)?;
    if changed == 0 {
        return Err(CoreError::NotFound {
            entity: "supplier",
            id,
        });
    }
    get(conn, shop_id, id)
}

/// Whether the supplier is one of this shop's. A purchase and a ledger row
/// both point at a supplier by id, and the foreign key alone would let
/// another shop's row through.
pub fn belongs_to_shop(
    conn: &mut SqliteConnection,
    shop_id: i32,
    supplier_id: i32,
) -> Result<bool, CoreError> {
    let found: Option<i32> = suppliers::table
        .filter(suppliers::shop_id.eq(shop_id))
        .filter(suppliers::id.eq(supplier_id))
        .select(suppliers::id)
        .first(conn)
        .optional()?;
    Ok(found.is_some())
}

#[cfg(test)]
mod tests {
    // A test may panic; the deny is for shipped code.
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::repos::testdb::{open, SHOP};

    fn draft(name: &str) -> SupplierRowWrite {
        SupplierRowWrite {
            shop_id: SHOP,
            name: name.to_string(),
            phone: None,
            address: None,
            rc: None,
            nif: None,
            nis: None,
            ai: None,
            notes: None,
            active: true,
            updated_at: crate::services::clock::now(),
        }
    }

    #[test]
    fn a_supplier_is_read_back_with_every_field_it_was_written_with() {
        let (_dir, mut conn) = open();
        let mut write = draft("Sarl Amrani");
        write.phone = Some("0550112233".to_string());
        write.rc = Some("16/00-7654321 B 22".to_string());
        write.notes = Some("livre le mardi".to_string());
        let made = insert(&mut conn, &write).unwrap();
        let read = get(&mut conn, SHOP, made.id).unwrap();
        assert_eq!(read, made);
        assert_eq!(read.phone.as_deref(), Some("0550112233"));
        assert_eq!(read.rc.as_deref(), Some("16/00-7654321 B 22"));
        assert_eq!(read.notes.as_deref(), Some("livre le mardi"));
        assert!(read.active);
    }

    #[test]
    fn another_shops_supplier_is_not_found_and_is_not_listed() {
        // Rule 3. The id alone would answer, which is the whole reason every
        // query here carries the shop.
        let (_dir, mut conn) = open();
        diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
            .execute(&mut conn)
            .unwrap();
        let mine = insert(&mut conn, &draft("Sarl Amrani")).unwrap();
        let mut theirs = draft("Sarl Amrani");
        theirs.shop_id = 2;
        insert(&mut conn, &theirs).unwrap();
        assert!(get(&mut conn, 2, mine.id).is_err());
        assert_eq!(list(&mut conn, SHOP).unwrap().len(), 1);
        assert_eq!(list(&mut conn, 2).unwrap().len(), 1);
    }

    #[test]
    fn the_list_puts_the_ones_still_dealt_with_first_then_the_name() {
        // The list is read by somebody looking for a supplier to order from,
        // and a deactivated fiche is kept for its ledger rather than for that.
        let (_dir, mut conn) = open();
        let mut closed = draft("Alpha");
        closed.active = false;
        insert(&mut conn, &closed).unwrap();
        insert(&mut conn, &draft("Zoubir")).unwrap();
        insert(&mut conn, &draft("Bensalem")).unwrap();
        let names: Vec<String> = list(&mut conn, SHOP)
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect();
        assert_eq!(names, vec!["Bensalem", "Zoubir", "Alpha"]);
    }

    #[test]
    fn an_update_reaches_every_field_and_can_clear_one() {
        // The screen sends the whole fiche back, so a phone number somebody
        // deleted has to reach the column rather than be read as "leave it".
        let (_dir, mut conn) = open();
        let mut write = draft("Sarl Amrani");
        write.phone = Some("0550112233".to_string());
        let made = insert(&mut conn, &write).unwrap();
        let mut edited = draft("Sarl Amrani et fils");
        edited.active = false;
        let after = update(&mut conn, SHOP, made.id, &edited).unwrap();
        assert_eq!(after.name, "Sarl Amrani et fils");
        assert_eq!(after.phone, None);
        assert!(!after.active);
        assert!(update(&mut conn, 2, made.id, &edited).is_err());
    }

    #[test]
    fn belonging_to_the_shop_is_a_question_the_foreign_key_cannot_answer() {
        let (_dir, mut conn) = open();
        diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
            .execute(&mut conn)
            .unwrap();
        let mine = insert(&mut conn, &draft("Sarl Amrani")).unwrap();
        assert!(belongs_to_shop(&mut conn, SHOP, mine.id).unwrap());
        assert!(!belongs_to_shop(&mut conn, 2, mine.id).unwrap());
    }
}
