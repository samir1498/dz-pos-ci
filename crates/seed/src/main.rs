//! Fills a development shop file with a catalogue and a month of trading.
//!
//! Why a crate of its own and not a flag on the API, or a binary inside it.
//! The API holds its file open for as long as it serves, and cleaning up here
//! deletes that file: a flag would mean the server wiping the database out
//! from under its own connection. A second `[[bin]]` inside `dzpos-api` would
//! have been worse than useless, because the API's own package would then
//! build it, and "the shipped app cannot contain the seeder" would rest on
//! nobody adding a dependency by accident. A package apart means the desktop
//! and the API depend on `dzpos-core` and not on this, so no release build of
//! either can produce it. `crates/api/tests/no_seed_entrypoint.rs` is what
//! holds that.
//!
//! Dev only, in three places, and this file is the second of them:
//! 1. the crate boundary above, which a release build cannot cross;
//! 2. `admit` below: `DZPOS_DEV=1`, a file under `.dev/`, and a refusal to
//!    touch a shop that carries a real shop's identifiers;
//! 3. `just seed` and `just seed-clean`, which take no path at all.
//!
//! What it decides: nothing about a shop. Every rule is
//! `dzpos_core::services::seed`, which drives the same services the till
//! does. This file parses flags, checks it is allowed to run, opens the
//! database and prints what was written.

use clap::Parser;
use dzpos_core::models::shop::Shop;
use dzpos_core::services::{seed, shops};

/// The environment variable that says this is a development box. Set by the
/// `just` recipes and by nothing the installer ships.
const DEV_VAR: &str = "DZPOS_DEV";

/// The only directory a seeded file may live in. Compared as the file's own
/// parent directory name, so neither `..` nor a symlinked path reaches a
/// shop's real database by spelling.
const DEV_DIR: &str = ".dev";

#[derive(Parser)]
#[command(
    name = "dzpos-seed",
    about = "fill a dz-pos development shop file with a catalogue and a month of trading"
)]
struct Args {
    /// Path to the development shop file, which has to be one directly inside
    /// `.dev/`. Created and migrated if missing.
    #[arg(long)]
    db: std::path::PathBuf,
    /// The one shop the rows belong to.
    #[arg(long, default_value_t = 1)]
    shop: i32,
    /// Who the audit log records as the author of every seeded row.
    #[arg(long, default_value_t = 1)]
    user: i32,
    /// The day the thirty days of history end on, `YYYY-MM-DD`. Default: the
    /// shop's today. Name one to get a file that reads the same next week.
    #[arg(long)]
    today: Option<String>,
    /// Write over a file whose settings carry a shop's own name and
    /// identifiers. Nothing else: it does not lift the `.dev/` rule and it
    /// does not lift `DZPOS_DEV`, because a real shop file that somebody
    /// copied into `.dev` is still a real shop file.
    #[arg(long, default_value_t = false)]
    force: bool,
}

/// Why the seeder will not run. Its own type so `admit` is a pure function
/// with a test per arm, rather than three `eprintln`s nobody can exercise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Refusal {
    NotADevBox,
    NotUnderDevDir,
    LooksLikeARealShop,
}

impl Refusal {
    fn why(self) -> String {
        match self {
            Refusal::NotADevBox => format!(
                "this is not a development box: {DEV_VAR}=1 is what says it is, and the \
                 `just seed` recipe is what sets it"
            ),
            Refusal::NotUnderDevDir => {
                format!("a seeded file lives directly inside {DEV_DIR}/, and nowhere else")
            }
            Refusal::LooksLikeARealShop => {
                "this file's settings carry a shop's own name and identifiers, so it is \
                 somebody's books and not a development file; pass --force if you are sure"
                    .to_string()
            }
        }
    }
}

/// What the file says about the shop, as much of it as the guard reads.
/// Passed in rather than read here so the rule is testable without a
/// database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Store<'a> {
    name: &'a str,
    has_identifiers: bool,
}

