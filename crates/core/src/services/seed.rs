//! A development shop, written the way a real one would have been.
//!
//! What this is for. Every screen past the till needs a file with a month
//! behind it: a dashboard with a chart on it, statements with movements in
//! them, exports with rows. Typing that in by hand takes an afternoon and
//! gives a file nobody else has, so the same screen looks different on two
//! machines and a bug that only shows on one of them has nowhere to be
//! reproduced.
//!
//! How it writes. Through the services and never through SQL. Every rule
//! therefore applies to the seeded rows exactly as it applies to a shop's:
//! the numbering series are real, the TVA recaps are computed, the stock
//! movements carry the cost the goods left on, the ledgers balance, and every
//! write leaves the audit row it would have left. That is also what makes the
//! file worth developing against: a figure it reads wrong is a service that
//! is wrong, and `seed_service` is where that shows first.
//!
//! The clock. There is no wall clock in here. `run` takes the day the history
//! ends on and every write names its own moment, the way `dashboard_prop`
//! does, so a file seeded twice is the same file and a screen developed
//! against it reads the same figures tomorrow.
//!
//! Determinism. The randomness is a splitmix64 written out below rather than
//! `rand`: a seeded generator from the crate would do the same job, and the
//! twenty lines here save a dependency in the shipped core and pin the exact
//! stream, which is what "two runs give the same file" needs. Nothing here
//! is cryptographic and nothing here is a rule.
//!
//! Nobody real. Every name is invented. The shop, its customers and its
//! suppliers are plausible for Algiers and belong to nobody.

use chrono::{Datelike, NaiveDate, NaiveDateTime};
use diesel::connection::Connection;
use diesel::sqlite::SqliteConnection;

use crate::error::CoreError;
use crate::models::category::CategoryRowWrite;
use crate::models::customer::NewCustomer;
use crate::models::product::{NewProduct, Unit};
use crate::models::shop::StoreBlock;
use crate::models::sql_types::{PartyKind, PaymentMethod};
use crate::models::supplier::NewSupplier;
use crate::money::{Money, PaymentMode};
use crate::repos::categories as categories_repo;
use crate::services::expenses::NewExpense;
use crate::services::purchases::{NewLine, NewPurchase, ReceiveLine};
use crate::services::sales::{NewSale, NewSaleLine, SaleKind};
use crate::services::{
    audit, avoir, cancellation, customers, debt, documents, expenses, products, purchases, sales,
    shops, supplier_debt, suppliers, users,
};

/// The name the seeded shop trades under. Public because the binary reads it
/// back off a file it is about to overwrite: a shop file carrying identifiers
/// is somebody's books unless it is this one, and that is the difference
/// between re-seeding a development file and writing over a real shop.
pub const SHOP_NAME: &str = "Supérette El Bahdja";

/// The PIN and the password the seeded owner gets (M4 T2). Public because the
/// binary prints them: a development file nobody can sign in to is a
/// development file nobody can use.
///
/// Why this is here at all. The first migration writes the owner with the
/// `'!unset'` sentinel and no password, which is right for a shop that a
/// person is about to set up, and wrong for a file a developer opens at nine
/// in the morning: from M4 on every route wants a session, and there is no
/// route that sets a first credential. Setting one belongs to the seeder,
/// which already only ever runs on a `.dev/` file on a box that said
/// `DZPOS_DEV=1`, and never to the API. What a real shop's first run does is
/// a decision the plan has not taken; it is not this.
pub const OWNER_PIN: &str = "1379";
/// The seeded owner's password, for the name-and-password door.
pub const OWNER_PASSWORD: &str = "developpement";

/// The one number the whole file comes out of. Changing it changes every
/// figure the seeded shop shows, which is why it is a constant and not an
/// argument: a screenshot in a bug report has to be reproducible from the
/// commit alone.
const SEED: u64 = 0x647A_706F_7300_0001;

/// How many days of history the shop gets. A month is what the dashboard's
/// two columns and its chart both want.
const DAYS: u32 = 30;

/// The catalogue's size.
const PRODUCTS: usize = 60;

/// A hundred dinars, as centimes. Prices below are written in whole dinars
/// and multiplied by this, because a shop's shelf label is in dinars and a
/// table of centimes is a table nobody can proofread.
const DA: i64 = 100;

/// What the shop had of a product before its first delivery, per point of
/// pace, in thousandths of the unit. A fast mover opens with a hundred units
/// and a slow one with twenty, which is about three weeks of each: a shelf
/// holding four hundred of everything is not a corner shop, and a catalogue
/// that never runs down leaves the low stock list on the dashboard empty.
const OPENING_STOCK_PER_PACE_MILLI: i64 = 30_000;

/// The reorder point, per point of pace: about a week and a half of selling.
const LOW_STOCK_PER_PACE_MILLI: i64 = 8_000;

/// What an order brings a product back up to, per point of pace. Above the
/// opening, because an order placed when the shelf is nearly empty has to
/// carry the shop to the next one.
const REORDER_TO_PER_PACE_MILLI: i64 = 20_000;

/// How many fiches one order covers.
const LINES_PER_ORDER: usize = 10;

/// The last day of the window an order goes in on. The shop is between
/// orders when the month ends, which is exactly why the dashboard it opens
/// that morning has a low stock list with rows in it.
const LAST_ORDER_STEP: u32 = 23;

/// How much a shop that has just opened a fiche is willing to let a customer
/// owe. High enough that a month of credit sales never trips the limit, so
/// the seeder never has to override one: an override is a decision, and a
/// file full of them would teach a reader that they are ordinary.
const CREDIT_LIMIT_DA: i64 = 2_000_000;

