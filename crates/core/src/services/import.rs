//! The product import of features.md §1: a template a shop downloads, a dry
//! run that says what would happen to every row, and an apply that writes the
//! whole file or none of it.
//!
//! Three rules shape the module.
//!
//! Nothing is written until every row is accepted. A shop importing a
//! catalogue is not watching each line go past; a file half in is worse than
//! a file refused, because the half that landed has to be found again by
//! hand. So `dry_run` writes nothing at all and `apply` runs the same check
//! inside its transaction before the first insert.
//!
//! A row is matched on its barcode, never on its name. A code the shop
//! already sells under updates that product; a blank code opens a new one and
//! the till numbers it, exactly as the fiche does. Two rows in one file
//! carrying the same code refuse each other: neither is more the right one
//! than the other, and picking one silently would drop a product the shop
//! meant to have.
//!
//! A number reaches an integer through its decimal spelling. A cell is an
//! IEEE double, so the value is written back out as the shortest decimal that
//! round-trips (which is what the person typed) and then read into centimes
//! or thousandths with integer arithmetic, rounding half away from zero, the
//! way the money module rounds everywhere else. No price is ever the result
//! of `value * 100.0`.

use calamine::{Data, Reader, Xlsx};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::category::CategoryRowWrite;
use crate::models::product::{NewProduct, Unit};
use crate::money::{Bps, Money};
use crate::print::strings::{text as word, Key};
use crate::repos::categories as categories_repo;
use crate::repos::products as products_repo;
use crate::services::{audit, products};

pub use crate::services::export::PRODUCT_COLUMNS;

/// A price is two decimals; a quantity is three (features.md §1, a quantity
/// is thousandths of the unit).
const MONEY_SCALE: u32 = 2;
const QTY_SCALE: u32 = 3;

/// The rates a row may name, as the percentages a person types them:
/// features.md, TVA rates row (19 % standard, 9 % reduced, 0 % exempt). A
/// product may be stored at any rate the file allows, but nothing typed into
/// a spreadsheet gets to invent one: an import is where a `7` slips in.
pub const ALLOWED_RATES_BPS: [u32; 3] = [1900, 900, 0];

/// The units a row may name. `Unit` has exactly these four and the products
/// table's CHECK has the same four; the list is here so the template can
/// print it.
pub const ALLOWED_UNITS: [Unit; 4] = [Unit::Piece, Unit::Kg, Unit::Litre, Unit::Box];

/// What the import would do with one row, or why it will not.
///
/// `field` and `reason` are stable keys the screen translates, never a
/// sentence: the same refusal has to read in French, English and Arabic, and
/// the core has no dictionary for a screen (architecture.md, error policy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Created,
    Updated,
    Refused {
        field: &'static str,
        reason: &'static str,
    },
}

/// One line of the report, named the way a person reading the spreadsheet
/// beside it would: the row number they see, and the product on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowReport {
    /// The spreadsheet's own row number, header counted: the first product
    /// is row 2, so a refusal can be pointed at without arithmetic.
    pub row: u32,
    pub name: String,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DryRun {
    pub rows: Vec<RowReport>,
    pub accepted: usize,
    pub refused: usize,
}

/// What an apply wrote. The three counts are what the audit entry carries and
/// what the screen says afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    pub created: usize,
    pub updated: usize,
    pub categories_created: usize,
}

/// A whole product import, audited once with its counts. Every product it
/// touched is audited by `services::products` on its own terms; this row is
/// what says the touches were one file and not an afternoon of edits.
pub const ACTION_IMPORT_PRODUCTS: &str = "product.import";

