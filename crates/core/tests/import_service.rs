// Tests may panic; the deny is for shipped code.
#![allow(clippy::unwrap_used, clippy::expect_used)]

//! The product import of features.md §1: the template a shop downloads, the
//! dry run that refuses a row and writes nothing, and the apply that writes
//! the file or none of it.
//!
//! Every workbook a test here feeds the import is written by `rust_xlsxwriter`
//! beside the test rather than committed as a binary fixture, so a reader can
//! see the cells that produce a refusal. The one committed workbook is the
//! e2e's, and `the_committed_e2e_fixture_still_passes_a_dry_run` reads it
//! back through this same service so a template change cannot orphan it.

use std::io::Cursor;
use std::path::PathBuf;

use calamine::{Data, Reader, Xlsx};
use diesel::sqlite::SqliteConnection;
use dzpos_core::error::CoreError;
use dzpos_core::lang::Lang;
use dzpos_core::models::product::{NewProduct, Unit};
use dzpos_core::money::{Bps, Money};
use dzpos_core::services::import::{self, Outcome};
use dzpos_core::services::{audit, products};
use rust_xlsxwriter::Workbook;

const SHOP: i32 = 1;
const OWNER: i32 = 1;

mod common;

use common::open_temp;

/// One cell of a row a test is building. A workbook holds numbers and text
/// in different cells, and half the refusals below are about which of the
/// two a column got, so the test writes each cell as what it means.
#[derive(Debug, Clone)]
enum Cell {
    Text(String),
    Number(f64),
    Blank,
}

fn t(value: &str) -> Cell {
    Cell::Text(value.to_string())
}

const fn n(value: f64) -> Cell {
    Cell::Number(value)
}

/// A workbook shaped like the template: the header row the import matches
/// on, then the rows the test wrote.
fn workbook(rows: &[Vec<Cell>]) -> Vec<u8> {
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet.set_name("Produits").unwrap();
    for (index, column) in import::PRODUCT_COLUMNS.iter().enumerate() {
        sheet
            .write_string(0, u16::try_from(index).unwrap(), *column)
            .unwrap();
    }
    for (r, row) in rows.iter().enumerate() {
        let at = u32::try_from(r + 1).unwrap();
        for (c, cell) in row.iter().enumerate() {
            let column = u16::try_from(c).unwrap();
            match cell {
                Cell::Text(value) => {
                    sheet.write_string(at, column, value).unwrap();
                }
                Cell::Number(value) => {
                    sheet.write_number(at, column, *value).unwrap();
                }
                Cell::Blank => (),
            }
        }
    }
    book.save_to_buffer().unwrap()
}

/// name, barcode, category, unit, cost, selling, wholesale, stock,
/// low_stock_at, rate_percent, active: one good row, in the column order the
/// template writes.
fn a_row(name: &str, barcode: Cell) -> Vec<Cell> {
    vec![
        t(name),
        barcode,
        t("Alimentation"),
        t("piece"),
        n(80.5),
        n(120.0),
        Cell::Blank,
        n(12.0),
        n(3.0),
        n(19.0),
        t("true"),
    ]
}

/// The report line for a row, by the name in it.
fn outcome(report: &import::DryRun, name: &str) -> Outcome {
    report
        .rows
        .iter()
        .find(|r| r.name == name)
        .unwrap_or_else(|| panic!("no row named {name} in {:?}", report.rows))
        .outcome
        .clone()
}

fn refusal(report: &import::DryRun, name: &str) -> (String, String) {
    match outcome(report, name) {
        Outcome::Refused { field, reason } => (field.to_string(), reason.to_string()),
        other => panic!("{name} was {other:?}, not refused"),
    }
}

fn a_stored_product(conn: &mut SqliteConnection, name: &str, barcode: &str, selling: i64) -> i32 {
    products::create(
        conn,
        SHOP,
        OWNER,
        NewProduct {
            name: name.to_string(),
            barcode: Some(barcode.to_string()),
            category_id: None,
            unit: Unit::Piece,
            cost: Money::centimes(1_000),
            selling: Money::centimes(selling),
            wholesale: None,
            qty_on_hand_milli: 0,
            low_stock_at_milli: 0,
            rate_bps: Some(Bps::new(1900).unwrap()),
            active: true,
        },
    )
    .unwrap()
    .id
}

