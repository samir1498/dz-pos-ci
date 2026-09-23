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
use dzpos_kernel::services::{audit, shops};
use dzpos_retail::models::product::{NewProduct, Unit};
use dzpos_retail::models::shop::StoreBlock;
use dzpos_retail::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_retail::services::documents::{
    Document, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};
use dzpos_retail::services::sales::{NewSale, NewSaleLine, SaleKind};
use dzpos_retail::services::{customers, documents, products, sales};

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

/// The second shop, set up well enough to issue a facture of its own: a
/// régime in force and the seller identifiers `sales::issue` refuses a
/// facture without.
///
/// A sibling of `a_second_shop` rather than a change to it, so the cash-only
/// case that already uses that helper keeps the fixture it was written
/// against. The régime row is written in SQL the way the first migration
/// seeds shop 1's, because there is no service for "a shop exists" — shops
/// are not created by this product yet.
pub fn a_second_shop_selling_factures(conn: &mut SqliteConnection, shop_id: i32) {
    a_second_shop(conn);
    diesel::sql_query(format!(
        "INSERT INTO settings (shop_id, key, value, valid_from) \
         VALUES ({shop_id}, 'regime_fiscal', 'reel', '2020-01-01 00:00:00')"
    ))
    .execute(conn)
    .unwrap();
    shops::update_store(
        conn,
        shop_id,
        OWNER,
        StoreBlock {
            name: "Deuxième magasin".to_string(),
            rc: Some("16/00-7654999 B 25".to_string()),
            nif: None,
            nis: Some("000216007654999 00".to_string()),
            ai: None,
            address: None,
            phone: None,
        },
    )
    .unwrap();
}

/// The same facture, on whichever shop's books.
///
/// A sibling rather than a `shop_id` parameter on `a_facture`: every case but
/// the scoping ones sells in shop 1 and reads better for saying nothing about
/// it, which is the shape `common::shifts::a_sale_in` already takes next
/// door. It exists because `a_facture` hardcodes `SHOP`, so before this no
/// credit facture in either suite could be built anywhere else and the
/// `shop_id` filters on the payments query had nothing standing on them.
pub fn a_facture_in(
    conn: &mut SqliteConnection,
    shop_id: i32,
    customer_id: i32,
    lines: Vec<NewSaleLine>,
    mode: PaymentMode,
    on: u32,
) -> Document {
    sales::issue(
        conn,
        shop_id,
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

/// A product on whichever shop's shelves, for the same reason.
pub fn product_in(
    conn: &mut SqliteConnection,
    shop_id: i32,
    name: &str,
    selling: i64,
    rate_bps: u32,
) -> i32 {
    products::create(
        conn,
        shop_id,
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

/// A customer on whichever shop's books, carrying the identifiers a facture
/// is refused without.
pub fn a_customer_in(conn: &mut SqliteConnection, shop_id: i32, name: &str) -> i32 {
    customers::create(conn, shop_id, OWNER, super::an_identified_fiche(name), None)
        .unwrap()
        .id
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
