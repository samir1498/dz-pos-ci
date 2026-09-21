//! What a test about cash handed back needs before it can say anything: the
//! papers the refund is written against, and the two shapes that are awkward
//! to build inline.
//!
//! Shared by `cash_refunds_service` and `cash_refunds_guards`, which are one
//! subject split at the line limit. Nothing here decides a figure: every
//! expected amount is written by hand in the test that asserts it, because a
//! fixture that computed one would be the code under test wearing a hat.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_core::services::documents::{
    Document, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};
use dzpos_core::services::sales::{NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{audit, documents, products, sales};

use super::shifts::AMINA;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

pub fn day(n: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, n).unwrap()
}

pub fn at(n: u32, hour: u32) -> NaiveDateTime {
    day(n).and_hms_opt(hour, 0, 0).unwrap()
}

pub fn product(conn: &mut SqliteConnection, name: &str, selling: i64, rate_bps: u32) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(selling / 2),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 100_000,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(rate_bps).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

pub fn line(product_id: i32, qty_milli: i64) -> NewSaleLine {
    NewSaleLine {
        product_id,
        qty_milli,
        unit_price: None,
        line_discount: Money::ZERO,
    }
}

/// A facture, on whichever payment mode the case is about.
pub fn a_facture(
    conn: &mut SqliteConnection,
    customer_id: i32,
    lines: Vec<NewSaleLine>,
    mode: PaymentMode,
    on: u32,
) -> Document {
    sales::issue(
        conn,
        SHOP,
        OWNER,
        NewSale {
            lines,
            global_discount: Money::ZERO,
            payment_mode: mode,
            tendered: match mode {
                PaymentMode::Cash => Some(Money::centimes(10_000_000)),
                _ => None,
            },
            customer_id: Some(customer_id),
            override_credit: false,
            kind: SaleKind::Facture,
            issued_at: Some(at(on, 10)),
        },
    )
    .unwrap()
    .document
}

/// A facture over the counter with no buyer on it, the walk-in who asked for
/// a facture and paid cash. Stated totals, no product on the line.
pub fn an_anonymous_cash_facture(
    conn: &mut SqliteConnection,
    total_ttc: i64,
    stamp: i64,
    issued_at: NaiveDateTime,
) -> Document {
    let ttc = Money::centimes(total_ttc);
    let stamp = Money::centimes(stamp);
    documents::issue(
        conn,
        SHOP,
        NewDocument {
            kind: DocumentKind::Facture,
            issued_at,
            user_id: AMINA,
            regime: Regime::Ifu,
            payment_mode: PaymentMode::Cash,
            seller: SellerBlock {
                name: "Mon magasin".to_string(),
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                address: None,
                phone: None,
            },
            customer: None,
            buyer: None,
            ref_document_id: None,
            balance: None,
            totals: Totals {
                total_ht: ttc,
                discount: Money::ZERO,
                subtotal_ht: ttc,
                tva_by_rate: vec![TvaLine {
                    rate: Bps::ZERO,
                    base: ttc,
                    amount: Money::ZERO,
                }],
                tva: Money::ZERO,
                total_ttc: ttc,
                stamp,
                net_to_pay: ttc.checked_add(stamp).unwrap(),
            },
            tendered: None,
            change: None,
            lines: vec![NewDocumentLine {
                product_id: None,
                name: "Article".to_string(),
                barcode: None,
                qty_milli: 1_000,
                unit_price: ttc,
                line_discount: Money::ZERO,
                rate_bps: Bps::ZERO,
                line_total: ttc,
                ref_line_id: None,
            }],
        },
    )
    .unwrap()
}

/// A second shop on the same file, with the seeded owner able to sell in it.
pub fn a_second_shop(conn: &mut SqliteConnection) {
    diesel::sql_query("INSERT INTO shops (id, name) VALUES (2, 'Deuxième magasin')")
        .execute(conn)
        .unwrap();
}

/// The `after` block of the newest entry carrying this action.
pub fn last_after(conn: &mut SqliteConnection, action: &str) -> serde_json::Value {
    let entry = audit::list(conn, SHOP)
        .unwrap()
        .into_iter()
        .filter(|e| e.action == action)
        .max_by_key(|e| e.id)
        .unwrap_or_else(|| panic!("no {action} entry"));
    serde_json::from_str(&entry.after.unwrap()).unwrap()
}
