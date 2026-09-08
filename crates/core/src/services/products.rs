//! Product rules: what a name and a price may be, where the TVA rate comes
//! from when none is given, and how a blank barcode gets numbered.

use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::product::{NewProduct, Product, ProductRowWrite};
use crate::money::{Bps, Money};
use crate::repos::counters;
use crate::repos::products as repo;

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

pub fn create(
    conn: &mut SqliteConnection,
    shop_id: i32,
    new: NewProduct,
) -> Result<Product, CoreError> {
    let mut write = validate(conn, shop_id, &new)?;
    conn.transaction(|conn| {
        if write.barcode.is_none() {
            write.barcode = Some(next_free_in_store_barcode(conn, shop_id)?);
        }
        repo::insert(conn, &write)
    })
}

pub fn update(
    conn: &mut SqliteConnection,
    shop_id: i32,
    id: i32,
    new: NewProduct,
) -> Result<Product, CoreError> {
    conn.transaction(|conn| {
        // The product is looked up before the draft is validated, so editing
        // another shop's row reports the product, not whatever its category
        // happens to be.
        let before = repo::get(conn, shop_id, id)?;
        let mut write = validate(conn, shop_id, &new)?;
        // A blank barcode on an update means "leave it alone"; the product
        // already has a number and renumbering it would orphan printed labels.
        if write.barcode.is_none() {
            write.barcode = before.barcode;
        }
        repo::update(conn, shop_id, id, &write)
    })
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
        .map(str::to_string);

    let rate = resolve_rate(conn, shop_id, new)?;

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

/// An explicit rate wins; otherwise the category's default. With neither,
/// there is nothing to guess from, so the caller is told.
fn resolve_rate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    new: &NewProduct,
) -> Result<Bps, CoreError> {
    if let Some(rate) = new.rate_bps {
        return Ok(rate);
    }
    let Some(category_id) = new.category_id else {
        return Err(CoreError::validation(
            "rate_bps",
            "a product without a category must name its TVA rate",
        ));
    };
    let raw = repo::category_default_rate_bps(conn, shop_id, category_id)?.ok_or(
        CoreError::NotFound {
            entity: "category",
            id: category_id,
        },
    )?;
    let raw = u32::try_from(raw).map_err(|_| crate::money::MoneyError::RateOutOfRange)?;
    Ok(Bps::new(raw)?)
}

/// Takes numbers from the shop's counter until one is free. The number can
/// never come from the row's id: a user is allowed to type the code the next
/// row would have taken, and deriving it from the id made that insert roll
/// back without consuming the id, so every later blank create hit the same
/// number and failed for ever.
fn next_free_in_store_barcode(
    conn: &mut SqliteConnection,
    shop_id: i32,
) -> Result<String, CoreError> {
    for _ in 0..IN_STORE_ATTEMPTS {
        let sequence = counters::take_next(conn, shop_id, counters::IN_STORE_BARCODE)?;
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
mod tests {
    use super::*;

    #[test]
    fn check_digit_matches_a_known_ean13() {
        // 978020137962-x, the ISBN example GS1 publishes; the digit is 4.
        assert_eq!(ean13_check_digit("978020137962").unwrap(), 4);
    }

    #[test]
    fn an_in_store_code_is_thirteen_digits_starting_at_two() {
        let code = in_store_barcode(1, 7).unwrap();
        assert_eq!(code.len(), 13);
        assert!(code.starts_with("200001000007"), "{code}");
    }
}
