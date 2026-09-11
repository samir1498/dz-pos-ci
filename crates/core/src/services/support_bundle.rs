//! The support bundle (M5 T3, `docs/architecture.md` § Release): one zip a
//! shop can send when something needs diagnosing from outside, holding
//! nothing a shop would mind a stranger reading.
//!
//! What goes in, and only this, six plain text files:
//!
//! - `README.txt`, headed by [`crate::build_info::header_line`] (the same
//!   call the log file's own header is, `crate::log::head_session`), then
//!   the sentence a shopkeeper is sent in French, English and Arabic, and a
//!   manifest of the five files beside it.
//! - `log.txt`, the shop's own `dzpos.log` verbatim: a sequence of sessions,
//!   each headed the same way.
//! - `migrations.txt`, which migrations the shop's file has applied and
//!   which this build ships, read off [`crate::db::applied_versions`] and
//!   [`crate::db::embedded_versions`].
//! - `schema.txt`, the shape of every table (its name and its columns' own
//!   names and SQL types), never a row of what any of them hold.
//! - `counts.txt`, how many products, documents and customers the shop has,
//!   how big its file is, and how many backups it holds and when the newest
//!   was taken.
//! - `system.txt`, the operating system, the machine's language and its
//!   time zone.
//!
//! What never goes in, because nothing here reads it: a customer's name or
//! identifiers, a product's name, a price or an amount, any part of a
//! document, a PIN, a password or a hash of either, the launch token, and
//! the shop's own registration numbers (RC, NIF, NIS, AI). The shop file
//! itself never goes in either. `crates/api/tests/support_bundle.rs` is the
//! test that holds this: it seeds a shop with names, prices and documents a
//! real one would carry, builds a bundle from it, and fails on any of that
//! turning up anywhere in the zip's bytes, entry names included.
//!
//! The cost of that list. A bundle that carries none of a shop's own data
//! cannot answer "why is this customer's balance wrong": there is a count of
//! customers and no row of any of them. What it can answer is "does this
//! file's schema match this build", "did the last upgrade's migrations run",
//! "is the shop file the size a month of trading should have made it", and
//! "was a backup ever taken". A fault that needs the actual figures needs
//! the shop to read a screen aloud, or a supervised look at the file itself;
//! this bundle is deliberately not that.

use std::io::{Cursor, Write};
use std::path::Path;

use chrono::NaiveDateTime;
use diesel::sql_types::{BigInt, Text};
use diesel::{QueryableByName, RunQueryDsl};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::build_info::{header_line, BUILD_INFO};
use crate::db::{self, Conn};
use crate::error::CoreError;
use crate::services::{audit, backup};

/// The entry names the zip carries, in the order they are written. Nothing
/// else may be in the archive; the walking test checks the set is exactly
/// this, names included, so a table dump added under any other name is
/// refused for not being on the list rather than for what it holds.
pub const ENTRIES: [&str; 6] = [
    "README.txt",
    "log.txt",
    "migrations.txt",
    "schema.txt",
    "counts.txt",
    "system.txt",
];

const README_BODY: &str = "\n\
FR: Ouvrez Param\u{e8}tres, appuyez sur \u{ab} Dossier d'assistance \u{bb}, puis envoyez-nous ce fichier zip. Il contient le journal de la caisse, la version du logiciel, la forme de la base de donn\u{e9}es et quelques totaux. Aucun nom de client, aucun nom de produit, aucun prix et aucun document ne s'y trouve.\n\
\n\
EN: Open Settings, press \"Support bundle\", then send us this zip file. It holds the till's own log, the software version, the shape of the database and a few counts. No customer name, no product name, no price and no document is in it.\n\
\n\
AR: \u{627}\u{641}\u{62a}\u{62d} \u{627}\u{644}\u{625}\u{639}\u{62f}\u{627}\u{62f}\u{627}\u{62a}\u{60c} \u{627}\u{636}\u{63a}\u{637} \u{639}\u{644}\u{649} \u{ab}\u{645}\u{644}\u{641} \u{627}\u{644}\u{62f}\u{639}\u{645}\u{bb}\u{60c} \u{62b}\u{645} \u{623}\u{631}\u{633}\u{644} \u{644}\u{646}\u{627} \u{647}\u{630}\u{627} \u{627}\u{644}\u{645}\u{644}\u{641} \u{627}\u{644}\u{645}\u{636}\u{63a}\u{648}\u{637}. \u{64a}\u{62d}\u{62a}\u{648}\u{64a} \u{639}\u{644}\u{649} \u{633}\u{62c}\u{644} \u{627}\u{644}\u{635}\u{646}\u{62f}\u{648}\u{642}\u{60c} \u{648}\u{625}\u{635}\u{62f}\u{627}\u{631} \u{627}\u{644}\u{628}\u{631}\u{646}\u{627}\u{645}\u{62c}\u{60c} \u{648}\u{634}\u{643}\u{644} \u{642}\u{627}\u{639}\u{62f}\u{629} \u{627}\u{644}\u{628}\u{64a}\u{627}\u{646}\u{627}\u{62a} \u{648}\u{628}\u{639}\u{636} \u{627}\u{644}\u{625}\u{62d}\u{635}\u{627}\u{621}\u{627}\u{62a}. \u{644}\u{627} \u{64a}\u{62d}\u{62a}\u{648}\u{64a} \u{639}\u{644}\u{649} \u{627}\u{633}\u{645} \u{623}\u{64a} \u{632}\u{628}\u{648}\u{646} \u{623}\u{648} \u{645}\u{646}\u{62a}\u{62c} \u{623}\u{648} \u{633}\u{639}\u{631} \u{623}\u{648} \u{623}\u{64a} \u{648}\u{62b}\u{64a}\u{642}\u{629}.\n\
\n\
This zip holds six files:\n\
- README.txt: this page\n\
- log.txt: the till's own session log, beside the shop file\n\
- migrations.txt: which migrations this file has applied, and which this build ships\n\
- schema.txt: the shape of every table (names and column types), no row of data\n\
- counts.txt: how many products, documents and customers the shop has, the shop file's size, and its backups\n\
- system.txt: the operating system, the machine's language and its time zone\n";

