//! Every route the server answers, in one table, and the CORS and body
//! limits around it.
//!
//! Apart from `lib.rs` because that file is about the shop file: opening it,
//! upgrading it, backing it up and putting it back. This one is about the
//! surface, and it is the file a new endpoint is added to. `gates/` holds
//! the permission each route needs and `tests/route_gates.rs` walks the two
//! against each other in both directions, so a route added here without a
//! gate fails closed rather than quietly open.

#[cfg(feature = "retail")]
use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue, Method};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post, put};
use axum::Router;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::token::{self, LaunchToken};
use crate::{device, routes, session, AppState};

/// The origins the product's own screens are served from: the browser
/// preview on 5173 and the two the Tauri webview uses.
const OWN_ORIGINS: [HeaderValue; 4] = [
    HeaderValue::from_static("http://127.0.0.1:5173"),
    HeaderValue::from_static("http://localhost:5173"),
    HeaderValue::from_static("tauri://localhost"),
    HeaderValue::from_static("http://tauri.localhost"),
];

pub fn router(state: AppState, token: &LaunchToken) -> Router {
    router_with_origin(state, token, None)
}

/// Same routes, plus one more origin the operator names. The UI served from
/// the WSL box and opened on the laptop is a real case and it is a decision,
/// never a default.
/// Turns the `--allow-origin` flag into a header value the CORS list will
/// take. Checked before the server binds: tower-http panics on `*` inside
/// `AllowOrigin::list`, and it did so after the listening line was printed,
/// so a harness waiting on that line hung on a dead server. `null` would be
/// echoed back to any sandboxed page. A trailing slash or a path never
/// matches what a browser sends, so it is refused rather than silently
/// never matching.
pub fn origin_from_flag(flag: &str) -> Result<HeaderValue, String> {
    let uri: axum::http::Uri = flag.parse().map_err(|_| format!("{flag}: not a URL"))?;
    let scheme = uri.scheme_str().unwrap_or_default();
    if !["http", "https", "tauri"].contains(&scheme) {
        return Err(format!("{flag}: the scheme must be http, https or tauri"));
    }
    if uri.host().is_none_or(str::is_empty) {
        return Err(format!("{flag}: no host"));
    }
    if uri
        .path_and_query()
        .is_some_and(|p| p.as_str() != "" && p.as_str() != "/")
        || flag.ends_with('/')
    {
        return Err(format!(
            "{flag}: an origin is scheme://host[:port], no path and no trailing slash"
        ));
    }
    HeaderValue::from_str(flag).map_err(|_| format!("{flag}: not a valid header value"))
}