/// What the seeder wrote. Counted as it goes rather than queried after, so a
/// number that does not match what the file holds is a seeder that failed
/// halfway and not a query that read the wrong table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub categories: i64,
    pub products: i64,
    pub customers: i64,
    pub suppliers: i64,
    pub days: i64,
    pub purchases: i64,
    pub receipts: i64,
    pub returns: i64,
    pub sales: i64,
    pub avoirs: i64,
    pub cancellations: i64,
    pub proformas: i64,
    pub customer_payments: i64,
    pub supplier_payments: i64,
    pub expenses: i64,
}

/// Whether the shop has nothing in it yet. Read through the services, so what
/// counts as "something" is what a screen would show: a fiche or a paper.
///
/// The binary asks this before it offers to wipe, and `run` asks it again
/// inside its own transaction, where the answer cannot go stale.
pub fn is_empty(conn: &mut SqliteConnection, shop_id: i32) -> Result<bool, CoreError> {
    Ok(products::list(conn, shop_id)?.is_empty()
        && customers::list(conn, shop_id, None)?.is_empty()
        && suppliers::list(conn, shop_id, None)?.is_empty()
        && documents::list(conn, shop_id, None)?.is_empty())
}

/// Fills an empty shop with a catalogue, its fiches and thirty days of
/// trading ending on `today`.
///
/// Refused on a shop that already holds something: seeding twice would put a
/// second catalogue beside the first and leave a file nobody can read. What
/// wipes a shop is deleting its file, which is the binary's business and not
/// a service's: no rule in this crate deletes a row, and adding one so a
/// development helper can start over would be the wrong place for it.
///
/// The whole run is one transaction. That is not only tidiness: a thousand
/// separate commits is a thousand fsyncs on a file that lives on a Windows
/// disk through WSL, which is the difference between a few seconds and
/// several minutes. Diesel nests the services' own transactions inside it as
/// savepoints, so every rule still runs in one.
pub fn run(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    today: NaiveDate,
) -> Result<Counts, CoreError> {
    conn.transaction(|conn| {
        if !is_empty(conn, shop_id)? {
            return Err(CoreError::validation(
                "shop",
                "this shop already holds rows, and a second seeding would sit beside them",
            ));
        }
        let first_day = today
            .checked_sub_days(chrono::Days::new(u64::from(DAYS.saturating_sub(1))))
            .ok_or_else(|| {
                CoreError::validation("today", "that day is outside the calendar the shop keeps")
            })?;

        let mut counts = Counts {
            days: i64::from(DAYS),
            ..Counts::default()
        };
        let mut rng = Rng::new(SEED);

        the_shop_itself(conn, shop_id, user_id)?;
        the_way_in(conn, shop_id, user_id)?;
        let categories = the_categories(conn, shop_id, user_id, &mut counts)?;
        let catalogue = the_catalogue(conn, shop_id, user_id, &categories, &mut counts)?;
        let buyers = the_customers(conn, shop_id, user_id, &mut counts)?;
        let sellers = the_suppliers(conn, shop_id, user_id, &mut counts)?;

        let mut shop = Shop {
            catalogue,
            buyers,
            sellers,
            open_factures: Vec::new(),
            open_orders: Vec::new(),
        };
        for step in 0..DAYS {
            let day = first_day
                .checked_add_days(chrono::Days::new(u64::from(step)))
                .ok_or_else(|| {
                    CoreError::validation(
                        "today",
                        "that day is outside the calendar the shop keeps",
                    )
                })?;
            one_day(
                conn,
                shop_id,
                user_id,
                day,
                step,
                &mut shop,
                &mut rng,
                &mut counts,
            )?;
        }
        Ok(counts)
    })
}

/// What the shop is carrying as the days go by: its fiches, the factures
/// still open on account, and the orders still waiting on goods.
struct Shop {
    catalogue: Vec<Fiche>,
    buyers: Vec<i32>,
    sellers: Vec<i32>,
    /// A facture that can still be credited: the paper, the line, and how
    /// much of that line went out, because a credit note for more than was
    /// sold is refused and a line sold by weight is not a whole unit.
    open_factures: Vec<(i32, i32, i64)>,
    /// An order, the line on it, and what is still to come in.
    open_orders: Vec<(i32, i32, i64)>,
}

/// One product as the seeder needs it back: what to sell and how fast.
struct Fiche {
    id: i32,
    /// The shelf price, so a remise off the bottom of the paper can be a
    /// share of what the basket came to rather than a figure that might be
    /// more than the basket itself.
    selling_centimes: i64,
    /// How often this one leaves the shelf, one to five. A corner shop sells
    /// bread every hour and a mop once a month, and a catalogue where every
    /// row moves at the same rate makes the top lists meaningless.
    pace: u32,
    /// Whether it is sold by the piece, which decides whether a line is a
    /// whole number of units or a weight.
    whole_units: bool,
}

// ---- the shop, its categories, its catalogue and its fiches ----

/// The seller block. A facture is refused until the shop carries RC and NIS
/// (décret 05-468 art. 3), so a seeded shop that could not issue one would be
/// a file half the screens cannot be developed against.
/// Gives the seeded owner a PIN and a password, so the file can be signed in
/// to. Through `users`, so the hashes are argon2id and the audit rows are the
/// ones a reset leaves: the seeder writes nothing by hand here either.
fn the_way_in(conn: &mut SqliteConnection, shop_id: i32, user_id: i32) -> Result<(), CoreError> {
    users::set_pin(conn, shop_id, user_id, user_id, OWNER_PIN, None)?;
    users::set_password(conn, shop_id, user_id, user_id, OWNER_PASSWORD, None)?;
    Ok(())
}

