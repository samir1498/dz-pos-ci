//! The four Excel workbooks of features.md §1: products, sales, customers,
//! suppliers. What the tables hold, one row per record, no pivot, no formula
//! and no styling beyond a bold header and the number formats a spreadsheet
//! needs to treat a column as money, a quantity or a day.
//!
//! Two decisions run through all four.
//!
//! The header row and the enumerated cells (`unit`, `party_kind`, `status`)
//! are the stable key names, never translated words. The products workbook is
//! the file `services::import` reads back, so its columns have to be named
//! the same in a shop working in Arabic and a shop working in French, or an
//! export could not be edited and imported again. The one thing a person
//! sees before the grid is the tab, and that is named in the shop's language.
//!
//! An amount reaches a cell through its decimal spelling and nothing else.
//! A spreadsheet cell is an IEEE double whatever is written into it, so the
//! honest thing is to convert once, at the edge, from the integer the app
//! stores: `format!("{}.{:02}")` of the centimes, parsed. There is no
//! arithmetic on the way, so no rounding of the app's own can go wrong here,
//! and the double a cell ends up holding is the same one Excel stores when a
//! person types the figure by hand. Writing the centimes with a display
//! format instead would have looked identical and made every `SUM` in the
//! file a hundred times too big.

use chrono::{NaiveDate, NaiveDateTime};
use diesel::sqlite::SqliteConnection;
use rust_xlsxwriter::{Format, Workbook, Worksheet};

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::product::Product;
use crate::money::Money;
use crate::print::number;
use crate::print::strings::{text, Key};
use crate::services::{audit, categories, customers, documents, products, suppliers};

/// Two decimals and a thousands separator: what a shop reads an amount as,
/// and what keeps a column summable.
const MONEY_FORMAT: &str = "#,##0.00";
/// A quantity is thousandths of the unit, so up to three decimals and none
/// of them printed when the number is whole: 2,5 kg reads as `2,5`.
const QTY_FORMAT: &str = "#,##0.###";
/// A rate as a person says it: `19`, `9`, `0`.
const RATE_FORMAT: &str = "0.##";
/// The day and the minute a document was issued, on the shop's calendar
/// (the stored `issued_at` is already there).
const DATE_FORMAT: &str = "dd/mm/yyyy hh:mm";

/// The days a sales export covers, both ends included, both optional. Absent
/// on either side means "as far back as the file goes" and "up to the last
/// document"; the whole thing absent is every document the shop ever issued,
/// which is what a shop asking for its sales with no dates means.
///
/// Only the sales workbook takes one. A product, a customer and a supplier
/// are current rows rather than events, so there is no range to ask them for.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DayRange {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

/// The three formats every sheet uses, made once per workbook.
struct Formats {
    header: Format,
    money: Format,
    qty: Format,
    rate: Format,
    date: Format,
}

impl Formats {
    fn new() -> Self {
        Formats {
            header: Format::new().set_bold(),
            money: Format::new().set_num_format(MONEY_FORMAT),
            qty: Format::new().set_num_format(QTY_FORMAT),
            rate: Format::new().set_num_format(RATE_FORMAT),
            date: Format::new().set_num_format(DATE_FORMAT),
        }
    }
}

/// An amount as the one double that stands for it. See the module note: the
/// decimal spelling of the stored centimes is parsed, and nothing is
/// computed on the way.
///
/// The parse cannot fail on a string this function built, and it is still
/// reported rather than defaulted: a silent zero in a money column is the
/// one failure a workbook must not have.
fn amount(money: Money) -> Result<f64, CoreError> {
    let centimes = money.as_centimes();
    let sign = if centimes < 0 { "-" } else { "" };
    let abs = centimes.unsigned_abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
        .parse::<f64>()
        .map_err(|_| CoreError::validation("amount", "an amount has no decimal form"))
}

/// A quantity the same way: thousandths of the unit, spelled out and parsed.
fn quantity(qty_milli: i64) -> Result<f64, CoreError> {
    let sign = if qty_milli < 0 { "-" } else { "" };
    let abs = qty_milli.unsigned_abs();
    format!("{sign}{}.{:03}", abs / 1000, abs % 1000)
        .parse::<f64>()
        .map_err(|_| CoreError::validation("qty_milli", "a quantity has no decimal form"))
}