pub fn router_with_origin(
    state: AppState,
    token: &LaunchToken,
    extra: Option<HeaderValue>,
) -> Router {
    let mut origins = OWN_ORIGINS.to_vec();
    origins.extend(extra);

    // Who may call (docs/architecture.md, "Transport and auth"): the process
    // that holds the launch token. Loopback is not a boundary, any process
    // or page on the machine can open 127.0.0.1, so the token is what says
    // "this is the desktop's own screen". Only /health answers without it.
    // The health route owes the same JSON 405 as every guarded one; the
    // method fallback below is the one on the router inside the guard.
    let open = Router::new().route(
        "/health",
        get(routes::health).fallback(routes::method_not_allowed),
    );
    // Which person is calling: the session (M4 T2, crate::session). Inside
    // the launch token and outside the session guard, because this is how a
    // device token comes to exist; a claim behind a session guard would be a
    // lock with its key inside. It still shows the launch token like
    // everything else.
    let auth = Router::new().route("/pairing/claim", post(routes::pairing::claim));
    // Signing in as a person, behind the device gate but outside the
    // session one (M7 T3): a sign-in behind a session guard would be a lock
    // with its key inside, but a LAN holder of only the launch token mints
    // no sessions and reads no me — the phone shows its pairing first, then
    // the person's PIN or password. `/auth/logout` and `/auth/me` read the
    // session token by hand rather than taking `CurrentUser`, and still
    // answer without one; the device layer, not the session one, is what
    // guards them. The desktop on loopback passes bare, exactly as before.
    let phone_auth = Router::new()
        // First setup is behind the device gate too (M6+M7 review,
        // 2026-09-16): claiming the owner of a shop that has none is not
        // how a device token comes to exist, so it has no reason to sit
        // outside. The desktop on loopback passes the gate bare, exactly as
        // before; a LAN holder of only the launch token no longer claims a
        // fresh shop's owner without pairing first.
        .route("/auth/first-setup", post(routes::auth::claim_first_owner))
        .route("/auth/login", post(routes::auth::login))
        // The names a sign-in screen offers before anyone is signed in, so a
        // cashier taps one and types only a PIN. Same layer as `login` for
        // the same reason: it is read by a caller who has no session yet,
        // and the device gate is what says the caller is the shop's own
        // phone. What it hands out is `StaffDto` — names and which door
        // opens, never a hash or a failure counter.
        .route("/auth/staff", get(routes::auth::staff))
        .route("/auth/logout", post(routes::auth::logout))
        .route("/auth/me", get(routes::auth::me))
        .layer(from_fn_with_state(state.clone(), device::require));
    // The kernel routes: every one of them stands with the feature off, and
    // `just gates`'s `route_gates.rs` reads this whole file as text
    // (`include_str!`), cfg lines included, so a row's own gate never moves
    // out of step with which half of this chain wrote its route.
    let guarded = Router::new()
        .route("/audit-log", get(routes::audit::list))
        .route("/auth/idle", get(routes::auth::idle))
        .route("/backups", get(routes::backups::list))
        .route("/backups", post(routes::backups::create))
        .route("/backups/{name}/restore", post(routes::backups::restore))
        .route("/build-info", get(routes::build_info))
        .route("/clock", get(routes::clock))
        .route("/settings", get(routes::settings::read))
        .route("/settings/store", put(routes::settings::update_store))
        .route("/settings/regime", post(routes::settings::change_regime))
        .route("/settings/theme", put(routes::settings::set_theme))
        .route(
            "/settings/facture-layout",
            put(routes::settings::set_facture_layout),
        )
        .route(
            "/settings/print-lang",
            put(routes::settings::set_print_lang),
        )
        .route(
            "/settings/thermal-mode",
            put(routes::settings::set_thermal_mode),
        )
        .route("/support-bundle", get(routes::support::bundle))
        .route(
            "/users",
            get(routes::users::list).post(routes::users::create),
        )
        .route("/pairing/qr", post(routes::pairing::create_qr))
        .route("/pairing/devices", get(routes::pairing::list_devices))
        .route(
            "/pairing/devices/{id}/revoke",
            post(routes::pairing::revoke_device),
        )
        .route("/users/{id}/pin", post(routes::users::set_pin))
        .route("/users/{id}/password", post(routes::users::set_password))
        .route("/users/{id}/deactivate", post(routes::users::deactivate))
        .route("/users/{id}/reactivate", post(routes::users::reactivate));

    // C3 of `the-first-clinic-module-patients-queue-appointments`: the
    // clinic's own routes, gated whole on their own feature for the same
    // reason the shop's below are, and before them so each block's span in
    // `route_gates.rs` is bounded by the next block's start. 8 of the calls
    // below, eleven method routes between them: the patient file's 3 calls
    // (five routes) and the waiting queue's 5 (six, C4).
    #[cfg(feature = "clinic")]
    let guarded = guarded
        .route(
            "/patients",
            get(routes::patients::list).post(routes::patients::create),
        )
        .route(
            "/patients/{id}",
            get(routes::patients::get_one).put(routes::patients::update),
        )
        .route("/patients/{id}/archive", post(routes::patients::archive))
        .route("/queue", get(routes::queue::today).post(routes::queue::add))
        .route("/queue/next", post(routes::queue::call_next))
        .route("/queue/{id}/call", post(routes::queue::call))
        .route("/queue/{id}/seen", post(routes::queue::seen))
        .route("/queue/{id}/left", post(routes::queue::left));

    // S5 of `a-kernel-crate-and-retail-as-the-first-module`: the shop's own
    // routes, gated whole rather than split across a second router, because
    // `route_gates.rs`'s source walk reads this file and would never see
    // one added anywhere else. 59 of the calls below.
    #[cfg(feature = "retail")]
    let guarded = guarded
        .route("/cash", get(routes::expenses::cash))
        .route("/categories", get(routes::categories::list))
        .route("/customers", get(routes::customers::list))
        .route("/customers", post(routes::customers::create))
        .route(
            "/customers/{id}",
            get(routes::customers::get_one).put(routes::customers::update),
        )
        .route("/customers/{id}/ledger", get(routes::customers::ledger))
        .route("/customers/{id}/payments", get(routes::customers::payments))
        .route("/customers/{id}/payments", post(routes::customers::pay))
        .route(
            "/customers/{id}/statement",
            get(routes::customers::statement),
        )
        .route(
            "/customers/{id}/debt-slip",
            get(routes::customers::debt_slip),
        )
        .route(
            "/customers/{id}/adjustments",
            post(routes::customers::adjust),
        )
        .route("/dashboard", get(routes::dashboard::read))
        .route("/dashboard/series", get(routes::dashboard::series))
        .route("/expense-categories", get(routes::expenses::categories))
        .route("/expenses", get(routes::expenses::list))
        .route("/expenses", post(routes::expenses::create))
        .route("/export/products", get(routes::export::products))
        .route("/export/sales", get(routes::export::sales))
        .route("/export/customers", get(routes::export::customers))
        .route("/export/suppliers", get(routes::export::suppliers))
        .route("/import/products/template", get(routes::import::template))
        .route(
            "/import/products/dry-run",
            post(routes::import::dry_run)
                .layer(DefaultBodyLimit::max(routes::import::IMPORT_BODY_LIMIT)),
        )
        .route(
            "/import/products",
            post(routes::import::apply)
                .layer(DefaultBodyLimit::max(routes::import::IMPORT_BODY_LIMIT)),
        )
        .route("/labels/sheet", post(routes::products::label_sheet))
        .route("/products", get(routes::products::list))
        .route("/products", post(routes::products::create))
        .route("/products/{id}/label", get(routes::products::label))
        .route(
            "/products/{id}",
            get(routes::products::get_one).put(routes::products::update),
        )
        .route("/purchases", get(routes::purchases::list))
        .route("/purchases", post(routes::purchases::create))
        .route("/purchases/{id}", get(routes::purchases::get_one))
        .route("/purchases/{id}/receipts", post(routes::purchases::receive))
        .route("/purchases/{id}/returns", post(routes::purchases::returns))
        .route("/purchases/{id}/cancel", post(routes::purchases::cancel))
        .route(
            "/purchases/{id}/close-short",
            post(routes::purchases::close_short),
        )
        .route("/sales", get(routes::sales::list))
        .route("/sales", post(routes::sales::create))
        .route("/sales/{id}", get(routes::sales::get_one))
        .route("/sales/{id}/avoir", post(routes::sales::avoir))
        .route("/sales/{id}/avoirs", get(routes::sales::avoirs))
        .route("/sales/{id}/cancel", post(routes::sales::cancel))
        .route("/sales/{id}/ticket", get(routes::sales::ticket))
        .route(
            "/sales/{id}/ticket/escpos",
            get(routes::sales::ticket_escpos),
        )
        .route("/sales/{id}/print", post(routes::sales::print_ticket))
        .route("/sales/{id}/facture", get(routes::sales::facture))
        .route(
            "/sales/{id}/facture/escpos",
            get(routes::sales::facture_escpos),
        )
        .route(
            "/settings/discount-threshold",
            post(routes::settings::set_discount_threshold),
        )
        .route(
            "/stock/recount",
            get(routes::stock::last).post(routes::stock::recount),
        )
        .route("/suppliers", get(routes::suppliers::list))
        .route("/suppliers", post(routes::suppliers::create))
        .route(
            "/suppliers/{id}",
            get(routes::suppliers::get_one).put(routes::suppliers::update),
        )
        .route("/suppliers/{id}/close", post(routes::suppliers::close))
        .route("/suppliers/{id}/ledger", get(routes::suppliers::ledger))
        .route("/suppliers/{id}/payments", post(routes::suppliers::pay))
        .route(
            "/suppliers/{id}/adjustments",
            post(routes::suppliers::adjust),
        )
        .route(
            "/suppliers/{id}/statement",
            get(routes::suppliers::statement),
        )
        // The static segment before the `{id}` one: matchit prefers a
        // literal to a parameter, so `/till/shifts/open` never reaches
        // `get_one` with "open" where an id should be.
        .route(
            "/till/shifts",
            get(routes::till::list).post(routes::till::open),
        )
        .route("/till/shifts/open", get(routes::till::open_shift))
        .route("/till/shifts/{id}", get(routes::till::get_one))
        .route("/till/shifts/{id}/close", post(routes::till::close));

    let guarded = guarded
        // Every route above takes its actor from the session; nothing reads a
        // seeded owner id any more.
        .layer(from_fn_with_state(state.clone(), session::require))
        // Which phone is asking, before which person (M7 T2): the last
        // layer added runs first, so a revoked phone fails before any
        // session is even looked up, and the desktop on loopback — which
        // shows no device token — passes bare.
        .layer(from_fn_with_state(state.clone(), device::require));

    let inside_the_launch_token = auth
        .merge(phone_auth)
        .merge(guarded)
        .fallback(routes::not_found)
        .method_not_allowed_fallback(routes::method_not_allowed)
        .layer(from_fn_with_state(token.clone(), token::require));

    open.merge(inside_the_launch_token)
        // Every browser on the machine can reach 127.0.0.1, so
        // allow_origin(Any) let any page a user happened to open read and
        // write the till. The list is what keeps a stranger's page from
        // being handed the answer, and the preflight is answered here,
        // before the token check, because a browser sends it bare.
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(origins))
                .allow_methods([Method::GET, Method::POST, Method::PUT])
                .allow_headers([
                    header::CONTENT_TYPE,
                    header::AUTHORIZATION,
                    // The desktop's session token travels in its own header
                    // (crate::session says why it is not Authorization); a
                    // header missing from this list is one the preflight
                    // silently kills.
                    header::HeaderName::from_static(crate::session::SESSION_HEADER),
                    // A paired phone's device token travels in its own too
                    // (M7 T2); same fate at preflight if it is missing here.
                    header::HeaderName::from_static(crate::device::DEVICE_HEADER),
                ])
                // The browser half of the same session sends an httpOnly
                // cookie, and a browser only attaches one cross-origin when
                // the server says it may. The origin list is a fixed list and
                // never `Any`, which is what makes this safe to turn on.
                .allow_credentials(true)
                // A browser hands a page only the few headers it is told to.
                // Without this the desktop can read the workbook's bytes and
                // not the name the server gave it, and every export would be
                // saved as whatever the anchor invented.
                .expose_headers([header::CONTENT_DISPOSITION]),
        )
        .with_state(state)
}
