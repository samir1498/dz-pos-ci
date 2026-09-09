//! Standalone API server. The desktop starts the same routes in-process;
//! this binary is what `just api` runs for the browser preview.

use clap::Parser;

#[derive(Parser)]
#[command(
    name = "dzpos-api",
    about = "dz-pos HTTP API over one shop's SQLite file"
)]
struct Args {
    /// Path to the shop's SQLite file. Created and migrated if missing.
    #[arg(long)]
    db: std::path::PathBuf,
    /// Loopback port. 0 picks a free one and prints it.
    #[arg(long, default_value_t = 4317)]
    port: u16,
    /// The one shop this server answers for.
    #[arg(long, default_value_t = 1)]
    shop: i32,
    /// One more browser origin cleared to call this server, on top of the
    /// app's own. The UI served from this box and opened on another machine
    /// needs it: `--allow-origin http://100.111.55.62:5173`.
    #[arg(long)]
    allow_origin: Option<String>,
}

/// The launch token every caller must show (`Authorization: Bearer`). Read
/// from the environment, never from a flag, so `ps` does not show it.
/// Absent: a random one is made and printed once on stdout.
const TOKEN_VAR: &str = "DZPOS_API_TOKEN";

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let args = Args::parse();
    match run(args).await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("dzpos-api failed: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = args.db.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    // Refused here, before anything binds, so a bad flag is one line on
    // stderr and a non-zero exit rather than a panic behind a live port.
    let extra_origin = args
        .allow_origin
        .as_deref()
        .map(dzpos_api::origin_from_flag)
        .transpose()
        .map_err(|why| format!("--allow-origin {why}"))?;
    let (token, made_here) = match std::env::var(TOKEN_VAR) {
        Ok(secret) => (
            dzpos_api::LaunchToken::from_secret(&secret)
                .map_err(|why| format!("{TOKEN_VAR}: {why}"))?,
            false,
        ),
        Err(_) => (
            dzpos_api::LaunchToken::generate().map_err(|e| format!("no randomness: {e}"))?,
            true,
        ),
    };
    let state = dzpos_api::AppState::open(&args.db, args.shop)?;
    let (listener, port) = dzpos_api::bind(args.port).await?;
    if made_here {
        // The operator's own terminal is the only place it goes; the UI
        // needs it as VITE_API_TOKEN (`just api` writes it to .dev instead).
        println!("dzpos-api launch token {}", token.expose());
    }
    // The e2e harness waits on this line to know the port is live.
    println!("dzpos-api listening on http://127.0.0.1:{port}");
    axum::serve(
        listener,
        dzpos_api::router_with_origin(state, &token, extra_origin),
    )
    .await?;
    Ok(())
}
