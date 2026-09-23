//! The fixtures both till-shift suites read: the people at the counter and
//! the one document helper that puts cash in a drawer.
//!
//! Shared between `shifts_service.rs` and `shifts_who_may_close.rs`, and only
//! between those two. `common/facture.rs` beside this one carries the warning
//! that two suites checking each other's work through one helper can both be
//! made green by editing the helper once. That is not the shape here: one
//! suite asks what a drawer is expected to hold and the other asks who is
//! allowed to count it, so a helper bent to make either of them pass does
//! not quietly satisfy the other — it changes the fixture under a question
//! it was never asked.
//!
//! Nothing here decides anything. `a_sale_with_stamp` states its totals
//! rather than computing them, because what these suites are about is whose
//! cash a shift counts and who may close it, never how a basket adds up.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::{RunQueryDsl, SqliteConnection};
use dzpos_retail::money::{Bps, Money, PaymentMode, Regime, Totals, TvaLine};
use dzpos_retail::services::documents::{
    self, DocumentKind, NewDocument, NewDocumentLine, SellerBlock,
};

use super::SHOP;

/// The shift's own cashier, the user the first migration seeds. An owner by
/// role, which is what lets her count a drawer that is not her own.
pub const AMINA: i32 = 1;
/// The second person at the same till, a cashier: her takings are hers, and a
/// shop-wide sum would hand them to Amina.
pub const KARIM: i32 = 2;
/// Whoever is running the floor. Ruling 10 makes counting somebody else's
/// drawer manager and owner work, so a case where the closer is not the
/// opener needs a person who holds `CloseAnotherPersonsTill`; a second
/// cashier does not.
pub const LEILA: i32 = 3;

/// The fixture day.
pub fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 14).unwrap()
}

/// A moment on the fixture day, on the shop's clock.
pub fn at(hour: u32, minute: u32, second: u32) -> NaiveDateTime {
    day().and_hms_opt(hour, minute, second).unwrap()
}

/// A second person at the till. The seeded file carries one user, and a
/// fixture with one user cannot tell a per-cashier figure from a shop-wide
/// one.
pub fn a_second_cashier(conn: &mut SqliteConnection) {
    a_colleague(conn, KARIM, "Karim", "cashier");
}

/// The person who counts a drawer its owner walked away from. A cashier may
/// not (ruling 10), so a fixture that wants that case wants this role.
pub fn a_floor_manager(conn: &mut SqliteConnection) {
    a_colleague(conn, LEILA, "Leila", "manager");
}

fn a_colleague(conn: &mut SqliteConnection, id: i32, name: &str, role: &str) {
    diesel::sql_query(format!(
        "INSERT INTO users (id, shop_id, name, role) VALUES ({id}, {SHOP}, '{name}', '{role}')"
    ))
    .execute(conn)
    .unwrap();
}

/// A document written straight through `services::documents`, with its totals
/// stated rather than computed.
///
/// `user_id` is a parameter and not a constant, which is the whole reason
/// this helper is not `cash_service.rs`'s: a fixture that could not name a
/// second ringer could not tell a per-cashier figure from a shop-wide one.
pub fn a_sale(
    conn: &mut SqliteConnection,
    user_id: i32,
    mode: PaymentMode,
    total_ttc: i64,
    issued_at: NaiveDateTime,
) -> i32 {
    a_sale_with_stamp(conn, SHOP, user_id, mode, total_ttc, 0, issued_at)
}

/// The same, on whichever shop's books. Only the second-shop cases name a
/// shop; everything else sells in this one and reads better for it.
pub fn a_sale_in(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    mode: PaymentMode,
    total_ttc: i64,
    issued_at: NaiveDateTime,
) -> i32 {
    a_sale_with_stamp(conn, shop_id, user_id, mode, total_ttc, 0, issued_at)
}

/// The same, on a facture carrying a droit de timbre. The customer hands the
/// stamp over with the rest, so the drawer holds `net_to_pay` and not
/// `total_ttc`.
pub fn a_sale_with_stamp(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    mode: PaymentMode,
    total_ttc: i64,
    stamp: i64,
    issued_at: NaiveDateTime,
) -> i32 {
    let ttc = Money::centimes(total_ttc);
    let stamp = Money::centimes(stamp);
    documents::issue(
        conn,
        shop_id,
        NewDocument {
            kind: DocumentKind::Ticket,
            issued_at,
            user_id,
            regime: Regime::Ifu,
            payment_mode: mode,
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
    .id
}