/// One table's shape: its name, and each column's own name and SQL type, in
/// `PRAGMA table_info`'s own order. Never a row of what the table holds.
pub type TableShape = (String, Vec<(String, String)>);

/// Everything the bundle carries, read off the shop's own connection and the
/// three paths beside its file. Public so the walking test can ask for the
/// same figures the zip was built from without re-deriving them from the
/// database a second time.
pub struct Facts {
    pub applied_migrations: Vec<String>,
    pub shipped_migrations: Vec<String>,
    pub schema: Vec<TableShape>,
    pub products: i64,
    pub documents: i64,
    pub customers: i64,
    pub shop_file_bytes: u64,
    pub backups: usize,
    pub backups_newest: Option<NaiveDateTime>,
    pub os: &'static str,
    pub language: String,
    pub timezone: String,
}

/// Reads every fact the bundle carries. `conn` is the shop's live
/// connection (already migrated, the way `AppState` always holds it);
/// `db_path` is the shop file itself, read only for its size; `backup_dir`
/// is where the daily copies live.
pub fn gather(conn: &mut Conn, db_path: &Path, backup_dir: &Path) -> Result<Facts, CoreError> {
    let applied_migrations = db::applied_versions(conn)?;
    let shipped_migrations = db::embedded_versions()?;
    let schema = table_shapes(conn)?;

    let products = count_rows(conn, "products")?;
    let documents = count_rows(conn, "documents")?;
    let customers = count_rows(conn, "customers")?;
    let shop_file_bytes = std::fs::metadata(db_path)?.len();

    let copies = backup::list(backup_dir)?;
    let backups = copies.len();
    let backups_newest = copies.first().map(|b| b.taken_at);

    Ok(Facts {
        applied_migrations,
        shipped_migrations,
        schema,
        products,
        documents,
        customers,
        shop_file_bytes,
        backups,
        backups_newest,
        os: std::env::consts::OS,
        language: machine_language(),
        timezone: machine_timezone(),
    })
}

/// Every table `sqlite_master` names, with each column's own name and SQL
/// type. Triggers and indexes are left out on purpose: a trigger's body is
/// the SQL that wrote it, which is closer to a rule than a shape, and this
/// is only ever asked for the shape.
fn table_shapes(conn: &mut Conn) -> Result<Vec<TableShape>, CoreError> {
    #[derive(QueryableByName)]
    struct TableRow {
        #[diesel(sql_type = Text)]
        name: String,
    }
    let tables: Vec<TableRow> = diesel::sql_query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .load(conn)?;

    let mut shapes = Vec::with_capacity(tables.len());
    for table in tables {
        #[derive(QueryableByName)]
        struct ColumnRow {
            #[diesel(sql_type = Text)]
            name: String,
            #[diesel(sql_type = Text, column_name = r#type)]
            col_type: String,
        }
        // `table.name` is never a caller's: it is read off this file's own
        // `sqlite_master` two lines above, the same trust boundary
        // `services::backup`'s own table-name interpolation already relies
        // on. Doubled the one way SQLite's own quoting rule doubles it,
        // the same escape `backup::copy_to` uses for a path.
        let escaped = table.name.replace('\'', "''");
        let columns: Vec<ColumnRow> =
            diesel::sql_query(format!("PRAGMA table_info('{escaped}')")).load(conn)?;
        shapes.push((
            table.name,
            columns.into_iter().map(|c| (c.name, c.col_type)).collect(),
        ));
    }
    Ok(shapes)
}

/// `SELECT COUNT(*) FROM <table>`. `table` is always one of this module's own
/// literals, never a caller's.
fn count_rows(conn: &mut Conn, table: &str) -> Result<i64, CoreError> {
    #[derive(QueryableByName)]
    struct Count {
        #[diesel(sql_type = BigInt)]
        n: i64,
    }
    let row: Count =
        diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {table}")).get_result(conn)?;
    Ok(row.n)
}