/// A rate in basis points as the percentage a person reads: 1900 is 19.
fn rate_percent(bps: u32) -> Result<f64, CoreError> {
    format!("{}.{:02}", bps / 100, bps % 100)
        .parse::<f64>()
        .map_err(|_| CoreError::validation("rate_bps", "a rate has no decimal form"))
}

/// One sheet with its header row already written and its columns widened.
/// Every workbook here has exactly one sheet, so the tab and the columns are
/// decided in one place.
fn open_sheet<'a>(
    workbook: &'a mut Workbook,
    tab: &str,
    columns: &[&str],
    formats: &Formats,
) -> Result<&'a mut Worksheet, CoreError> {
    let sheet = workbook.add_worksheet();
    sheet.set_name(tab)?;
    for (index, name) in columns.iter().enumerate() {
        let column = u16::try_from(index)
            .map_err(|_| CoreError::validation("column", "a sheet has more columns than Excel"))?;
        sheet.write_string_with_format(0, column, *name, &formats.header)?;
        sheet.set_column_width(column, 18)?;
    }
    Ok(sheet)
}

/// The header row of the products sheet, which is also the header row the
/// import template writes and the one `services::import` matches on. One
/// constant, so a column renamed on one side stops compiling on the other.
pub const PRODUCT_COLUMNS: [&str; 11] = [
    "name",
    "barcode",
    "category",
    "unit",
    "cost_da",
    "selling_da",
    "wholesale_da",
    "stock",
    "low_stock_at",
    "rate_percent",
    "active",
];

const SALE_COLUMNS: [&str; 16] = [
    "kind",
    "number",
    "date",
    "customer",
    "status",
    "line_name",
    "qty",
    "unit_price_da",
    "line_discount_da",
    "rate_percent",
    "line_total_da",
    "document_total_ht_da",
    "document_discount_da",
    "document_tva_da",
    "document_stamp_da",
    "document_net_to_pay_da",
];

const CUSTOMER_COLUMNS: [&str; 13] = [
    "name",
    "party_kind",
    "phone",
    "address",
    "rc",
    "nif",
    "nis",
    "ai",
    "credit_limit_da",
    "warn_threshold_da",
    "notes",
    "active",
    "balance_da",
];

const SUPPLIER_COLUMNS: [&str; 10] = [
    "name",
    "phone",
    "address",
    "rc",
    "nif",
    "nis",
    "ai",
    "notes",
    "active",
    "balance_da",
];

/// Where a column sits, by the name it carries. The writers below say
/// `at(&COLUMNS, "selling_da")` rather than `5`, so inserting a column in the
/// list moves every cell with it.
fn at(columns: &[&str], name: &str) -> Result<u16, CoreError> {
    let index = columns
        .iter()
        .position(|c| *c == name)
        .ok_or_else(|| CoreError::validation("column", "a sheet writes a column it never named"))?;
    u16::try_from(index)
        .map_err(|_| CoreError::validation("column", "a sheet has more columns than Excel"))
}

/// A `None` text field is an empty cell, not the word "None": a shop reading
/// the file has to see a blank where the fiche is blank.
fn optional(value: Option<&str>) -> &str {
    value.unwrap_or("")
}

/// The row every export writes: an export used to leave
/// nothing in the log. `which` is the stable name a reader of the log can
/// tell apart from the other three, and `rows` is how much of the shop left
/// with it, when the caller already counted them building the sheet: no
/// second pass over the data just to answer this.
///
/// Called after the workbook is built and before it is handed back, so a row
/// that fails to write fails the export instead of letting data walk out the
/// door with nothing behind it: the whole point of this row is that it is
/// there, and a caller that got the bytes anyway despite the log call failing
/// would have the control this milestone is about in name only.
fn record_export(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    which: &'static str,
    rows: usize,
) -> Result<(), CoreError> {
    audit::record(
        conn,
        shop_id,
        actor_id,
        audit::Change {
            action: audit::ACTION_EXPORT,
            entity: "export",
            entity_id: None,
            before: None,
            after: Some(serde_json::json!({ "which": which, "rows": rows }).to_string()),
        },
    )
}