fn the_shop_itself(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
) -> Result<(), CoreError> {
    shops::update_store(
        conn,
        shop_id,
        user_id,
        StoreBlock {
            name: SHOP_NAME.to_string(),
            rc: Some("16/00-1048573 B 21".to_string()),
            nif: Some("000216104857321".to_string()),
            nis: Some("000216104857300".to_string()),
            ai: Some("16104857321".to_string()),
            address: Some("12 rue des Frères Bouadou, Bir Mourad Raïs, Alger".to_string()),
            phone: Some("021 54 12 87".to_string()),
        },
    )?;
    Ok(())
}

/// The rows a catalogue is filed under, and the rate each one carries.
///
/// Nine per cent on the staples the loi de finances taxes at the reduced rate
/// and nineteen on the rest, which is the split a corner shop actually rings
/// up. The rate is on the category so a product created under one inherits
/// it, which is the path the add-product form takes.
///
/// Written through the repo with its own audit row, the way the product
/// import opens a category a spreadsheet named: there is no category service
/// beyond the list, and inventing one so a development helper can call it
/// would be a public API nobody asked for.
fn the_categories(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    counts: &mut Counts,
) -> Result<Vec<i32>, CoreError> {
    const ROWS: [(&str, u32); 6] = [
        ("Épicerie", 900),
        ("Boissons", 1900),
        ("Produits laitiers", 900),
        ("Entretien", 1900),
        ("Hygiène", 1900),
        ("Fruits et légumes", 900),
    ];
    let mut ids = Vec::with_capacity(ROWS.len());
    for (name, rate_bps) in ROWS {
        let made = categories_repo::insert(
            conn,
            &CategoryRowWrite {
                shop_id,
                name: name.to_string(),
                default_rate_bps: i32::try_from(rate_bps)
                    .map_err(|_| CoreError::validation("rate_bps", "rate out of range"))?,
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
                        "source": "seed",
                    })
                    .to_string(),
                ),
            },
        )?;
        ids.push(made.id);
        counts.categories += 1;
    }
    Ok(ids)
}

/// Sixty products a shop in Algiers would carry: what it is called, which of
/// the six rows it is filed under, the unit it is sold in, what it costs the
/// shop and what it sells for, both in whole dinars, and how fast it moves.
///
/// Descriptive names and no brand anybody owns: a seeded file gets shared in
/// a bug report, and a screenshot of somebody's trademark on somebody else's
/// shelf is not a thing to hand around.
const CATALOGUE: [(&str, usize, Unit, i64, i64, u32); PRODUCTS] = [
    ("Semoule fine 5 kg", 0, Unit::Box, 420, 520, 5),
    ("Semoule moyenne 5 kg", 0, Unit::Box, 410, 500, 4),
    ("Farine panifiable 5 kg", 0, Unit::Box, 380, 460, 4),
    ("Riz long grain 1 kg", 0, Unit::Kg, 150, 200, 4),
    ("Pâtes coudées 500 g", 0, Unit::Piece, 75, 100, 5),
    ("Spaghetti 500 g", 0, Unit::Piece, 78, 105, 4),
    ("Vermicelle 500 g", 0, Unit::Piece, 70, 95, 3),
    ("Couscous roulé 1 kg", 0, Unit::Kg, 190, 250, 4),
    ("Lentilles 1 kg", 0, Unit::Kg, 260, 340, 3),
    ("Pois chiches 1 kg", 0, Unit::Kg, 280, 360, 3),
    ("Haricots blancs 1 kg", 0, Unit::Kg, 250, 330, 2),
    ("Sucre blanc 1 kg", 0, Unit::Kg, 105, 130, 5),
    ("Sucre en morceaux 1 kg", 0, Unit::Piece, 145, 185, 2),
    ("Sel de table 1 kg", 0, Unit::Kg, 30, 50, 3),
    ("Huile de tournesol 5 L", 0, Unit::Box, 690, 800, 5),
    ("Huile de tournesol 1 L", 0, Unit::Litre, 150, 185, 5),
    ("Huile d'olive 1 L", 0, Unit::Litre, 950, 1_250, 2),
    ("Concentré de tomate 800 g", 0, Unit::Piece, 210, 270, 4),
    ("Concentré de tomate 400 g", 0, Unit::Piece, 115, 150, 4),
    ("Thon à l'huile 160 g", 0, Unit::Piece, 190, 250, 3),
    ("Sardines à l'huile 125 g", 0, Unit::Piece, 130, 175, 3),
    ("Confiture d'abricot 400 g", 0, Unit::Piece, 210, 275, 2),
    ("Miel toutes fleurs 500 g", 0, Unit::Piece, 900, 1_150, 1),
    ("Café moulu 250 g", 0, Unit::Piece, 320, 420, 4),
    ("Thé vert en vrac 200 g", 0, Unit::Piece, 240, 320, 3),
    ("Levure boulangère 100 g", 0, Unit::Piece, 55, 80, 3),
    ("Chocolat en poudre 400 g", 0, Unit::Piece, 330, 430, 2),
    ("Biscuits fourrés 300 g", 0, Unit::Piece, 120, 165, 4),
    ("Dattes Deglet Nour 1 kg", 0, Unit::Kg, 650, 850, 2),
    ("Olives vertes en saumure 1 kg", 0, Unit::Kg, 380, 490, 2),
    ("Eau minérale 1,5 L", 1, Unit::Piece, 35, 50, 5),
    ("Eau minérale 5 L", 1, Unit::Piece, 90, 120, 4),
    ("Eau gazeuse 1 L", 1, Unit::Piece, 55, 75, 3),
    ("Soda cola 1 L", 1, Unit::Piece, 95, 130, 5),
    ("Soda orange 1 L", 1, Unit::Piece, 92, 125, 4),
    ("Limonade artisanale 1 L", 1, Unit::Piece, 80, 110, 3),
    ("Jus d'orange 1 L", 1, Unit::Piece, 120, 160, 4),
    ("Jus de pomme 1 L", 1, Unit::Piece, 120, 160, 3),
    ("Nectar d'abricot 1 L", 1, Unit::Piece, 115, 155, 3),
    ("Boisson énergisante 250 ml", 1, Unit::Piece, 130, 190, 2),
    ("Lait UHT demi écrémé 1 L", 2, Unit::Litre, 105, 130, 5),
    ("Lait en poudre 500 g", 2, Unit::Piece, 590, 720, 3),
    ("Yaourt nature 4 x 125 g", 2, Unit::Box, 130, 175, 4),
    ("Yaourt aux fruits 4 x 125 g", 2, Unit::Box, 145, 195, 4),
    ("Fromage fondu 8 portions", 2, Unit::Piece, 210, 275, 3),
    ("Beurre doux 250 g", 2, Unit::Piece, 340, 430, 2),
    ("Crème fraîche 200 ml", 2, Unit::Piece, 150, 200, 2),
    ("Camembert 250 g", 2, Unit::Piece, 380, 490, 1),
    ("Eau de javel 2 L", 3, Unit::Piece, 130, 175, 4),
    ("Liquide vaisselle 1 L", 3, Unit::Piece, 180, 240, 4),
    ("Lessive en poudre 3 kg", 3, Unit::Box, 720, 900, 3),
    ("Nettoyant sols 1 L", 3, Unit::Piece, 190, 250, 3),
    ("Éponges grattantes x 3", 3, Unit::Piece, 90, 130, 2),
    ("Sacs poubelle 30 L x 20", 3, Unit::Piece, 140, 190, 3),
    ("Savon de Marseille 300 g", 4, Unit::Piece, 110, 150, 3),
    ("Shampooing 400 ml", 4, Unit::Piece, 320, 420, 2),
    ("Dentifrice 75 ml", 4, Unit::Piece, 180, 240, 3),
    ("Papier hygiénique x 8", 4, Unit::Piece, 300, 390, 4),
    ("Pommes de terre 1 kg", 5, Unit::Kg, 65, 95, 5),
    ("Oignons 1 kg", 5, Unit::Kg, 60, 90, 5),
];

