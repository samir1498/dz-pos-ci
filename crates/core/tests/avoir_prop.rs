// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The one thing a shop and a comptable read across a facture and its credit
//! notes: the avoirs add up to the facture. Over splits nobody chose, at
//! prices whose tax does not land on a centime, against a real temp SQLite
//! file.
//!
//! A slice of a facture is not a fraction of it. The tax on each avoir is
//! rounded once on that avoir's own base, so slicing a facture and adding the
//! slices back up gives a different figure from the facture itself, by a
//! centime here and a centime there. The closing avoir is the difference
//! rather than a slice, and this is what says so (features.md §3).

use diesel::sqlite::SqliteConnection;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::models::shop::StoreBlock;
use dzpos_core::money::{Bps, Money, PaymentMode};
use dzpos_core::services::avoir::{self, AvoirLine};
use dzpos_core::services::customers::{self, NewCustomer, PartyKind};
use dzpos_core::services::documents::{Document, DocumentStatus};
use dzpos_core::services::sales::{self, NewSale, NewSaleLine, SaleKind};
use dzpos_core::services::{debt, documents, products, shops};
use proptest::prelude::*;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

/// Prices whose tax lands between two centimes at the rates a shop uses, and
/// quantities that split into halves and thirds. 50 at 19 % is 9,5 centimes a
/// unit and 80 at 19 % is 15,2: one rounds up and the other down, which is the
/// pair that makes a sum of slices land on either side of its facture.
fn a_price() -> impl Strategy<Value = i64> {
    prop_oneof![Just(50i64), Just(80), Just(333), Just(777), Just(1_999)]
}

fn a_rate() -> impl Strategy<Value = u32> {
    prop_oneof![Just(1900u32), Just(900), Just(0)]
}

/// Two or three lines, each a price, a rate and a quantity in thousandths,
/// with a global discount small enough to leave every line positive.
type Basket = (Vec<(i64, u32, i64)>, i64);

fn a_basket() -> impl Strategy<Value = Basket> {
    (
        prop::collection::vec((a_price(), a_rate(), 1_000i64..=6_000), 2..=3),
        0i64..=97,
    )
}

/// How each line is cut up: for every line, the quantities of the successive
/// partial avoirs. What is left after them is taken by the closing one.
fn a_split() -> impl Strategy<Value = Vec<Vec<u32>>> {
    prop::collection::vec(prop::collection::vec(1u32..=3, 0..=3), 3)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    #[test]
    fn the_avoirs_on_a_facture_add_up_to_it((basket, discount) in a_basket(), split in a_split()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let mut conn = dzpos_core::db::open(&path).unwrap();
        a_shop(&mut conn);
        let customer = a_customer(&mut conn);

        let lines: Vec<NewSaleLine> = basket
            .iter()
            .enumerate()
            .map(|(n, (price, rate, qty))| NewSaleLine {
                product_id: a_product(&mut conn, n, *price, *rate),
                qty_milli: *qty,
                unit_price: None,
                line_discount: Money::ZERO,
            })
            .collect();
        let facture = sales::issue(
            &mut conn,
            SHOP,
            OWNER,
            NewSale {
                lines,
                global_discount: Money::centimes(discount),
                payment_mode: PaymentMode::Credit,
                tendered: None,
                customer_id: Some(customer),
                override_credit: false,
                kind: SaleKind::Facture,
                issued_at: chrono::NaiveDate::from_ymd_opt(2026, 9, 10)
                    .and_then(|d| d.and_hms_opt(10, 0, 0)),
            },
        )
        .unwrap()
        .document;

        // The partials, each taking a slice off whichever lines still have
        // one. A slice that would empty the facture is left to the closing
        // avoir, so what is under test stays the arithmetic and not the
        // ordering.
        let mut day = 11;
        for (round, cuts) in split.iter().enumerate() {
            let mut asked: Vec<AvoirLine> = Vec::new();
            for (n, line) in facture.lines.iter().enumerate() {
                let want = i64::from(cuts.get(n).copied().unwrap_or(0)) * 1_000;
                if want == 0 {
                    continue;
                }
                let left = left_on(&mut conn, &facture, line.id);
                // Strictly less, so a partial never closes the facture.
                if want < left {
                    asked.push(AvoirLine { document_line_id: line.id, qty_milli: want });
                }
            }
            if asked.is_empty() {
                continue;
            }
            avoir::issue(&mut conn, SHOP, OWNER, facture.id, Some(asked), None, on(day))
                .map_err(|e| TestCaseError::fail(format!("partial {round}: {e:?}")))?;
            day += 1;
        }

        // The cancellation writes the closing avoir, which is the whole point:
        // it must never be refused for a centime of rounding.
        documents::cancel(
            &mut conn,
            SHOP,
            OWNER,
            facture.id,
            "commande annulée".to_string(),
            on(day),
        )
        .map_err(|e| TestCaseError::fail(format!("the cancellation was refused: {e:?}")))?;

        let avoirs = avoir::list_for(&mut conn, SHOP, facture.id).unwrap();
        let mut ht = Money::ZERO;
        let mut disc = Money::ZERO;
        let mut sub = Money::ZERO;
        let mut tva = Money::ZERO;
        let mut ttc = Money::ZERO;
        for a in &avoirs {
            prop_assert_eq!(a.totals.stamp, Money::ZERO);
            prop_assert_eq!(a.totals.net_to_pay, a.totals.total_ttc);
            ht = ht.checked_add(a.totals.total_ht).unwrap();
            disc = disc.checked_add(a.totals.discount).unwrap();
            sub = sub.checked_add(a.totals.subtotal_ht).unwrap();
            tva = tva.checked_add(a.totals.tva).unwrap();
            ttc = ttc.checked_add(a.totals.total_ttc).unwrap();
            // Every stored document adds up from its own stored lines, the
            // closing one included: the reprint reads the lines.
            let summed = a
                .lines
                .iter()
                .try_fold(Money::ZERO, |acc, l| acc.checked_add(l.line_total))
                .unwrap();
            prop_assert_eq!(summed, a.totals.total_ht, "avoir {} lines", a.id);
        }
        prop_assert_eq!(ht, facture.totals.total_ht, "HT");
        prop_assert_eq!(disc, facture.totals.discount, "discount");
        prop_assert_eq!(sub, facture.totals.subtotal_ht, "subtotal");
        prop_assert_eq!(tva, facture.totals.tva, "TVA");
        prop_assert_eq!(ttc, facture.totals.total_ttc, "TTC");

        // Per rate too, which is the line of the yearly declaration.
        for row in &facture.totals.tva_by_rate {
            let mut base = Money::ZERO;
            let mut amount = Money::ZERO;
            for a in &avoirs {
                for r in a.totals.tva_by_rate.iter().filter(|r| r.rate == row.rate) {
                    base = base.checked_add(r.base).unwrap();
                    amount = amount.checked_add(r.amount).unwrap();
                }
            }
            prop_assert_eq!(base, row.base, "base at {:?}", row.rate);
            prop_assert_eq!(amount, row.amount, "tva at {:?}", row.rate);
        }

        // And the account is square: the facture asks for nothing and the
        // customer neither owes nor is owed.
        let after = documents::get(&mut conn, SHOP, facture.id).unwrap();
        prop_assert_eq!(after.status, DocumentStatus::Cancelled);
        prop_assert_eq!(after.balance.map(|b| b.remaining_debt), Some(Money::ZERO));
        prop_assert_eq!(debt::balance(&mut conn, SHOP, customer).unwrap(), Money::ZERO);
    }
}