#[test]
fn the_template_carries_the_columns_the_import_matches_and_says_what_a_known_barcode_does() {
    let bytes = import::template(Lang::Fr).unwrap();
    let mut book: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes.clone())).unwrap();
    assert_eq!(book.sheet_names(), vec!["Produits", "Valeurs autorisées"]);

    let products = book.worksheet_range("Produits").unwrap();
    let rows: Vec<Vec<Data>> = products.rows().map(<[Data]>::to_vec).collect();
    let header: Vec<String> = rows[0]
        .iter()
        .map(std::string::ToString::to_string)
        .collect();
    assert_eq!(header, import::PRODUCT_COLUMNS.to_vec());
    assert_eq!(rows.len(), 2, "a header row and one example row");

    let allowed = book.worksheet_range("Valeurs autorisées").unwrap();
    let words: Vec<String> = allowed
        .rows()
        .flat_map(|row| row.iter().map(std::string::ToString::to_string))
        .collect();
    for unit in ["piece", "kg", "litre", "box"] {
        assert!(words.iter().any(|w| w == unit), "no {unit} in {words:?}");
    }
    for rate in ["19", "9", "0"] {
        assert!(
            words.iter().any(|w| w == rate),
            "no rate {rate} in {words:?}"
        );
    }
    assert!(
        words.iter().any(|w| w.contains("met à jour")),
        "the template never says a known barcode updates: {words:?}"
    );
    // And what the stock column does, which is nothing on a product the
    // shop already has: a shop that reads the sheet must not expect an
    // import to correct a shelf count.
    assert!(
        words
            .iter()
            .any(|w| w.contains("ne fait jamais bouger le stock")),
        "the template never says an import does not move stock: {words:?}"
    );

    // And the example row it ships is one the import accepts, so a shop that
    // downloads the template and adds rows under the example is not refused
    // by the example itself.
    let (_dir, mut conn) = open_temp();
    let report = import::dry_run(&mut conn, SHOP, &bytes).unwrap();
    assert_eq!(report.refused, 0, "{:?}", report.rows);
}

