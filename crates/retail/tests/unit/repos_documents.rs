use super::{insert, series_names_its_year};
use crate::models::document::{DocumentKind, DocumentRowWrite, DocumentStatus};
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

#[test]
fn a_series_names_its_year_only_when_the_year_is_the_whole_last_segment() {
    assert!(series_names_its_year("doc_ticket:2026", 2026));
    assert!(!series_names_its_year("doc_ticket:2026", 2025));
    // The stem alone: the key of the scheme before the year existed.
    assert!(!series_names_its_year("doc_ticket", 2026));
    // Ends with the digits and is not the year. A suffix match on its own
    // would take both of these.
    assert!(!series_names_its_year("doc_ticket:12026", 2026));
    assert!(!series_names_its_year("doc_ticket2026", 2026));
}

fn a_ticket(series: &str, series_year: i32) -> DocumentRowWrite {
    DocumentRowWrite {
        shop_id: 1,
        kind: DocumentKind::Ticket,
        series: series.to_string(),
        series_year,
        number: 1,
        issued_at: NaiveDate::from_ymd_opt(2026, 9, 9)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap(),
        user_id: 1,
        regime: "reel",
        payment_mode: "cash",
        seller_name: "Mon magasin".to_string(),
        seller_rc: None,
        seller_nif: None,
        seller_nis: None,
        seller_ai: None,
        seller_address: None,
        seller_phone: None,
        customer_id: None,
        buyer_name: None,
        buyer_party_kind: None,
        buyer_rc: None,
        buyer_nif: None,
        buyer_nis: None,
        buyer_ai: None,
        buyer_address: None,
        ref_document_id: None,
        old_balance_centimes: None,
        remaining_debt_centimes: None,
        total_debt_centimes: None,
        total_ht_centimes: 10_000,
        discount_centimes: 0,
        subtotal_ht_centimes: 10_000,
        tva_centimes: 1_900,
        total_ttc_centimes: 11_900,
        stamp_centimes: 0,
        net_to_pay_centimes: 11_900,
        tendered_centimes: None,
        change_centimes: None,
        status: DocumentStatus::Issued,
    }
}

fn open() -> (tempfile::TempDir, SqliteConnection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = crate::db::open(dir.path().join("t.db")).unwrap();
    (dir, conn)
}

#[test]
fn a_row_whose_series_and_year_disagree_is_refused_rather_than_stored() {
    let (_dir, mut conn) = open();
    let err = insert(&mut conn, &a_ticket("doc_ticket:2026", 2025)).unwrap_err();
    assert_eq!(err.code(), "validation", "{err:?}");
    assert_eq!(
        super::documents::table
            .count()
            .get_result::<i64>(&mut conn)
            .unwrap(),
        0,
        "the refused row was written anyway"
    );
}

#[test]
fn a_row_whose_series_names_its_year_goes_in() {
    let (_dir, mut conn) = open();
    assert!(insert(&mut conn, &a_ticket("doc_ticket:2026", 2026)).is_ok());
}