/// The template a shop downloads: the columns the import matches on, one
/// example row under them, and a second sheet naming the units and the rates
/// a row may hold and saying what a barcode the shop already uses will do.
pub fn template(lang: Lang) -> Result<Vec<u8>, CoreError> {
    let mut workbook = rust_xlsxwriter::Workbook::new();
    let bold = rust_xlsxwriter::Format::new().set_bold();

    let sheet = workbook.add_worksheet();
    sheet.set_name(word(Key::SheetProducts, lang))?;
    for (index, column) in PRODUCT_COLUMNS.iter().enumerate() {
        let at = u16::try_from(index)
            .map_err(|_| CoreError::validation("column", "more columns than Excel holds"))?;
        sheet.write_string_with_format(0, at, *column, &bold)?;
        sheet.set_column_width(at, 18)?;
    }
    // The example is a row the import accepts as it stands, so a shop that
    // downloads the template, adds its own rows and sends the file back is
    // not refused by the line it was shown. Its barcode is blank on purpose:
    // that is the case the note on the second sheet is about.
    let example: [&str; 11] = [
        "Café moulu 250 g",
        "",
        "Alimentation",
        "piece",
        "80.50",
        "120.00",
        "",
        "12",
        "3",
        "19",
        "true",
    ];
    for (index, value) in example.iter().enumerate() {
        let at = u16::try_from(index)
            .map_err(|_| CoreError::validation("column", "more columns than Excel holds"))?;
        sheet.write_string(1, at, *value)?;
    }

    let allowed = workbook.add_worksheet();
    allowed.set_name(word(Key::SheetAllowedValues, lang))?;
    allowed.set_column_width(0, 24)?;
    allowed.set_column_width(1, 60)?;
    allowed.write_string_with_format(0, 0, word(Key::TemplateUnits, lang), &bold)?;
    for (index, unit) in ALLOWED_UNITS.iter().enumerate() {
        let row = u32::try_from(index + 1)
            .map_err(|_| CoreError::validation("row", "more units than a sheet holds"))?;
        allowed.write_string(row, 0, unit.as_str())?;
    }
    let rates_row = u32::try_from(ALLOWED_UNITS.len() + 2)
        .map_err(|_| CoreError::validation("row", "more units than a sheet holds"))?;
    allowed.write_string_with_format(rates_row, 0, word(Key::TemplateRates, lang), &bold)?;
    for (index, bps) in ALLOWED_RATES_BPS.iter().enumerate() {
        let row = rates_row
            .checked_add(u32::try_from(index + 1).unwrap_or(u32::MAX))
            .ok_or_else(|| CoreError::validation("row", "more rates than a sheet holds"))?;
        // As the percentage the column takes: the cell says `19`, not `1900`.
        allowed.write_string(row, 0, (bps / 100).to_string())?;
    }
    let note_row = rates_row
        .checked_add(u32::try_from(ALLOWED_RATES_BPS.len() + 2).unwrap_or(u32::MAX))
        .ok_or_else(|| CoreError::validation("row", "more rates than a sheet holds"))?;
    allowed.write_string(note_row, 0, word(Key::TemplateBarcodeNote, lang))?;

    Ok(workbook.save_to_buffer()?)
}

/// What the file would do, row by row, with nothing written. The connection
/// is read from: a barcode the shop already sells under is what tells a
/// create from an update, and a category the file names has to be looked up
/// before a rate can be defaulted from it.
pub fn dry_run(
    conn: &mut SqliteConnection,
    shop_id: i32,
    bytes: &[u8],
) -> Result<DryRun, CoreError> {
    let drafts = parse(bytes)?;
    assess(conn, shop_id, &drafts)
}

/// The file, written, in one transaction, or nothing at all.
///
/// The dry run happens again in here rather than being trusted from an
/// earlier call: between the screen's dry run and its apply the shop could
/// have sold, renamed or renumbered anything, and a file checked against a
/// state that has moved on is not checked.
pub fn apply(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    bytes: &[u8],
) -> Result<Applied, CoreError> {
    let drafts = parse(bytes)?;
    conn.transaction(|conn| {
        let report = assess(conn, shop_id, &drafts)?;
        if report.refused > 0 {
            return Err(CoreError::validation(
                "rows",
                "the file has rows this shop cannot take; nothing was written",
            ));
        }
        let mut done = Applied {
            created: 0,
            updated: 0,
            categories_created: 0,
        };
        for draft in &drafts {
            let fields = draft
                .parsed
                .as_ref()
                .map_err(|_| CoreError::validation("rows", "a refused row reached the write"))?;
            let category = match fields.category.as_deref() {
                None => None,
                Some(name) => Some(category_id(
                    conn, shop_id, user_id, name, fields, &mut done,
                )?),
            };
            let rate = resolve_rate(conn, shop_id, category, fields)?;
            let new = NewProduct {
                name: draft.name.clone(),
                barcode: fields.barcode.clone(),
                category_id: category,
                unit: fields.unit,
                cost: fields.cost,
                selling: fields.selling,
                wholesale: fields.wholesale,
                qty_on_hand_milli: fields.qty_on_hand_milli,
                low_stock_at_milli: fields.low_stock_at_milli,
                rate_bps: Some(rate),
                active: fields.active,
            };
            match existing(conn, shop_id, fields.barcode.as_deref())? {
                Some(id) => {
                    products::update(conn, shop_id, user_id, id, new)?;
                    done.updated = done.updated.saturating_add(1);
                }
                None => {
                    products::create(conn, shop_id, user_id, new)?;
                    done.created = done.created.saturating_add(1);
                }
            }
        }
        audit::record(
            conn,
            shop_id,
            user_id,
            audit::Change {
                action: ACTION_IMPORT_PRODUCTS,
                entity: "product",
                entity_id: None,
                before: None,
                after: Some(
                    serde_json::json!({
                        "created": done.created,
                        "updated": done.updated,
                        "categories_created": done.categories_created,
                    })
                    .to_string(),
                ),
            },
        )?;
        Ok(done)
    })
}