#[test]
fn a_dry_run_accepts_a_clean_file_and_writes_nothing() {
    let (_dir, mut conn) = open_temp();
    let bytes = workbook(&[a_row("Café moulu", t("6130001234567"))]);

    let report = import::dry_run(&mut conn, SHOP, &bytes).unwrap();
    assert_eq!(report.accepted, 1);
    assert_eq!(report.refused, 0);
    assert_eq!(outcome(&report, "Café moulu"), Outcome::Created);
    assert_eq!(report.rows[0].row, 2, "the spreadsheet row, header counted");

    assert!(products::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn apply_writes_the_products_and_the_category_the_file_names() {
    let (_dir, mut conn) = open_temp();
    let bytes = workbook(&[
        a_row("Café moulu", t("6130001234567")),
        a_row("Thé vert", Cell::Blank),
    ]);

    let done = import::apply(&mut conn, SHOP, OWNER, &bytes).unwrap();
    assert_eq!(done.created, 2);
    assert_eq!(done.updated, 0);
    assert_eq!(done.categories_created, 1, "Alimentation was opened");

    let stored = products::list(&mut conn, SHOP).unwrap();
    assert_eq!(stored.len(), 2);
    let coffee = stored.iter().find(|p| p.name == "Café moulu").unwrap();
    assert_eq!(coffee.selling, Money::centimes(12_000));
    assert_eq!(coffee.cost, Money::centimes(8_050));
    assert_eq!(coffee.unit, Unit::Piece);
    assert_eq!(coffee.rate_bps, Bps::new(1900).unwrap());
    assert_eq!(coffee.qty_on_hand_milli, 12_000);
    assert_eq!(coffee.barcode.as_deref(), Some("6130001234567"));
    // A blank code is numbered by the till, the way the fiche numbers one.
    let tea = stored.iter().find(|p| p.name == "Thé vert").unwrap();
    assert_eq!(tea.barcode.as_deref().unwrap_or("").len(), 13);
    assert_eq!(tea.category_id, coffee.category_id, "one category, not two");
}

#[test]
fn a_barcode_the_shop_already_sells_under_updates_that_product() {
    let (_dir, mut conn) = open_temp();
    let id = a_stored_product(&mut conn, "Café", "6130001234567", 10_000);
    let mut row = a_row("Café moulu 250 g", t("6130001234567"));
    row[5] = n(155.0);

    let report = import::dry_run(&mut conn, SHOP, &workbook(&[row.clone()])).unwrap();
    assert_eq!(outcome(&report, "Café moulu 250 g"), Outcome::Updated);

    let done = import::apply(&mut conn, SHOP, OWNER, &workbook(&[row])).unwrap();
    assert_eq!(done.created, 0);
    assert_eq!(done.updated, 1);

    let stored = products::list(&mut conn, SHOP).unwrap();
    assert_eq!(stored.len(), 1, "one product, not two");
    assert_eq!(stored[0].id, id);
    assert_eq!(stored[0].name, "Café moulu 250 g");
    assert_eq!(stored[0].selling, Money::centimes(15_500));
    assert_eq!(stored[0].cost, Money::centimes(8_050));
}

#[test]
fn a_row_with_no_name_is_refused_under_the_name_column() {
    let (_dir, mut conn) = open_temp();
    let mut row = a_row("", t("6130001234567"));
    row[0] = Cell::Blank;

    let report = import::dry_run(&mut conn, SHOP, &workbook(&[row])).unwrap();
    assert_eq!(report.refused, 1);
    match &report.rows[0].outcome {
        Outcome::Refused { field, reason } => {
            assert_eq!(*field, "name");
            assert_eq!(*reason, "missing_name");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_unknown_unit_a_rate_off_the_list_and_a_price_below_zero_are_each_refused() {
    let (_dir, mut conn) = open_temp();
    let mut bad_unit = a_row("Sac", t("6130000000001"));
    bad_unit[3] = t("sachet");
    let mut bad_rate = a_row("Lait", t("6130000000002"));
    bad_rate[9] = n(7.0);
    let mut bad_price = a_row("Pain", t("6130000000003"));
    bad_price[5] = n(-1.0);
    let mut bad_stock = a_row("Sucre", t("6130000000004"));
    bad_stock[7] = n(-2.0);

    let report = import::dry_run(
        &mut conn,
        SHOP,
        &workbook(&[bad_unit, bad_rate, bad_price, bad_stock]),
    )
    .unwrap();

    assert_eq!(report.refused, 4);
    assert_eq!(
        refusal(&report, "Sac"),
        ("unit".into(), "unknown_unit".into())
    );
    assert_eq!(
        refusal(&report, "Lait"),
        ("rate_percent".into(), "rate_not_allowed".into())
    );
    assert_eq!(
        refusal(&report, "Pain"),
        ("selling_da".into(), "negative_amount".into())
    );
    assert_eq!(
        refusal(&report, "Sucre"),
        ("stock".into(), "negative_quantity".into())
    );
}

#[test]
fn an_update_leaves_the_stock_where_it_was_because_an_import_is_not_a_movement() {
    // The quantity belongs to the stock ledger (features.md §1): a purchase,
    // a sale and a recount move it and nothing else does. A file carrying a
    // stock column that silently overwrote what the shelf holds would put a
    // shop's count out with no movement to explain it.
    let (_dir, mut conn) = open_temp();
    let id = a_stored_product(&mut conn, "Café en stock", "6130001234563", 12_000);
    let before = products::get(&mut conn, SHOP, id).unwrap().qty_on_hand_milli;

    let mut row = a_row("Café en stock", t("6130001234563"));
    row[7] = n(999.0);
    let bytes = workbook(&[row]);

    let report = import::dry_run(&mut conn, SHOP, &bytes).unwrap();
    assert_eq!(outcome(&report, "Café en stock"), Outcome::Updated);
    import::apply(&mut conn, SHOP, OWNER, &bytes).unwrap();

    let after = products::get(&mut conn, SHOP, id).unwrap();
    assert_eq!(
        after.qty_on_hand_milli, before,
        "the import moved stock that no purchase, sale or recount moved"
    );
    // The prices it did come to change are changed.
    assert_eq!(after.selling.as_centimes(), 12_000);
}

#[test]
fn a_price_with_a_third_decimal_is_refused_and_never_rounded_into_the_shop() {
    // Ruling, 2026-09-10. A shop that typed 80.505 and found the till
    // charging 80.51 would have no way of knowing where the centime came
    // from, and the file it kept would not say. The row is refused and the
    // cell is fixed where it was typed.
    let (_dir, mut conn) = open_temp();
    let mut priced = a_row("Trop précis", Cell::Blank);
    priced[5] = t("120.505");
    let mut costed = a_row("Coût trop précis", Cell::Blank);
    costed[4] = t("80.505");
    // A quantity is thousandths, so a fourth decimal is the same refusal
    // one place further out.
    let mut counted = a_row("Compté trop précis", Cell::Blank);
    counted[7] = t("12.0005");
    // And the zeros a three place column writes are not a decimal anybody
    // typed: this row stands.
    let mut formatted = a_row("Colonne à trois décimales", Cell::Blank);
    formatted[5] = t("120.000");

    let bytes = workbook(&[priced, costed, counted, formatted]);
    let report = import::dry_run(&mut conn, SHOP, &bytes).unwrap();

    assert_eq!(
        refusal(&report, "Trop précis"),
        ("selling_da".to_string(), "too_many_decimals".to_string())
    );
    assert_eq!(
        refusal(&report, "Coût trop précis"),
        ("cost_da".to_string(), "too_many_decimals".to_string())
    );
    assert_eq!(
        refusal(&report, "Compté trop précis"),
        ("stock".to_string(), "too_many_decimals".to_string())
    );
    assert_eq!(outcome(&report, "Colonne à trois décimales"), Outcome::Created);
    assert_eq!(report.refused, 3);

    // And nothing is written, so no rounded price ever reaches a product.
    assert!(import::apply(&mut conn, SHOP, OWNER, &bytes).is_err());
    assert!(products::list(&mut conn, SHOP).unwrap().is_empty());
}

#[test]
fn two_rows_carrying_the_same_barcode_refuse_each_other() {
    let (_dir, mut conn) = open_temp();
    let bytes = workbook(&[
        a_row("Café moulu", t("6130001234567")),
        a_row("Café en grains", t("6130001234567")),
    ]);

    let report = import::dry_run(&mut conn, SHOP, &bytes).unwrap();
    assert_eq!(report.refused, 2, "neither row can be the one that wins");
    for name in ["Café moulu", "Café en grains"] {
        assert_eq!(
            refusal(&report, name),
            ("barcode".into(), "duplicate_in_file".into())
        );
    }
}

#[test]
fn apply_writes_nothing_at_all_when_one_row_is_refused() {
    let (_dir, mut conn) = open_temp();
    let mut bad = a_row("Lait", t("6130000000002"));
    bad[9] = n(7.0);
    let bytes = workbook(&[a_row("Café moulu", t("6130001234567")), bad]);

    let refused = import::apply(&mut conn, SHOP, OWNER, &bytes).unwrap_err();
    match refused {
        CoreError::Validation { field, .. } => assert_eq!(field, "rows"),
        other => panic!("{other:?}"),
    }
    // Not the good row, and not the category the good row named either.
    assert!(products::list(&mut conn, SHOP).unwrap().is_empty());
    let categories = dzpos_core::services::categories::list(&mut conn, SHOP).unwrap();
    assert!(
        !categories.iter().any(|c| c.name == "Alimentation"),
        "the category of a file that was refused was still opened"
    );
}

#[test]
fn apply_audits_the_import_with_the_count() {
    let (_dir, mut conn) = open_temp();
    a_stored_product(&mut conn, "Café", "6130001234567", 10_000);
    let bytes = workbook(&[
        a_row("Café moulu 250 g", t("6130001234567")),
        a_row("Thé vert", Cell::Blank),
    ]);

    import::apply(&mut conn, SHOP, OWNER, &bytes).unwrap();

    let entries = audit::list(&mut conn, SHOP).unwrap();
    let imported = entries
        .iter()
        .find(|e| e.action == "product.import")
        .unwrap_or_else(|| panic!("no product.import entry in {entries:?}"));
    let after: serde_json::Value =
        serde_json::from_str(imported.after.as_deref().unwrap_or("")).unwrap();
    assert_eq!(after["created"], 1);
    assert_eq!(after["updated"], 1);
    assert_eq!(after["categories_created"], 1);
}

#[test]
fn a_file_that_is_not_a_workbook_is_refused_as_input_and_never_as_a_server_fault() {
    let (_dir, mut conn) = open_temp();
    let refused = import::dry_run(&mut conn, SHOP, b"not a workbook at all").unwrap_err();
    // The code the UI translates, not a 500: a file a shop picked is input.
    assert_eq!(refused.code(), "validation");
    match refused {
        CoreError::Validation { field, .. } => assert_eq!(field, "file"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_file_whose_header_row_is_not_the_templates_is_refused_whole() {
    let (_dir, mut conn) = open_temp();
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet.write_string(0, 0, "produit").unwrap();
    sheet.write_string(1, 0, "Café").unwrap();
    let bytes = book.save_to_buffer().unwrap();

    let refused = import::dry_run(&mut conn, SHOP, &bytes).unwrap_err();
    match refused {
        CoreError::Validation { field, .. } => assert_eq!(field, "header"),
        other => panic!("{other:?}"),
    }
}

/// Where the browser suite's committed workbook lives.
fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/e2e/fixtures/products-import.xlsx")
}

/// The e2e's workbook, written from the template itself with one cell
/// edited, the way a shop uses it: download, change the name, send it back.
///
/// It is a test rather than a script so the fixture cannot drift from the
/// template: the header row and the example row are read back out of
/// `template` with calamine and written straight through, so a column
/// renamed in the service rewrites the fixture here and nowhere else.
///
/// `UPDATE_FIXTURE=1 cargo test -p dzpos-core --test import_service` writes
/// it, and fails the run on purpose, the way the print goldens do: a binary
/// nobody looked at must not be green in the run that wrote it.
#[test]
fn writes_the_e2e_fixture_when_asked_and_fails_the_run_that_wrote_it() {
    if std::env::var_os("UPDATE_FIXTURE").is_none() {
        return;
    }
    let bytes = import::template(Lang::Fr).unwrap();
    let mut book: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes)).unwrap();
    let range = book.worksheet_range_at(0).unwrap().unwrap();
    let rows: Vec<Vec<Data>> = range.rows().map(<[Data]>::to_vec).collect();

    let mut out = Workbook::new();
    let sheet = out.add_worksheet();
    sheet.set_name("Produits").unwrap();
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let value = cell.to_string();
            let value = if value == "Empty" {
                String::new()
            } else {
                value
            };
            sheet
                .write_string(u32::try_from(r).unwrap(), u16::try_from(c).unwrap(), &value)
                .unwrap();
        }
    }
    // The one cell a shop edits before sending the file back.
    sheet.write_string(1, 0, E2E_PRODUCT).unwrap();
    let path = fixture_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, out.save_to_buffer().unwrap()).unwrap();
    panic!(
        "wrote {}: open it, read the row, then run the suite again without UPDATE_FIXTURE",
        path.display()
    );
}

/// The name the browser suite looks for after it applies the import.
const E2E_PRODUCT: &str = "Café importé 250 g";

#[test]
fn the_committed_e2e_fixture_still_passes_a_dry_run() {
    // The browser suite imports a workbook committed under
    // `apps/desktop/e2e/fixtures/`. It was written once by `template` and
    // edited by hand, so a column renamed here would leave the browser suite
    // importing a file this service no longer reads. This is the test that
    // says so.
    let path = fixture_path();
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run UPDATE_FIXTURE=1 to write it", path.display()));

    let (_dir, mut conn) = open_temp();
    let report = import::dry_run(&mut conn, SHOP, &bytes).unwrap();
    assert_eq!(report.refused, 0, "{:?}", report.rows);
    assert_eq!(report.accepted, 1);
    assert_eq!(report.rows[0].name, E2E_PRODUCT);
}
