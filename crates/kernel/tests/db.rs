// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use diesel::connection::SimpleConnection;
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
    let mut conn = dzpos_kernel::db::open(&path).unwrap();
    assert!(path.exists());
    let row: Row = diesel::sql_query("PRAGMA foreign_keys")
        .get_result(&mut conn)
        .unwrap();
    assert_eq!(row.foreign_keys, 1);
}

/// `checkpoint` is what makes a restore's leftovers harmless: it folds the
/// write-ahead log back into the shop file and empties it. SQLite reports a
/// checkpoint it could not take as a `busy` column on a row it answers
/// happily, not as an error, so a caller that only executes the pragma is
/// told nothing and swaps the file with a log still holding committed pages.
#[test]
fn a_checkpoint_empties_the_log_and_says_so_when_it_could_not() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut till = dzpos_kernel::db::open(&path).unwrap();
    diesel::sql_query(
        "INSERT INTO categories (shop_id, name, default_rate_bps) VALUES (1, 'Épicerie', 900)",
    )
    .execute(&mut till)
    .unwrap();

    // Nothing else is reading, so the log folds back and the sidecar is
    // truncated to nothing.
    dzpos_kernel::db::checkpoint(&mut till).unwrap();
    let wal = dir.path().join("t.db-wal");
    assert_eq!(
        wal.metadata().map(|m| m.len()).unwrap_or(0),
        0,
        "the log was not truncated"
    );

    // A second connection with a read transaction open is exactly what a
    // second till, or a backup being verified, looks like. The checkpoint
    // cannot take the file, and that has to reach the caller.
    let mut reader = dzpos_kernel::db::open(&path).unwrap();
    reader.batch_execute("BEGIN").unwrap();
    let _: Vec<Count> = diesel::sql_query("SELECT count(*) AS n FROM categories")
        .load(&mut reader)
        .unwrap();
    diesel::sql_query(
        "INSERT INTO categories (shop_id, name, default_rate_bps) VALUES (1, 'Boissons', 900)",
    )
    .execute(&mut till)
    .unwrap();

    let refused = dzpos_kernel::db::checkpoint(&mut till).unwrap_err();
    assert!(
        refused.to_string().contains("in use"),
        "a checkpoint that could not be taken passed for one that was: {refused}"
    );
    reader.batch_execute("COMMIT").unwrap();
}

#[derive(QueryableByName)]
struct Count {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    #[allow(dead_code)]
    n: i64,
}