/// The category by that name, opened when the shop has none. A category the
/// file invented is a row nobody asked for on any screen, so it is audited
/// as the create it is; its default rate is the one the row that named it
/// carries, which is the only rate the file gives us to go on.
fn category_id(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    name: &str,
    fields: &Fields,
    done: &mut Applied,
) -> Result<i32, CoreError> {
    if let Some(found) = categories_repo::by_name(conn, shop_id, name)? {
        return Ok(found.id);
    }
    let rate = fields.rate_bps.ok_or_else(|| {
        CoreError::validation("rate_percent", "a new category needs the rate of its row")
    })?;
    let made = categories_repo::insert(
        conn,
        &CategoryRowWrite {
            shop_id,
            name: name.to_string(),
            default_rate_bps: i32::try_from(rate.as_u32())
                .map_err(|_| CoreError::validation("rate_percent", "rate out of range"))?,
        },
    )?;
    audit::record(
        conn,
        shop_id,
        user_id,
        audit::Change {
            action: audit::ACTION_CREATE,
            entity: "category",
            entity_id: Some(made.id),
            before: None,
            after: Some(
                serde_json::json!({
                    "name": made.name,
                    "default_rate_bps": made.default_rate_bps.as_u32(),
                    "source": ACTION_IMPORT_PRODUCTS,
                })
                .to_string(),
            ),
        },
    )?;
    done.categories_created = done.categories_created.saturating_add(1);
    Ok(made.id)
}

/// The row's rate, or the category's when the cell was left blank. The same
/// order `services::products` resolves in, so a product opened by the import
/// and a product opened on the fiche take their rate from the same place.
fn resolve_rate(
    conn: &mut SqliteConnection,
    shop_id: i32,
    category: Option<i32>,
    fields: &Fields,
) -> Result<Bps, CoreError> {
    if let Some(rate) = fields.rate_bps {
        return Ok(rate);
    }
    let id = category.ok_or_else(|| {
        CoreError::validation(
            "rate_percent",
            "a row without a category must name its rate",
        )
    })?;
    let raw = categories_repo::default_rate_bps(conn, shop_id, id)?.ok_or(CoreError::NotFound {
        entity: "category",
        id,
    })?;
    let raw = u32::try_from(raw)
        .map_err(|_| CoreError::validation("rate_percent", "rate out of range"))?;
    Ok(Bps::new(raw)?)
}

fn existing(
    conn: &mut SqliteConnection,
    shop_id: i32,
    barcode: Option<&str>,
) -> Result<Option<i32>, CoreError> {
    let Some(barcode) = barcode else {
        return Ok(None);
    };
    Ok(products_repo::by_barcode(conn, shop_id, barcode)?.map(|p| p.id))
}

/// The cells of one row, once they have been read and found well formed.
#[derive(Debug, Clone)]
struct Fields {
    barcode: Option<String>,
    category: Option<String>,
    unit: Unit,
    cost: Money,
    selling: Money,
    wholesale: Option<Money>,
    qty_on_hand_milli: i64,
    low_stock_at_milli: i64,
    /// `None` when the cell was blank, which means "take the category's".
    rate_bps: Option<Bps>,
    active: bool,
}