/// The catalogue, written. Every other product carries a barcode that reads
/// like the one printed on the packet; the rest get an in store one from the
/// generator in `products`, which is the path a shop takes for loose goods it
/// prints its own label for.
fn the_catalogue(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    categories: &[i32],
    counts: &mut Counts,
) -> Result<Vec<Fiche>, CoreError> {
    let mut fiches = Vec::with_capacity(PRODUCTS);
    for (nth, (name, category, unit, cost_da, selling_da, pace)) in
        CATALOGUE.into_iter().enumerate()
    {
        let category_id = categories.get(category).copied();
        let low_stock_at_milli = i64::from(pace) * LOW_STOCK_PER_PACE_MILLI;
        let barcode = if nth % 2 == 0 {
            Some(printed_barcode(nth)?)
        } else {
            None
        };
        let made = products::create(
            conn,
            shop_id,
            user_id,
            NewProduct {
                name: name.to_string(),
                barcode,
                category_id,
                unit,
                cost: Money::centimes(cost_da.saturating_mul(DA)),
                selling: Money::centimes(selling_da.saturating_mul(DA)),
                // A price for the customers who buy by the box. Below the
                // shelf price and above the cost, so a shop that used it
                // would still be trading at a margin.
                wholesale: Some(Money::centimes(
                    ((cost_da.saturating_mul(9) + selling_da) / 10).saturating_mul(DA),
                )),
                qty_on_hand_milli: i64::from(pace) * OPENING_STOCK_PER_PACE_MILLI,
                low_stock_at_milli,
                // Unset: the fiche takes the rate of the row it is filed
                // under, which is the path the add-product form takes.
                rate_bps: None,
                active: true,
            },
        )?;
        fiches.push(Fiche {
            id: made.id,
            selling_centimes: made.selling.as_centimes(),
            pace,
            whole_units: matches!(unit, Unit::Piece | Unit::Box),
        });
        counts.products += 1;
    }
    Ok(fiches)
}

/// An EAN-13 that reads like the one on the packet: the 613 prefix GS1 gave
/// Algeria, an invented manufacturer and item, and the check digit that makes
/// a scanner accept it.
///
/// Not a real registration. 613 says only "an Algerian company"; who the next
/// digits belong to is a register this file does not touch, and the numbers
/// below are made up.
fn printed_barcode(nth: usize) -> Result<String, CoreError> {
    let item = 100_000 + u64::try_from(nth).unwrap_or(0).saturating_mul(37);
    let body = format!("613{:09}", item % 1_000_000_000);
    let mut sum = 0u32;
    for (place, digit) in body.chars().enumerate() {
        let value = digit
            .to_digit(10)
            .ok_or_else(|| CoreError::validation("barcode", "a barcode is digits"))?;
        // EAN-13 weights the digits three, one, three, one from the left of
        // the twelve.
        sum += if place % 2 == 0 { value } else { value * 3 };
    }
    Ok(format!("{body}{}", (10 - sum % 10) % 10))
}