/// Whether the seeder may touch this file. Every arm is a refusal a person
/// can act on, and the order matters: the environment first, because a box
/// that is not a development box has no business having its paths inspected;
/// the path next, because it is decided without opening anything; the shop's
/// own identity last, because reading it means opening the file.
fn admit(
    db: &std::path::Path,
    dev: Option<&str>,
    force: bool,
    store: Option<Store<'_>>,
) -> Result<(), Refusal> {
    if dev != Some("1") {
        return Err(Refusal::NotADevBox);
    }
    let under_dev = db
        .parent()
        .and_then(|p| p.file_name())
        .is_some_and(|name| name == DEV_DIR);
    if !under_dev {
        return Err(Refusal::NotUnderDevDir);
    }
    // A file the seeder wrote carries the seeder's own invented shop, so it
    // is not somebody's books however many identifiers it holds.
    if let Some(store) = store {
        if store.has_identifiers && store.name != seed::SHOP_NAME && !force {
            return Err(Refusal::LooksLikeARealShop);
        }
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run(Args::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("dzpos-seed failed: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let dev = std::env::var(DEV_VAR).ok();
    // The two guards that need nothing open. Checked before the file is
    // created, so a refused run leaves no `.db` behind where it was refused.
    admit(&args.db, dev.as_deref(), args.force, None).map_err(Refusal::why)?;

    let today = match args.today.as_deref() {
        Some(text) => chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map_err(|_| "--today is a day written YYYY-MM-DD")?,
        None => dzpos_core::services::clock::now().date(),
    };

    if let Some(parent) = args.db.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // Opened before anything is deleted: the third guard reads the shop this
    // file already holds, and a wipe that ran first would have nothing left
    // to refuse.
    if args.db.exists() {
        let mut conn = dzpos_core::db::open(&args.db)?;
        let shop: Shop = shops::get(&mut conn, args.shop)?;
        admit(
            &args.db,
            dev.as_deref(),
            args.force,
            Some(Store {
                name: &shop.name,
                has_identifiers: shop.rc.is_some() || shop.nif.is_some() || shop.nis.is_some(),
            }),
        )
        .map_err(Refusal::why)?;
        // Closed before the file goes: a connection to a deleted inode is a
        // connection that writes into nothing.
        drop(conn);
        remove_the_file(&args.db)?;
    }

    let mut conn = dzpos_core::db::open(&args.db)?;
    let counts = seed::run(&mut conn, args.shop, args.user, today)?;
    println!("seeded {} up to {today}", args.db.display());
    for (label, count) in [
        ("categories", counts.categories),
        ("products", counts.products),
        ("customers", counts.customers),
        ("suppliers", counts.suppliers),
        ("days", counts.days),
        ("purchases", counts.purchases),
        ("receipts", counts.receipts),
        ("returns to supplier", counts.returns),
        ("sales", counts.sales),
        ("avoirs", counts.avoirs),
        ("cancellations", counts.cancellations),
        ("proformas", counts.proformas),
        ("customer payments", counts.customer_payments),
        ("supplier payments", counts.supplier_payments),
        ("expenses", counts.expenses),
    ] {
        println!("  {label:<20} {count}");
    }
    Ok(())
}

/// The file and the two SQLite writes beside it. Deleting the database and
/// leaving its `-wal` behind gives SQLite a journal for a database that no
/// longer exists.
///
/// A whole file and never a row. The ledgers are append only and the document
/// series are gapless by rule, so there is no honest way to take a seeded sale
/// back out of a shop: what a development file is cleaned with is `rm`, which
/// is why the recipe is called `seed-clean` and not `seed-undo`.
fn remove_the_file(db: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    for suffix in ["", "-wal", "-shm"] {
        let path = with_suffix(db, suffix);
        match std::fs::remove_file(&path) {
            Ok(()) => println!("removed {}", path.display()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", path.display()).into()),
        }
    }
    Ok(())
}

/// The path with a suffix on the file name, which is how SQLite names the
/// write ahead log and the shared memory file beside a database.
fn with_suffix(path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    if suffix.is_empty() {
        return path.to_path_buf();
    }
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    std::path::PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::{admit, Refusal, Store};
    use std::path::Path;

    fn dev_file() -> &'static Path {
        Path::new(".dev/dev.db")
    }

    #[test]
    fn a_box_that_did_not_say_it_is_for_development_is_refused_whatever_the_path_says() {
        assert_eq!(
            admit(dev_file(), None, false, None),
            Err(Refusal::NotADevBox)
        );
        assert_eq!(
            admit(dev_file(), Some("0"), false, None),
            Err(Refusal::NotADevBox)
        );
        assert_eq!(
            admit(dev_file(), Some("yes"), false, None),
            Err(Refusal::NotADevBox)
        );
        // And --force does not buy it: a shop's PC is not a development box
        // because somebody passed a flag.
        assert_eq!(
            admit(dev_file(), None, true, None),
            Err(Refusal::NotADevBox)
        );
    }

    #[test]
    fn a_file_outside_the_dev_directory_is_refused_and_force_does_not_lift_it() {
        for path in [
            "shop.db",
            "/home/samir/shop.db",
            "backups/.dev.db",
            ".dev/nested/dev.db",
            "/var/lib/dzpos/.development/dev.db",
        ] {
            assert_eq!(
                admit(Path::new(path), Some("1"), true, None),
                Err(Refusal::NotUnderDevDir),
                "{path}"
            );
        }
        // Relative and absolute alike, as long as the file sits directly in a
        // directory called `.dev`.
        assert!(admit(dev_file(), Some("1"), false, None).is_ok());
        assert!(admit(
            Path::new("/home/samir/dz-pos/.dev/other.db"),
            Some("1"),
            false,
            None
        )
        .is_ok());
    }

    #[test]
    fn a_file_carrying_a_real_shops_identifiers_is_refused_until_somebody_says_otherwise() {
        let real = Some(Store {
            name: "Alimentation Générale Boukhalfa",
            has_identifiers: true,
        });
        assert_eq!(
            admit(dev_file(), Some("1"), false, real),
            Err(Refusal::LooksLikeARealShop)
        );
        // This is the one thing --force is for.
        assert!(admit(dev_file(), Some("1"), true, real).is_ok());
    }

    #[test]
    fn the_seeders_own_file_and_a_fresh_one_are_both_let_through() {
        // What the seeder wrote last time: identifiers, but its own invented
        // shop, so re-seeding it needs no flag.
        assert!(admit(
            dev_file(),
            Some("1"),
            false,
            Some(Store {
                name: dzpos_core::services::seed::SHOP_NAME,
                has_identifiers: true,
            })
        )
        .is_ok());
        // And a file the first migration made and nobody filled in.
        assert!(admit(
            dev_file(),
            Some("1"),
            false,
            Some(Store {
                name: "Mon magasin",
                has_identifiers: false,
            })
        )
        .is_ok());
    }
}
