use diesel::prelude::*;
use diesel::sql_types::Integer;

#[derive(QueryableByName)]
struct Row {
    #[diesel(sql_type = Integer)]
    foreign_keys: i32,
}

#[test]
fn open_creates_file_and_enables_foreign_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut conn = dzpos_core::db::open(&path).unwrap();
    assert!(path.exists());
    let row: Row = diesel::sql_query("PRAGMA foreign_keys")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(row.foreign_keys, 1);
}