fn on(day: u32) -> Option<chrono::NaiveDateTime> {
    chrono::NaiveDate::from_ymd_opt(2026, 9, day).and_then(|d| d.and_hms_opt(10, 0, 0))
}

/// What is still on one facture line after the avoirs written so far.
fn left_on(conn: &mut SqliteConnection, facture: &Document, line_id: i32) -> i64 {
    let taken: i64 = avoir::list_for(conn, SHOP, facture.id)
        .unwrap()
        .iter()
        .flat_map(|a| a.lines.clone())
        .filter(|l| l.ref_line_id == Some(line_id))
        .map(|l| l.qty_milli)
        .sum();
    facture
        .lines
        .iter()
        .find(|l| l.id == line_id)
        .map_or(0, |l| l.qty_milli)
        - taken
}

fn a_shop(conn: &mut SqliteConnection) {
    shops::update_store(
        conn,
        SHOP,
        OWNER,
        StoreBlock {
            name: "Mon magasin".to_string(),
            rc: Some("16/00-1234567 B 25".to_string()),
            nif: None,
            nis: Some("000216001234567 00".to_string()),
            ai: None,
            address: None,
            phone: None,
        },
    )
    .unwrap();
}

fn a_customer(conn: &mut SqliteConnection) -> i32 {
    customers::create(
        conn,
        SHOP,
        OWNER,
        NewCustomer {
            name: "Entreprise Benali".to_string(),
            party_kind: PartyKind::Company,
            phone: None,
            address: None,
            rc: Some("16/00-7654321 B 22".to_string()),
            nif: None,
            nis: Some("000216007654321 00".to_string()),
            ai: None,
            credit_limit: None,
            warn_threshold: None,
            notes: None,
            active: true,
        },
        None,
    )
    .unwrap()
    .id
}

fn a_product(conn: &mut SqliteConnection, n: usize, selling: i64, rate_bps: u32) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: format!("Article {n}"),
            barcode: None,
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(selling / 2),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 100_000,
            low_stock_at_milli: 0,
            rate_bps: Bps::new(rate_bps).ok(),
            active: true,
        },
    )
    .unwrap()
    .id
}