/// The twelve fiches the shop sells to on account: eight companies carrying
/// the identifiers a facture has to print (décret 05-468 art. 3) and four
/// people who walk in with a notebook.
fn the_customers(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    counts: &mut Counts,
) -> Result<Vec<i32>, CoreError> {
    const COMPANIES: [(&str, &str); 8] = [
        ("Café Essalem", "1048"),
        ("Restaurant Le Palmier", "2371"),
        ("Boulangerie Ennour", "3095"),
        ("Cantine scolaire Ibn Badis", "4162"),
        ("Hôtel Les Oliviers", "5238"),
        ("Snack Dar Diaf", "6417"),
        ("Pâtisserie El Firdaws", "7503"),
        ("Traiteur Zenith", "8629"),
    ];
    const PEOPLE: [&str; 4] = [
        "Karim Belkacem",
        "Nadia Ferhat",
        "Yacine Haddad",
        "Samira Ould Ali",
    ];
    let mut ids = Vec::with_capacity(12);
    for (nth, (name, key)) in COMPANIES.into_iter().enumerate() {
        let made = customers::create(
            conn,
            shop_id,
            user_id,
            NewCustomer {
                name: name.to_string(),
                party_kind: PartyKind::Company,
                phone: Some(format!("0550 {key} {:02}", nth * 7 % 100)),
                address: Some(format!("{} rue Larbi Ben M'hidi, Alger", 4 + nth * 3)),
                rc: Some(format!("16/00-1{key}567 B {}", 18 + nth)),
                nif: Some(format!("0002161{key}56789")),
                nis: Some(format!("0002161{key}56700")),
                ai: Some(format!("161{key}56789")),
                credit_limit: Some(Money::centimes(CREDIT_LIMIT_DA.saturating_mul(DA))),
                warn_threshold: Some(Money::centimes(CREDIT_LIMIT_DA.saturating_mul(DA) * 4 / 5)),
                notes: None,
                active: true,
            },
            None,
        )?;
        ids.push(made.id);
        counts.customers += 1;
    }
    for (nth, name) in PEOPLE.into_iter().enumerate() {
        let made = customers::create(
            conn,
            shop_id,
            user_id,
            NewCustomer {
                name: name.to_string(),
                party_kind: PartyKind::Consumer,
                phone: Some(format!("0661 22 {:02} {:02}", 30 + nth, 40 + nth * 3)),
                address: None,
                rc: None,
                nif: None,
                nis: None,
                ai: None,
                credit_limit: Some(Money::centimes(30_000 * DA)),
                warn_threshold: Some(Money::centimes(24_000 * DA)),
                notes: Some("carnet du quartier".to_string()),
                active: true,
            },
            None,
        )?;
        ids.push(made.id);
        counts.customers += 1;
    }
    Ok(ids)
}

/// The five wholesalers the shop buys from.
fn the_suppliers(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    counts: &mut Counts,
) -> Result<Vec<i32>, CoreError> {
    const ROWS: [(&str, &str); 5] = [
        ("Grossiste El Mouna", "1204"),
        ("Distribution Sidi Moussa", "2318"),
        ("Comptoir des Boissons", "3427"),
        ("Laiterie de la Mitidja", "4531"),
        ("Droguerie Centrale", "5648"),
    ];
    let mut ids = Vec::with_capacity(ROWS.len());
    for (nth, (name, key)) in ROWS.into_iter().enumerate() {
        let made = suppliers::create(
            conn,
            shop_id,
            user_id,
            NewSupplier {
                name: name.to_string(),
                phone: Some(format!("023 {key} {:02}", 10 + nth * 4)),
                address: Some(format!("Zone d'activité, lot {}, Alger", 12 + nth * 5)),
                rc: Some(format!("16/00-2{key}891 B {}", 15 + nth)),
                nif: Some(format!("0002162{key}89100")),
                nis: Some(format!("0002162{key}89122")),
                ai: Some(format!("162{key}89100")),
                notes: None,
                active: true,
            },
            None,
        )?;
        ids.push(made.id);
        counts.suppliers += 1;
    }
    Ok(ids)
}

// ---- one day of trading ----

/// Everything the shop did on one day, in the order it would have done it:
/// the deliveries in the morning, the counter all day, the money out at the
/// end.
#[allow(clippy::too_many_arguments)]
fn one_day(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    day: NaiveDate,
    step: u32,
    shop: &mut Shop,
    rng: &mut Rng,
    counts: &mut Counts,
) -> Result<(), CoreError> {
    the_deliveries(conn, shop_id, user_id, day, step, shop, rng, counts)?;
    the_counter(conn, shop_id, user_id, day, step, shop, rng, counts)?;
    the_money_out(conn, shop_id, user_id, day, step, shop, rng, counts)?;
    Ok(())
}

