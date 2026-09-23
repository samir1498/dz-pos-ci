//! Product rules: what a name and a price may be, where the TVA rate comes
//! from when none is given, and how a blank barcode gets numbered.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::{CoreError, RetailError};
use crate::models::product::{NewProduct, Product, ProductRowWrite};
use crate::models::stock::{Movement, MovementKind};
use crate::money::{Bps, Money};
use crate::repos::counters;
use crate::repos::products as repo;
use crate::services::categories;
use crate::services::stock;
use dzpos_kernel::services::audit;

/// GS1 prefix 2 is reserved for restricted circulation: codes a shop makes
/// up for itself, which never collide with a manufacturer's barcode.
const IN_STORE_PREFIX: u8 = 2;
const EAN13_LEN: usize = 13;
/// How many taken numbers the allocator walks past before it gives up. A
/// shop would have to type a thousand in-store codes by hand in a row to
/// reach it, and the bound is what keeps the loop from spinning.
const IN_STORE_ATTEMPTS: u32 = 1_000;

pub fn list(conn: &mut SqliteConnection, shop_id: i32) -> Result<Vec<Product>, CoreError> {
    repo::list(conn, shop_id)
}

pub fn get(conn: &mut SqliteConnection, shop_id: i32, id: i32) -> Result<Product, CoreError> {
    repo::get(conn, shop_id, id)
}

/// The product this shop already sells under that barcode, if any. The Excel
/// import asks before it decides whether a row updates a fiche or opens one
/// (features.md §1); it used to ask `repos::products` itself, which is this
/// service's table to answer for.
///
/// `canonical_barcode` is deliberately not applied here. `create` puts a
/// twelve-digit UPC into its thirteen-digit form on the way in, so a lookup
/// that canonicalised as well would start matching rows this door used to
/// miss. That may well be the right answer for the import, but it is a
/// change to what the import does, not to where it knocks, and this move is
/// only the door.
pub fn by_barcode(
    conn: &mut SqliteConnection,
    shop_id: i32,
    barcode: &str,
) -> Result<Option<Product>, CoreError> {
    repo::by_barcode(conn, shop_id, barcode)
}

/// Writes the fiche's cost price: what the goods last landed at, in
/// centimes, as `services::purchases` computed it inside the transaction
/// that received them.
///
/// A pass-through on purpose, adding no check and changing no value. Two
/// reasons, and the second is the fiscal one. The amount arrives already
/// decided: the landing cost is the receipt's own arithmetic, checked there,
/// and a second opinion taken here would either agree (noise) or disagree
/// (two costs for one delivery). And nothing fiscal is measured against this
/// column: the "Cost of goods sold" row of the features.md §3 totals table
/// says a unit's cost is "what it cost when it left, written on the sale's
/// stock movement, never the fiche's cost price, which is the last
/// delivery's and moves with every purchase". The movement that does carry
/// the margin is written a few lines earlier in the same transaction, by
/// `services::stock::record`. So this door exists to stop `purchases` from
/// reaching past `products` into its repo, not to add a rule; a rule added
/// here would be a rule the receipt already applied.
///
/// What the door does skip, named because the rule this refactor serves is
/// that reaching past a sibling skips that sibling's own rules, and this is
/// the one of `products`' rules the receipt path does not get.
/// `update` beside it writes a `product` audit row whenever the cost moves,
/// through `price_or_active_changed`. A receipt moving the same column
/// writes no `product` row: what the log carries is the receipt's own entry
/// (`ACTION_RECEIVE_PURCHASE`, with the order's value and its line count),
/// so a cost change is answerable from the receipt rather than from the
/// fiche. Whether that is enough is a question for Samir and not something
/// this door decides, because the alternative is a `product` row per line on
/// every delivery, which is what `update`'s own filter exists to avoid.
///
/// A negative cost is not refused here for the same reason.
/// `services::purchases::save` refuses a negative `transport` and a
/// negative `extra_costs`, and `spread` beside it refuses a negative line
/// `unit_cost`, so the share it hands a line is never below zero and the
/// landed cost is a `checked_add` of two non-negative amounts. A guard here
/// would be a second reading of a rule the order already applied. That
/// reasoning is about the one caller there is: the guard lives in
/// `purchases`, not here, so a second caller would inherit none of it, and
/// this is a `pub fn` on a `pub mod` where the repo it wraps was reachable
/// only inside the crate.
pub fn set_cost(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    cost: Money,
) -> Result<(), CoreError> {
    repo::set_cost(conn, shop_id, id, cost)
}

pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    new: NewProduct,
) -> Result<Product, RetailError> {
    // The category is checked inside the same transaction as the insert:
    // checked outside, a category deleted in between surfaced as the FK
    // failure, a 500, instead of the 404 the check is there to give.
    conn.transaction(|conn| {
        let mut write = validate(conn, shop_id, &new)?;
        if write.barcode.is_none() {
            write.barcode = Some(next_free_in_store_barcode(conn, shop_id)?);
        }
        // The ledger owns the quantity (architecture.md, Data), so the row
        // starts empty and an opening movement puts the stock in. Written
        // into the column instead, the count was a number no movement
        // explained and the nightly re-derivation would report it for ever.
        let opening = write.qty_on_hand_milli;
        write.qty_on_hand_milli = 0;
        let made = repo::insert(conn, &write)?;
        if opening == 0 {
            return Ok(made);
        }
        stock::record(
            conn,
            shop_id,
            &Movement {
                product_id: made.id,
                kind: MovementKind::Opening,
                qty_milli: opening,
                unit_cost: made.cost,
                document_id: None,
                user_id,
            },
        )?;
        repo::get(conn, shop_id, made.id).map_err(RetailError::from)
    })
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    id: i32,
    new: NewProduct,
) -> Result<Product, RetailError> {
    conn.transaction(|conn| {
        // The product is looked up before the draft is validated, so editing
        // another shop's row reports the product, not whatever its category
        // happens to be.
        let before = repo::get(conn, shop_id, id)?;
        let mut write = validate(conn, shop_id, &new)?;
        // A blank barcode on an update means "leave it alone"; the product
        // already has a number and renumbering it would orphan printed labels.
        if write.barcode.is_none() {
            write.barcode.clone_from(&before.barcode);
        }
        // The quantity on hand belongs to the stock ledger (features.md §1),
        // not to the fiche: an edit made from a list read minutes ago must
        // not undo the sales since. The field rides along on the wire because
        // add and edit share one shape; here it is the stored value.
        write.qty_on_hand_milli = before.qty_on_hand_milli;
        let after = repo::update(conn, shop_id, id, &write)?;
        // Only a price or the active flag: features.md §5 names those as the
        // sensitive ones, and logging a renamed product on every edit would
        // bury them.
        if price_or_active_changed(&before, &after) {
            audit::record(
                conn,
                shop_id,
                user_id,
                audit::Change {
                    action: audit::ACTION_UPDATE,
                    entity: "product",
                    entity_id: Some(id),
                    before: Some(as_json(&before)),
                    after: Some(as_json(&after)),
                },
            )?;
        }
        Ok(after)
    })
}

fn price_or_active_changed(before: &Product, after: &Product) -> bool {
    before.cost != after.cost
        || before.selling != after.selling
        || before.wholesale != after.wholesale
        || before.active != after.active
}

fn as_json(p: &Product) -> String {
    serde_json::json!({
        "cost_centimes": p.cost.as_centimes(),
        "selling_centimes": p.selling.as_centimes(),
        "wholesale_centimes": p.wholesale.map(Money::as_centimes),
        "active": p.active,
    })
    .to_string()
}

fn validate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    new: &NewProduct,
) -> Result<ProductRowWrite, CoreError> {
    let name = new.name.trim();
    if name.is_empty() {
        return Err(CoreError::validation("name", "a product needs a name"));
    }
    if new.selling.is_negative() {
        return Err(CoreError::validation(
            "selling_centimes",
            "a selling price cannot be negative",
        ));
    }
    if new.cost.is_negative() {
        return Err(CoreError::validation(
            "cost_centimes",
            "a cost price cannot be negative",
        ));
    }
    if new.wholesale.is_some_and(Money::is_negative) {
        return Err(CoreError::validation(
            "wholesale_centimes",
            "a wholesale price cannot be negative",
        ));
    }
    if new.qty_on_hand_milli < 0 {
        return Err(CoreError::validation(
            "qty_on_hand_milli",
            "stock on hand cannot be negative",
        ));
    }
    if new.low_stock_at_milli < 0 {
        return Err(CoreError::validation(
            "low_stock_at_milli",
            "a low stock threshold cannot be negative",
        ));
    }

    let barcode = new
        .barcode
        .as_deref()
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .map(canonical_barcode);

    // The category is looked up whatever the rate says. It used to be read
    // only when it had to supply a rate, so a caller who named its own rate
    // could point a product at another shop's category (rule 3).
    let category_rate = match new.category_id {
        Some(id) => Some(categories::rate_of(conn, shop_id, id)?),
        None => None,
    };
    let rate = resolve_rate(new, category_rate)?;

    Ok(ProductRowWrite {
        shop_id,
        name: name.to_string(),
        barcode,
        category_id: new.category_id,
        unit: new.unit,
        cost_centimes: new.cost.as_centimes(),
        selling_centimes: new.selling.as_centimes(),
        wholesale_centimes: new.wholesale.map(Money::as_centimes),
        qty_on_hand_milli: new.qty_on_hand_milli,
        low_stock_at_milli: new.low_stock_at_milli,
        rate_bps: i32::try_from(rate.as_u32())
            .map_err(|_| CoreError::validation("rate_bps", "rate out of range"))?,
        active: new.active,
        updated_at: chrono::Utc::now().naive_utc(),
    })
}