/// One row of the file: what it says it is, and either its cells or the
/// refusal the cells earned on their own, before anything was looked up.
#[derive(Debug, Clone)]
struct Draft {
    row: u32,
    name: String,
    parsed: Result<Fields, (&'static str, &'static str)>,
}

/// Every row of the first sheet, read and checked against nothing but itself
/// and the rest of the file. No connection: this half is the same answer on
/// any database, which is what makes the duplicate-in-file rule a property
/// of the file.
fn parse(bytes: &[u8]) -> Result<Vec<Draft>, CoreError> {
    let mut book: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes.to_vec()))
        .map_err(|_| CoreError::validation("file", "this file is not an Excel workbook"))?;
    // The first sheet, not one found by name: the template names its tab in
    // the shop's language, and a file made from the Arabic template has to
    // import into a shop working in French.
    let range = book
        .worksheet_range_at(0)
        .ok_or_else(|| CoreError::validation("file", "the workbook has no sheet"))?
        .map_err(|_| CoreError::validation("file", "the first sheet could not be read"))?;
    let rows: Vec<Vec<Data>> = range.rows().map(<[Data]>::to_vec).collect();
    let Some(header) = rows.first() else {
        return Err(CoreError::validation("file", "the sheet is empty"));
    };
    let header: Vec<String> = header.iter().map(cell_text).collect();
    // Matched by name and not by position, so a shop that moved a column is
    // still importable and a shop that dropped one is told which.
    let mut index = HashMap::new();
    for column in PRODUCT_COLUMNS {
        let at = header.iter().position(|h| h == column).ok_or_else(|| {
            CoreError::validation("header", "the header row is not the template's")
        })?;
        index.insert(column, at);
    }

    let mut drafts: Vec<Draft> = Vec::new();
    for (offset, row) in rows.iter().skip(1).enumerate() {
        // A wholly blank row is the empty space under a shop's last product,
        // not a product with nothing in it.
        if row.iter().all(|c| cell_text(c).is_empty()) {
            continue;
        }
        let number = u32::try_from(offset + 2)
            .map_err(|_| CoreError::validation("file", "more rows than a sheet holds"))?;
        let name = column(row, &index, "name");
        drafts.push(Draft {
            row: number,
            name: name.clone(),
            parsed: fields(row, &index, &name),
        });
    }

    refuse_duplicates(&mut drafts);
    Ok(drafts)
}

/// Two rows carrying one barcode refuse each other. Neither is the row the
/// shop meant, and taking the last would drop the first without saying so.
fn refuse_duplicates(drafts: &mut [Draft]) {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut clashing: HashSet<String> = HashSet::new();
    for draft in drafts.iter() {
        if let Ok(fields) = &draft.parsed {
            if let Some(barcode) = &fields.barcode {
                let count = seen.entry(barcode.clone()).or_insert(0);
                *count = count.saturating_add(1);
                if *count > 1 {
                    clashing.insert(barcode.clone());
                }
            }
        }
    }
    for draft in drafts.iter_mut() {
        let clashes = draft
            .parsed
            .as_ref()
            .ok()
            .and_then(|f| f.barcode.as_ref())
            .is_some_and(|b| clashing.contains(b));
        if clashes {
            draft.parsed = Err(("barcode", "duplicate_in_file"));
        }
    }
}

fn column(row: &[Data], index: &HashMap<&str, usize>, name: &str) -> String {
    index
        .get(name)
        .and_then(|at| row.get(*at))
        .map(cell_text)
        .unwrap_or_default()
}