/// An order every third day, taken in whole or in two parts, and one lot sent
/// back over the month.
///
/// The moment is named on both the order and the delivery, so the supplier's
/// statement reads as a month and not as thirty rows stamped with the moment
/// the file happened to be written.
#[allow(clippy::too_many_arguments)]
fn the_deliveries(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    day: NaiveDate,
    step: u32,
    shop: &mut Shop,
    rng: &mut Rng,
    counts: &mut Counts,
) -> Result<(), CoreError> {
    // What arrived against an order placed earlier and only half taken in.
    // Done first, so a second delivery lands on a later day than its first.
    if step % 5 == 4 {
        if let Some((purchase_id, line_id, left)) = shop.open_orders.pop() {
            purchases::receive_at(
                conn,
                shop_id,
                user_id,
                purchase_id,
                vec![ReceiveLine {
                    purchase_line_id: line_id,
                    qty_milli: left,
                }],
                Some("solde de la commande".to_string()),
                Some(moment(day, 8, step)?),
            )?;
            counts.receipts += 1;
        }
    }
    if !step.is_multiple_of(2) || step > LAST_ORDER_STEP {
        return Ok(());
    }

    let supplier = shop.sellers[(step as usize / 2) % shop.sellers.len()];
    // What the shop orders is what it is running out of, which is what a shop
    // orders. Chosen by how far each fiche has fallen towards its own reorder
    // point rather than by walking the catalogue: the fast movers then come
    // back on the orders, the slow ones stay on the shelf, and the low stock
    // list on the dashboard has the rows a real one would.
    let mut shelf: Vec<(i64, i32, i64, i64)> = Vec::with_capacity(shop.catalogue.len());
    for fiche in &shop.catalogue {
        let on_hand = products::get(conn, shop_id, fiche.id)?;
        shelf.push((
            on_hand.qty_on_hand_milli - on_hand.low_stock_at_milli,
            fiche.id,
            i64::from(fiche.pace) * REORDER_TO_PER_PACE_MILLI - on_hand.qty_on_hand_milli,
            on_hand.cost.as_centimes(),
        ));
    }
    // The id breaks a tie, so two runs order the same four fiches.
    shelf.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut lines = Vec::with_capacity(LINES_PER_ORDER);
    for (_, product_id, wanted, cost_centimes) in shelf.into_iter().take(LINES_PER_ORDER) {
        // A whole number of units, and at least twenty of them so half an
        // order is still a quantity a delivery can carry.
        let units = (wanted / 1_000).max(20) + i64::from(rng.upto(20));
        lines.push(NewLine {
            product_id,
            qty_ordered_milli: units * 1_000,
            unit_cost: Money::centimes(cost_centimes),
        });
    }
    let order = purchases::save(
        conn,
        shop_id,
        user_id,
        NewPurchase {
            supplier_id: supplier,
            supplier_document_number: Some(format!("BL-{}-{:04}", day.year(), 1_200 + step)),
            purchase_date: day.format("%Y-%m-%d").to_string(),
            due_date: day
                .checked_add_days(chrono::Days::new(30))
                .map(|d| d.format("%Y-%m-%d").to_string()),
            transport: Money::centimes(i64::from(500 + rng.upto(2_000)) * DA),
            extra_costs: Money::ZERO,
            note: None,
            lines,
            // Nothing handed over with the order. What the shop pays and when
            // is `the_money_out` below, which dates its own payments; the
            // payment `paid_now` writes takes the wall clock, and a month of
            // history cannot be written with one.
            paid_now: None,
            receive_now: false,
        },
    )?;
    counts.purchases += 1;

    // Every other order arrives whole; the rest come in two parts, so the
    // dashboard's count of orders still waiting on goods is not zero and the
    // receipts screen has a partial to show.
    let whole = step.is_multiple_of(6);
    let mut arriving = Vec::with_capacity(order.lines.len());
    for line in &order.lines {
        let taken = if whole {
            line.qty_ordered_milli
        } else {
            line.qty_ordered_milli / 2
        };
        arriving.push(ReceiveLine {
            purchase_line_id: line.id,
            qty_milli: taken,
        });
        if !whole {
            shop.open_orders
                .push((order.purchase.id, line.id, line.qty_ordered_milli - taken));
        }
    }
    let received = purchases::receive_at(
        conn,
        shop_id,
        user_id,
        order.purchase.id,
        arriving,
        None,
        Some(moment(day, 7, step)?),
    )?;
    counts.receipts += 1;

    // One lot sent back over the month: goods that came in damaged. Once, on
    // a day far enough in that there is a delivery to send back.
    if step == 12 {
        if let Some(line) = received.lines.first() {
            purchases::return_to_supplier_at(
                conn,
                shop_id,
                user_id,
                order.purchase.id,
                vec![ReceiveLine {
                    purchase_line_id: line.id,
                    qty_milli: (line.qty_received_milli / 4).max(1_000),
                }],
                Some("marchandise abîmée au transport".to_string()),
                Some(moment(day, 9, step)?),
            )?;
            counts.returns += 1;
        }
    }
    Ok(())
}