/// An explicit rate wins; otherwise the category's default, already read and
/// already proved to belong to this shop. With neither, there is nothing to
/// guess from, so the caller is told.
fn resolve_rate(new: &NewProduct, category_rate_bps: Option<i32>) -> Result<Bps, CoreError> {
    if let Some(rate) = new.rate_bps {
        return Ok(rate);
    }
    let Some(raw) = category_rate_bps else {
        return Err(CoreError::validation(
            "rate_bps",
            "a product without a category must name its TVA rate",
        ));
    };
    let raw = u32::try_from(raw).map_err(|_| crate::money::MoneyError::RateOutOfRange)?;
    Ok(Bps::new(raw)?)
}

/// Takes numbers from the shop's counter until one is free. The number can
/// never come from the row's id: a user is allowed to type the code the next
/// row would have taken, and deriving it from the id made that insert roll
/// back without consuming the id, so every later blank create hit the same
/// number and failed for ever.
/// The one form a barcode is stored and compared in.
///
/// A UPC-A is twelve digits and the same article's EAN-13 is those twelve
/// with a zero in front. A scanner reads whichever is printed on the box, so
/// the till has to compare both sides in one form; folding it in here as
/// well is what stops the same article being filed twice under its two
/// numbers, which the UNIQUE index cannot see because the two are different
/// strings. `canonicalBarcode` in apps/desktop/src/lib/scan.ts is the same
/// rule on the other side of the wire.
fn canonical_barcode(code: &str) -> String {
    if code.len() == 12 && code.bytes().all(|b| b.is_ascii_digit()) {
        format!("0{code}")
    } else {
        code.to_string()
    }
}

fn next_free_in_store_barcode(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<String, CoreError> {
    for _ in 0..IN_STORE_ATTEMPTS {
        // The barcode series carries no year: it is not a document series and
        // it never restarts (features.md §1).
        let sequence = counters::take_next(conn, shop_id, counters::IN_STORE_BARCODE.to_string())?;
        let code = in_store_barcode(shop_id, sequence)?;
        if !repo::barcode_exists(conn, shop_id, &code)? {
            return Ok(code);
        }
    }
    Err(CoreError::validation(
        "barcode",
        "no free in-store barcode was found",
    ))
}

/// EAN-13 in the restricted-circulation range: prefix, shop, sequence, check
/// digit. Assumption, not law: see the report and features.md §1.
fn in_store_barcode(shop_id: i32, sequence: i64) -> Result<String, CoreError> {
    let shop = u64::try_from(shop_id)
        .map_err(|_| CoreError::validation("shop_id", "shop id is not a barcode-able number"))?;
    let seq = u64::try_from(sequence).map_err(|_| {
        CoreError::validation("barcode", "the barcode counter is not a usable number")
    })?;
    if shop > 99_999 || seq > 999_999 {
        return Err(CoreError::validation(
            "barcode",
            "the shop or the barcode counter no longer fits an in-store EAN-13",
        ));
    }
    let body = format!("{IN_STORE_PREFIX}{shop:05}{seq:06}");
    let check = ean13_check_digit(&body)?;
    Ok(format!("{body}{check}"))
}

/// GS1 check digit: odd positions weigh 1, even weigh 3, the digit brings
/// the total to the next multiple of ten.
fn ean13_check_digit(body: &str) -> Result<u32, CoreError> {
    if body.len() != EAN13_LEN - 1 {
        return Err(CoreError::validation("barcode", "not 12 digits"));
    }
    let mut sum: u32 = 0;
    for (i, c) in body.chars().enumerate() {
        let d = c
            .to_digit(10)
            .ok_or_else(|| CoreError::validation("barcode", "not numeric"))?;
        sum += if i % 2 == 0 { d } else { d * 3 };
    }
    Ok((10 - (sum % 10)) % 10)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "../../tests/unit/services_products.rs"]
mod tests;