/// The shop's products, one row each, active and inactive alike: a workbook
/// that dropped the inactive ones would come back through the import as a
/// catalogue with holes in it.
pub fn products(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    lang: Lang,
) -> Result<Vec<u8>, CoreError> {
    let rows = products::list(conn, shop_id)?;
    // The category is named rather than numbered: an id means nothing in a
    // spreadsheet and the import matches a category by its name.
    let categories = categories::list(conn, shop_id)?;
    let name_of = |id: Option<i32>| -> String {
        id.and_then(|id| categories.iter().find(|c| c.id == id))
            .map_or_else(String::new, |c| c.name.clone())
    };

    let formats = Formats::new();
    let mut workbook = Workbook::new();
    let sheet = open_sheet(
        &mut workbook,
        text(Key::SheetProducts, lang),
        &PRODUCT_COLUMNS,
        &formats,
    )?;
    for (index, product) in rows.iter().enumerate() {
        let row = u32::try_from(index + 1)
            .map_err(|_| CoreError::validation("row", "more products than a sheet holds"))?;
        write_product_row(sheet, row, product, &name_of(product.category_id), &formats)?;
    }
    record_export(conn, shop_id, actor_id, "products", rows.len())?;
    Ok(workbook.save_to_buffer()?)
}

fn write_product_row(
    sheet: &mut Worksheet,
    row: u32,
    product: &Product,
    category: &str,
    formats: &Formats,
) -> Result<(), CoreError> {
    let column = |name: &str| at(&PRODUCT_COLUMNS, name);
    sheet.write_string(row, column("name")?, &product.name)?;
    sheet.write_string(
        row,
        column("barcode")?,
        optional(product.barcode.as_deref()),
    )?;
    sheet.write_string(row, column("category")?, category)?;
    sheet.write_string(row, column("unit")?, product.unit.as_str())?;
    sheet.write_number_with_format(
        row,
        column("cost_da")?,
        amount(product.cost)?,
        &formats.money,
    )?;
    sheet.write_number_with_format(
        row,
        column("selling_da")?,
        amount(product.selling)?,
        &formats.money,
    )?;
    // Blank rather than zero when there is no wholesale price: zero is a
    // price a shop could mean, and this product has none.
    match product.wholesale {
        Some(price) => {
            sheet.write_number_with_format(
                row,
                column("wholesale_da")?,
                amount(price)?,
                &formats.money,
            )?;
        }
        None => {
            sheet.write_string(row, column("wholesale_da")?, "")?;
        }
    }
    sheet.write_number_with_format(
        row,
        column("stock")?,
        quantity(product.qty_on_hand_milli)?,
        &formats.qty,
    )?;
    sheet.write_number_with_format(
        row,
        column("low_stock_at")?,
        quantity(product.low_stock_at_milli)?,
        &formats.qty,
    )?;
    sheet.write_number_with_format(
        row,
        column("rate_percent")?,
        rate_percent(product.rate_bps.as_u32())?,
        &formats.rate,
    )?;
    sheet.write_boolean(row, column("active")?, product.active)?;
    Ok(())
}