/// The counter: between twenty five and sixty papers a day, most of them cash
/// tickets, some on card, and the rest factures on account. Two of the month's
/// papers are annulled, several factures are credited in part, and one
/// customer asks for a quotation.
#[allow(clippy::too_many_arguments)]
fn the_counter(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    day: NaiveDate,
    step: u32,
    shop: &mut Shop,
    rng: &mut Rng,
    counts: &mut Counts,
) -> Result<(), CoreError> {
    let how_many = 25 + rng.upto(36);
    for nth in 0..how_many {
        // Roughly two thirds cash, one in six on card, the rest on account.
        let (mode, kind) = match rng.upto(12) {
            0..=7 => (PaymentMode::Cash, SaleKind::Ticket),
            8..=9 => (PaymentMode::Card, SaleKind::Ticket),
            _ => (PaymentMode::Credit, SaleKind::Facture),
        };
        let customer = match kind {
            // A facture is made out to somebody, and to somebody carrying the
            // identifiers it has to print: the eight companies are first in
            // the list.
            SaleKind::Facture => Some(shop.buyers[usize::from(rng.upto(8))]),
            _ => None,
        };
        let basket = the_basket(shop, rng);
        // A remise off the bottom of the paper now and then, so the margin
        // the dashboard reads is not simply the lines' own. Five per cent of
        // what the basket came to and never a figure of its own: a remise
        // above the basket is refused, and a shop knocking a round number off
        // a small ticket is not what a shop does anyway.
        let global_discount = if nth % 9 == 3 {
            Money::centimes(a_twentieth_of(shop, &basket))
        } else {
            Money::ZERO
        };
        let sale = sales::issue(
            conn,
            shop_id,
            user_id,
            NewSale {
                lines: basket,
                global_discount,
                payment_mode: mode,
                tendered: match mode {
                    // What the customer put on the counter. Generous, so no
                    // sale is refused for a note that was too small; what the
                    // till gives back is the change the service computes.
                    PaymentMode::Cash => Some(Money::centimes(2_000_000 * DA)),
                    _ => None,
                },
                customer_id: customer,
                // Never overridden. The limits above are high enough that a
                // month of credit never reaches one, so a refusal here would
                // be a real bug and not a shop pushing past its own rule.
                override_credit: false,
                kind,
                issued_at: Some(moment(day, u32::from(8 + nth % 11), u32::from(nth))?),
            },
        )?;
        counts.sales += 1;
        if kind == SaleKind::Facture {
            if let Some(line) = sale.document.lines.first() {
                shop.open_factures
                    .push((sale.document.id, line.id, line.qty_milli));
            }
        }
    }

    // A credit note every fourth day, on a facture written earlier: goods a
    // customer brought back.
    if step % 4 == 3 {
        if let Some((facture_id, line_id, sold_milli)) = shop.open_factures.pop() {
            // Half of what went out, and the whole of it when half is
            // nothing: a credit note for more than was sold is refused, and a
            // line sold by weight can be less than a unit.
            let back = (sold_milli / 2).max(1).min(sold_milli);
            avoir::issue(
                conn,
                shop_id,
                user_id,
                facture_id,
                Some(vec![avoir::AvoirLine {
                    document_line_id: line_id,
                    qty_milli: back,
                }]),
                Some("retour client".to_string()),
                Some(moment(day, 17, step)?),
            )?;
            counts.avoirs += 1;
        }
    }

    // Two papers annulled over the month, one of each kind: a ticket rung up
    // twice, and a facture made out to the wrong customer. A cancelled
    // facture writes its own credit note, which is why the counts below name
    // the cancellation and not the paper it produced.
    if step == 9 || step == 21 {
        let kind = if step == 9 {
            crate::models::sql_types::DocumentKind::Ticket
        } else {
            crate::models::sql_types::DocumentKind::Facture
        };
        let latest = documents::list(conn, shop_id, Some(kind))?
            .into_iter()
            .find(|d| {
                d.issued_at.date() == day
                    && d.status == crate::models::sql_types::DocumentStatus::Issued
            });
        if let Some(paper) = latest {
            cancellation::cancel(
                conn,
                shop_id,
                user_id,
                paper.id,
                "erreur de saisie au comptoir".to_string(),
                Some(moment(day, 19, step)?),
            )?;
            counts.cancellations += 1;
            shop.open_factures.retain(|(id, _, _)| *id != paper.id);
        }
    }

    // One quotation over the month: a basket priced for a customer who has
    // not decided. It moves neither stock nor money, which is the point of
    // having one on the file.
    if step == 16 {
        sales::issue(
            conn,
            shop_id,
            user_id,
            NewSale {
                lines: the_basket(shop, rng),
                global_discount: Money::ZERO,
                payment_mode: PaymentMode::Credit,
                tendered: None,
                customer_id: Some(shop.buyers[0]),
                override_credit: false,
                kind: SaleKind::Proforma,
                issued_at: Some(moment(day, 11, step)?),
            },
        )?;
        counts.proformas += 1;
    }
    Ok(())
}

/// One basket: one to four lines off the shelf, weighted so a fast mover
/// turns up more often than a jar of honey.
fn the_basket(shop: &Shop, rng: &mut Rng) -> Vec<NewSaleLine> {
    let mut lines = Vec::new();
    let mut tries = 0u32;
    let how_many = 1 + rng.upto(4);
    while lines.len() < usize::from(how_many) && tries < 40 {
        tries += 1;
        let fiche = &shop.catalogue
            [usize::from(rng.upto(u16::try_from(shop.catalogue.len()).unwrap_or(u16::MAX)))];
        // The pace is the weight: a five turns up five times as often as a
        // one, which is what gives the top lists a shape.
        if rng.upto(5) >= u16::try_from(fiche.pace).unwrap_or(1) {
            continue;
        }
        if lines.iter().any(|l: &NewSaleLine| l.product_id == fiche.id) {
            continue;
        }
        let qty_milli = if fiche.whole_units {
            i64::from(1 + rng.upto(3)) * 1_000
        } else {
            // By weight: half a kilo to two and a half, in hundred gram
            // steps, so a line total and a cost both round on their own.
            i64::from(5 + rng.upto(21)) * 100
        };
        lines.push(NewSaleLine {
            product_id: fiche.id,
            qty_milli,
            unit_price: None,
            line_discount: Money::ZERO,
        });
    }
    if lines.is_empty() {
        // The loop can come up empty on a run of unlucky weights, and a sale
        // with no line is refused. The first fiche is the fallback.
        lines.push(NewSaleLine {
            product_id: shop.catalogue[0].id,
            qty_milli: 1_000,
            unit_price: None,
            line_discount: Money::ZERO,
        });
    }
    lines
}