/// The cells of one row, or the first thing wrong with them. First and not
/// all of them: a screen shows one reason per row, and a row with four
/// problems is fixed one at a time anyway.
fn fields(
    row: &[Data],
    index: &HashMap<&str, usize>,
    name: &str,
) -> Result<Fields, (&'static str, &'static str)> {
    if name.is_empty() {
        return Err(("name", "missing_name"));
    }
    let unit = Unit::parse(&column(row, index, "unit")).ok_or(("unit", "unknown_unit"))?;

    let cost = money(row, index, "cost_da")?.unwrap_or(Money::ZERO);
    let selling = money(row, index, "selling_da")?.ok_or(("selling_da", "missing_amount"))?;
    let wholesale = money(row, index, "wholesale_da")?;
    let qty_on_hand_milli = qty(row, index, "stock")?.unwrap_or(0);
    let low_stock_at_milli = qty(row, index, "low_stock_at")?.unwrap_or(0);

    let rate_bps = match decimal(row, index, "rate_percent", MONEY_SCALE)? {
        None => None,
        Some(hundredths) => {
            let bps = u32::try_from(hundredths).map_err(|_| ("rate_percent", "negative_rate"))?;
            if !ALLOWED_RATES_BPS.contains(&bps) {
                return Err(("rate_percent", "rate_not_allowed"));
            }
            Some(Bps::new(bps).map_err(|_| ("rate_percent", "rate_not_allowed"))?)
        }
    };

    let barcode = {
        let value = column(row, index, "barcode");
        (!value.is_empty()).then_some(value)
    };
    let category = {
        let value = column(row, index, "category");
        (!value.is_empty()).then_some(value)
    };

    Ok(Fields {
        barcode,
        category,
        unit,
        cost,
        selling,
        wholesale,
        qty_on_hand_milli,
        low_stock_at_milli,
        rate_bps,
        active: boolean(row, index, "active").unwrap_or(true),
    })
}

fn money(
    row: &[Data],
    index: &HashMap<&str, usize>,
    name: &'static str,
) -> Result<Option<Money>, (&'static str, &'static str)> {
    let Some(centimes) = decimal(row, index, name, MONEY_SCALE)? else {
        return Ok(None);
    };
    if centimes < 0 {
        return Err((name, "negative_amount"));
    }
    Ok(Some(Money::centimes(centimes)))
}

fn qty(
    row: &[Data],
    index: &HashMap<&str, usize>,
    name: &'static str,
) -> Result<Option<i64>, (&'static str, &'static str)> {
    let Some(milli) = decimal(row, index, name, QTY_SCALE)? else {
        return Ok(None);
    };
    if milli < 0 {
        return Err((name, "negative_quantity"));
    }
    Ok(Some(milli))
}

fn decimal(
    row: &[Data],
    index: &HashMap<&str, usize>,
    name: &'static str,
    scale: u32,
) -> Result<Option<i64>, (&'static str, &'static str)> {
    let raw = column(row, index, name);
    if raw.is_empty() {
        return Ok(None);
    }
    scaled(&raw, scale).map(Some).ok_or((name, "not_a_number"))
}

/// A decimal spelling as an integer of `10^scale`ths, rounded half away from
/// zero. Every digit is a character here and nothing is multiplied as a
/// float: `"80.5"` at scale 2 is 8050, and `"0.125"` is 13 rather than the
/// 12 a binary tie would round to.
fn scaled(raw: &str, scale: u32) -> Option<i64> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{202f}' && *c != '\u{a0}')
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    let (negative, digits) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest.to_string()),
        None => (
            false,
            cleaned.strip_prefix('+').unwrap_or(&cleaned).to_string(),
        ),
    };
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, f),
        None => (digits.as_str(), ""),
    };
    // An empty whole part is "0.5"; an empty fraction is "12.". Both are
    // things a spreadsheet writes, and neither may be read as a zero when
    // the other half has digits in it.
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole.chars().all(|c| c.is_ascii_digit()) || !fraction.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut value: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let factor = 10_i64.checked_pow(scale)?;
    value = value.checked_mul(factor)?;
    let kept: String = fraction
        .chars()
        .chain(std::iter::repeat('0'))
        .take(scale as usize)
        .collect();
    if !kept.is_empty() {
        value = value.checked_add(kept.parse::<i64>().ok()?)?;
    }
    // Half away from zero on the first digit the scale drops, which is the
    // rounding the money module uses everywhere else.
    let next = fraction
        .chars()
        .nth(scale as usize)
        .and_then(|c| c.to_digit(10));
    if next.is_some_and(|d| d >= 5) {
        value = value.checked_add(1)?;
    }
    if negative {
        value = value.checked_neg()?;
    }
    Some(value)
}

/// `active` as a spreadsheet writes it: a real boolean when the cell is one,
/// and otherwise the words a person types in the three languages the app
/// speaks. Anything else leaves the column unanswered and the caller takes
/// its default, which is that a product a shop is importing is one it sells.
fn boolean(row: &[Data], index: &HashMap<&str, usize>, name: &str) -> Option<bool> {
    let raw = column(row, index, name).to_lowercase();
    match raw.as_str() {
        "true" | "vrai" | "oui" | "yes" | "1" | "نعم" => Some(true),
        "false" | "faux" | "non" | "no" | "0" | "لا" => Some(false),
        _ => None,
    }
}