/// The first of `LC_ALL`, `LC_MESSAGES`, `LANG` and `LANGUAGE` that is set
/// and not blank; the order glibc itself resolves a locale in. `"not set"`
/// is the answer a bundle can carry and a reader can trust rather than a
/// guess this module never made: Windows sets none of these, so the
/// shipped Windows binary reports `"not set"` here too, not its actual
/// locale — a real Windows-API lookup is not implemented.
fn machine_language() -> String {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"] {
        if let Ok(value) = std::env::var(var) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    "not set".to_string()
}

/// The machine's own IANA zone name (`Africa/Algiers`, `Europe/Paris`), read
/// through `iana-time-zone`, already in the workspace's lock file through
/// another crate's own dependency tree; this makes it a direct one for the
/// single caller that reads it.
fn machine_timezone() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "unknown".to_string())
}

fn readme() -> String {
    format!("{}\n{README_BODY}", header_line(&BUILD_INFO))
}

fn migrations_txt(facts: &Facts) -> String {
    let mut out = String::from("applied:\n");
    for v in &facts.applied_migrations {
        out.push_str(v);
        out.push('\n');
    }
    out.push_str("shipped:\n");
    for v in &facts.shipped_migrations {
        out.push_str(v);
        out.push('\n');
    }
    out
}

fn schema_txt(facts: &Facts) -> String {
    let mut out = String::new();
    for (table, columns) in &facts.schema {
        out.push_str(table);
        out.push('(');
        let cols: Vec<String> = columns
            .iter()
            .map(|(name, ty)| format!("{name} {ty}"))
            .collect();
        out.push_str(&cols.join(", "));
        out.push_str(")\n");
    }
    out
}

fn counts_txt(facts: &Facts) -> String {
    let newest = facts.backups_newest.map_or_else(
        || "none".to_string(),
        |at| at.format("%Y-%m-%dT%H:%M:%S").to_string(),
    );
    format!(
        "products: {}\ndocuments: {}\ncustomers: {}\nshop_file_bytes: {}\nbackups: {}\nbackups_newest: {newest}\n",
        facts.products, facts.documents, facts.customers, facts.shop_file_bytes, facts.backups,
    )
}

fn system_txt(facts: &Facts) -> String {
    format!(
        "os: {}\nlanguage: {}\ntimezone: {}\n",
        facts.os, facts.language, facts.timezone
    )
}

/// The zip itself: the six files above, in [`ENTRIES`]'s order, each
/// written through `start_file` and never appended to once closed, so a
/// caller reading the archive back sees exactly what this wrote and nothing
/// a later write grew onto an open entry.
pub fn build_zip(facts: &Facts, log: &str) -> Result<Vec<u8>, CoreError> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();

    let files: [(&str, String); 6] = [
        (ENTRIES[0], readme()),
        (ENTRIES[1], log.to_string()),
        (ENTRIES[2], migrations_txt(facts)),
        (ENTRIES[3], schema_txt(facts)),
        (ENTRIES[4], counts_txt(facts)),
        (ENTRIES[5], system_txt(facts)),
    ];
    for (name, contents) in files {
        writer.start_file(name, options).map_err(zip_err)?;
        writer.write_all(contents.as_bytes())?;
    }
    let cursor = writer.finish().map_err(zip_err)?;
    Ok(cursor.into_inner())
}

fn zip_err(e: zip::result::ZipError) -> CoreError {
    CoreError::Io(std::io::Error::other(e.to_string()))
}

/// The row a bundle leaves behind: that one was built, by whom and when.
/// No `before` and no `after`, because there is only ever one kind of
/// bundle and nothing here is worth a second copy of in the log a row about
/// it would have to redact anyway. `crates/api::AppState::support_bundle` is
/// the only caller, and it calls this after the zip is already built and
/// before it is handed back, the same order `services::export`'s own row
/// is written in and for the same reason: the file is on its way out of the
/// shop, and a caller that got the bytes anyway despite this write failing
/// would have no record that it happened.
pub fn record(conn: &mut Conn, shop_id: i32, actor_id: i32) -> Result<(), CoreError> {
    audit::record(
        conn,
        shop_id,
        actor_id,
        audit::Change {
            action: audit::ACTION_SUPPORT_BUNDLE,
            entity: "support_bundle",
            entity_id: None,
            before: None,
            after: None,
        },
    )
}

/// Reads the shop's own log file, the one [`crate::log::head_session`]
/// writes, verbatim. Empty when it has never been written (a fresh file
/// with no session yet), read as empty rather than refused: a bundle asked
/// for the moment a shop is set up should still build.
pub fn read_log(log_path: &Path) -> Result<String, CoreError> {
    match std::fs::read_to_string(log_path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.into()),
    }
}
