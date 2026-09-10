//! Fills a development shop file with a catalogue and a month of trading.
//!
//! Why a binary of its own and not a `--seed` flag on the API. The API holds
//! the file open for as long as it serves, and `--force` here deletes that
//! file and migrates a new one: a flag would mean the server wiping the
//! database out from under its own connection, and a `just seed` that had to
//! start a server to fill a file it then wants nobody holding. A binary
//! opens the file, writes, closes and exits, which is what the recipe wants
//! and what a developer can run twice in a row.
//!
//! What it decides: nothing. Every rule is `dzpos_core::services::seed`,
//! which drives the same services the till does. This file parses flags,
//! opens the database, and prints what was written.

use clap::Parser;
use dzpos_core::services::seed;

#[derive(Parser)]
#[command(
    name = "dzpos-seed",
    about = "fill a dz-pos shop file with a development catalogue and a month of trading"
)]
struct Args {
    /// Path to the shop's SQLite file. Created and migrated if missing.
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
    /// Delete the file and start over. Without it a shop that already holds
    /// rows is refused, because a second seeding would sit beside the first.
    #[arg(long, default_value_t = false)]
    force: bool,
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
    let today = match args.today.as_deref() {
        Some(text) => chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map_err(|_| "--today is a day written YYYY-MM-DD")?,
        None => dzpos_core::services::clock::now().date(),
    };
    if args.force {
        // The file and the two WAL companions beside it. Deleting the file
        // and leaving its -wal behind gives SQLite a journal for a database
        // that no longer exists.
        for suffix in ["", "-wal", "-shm"] {
            let path = with_suffix(&args.db, suffix);
            match std::fs::remove_file(&path) {
                Ok(()) => println!("removed {}", path.display()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(format!("{}: {e}", path.display()).into()),
            }
        }
    }
    if let Some(parent) = args.db.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut conn = dzpos_core::db::open(&args.db)?;
    if !seed::is_empty(&mut conn, args.shop)? {
        return Err(format!(
            "{} already holds rows; pass --force to delete it and start over",
            args.db.display()
        )
        .into());
    }
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
