//! The shelf label: 58 × 40 mm, the product's name, its selling price and
//! its EAN-13 in bars and in digits (features.md §1 and §4). Rendered by the
//! core like every other paper, so the desktop prints the same bytes a
//! server would.
//!
//! The bars are the picture of one number and the digits under them are that
//! same number spelled out, so a person can check by eye what a scanner will
//! read. That is why a code the encoder will not take is a refusal and never
//! a label with the picture left off: a label with digits and no bars gets
//! stuck on a shelf and scans as nothing, and a label with bars that encode
//! a different number is worse.
//!
//! Only EAN-13 is drawn. The fiche lets a shop type any code it likes (a
//! supplier's own reference, a short internal number), and a code that is
//! not thirteen digits with a right check digit has no EAN-13 picture. See
//! the report: whether such a product should get a label in another
//! symbology is a question for Samir.

use askama::Template;
use barcoders::generators::svg::SVG;
use barcoders::sym::ean13::EAN13;

use crate::error::CoreError;
use crate::lang::Lang;
use crate::models::product::Product;
use crate::money::format::format_centimes;
use crate::print::strings::{text, Key};

/// The bars in SVG user units: one unit per module, so the 95 modules of an
/// EAN-13 are 95 wide and the stylesheet scales the whole picture to the
/// millimetres the label gives it. Nothing here is a millimetre: the CSS
/// owns the paper and this owns the number.
const BAR_HEIGHT: u32 = 60;

/// Thirteen ASCII digits carrying a correct GS1 check digit: the shape
/// `EAN13::new` accepts, and the one shape both the label and the import
/// care about (`services::import`, dz-review 2026-09-10). A code the
/// encoder makes for a shop's own shelf (`services::products::in_store_barcode`)
/// is thirteen digits with a correct check digit by construction, so it
/// always passes this.
pub fn is_ean13(code: &str) -> bool {
    code.len() == EAN13_LEN && code.bytes().all(|b| b.is_ascii_digit()) && EAN13::new(code).is_ok()
}

/// GS1 prefix and check digit length: thirteen digits, always.
const EAN13_LEN: usize = 13;

/// One label, made. The template has a `for` and no rule at all, the way the
/// ticket's does.
struct LabelView {
    name: String,
    price: String,
    currency: &'static str,
    /// The EAN-13 as SVG markup, already escaped-safe: it is this module's
    /// own output and carries no shop text.
    bars: String,
    /// The thirteen digits, printed under the bars.
    code: String,
}

#[derive(Template)]
#[template(path = "barcode_label_58mm.html")]
struct OneLabel {
    lang_tag: &'static str,
    dir: &'static str,
    arabic_unreviewed: bool,
    label: LabelView,
}

#[derive(Template)]
#[template(path = "barcode_label_sheet_a4.html")]
struct LabelSheet {
    lang_tag: &'static str,
    dir: &'static str,
    arabic_unreviewed: bool,
    title: String,
    labels: Vec<LabelView>,
}

/// One 58 × 40 mm label for `product`, in `lang`, as one standalone HTML
/// page. Pinned by `fixtures/print/barcode_label/{fr,en,ar}.html`.
pub fn render_label(product: &Product, lang: Lang) -> Result<String, CoreError> {
    let page = OneLabel {
        lang_tag: lang.tag(),
        dir: lang.dir(),
        arabic_unreviewed: lang == Lang::Ar,
        label: label(product, lang)?,
    };
    page.render().map_err(CoreError::from)
}

/// A sheet of labels on A4, laid out as a grid of the same 58 × 40 label.
/// Pinned by `fixtures/print/barcode_label/sheet-fr.html`.
///
/// One product the encoder refuses refuses the sheet. Printing the rest and
/// silently dropping it would hand back a page that looks complete, and the
/// product with no label on it is exactly the one somebody was looking for.
pub fn render_label_sheet(products: &[Product], lang: Lang) -> Result<String, CoreError> {
    if products.is_empty() {
        return Err(CoreError::validation(
            "products",
            "a sheet of labels needs at least one product",
        ));
    }
    let labels = products
        .iter()
        .map(|p| label(p, lang))
        .collect::<Result<Vec<_>, _>>()?;
    let page = LabelSheet {
        lang_tag: lang.tag(),
        dir: lang.dir(),
        arabic_unreviewed: lang == Lang::Ar,
        title: text(Key::SheetProducts, lang).to_owned(),
        labels,
    };
    page.render().map_err(CoreError::from)
}

fn label(product: &Product, lang: Lang) -> Result<LabelView, CoreError> {
    let code = product.barcode.as_deref().unwrap_or("").trim();
    if code.is_empty() {
        return Err(CoreError::validation(
            "barcode",
            "this product has no barcode to print",
        ));
    }
    // Thirteen ASCII digits, checked here and not left to the encoder.
    // barcoders takes a twelve-digit code as an EAN-13 body and appends the
    // check digit it computes, so a fiche carrying twelve would get bars
    // for thirteen digits under a printed number of twelve: the scanner and
    // the person reading the same sticker would come away with different
    // products. Fourteen is an ITF-14 carton code and not a shelf label,
    // and a non-ASCII digit is not a number at all.
    //
    // Past this gate `EAN13::new` sees exactly thirteen digits, where it
    // recomputes the check digit and refuses a code whose own thirteenth is
    // wrong. That is the checksum test: the encoder and the printed digits
    // cannot disagree, because the same string feeds both.
    // A field of its own, not `barcode`: the screen shows two different
    // sentences and the shop does two different things about them. A fiche
    // with nothing in the column gets a code typed or generated; a fiche
    // carrying a supplier's own reference is not going to get an EAN-13
    // label whatever anybody types, and telling that shop it has "no
    // barcode" sends it looking at a field that is visibly filled.
    if !is_ean13(code) {
        return Err(CoreError::validation(
            "barcode_digits",
            "this barcode is not a valid EAN-13 and has no bars",
        ));
    }
    // `is_ean13` above already proved `EAN13::new` accepts this code; this
    // second call only gets at the symbol to encode, not a fresh check.
    let symbol = EAN13::new(code).map_err(|_| {
        CoreError::validation(
            "barcode_digits",
            "this barcode is not a valid EAN-13 and has no bars",
        )
    })?;
    let bars = SVG::new(BAR_HEIGHT)
        .xmlns("http://www.w3.org/2000/svg".to_owned())
        .generate(symbol.encode())
        .map_err(|_| {
            CoreError::validation(
                "barcode_digits",
                "the bars for this barcode cannot be drawn",
            )
        })?;
    Ok(LabelView {
        name: product.name.clone(),
        price: format_centimes(product.selling),
        currency: text(Key::Currency, lang),
        bars,
        code: code.to_owned(),
    })
}