/// Five per cent of what a basket comes to at the shelf price, in whole
/// dinars. Read off the fiches the seeder is holding rather than off the
/// database: this is a remise somebody decided on before the paper was
/// written, and the paper's own total does not exist yet.
fn a_twentieth_of(shop: &Shop, basket: &[NewSaleLine]) -> i64 {
    let mut gross: i128 = 0;
    for line in basket {
        let Some(fiche) = shop.catalogue.iter().find(|f| f.id == line.product_id) else {
            continue;
        };
        gross += i128::from(fiche.selling_centimes) * i128::from(line.qty_milli) / 1_000;
    }
    let cut = i64::try_from(gross / 20).unwrap_or(0);
    // Rounded down to the dinar: a remise of 43,17 DA is not one a shop gives.
    (cut / DA) * DA
}

/// The end of the day: what the customers paid off their accounts, what the
/// shop paid its suppliers, and what it spent on everything that is not
/// stock.
#[allow(clippy::too_many_arguments)]
fn the_money_out(
    conn: &mut SqliteConnection,
    shop_id: i32,
    user_id: i32,
    day: NaiveDate,
    step: u32,
    shop: &mut Shop,
    rng: &mut Rng,
    counts: &mut Counts,
) -> Result<(), CoreError> {
    // Two customers settle something every other day. Never the whole
    // balance: a screen with nothing outstanding on it shows nothing.
    if step.is_multiple_of(2) {
        for nth in 0..2usize {
            let customer = shop.buyers[(step as usize + nth * 5) % shop.buyers.len()];
            let owed = debt::balance(conn, shop_id, customer)?;
            if owed <= Money::ZERO {
                continue;
            }
            let paying = Money::centimes((owed.as_centimes() * 3 / 5).max(100 * DA));
            let paying = if paying > owed { owed } else { paying };
            debt::pay(
                conn,
                shop_id,
                user_id,
                customer,
                paying,
                if nth == 0 {
                    PaymentMethod::Cash
                } else {
                    PaymentMethod::Card
                },
                Some("versement sur compte".to_string()),
                moment(day, 18, step)?,
            )?;
            counts.customer_payments += 1;
        }
    }

    // The shop pays a supplier every fourth day, and leaves something on
    // every account: a supplier statement with a zero balance says nothing.
    if step % 4 == 1 {
        let supplier = shop.sellers[(step as usize / 4) % shop.sellers.len()];
        let owed = supplier_debt::balance(conn, shop_id, supplier)?;
        if owed > Money::ZERO {
            let paying = Money::centimes((owed.as_centimes() / 2).max(100 * DA));
            let paying = if paying > owed { owed } else { paying };
            supplier_debt::pay(
                conn,
                shop_id,
                user_id,
                supplier,
                paying,
                PaymentMethod::Cash,
                Some("acompte fournisseur".to_string()),
                moment(day, 16, step)?,
            )?;
            counts.supplier_payments += 1;
        }
    }

    // Money out that is not stock. The seven categories the migration seeded
    // each get rows: rent once a month, salaries once a month, the utilities
    // on their own days and the small stuff whenever.
    let categories = expenses::categories(conn, shop_id)?;
    for (nth, category) in categories.iter().enumerate() {
        let due = match category.key.as_str() {
            "rent" => day.day() == 1,
            "salaries" => day.day() == 28,
            "electricity" | "water" => step % 15 == u32::try_from(nth).unwrap_or(0) % 15,
            _ => step % 6 == u32::try_from(nth).unwrap_or(0) % 6,
        };
        // Every category gets at least one row over the month, whatever the
        // calendar did: a screen that groups by category should have no blank
        // column on the seeded file.
        if !due && step != u32::try_from(nth).unwrap_or(0) {
            continue;
        }
        let amount_da = match category.key.as_str() {
            "rent" => 45_000,
            "salaries" => 90_000,
            "electricity" => 6_000 + i64::from(rng.upto(3_000)),
            "water" => 2_500 + i64::from(rng.upto(1_500)),
            "transport" => 1_800 + i64::from(rng.upto(2_200)),
            "maintenance" => 2_000 + i64::from(rng.upto(6_000)),
            _ => 900 + i64::from(rng.upto(2_500)),
        };
        expenses::create(
            conn,
            shop_id,
            user_id,
            NewExpense {
                category_id: category.id,
                amount: Money::centimes(amount_da.saturating_mul(DA)),
                expense_date: day,
                note: None,
            },
        )?;
        counts.expenses += 1;
    }
    Ok(())
}

/// The moment a paper of that day is written at. Inside the day whatever the
/// caller passes, so a walking hour never spills into tomorrow and the
/// figures of a day stay on it.
fn moment(day: NaiveDate, hour: u32, nth: u32) -> Result<NaiveDateTime, CoreError> {
    day.and_hms_opt(hour.min(22), nth % 60, (nth * 7) % 60)
        .ok_or_else(|| {
            CoreError::validation("day", "that day is outside the calendar the shop keeps")
        })
}

// ---- the randomness ----

/// splitmix64. Twenty lines, no dependency, and the same stream on every
/// machine and every build, which is the whole requirement: nothing here is
/// cryptographic and nothing here decides an amount a comptable reads.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `bound`, or zero when the caller asks for nothing.
    /// Modulo, which skews the low end by a part in 2^64 and is not something
    /// a development file can tell apart from uniform.
    fn upto(&mut self, bound: u16) -> u16 {
        if bound == 0 {
            return 0;
        }
        u16::try_from(self.next() % u64::from(bound)).unwrap_or(0)
    }
}