/// Every document the shop issued in the range, one row per line, with the
/// document's own totals repeated on each of its lines.
///
/// Every kind and every status: a cancelled facture is in the file with
/// `cancelled` in its status column, because a sales export a comptable
/// reads has to show the paper that was annulled as well as the ones that
/// stand. The number is the one the paper prints (`print::number`), so a row
/// here and the document in the shop's file are quotable against each other.
pub fn sales(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    lang: Lang,
    range: DayRange,
) -> Result<Vec<u8>, CoreError> {
    let documents = documents::list_in_range(conn, shop_id, range.from, range.to)?;

    let formats = Formats::new();
    let mut workbook = Workbook::new();
    let sheet = open_sheet(
        &mut workbook,
        text(Key::SheetSales, lang),
        &SALE_COLUMNS,
        &formats,
    )?;
    let column = |name: &str| at(&SALE_COLUMNS, name);
    let mut row: u32 = 0;
    for document in &documents {
        let printed = number(document);
        let customer = document
            .buyer
            .as_ref()
            .map_or_else(String::new, |b| b.name.clone());
        for line in &document.lines {
            row = row
                .checked_add(1)
                .ok_or_else(|| CoreError::validation("row", "more lines than a sheet holds"))?;
            sheet.write_string(row, column("kind")?, document.kind.as_str())?;
            sheet.write_string(row, column("number")?, &printed)?;
            write_day(
                sheet,
                row,
                column("date")?,
                document.issued_at,
                &formats.date,
            )?;
            sheet.write_string(row, column("customer")?, &customer)?;
            sheet.write_string(row, column("status")?, document.status.as_str())?;
            sheet.write_string(row, column("line_name")?, &line.name)?;
            sheet.write_number_with_format(
                row,
                column("qty")?,
                quantity(line.qty_milli)?,
                &formats.qty,
            )?;
            for (name, value) in [
                ("unit_price_da", line.unit_price),
                ("line_discount_da", line.line_discount),
                ("line_total_da", line.line_total),
                ("document_total_ht_da", document.totals.total_ht),
                ("document_discount_da", document.totals.discount),
                ("document_tva_da", document.totals.tva),
                ("document_stamp_da", document.totals.stamp),
                ("document_net_to_pay_da", document.totals.net_to_pay),
            ] {
                sheet.write_number_with_format(
                    row,
                    column(name)?,
                    amount(value)?,
                    &formats.money,
                )?;
            }
            sheet.write_number_with_format(
                row,
                column("rate_percent")?,
                rate_percent(line.rate_bps.as_u32())?,
                &formats.rate,
            )?;
        }
    }
    record_export(conn, shop_id, actor_id, "sales", row as usize)?;
    Ok(workbook.save_to_buffer()?)
}

fn write_day(
    sheet: &mut Worksheet,
    row: u32,
    column: u16,
    at: NaiveDateTime,
    format: &Format,
) -> Result<(), CoreError> {
    sheet.write_datetime_with_format(row, column, at, format)?;
    Ok(())
}

/// The customer fiches with the balance their ledger sums to. The balance is
/// read the way the list screen reads it, so the workbook and the screen can
/// never disagree about what somebody owes.
pub fn customers(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    lang: Lang,
) -> Result<Vec<u8>, CoreError> {
    let rows = customers::list_with_balance(conn, shop_id, None)?;

    let formats = Formats::new();
    let mut workbook = Workbook::new();
    let sheet = open_sheet(
        &mut workbook,
        text(Key::SheetCustomers, lang),
        &CUSTOMER_COLUMNS,
        &formats,
    )?;
    let column = |name: &str| at(&CUSTOMER_COLUMNS, name);
    for (index, entry) in rows.iter().enumerate() {
        let row = u32::try_from(index + 1)
            .map_err(|_| CoreError::validation("row", "more customers than a sheet holds"))?;
        let c = &entry.customer;
        sheet.write_string(row, column("name")?, &c.name)?;
        sheet.write_string(row, column("party_kind")?, c.party_kind.as_str())?;
        sheet.write_string(row, column("phone")?, optional(c.phone.as_deref()))?;
        sheet.write_string(row, column("address")?, optional(c.address.as_deref()))?;
        sheet.write_string(row, column("rc")?, optional(c.rc.as_deref()))?;
        sheet.write_string(row, column("nif")?, optional(c.nif.as_deref()))?;
        sheet.write_string(row, column("nis")?, optional(c.nis.as_deref()))?;
        sheet.write_string(row, column("ai")?, optional(c.ai.as_deref()))?;
        // A limit and a warning threshold are absent as often as they are
        // set, and no limit at all is not a limit of zero (models::customer).
        write_optional_amount(
            sheet,
            row,
            column("credit_limit_da")?,
            c.credit_limit,
            &formats,
        )?;
        write_optional_amount(
            sheet,
            row,
            column("warn_threshold_da")?,
            c.warn_threshold,
            &formats,
        )?;
        sheet.write_string(row, column("notes")?, optional(c.notes.as_deref()))?;
        sheet.write_boolean(row, column("active")?, c.active)?;
        sheet.write_number_with_format(
            row,
            column("balance_da")?,
            amount(entry.balance)?,
            &formats.money,
        )?;
    }
    record_export(conn, shop_id, actor_id, "customers", rows.len())?;
    Ok(workbook.save_to_buffer()?)
}

