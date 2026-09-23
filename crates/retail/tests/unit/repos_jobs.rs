// A test may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::repos::testdb::{open, SHOP};
use crate::services::stock::JOB_STOCK_RECOUNT;

#[test]
fn a_job_nobody_has_run_has_no_last_day_at_all() {
    // Null is never, and a date standing in for that would say the job
    // had already run today.
    let (_dir, mut conn) = open();
    assert_eq!(get(&mut conn, SHOP, JOB_STOCK_RECOUNT).unwrap(), None);
}

#[test]
fn the_day_is_written_on_the_first_run_and_moved_on_the_next() {
    let (_dir, mut conn) = open();
    let first = mark_run(&mut conn, SHOP, JOB_STOCK_RECOUNT, "2026-09-10").unwrap();
    assert_eq!(first.last_run_day.as_deref(), Some("2026-09-10"));
    let again = mark_run(&mut conn, SHOP, JOB_STOCK_RECOUNT, "2026-09-11").unwrap();
    assert_eq!(again.id, first.id, "a second run made a second row");
    assert_eq!(again.last_run_day.as_deref(), Some("2026-09-11"));
    assert_eq!(
        get(&mut conn, SHOP, JOB_STOCK_RECOUNT)
            .unwrap()
            .and_then(|j| j.last_run_day)
            .as_deref(),
        Some("2026-09-11")
    );
}

#[test]
fn each_shop_keeps_its_own_marker() {
    let (_dir, mut conn) = open();
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Autre magasin')")
        .execute(&mut conn)
        .unwrap();
    mark_run(&mut conn, SHOP, JOB_STOCK_RECOUNT, "2026-09-10").unwrap();
    assert_eq!(get(&mut conn, 2, JOB_STOCK_RECOUNT).unwrap(), None);
    mark_run(&mut conn, 2, JOB_STOCK_RECOUNT, "2026-09-09").unwrap();
    assert_eq!(
        get(&mut conn, SHOP, JOB_STOCK_RECOUNT)
            .unwrap()
            .and_then(|j| j.last_run_day)
            .as_deref(),
        Some("2026-09-10")
    );
}