/// A cell as the text it stands for. A barcode typed into a spreadsheet
/// arrives as a number more often than as text, so a float is written back
/// out as the shortest decimal that round-trips (`6130001234567`, not
/// `6130001234567.0`) rather than being refused as "not text".
fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => {
            let printed = format!("{value}");
            printed
                .strip_suffix(".0")
                .map_or(printed.clone(), std::string::ToString::to_string)
        }
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        other => other.to_string().trim().to_string(),
    }
}

/// What each row would do, against this shop as it stands right now.
fn assess(
    conn: &mut SqliteConnection,
    shop_id: i32,
    drafts: &[Draft],
) -> Result<DryRun, CoreError> {
    let mut rows = Vec::with_capacity(drafts.len());
    let mut accepted = 0_usize;
    let mut refused = 0_usize;
    for draft in drafts {
        let outcome = match &draft.parsed {
            Err((field, reason)) => Outcome::Refused { field, reason },
            Ok(fields) => match assess_row(conn, shop_id, fields) {
                Err(refusal) => Outcome::Refused {
                    field: refusal.0,
                    reason: refusal.1,
                },
                Ok(outcome) => outcome,
            },
        };
        match outcome {
            Outcome::Refused { .. } => refused = refused.saturating_add(1),
            Outcome::Created | Outcome::Updated => accepted = accepted.saturating_add(1),
        }
        rows.push(RowReport {
            row: draft.row,
            name: draft.name.clone(),
            outcome,
        });
    }
    Ok(DryRun {
        rows,
        accepted,
        refused,
    })
}

fn assess_row(
    conn: &mut SqliteConnection,
    shop_id: i32,
    fields: &Fields,
) -> Result<Outcome, (&'static str, &'static str)> {
    // A row with no rate of its own is only writable when the category it
    // names already carries one, and a category the file is about to open
    // takes its rate from the row: with neither there is nothing to guess
    // from, and the refusal belongs in the report rather than halfway
    // through the write.
    if fields.rate_bps.is_none() {
        let known = fields
            .category
            .as_deref()
            .map(|name| categories_repo::by_name(conn, shop_id, name))
            .transpose()
            .map_err(|_| ("rate_percent", "rate_missing"))?
            .flatten();
        if known.is_none() {
            return Err(("rate_percent", "rate_missing"));
        }
    }
    let found = existing(conn, shop_id, fields.barcode.as_deref())
        .map_err(|_| ("barcode", "not_a_number"))?;
    Ok(if found.is_some() {
        Outcome::Updated
    } else {
        Outcome::Created
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::scaled;

    #[test]
    fn a_decimal_reaches_centimes_without_a_float_multiply() {
        assert_eq!(scaled("80.5", 2), Some(8_050));
        assert_eq!(scaled("120", 2), Some(12_000));
        assert_eq!(scaled("1 234,56", 2), Some(123_456));
        assert_eq!(scaled("0.5", 2), Some(50));
        assert_eq!(scaled("-1.00", 2), Some(-100));
        assert_eq!(scaled("12.", 2), Some(1_200));
    }

    #[test]
    fn a_third_decimal_rounds_half_away_from_zero() {
        assert_eq!(scaled("0.125", 2), Some(13));
        assert_eq!(scaled("0.124", 2), Some(12));
        assert_eq!(scaled("-0.125", 2), Some(-13));
        assert_eq!(scaled("0.135", 2), Some(14));
    }

    #[test]
    fn a_quantity_is_thousandths() {
        assert_eq!(scaled("1.5", 3), Some(1_500));
        assert_eq!(scaled("12", 3), Some(12_000));
        assert_eq!(scaled("0.0005", 3), Some(1));
    }

    #[test]
    fn anything_that_is_not_a_number_is_no_number_at_all() {
        assert_eq!(scaled("abc", 2), None);
        assert_eq!(scaled("", 2), None);
        assert_eq!(scaled("1.2.3", 2), None);
        assert_eq!(scaled("12x", 2), None);
    }
}