fn write_optional_amount(
    sheet: &mut Worksheet,
    row: u32,
    column: u16,
    value: Option<Money>,
    formats: &Formats,
) -> Result<(), CoreError> {
    match value {
        Some(money) => {
            sheet.write_number_with_format(row, column, amount(money)?, &formats.money)?;
        }
        None => {
            sheet.write_string(row, column, "")?;
        }
    }
    Ok(())
}

/// The supplier fiches with the balance their ledger sums to, the same shape
/// the customers workbook has on the side that owes rather than the side that
/// is owed. A balance below zero is money the shop has paid ahead, and it
/// keeps its sign.
pub fn suppliers(
    conn: &mut SqliteConnection,
    shop_id: i32,
    actor_id: i32,
    lang: Lang,
) -> Result<Vec<u8>, CoreError> {
    let rows = suppliers::list_with_balance(conn, shop_id, None)?;

    let formats = Formats::new();
    let mut workbook = Workbook::new();
    let sheet = open_sheet(
        &mut workbook,
        text(Key::SheetSuppliers, lang),
        &SUPPLIER_COLUMNS,
        &formats,
    )?;
    let column = |name: &str| at(&SUPPLIER_COLUMNS, name);
    for (index, entry) in rows.iter().enumerate() {
        let row = u32::try_from(index + 1)
            .map_err(|_| CoreError::validation("row", "more suppliers than a sheet holds"))?;
        let s = &entry.supplier;
        sheet.write_string(row, column("name")?, &s.name)?;
        sheet.write_string(row, column("phone")?, optional(s.phone.as_deref()))?;
        sheet.write_string(row, column("address")?, optional(s.address.as_deref()))?;
        sheet.write_string(row, column("rc")?, optional(s.rc.as_deref()))?;
        sheet.write_string(row, column("nif")?, optional(s.nif.as_deref()))?;
        sheet.write_string(row, column("nis")?, optional(s.nis.as_deref()))?;
        sheet.write_string(row, column("ai")?, optional(s.ai.as_deref()))?;
        sheet.write_string(row, column("notes")?, optional(s.notes.as_deref()))?;
        sheet.write_boolean(row, column("active")?, s.active)?;
        sheet.write_number_with_format(
            row,
            column("balance_da")?,
            amount(entry.balance)?,
            &formats.money,
        )?;
    }
    record_export(conn, shop_id, actor_id, "suppliers", rows.len())?;
    Ok(workbook.save_to_buffer()?)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::{amount, quantity, rate_percent};
    use crate::money::Money;

    #[test]
    fn an_amount_reaches_a_cell_as_the_double_a_person_would_have_typed() {
        assert_eq!(amount(Money::centimes(123_456)).unwrap(), 1234.56_f64);
        assert_eq!(amount(Money::ZERO).unwrap(), 0.0_f64);
        assert_eq!(amount(Money::centimes(5)).unwrap(), 0.05_f64);
        // The sign survives a whole-dinar part of zero, which is where a
        // naive `centimes / 100` loses it.
        assert_eq!(amount(Money::centimes(-5)).unwrap(), -0.05_f64);
        assert_eq!(amount(Money::centimes(-123_456)).unwrap(), -1234.56_f64);
    }

    #[test]
    fn a_quantity_reaches_a_cell_in_thousandths() {
        assert_eq!(quantity(2_500).unwrap(), 2.5_f64);
        assert_eq!(quantity(1).unwrap(), 0.001_f64);
        assert_eq!(quantity(-1_500).unwrap(), -1.5_f64);
    }

    #[test]
    fn a_rate_reaches_a_cell_as_the_percentage_a_person_reads() {
        assert_eq!(rate_percent(1900).unwrap(), 19.0_f64);
        assert_eq!(rate_percent(0).unwrap(), 0.0_f64);
        assert_eq!(rate_percent(950).unwrap(), 9.5_f64);
    }
}
