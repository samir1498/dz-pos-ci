//! Signing in, signing out, and who is signed in (M4 T2).
//!
//! The three routes that sit inside the launch-token guard and outside the
//! session one, because they are how a session comes to exist. Everything
//! else on this API is behind both.
//!
//! Nothing here decides anything about a credential: `services::users` owns
//! the single refusal, the wrong-try counter and the lockout, and
//! `services::sessions` owns the token and the idle time. These handlers
//! translate, the way every other handler in this folder does.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use dzpos_core::services::permissions::{self, Permission};
use dzpos_core::services::sessions::SignedIn;
use dzpos_core::services::{preferences, sessions, users};

use crate::dto::{ClaimFirstOwnerDto, LoginDto, MeDto, PermissionDto, RoleDto, SessionDto};
use crate::error::ApiError;
use crate::session::{self, CurrentUser};
use crate::AppState;

/// A user id and a PIN, or a name and a password. The answer carries the
/// session token in the body and sets it as an httpOnly cookie; which of the
/// two a client uses is the client's business (`crate::session` says why
/// there are two).
///
/// Every refusal this can give is `services::users`': one `auth_refused` for
/// a wrong PIN, a wrong password, a name nobody answers to, a user id nobody
/// answers to and a deactivated fiche, and `locked_out` with the wait when
/// the counter has been crossed. Nothing here tells a caller which.
pub async fn login(
    State(state): State<AppState>,
    body: Result<Json<LoginDto>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(dto) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let at = session::now();
    let (signed_in, idle) = state
        .blocking(move |c| {
            let signed_in = match dto {
                LoginDto::Pin { user_id, pin } => {
                    sessions::sign_in_with_pin(c, shop, user_id, &pin, at)?
                }
                LoginDto::Password { name, password } => {
                    sessions::sign_in_with_password(c, shop, &name, &password, at)?
                }
            };
            let idle = preferences::session_idle(c, shop)?.num_minutes();
            Ok((signed_in, idle))
        })
        .await?;

    Ok(session_response(signed_in, idle))
}

/// The one door into a shop nobody has ever signed into.
/// `services::users::claim_first_owner` is the whole rule: it acts on the
/// shop's own owner rather than an id the caller names, and it refuses the
/// moment any credential anywhere in the shop already exists.
///
/// Answers the same `SessionDto` as `login`, so the owner who just claimed
/// the shop is inside it and not sent back to a sign-in screen.
pub async fn claim_first_owner(
    State(state): State<AppState>,
    body: Result<Json<ClaimFirstOwnerDto>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(ClaimFirstOwnerDto { name, password }) = body.map_err(ApiError::from)?;
    let shop = state.shop_id;
    let at = session::now();
    let (signed_in, idle) = state
        .blocking(move |c| {
            let owner = users::claim_first_owner(c, shop, &name, &password)?;
            let signed_in = sessions::sign_in_with_password(c, shop, &owner.name, &password, at)?;
            let idle = preferences::session_idle(c, shop)?.num_minutes();
            Ok((signed_in, idle))
        })
        .await?;
    Ok(session_response(signed_in, idle))
}

/// What a sign-in answers with, whichever of the two doors it came through:
/// who is now acting, the token in the body for the desktop webview, and the
/// same token as an httpOnly cookie for a browser.
fn session_response(signed_in: SignedIn, idle: i64) -> Response {
    let body = SessionDto {
        me: MeDto {
            user_id: signed_in.actor.user_id,
            name: signed_in.name,
            role: RoleDto::from(signed_in.actor.role),
            permissions: held_by(signed_in.actor.role),
        },
        token: signed_in.token.expose().to_owned(),
        idle_minutes: idle,
    };
    let mut res = Json(body).into_response();
    if let Some(cookie) = session::set_cookie(signed_in.token.expose(), idle) {
        res.headers_mut().append(header::SET_COOKIE, cookie);
    }
    res
}

/// Ends the session this request carries and clears the cookie.
///
/// 204 whether or not there was one to end. A caller signing out has nothing
/// to do differently on "you were not signed in", and answering it would be a
/// way to ask this server whether a token is live without using it.
pub async fn logout(State(state): State<AppState>, req: Request) -> Result<Response, ApiError> {
    if let Some(token) = session::token_of(&req) {
        let shop = state.shop_id;
        let at = session::now();
        state
            .blocking(move |c| sessions::sign_out(c, shop, &token, at))
            .await?;
    }
    let mut res = StatusCode::NO_CONTENT.into_response();
    res.headers_mut()
        .append(header::SET_COOKIE, session::clear_cookie());
    Ok(res)
}

/// Who is signed in, and what their role may do.
///
/// Outside the session middleware on purpose, so it can answer 401
/// `session_required` for a screen asking "am I still signed in?" without
/// that answer arriving through a guard. Reading it does not slide the idle
/// time forward either (`services::sessions::describe`): a screen polling
/// this would otherwise be a till that never locks.
pub async fn me(State(state): State<AppState>, req: Request) -> Result<Json<MeDto>, ApiError> {
    let token = session::token_of(&req).ok_or(ApiError::SessionRequired)?;
    let shop = state.shop_id;
    let at = session::now();
    let (actor, name) = state
        .blocking(move |c| sessions::describe(c, shop, &token, at))
        .await?
        .ok_or(ApiError::SessionRequired)?;
    Ok(Json(MeDto {
        user_id: actor.user_id,
        name,
        role: RoleDto::from(actor.role),
        permissions: held_by(actor.role),
    }))
}

/// How long a session survives with nothing happening on it, so the desktop's
/// lock screen counts against the shop's figure rather than one of its own.
/// Behind the session guard: it is not something to hand an unauthenticated
/// caller, and every signed-in role may read it.
pub async fn idle(
    State(state): State<AppState>,
    _who: CurrentUser,
) -> Result<Json<crate::dto::SessionIdleDto>, ApiError> {
    let shop = state.shop_id;
    let minutes = state
        .blocking(move |c| preferences::session_idle(c, shop).map(|d| d.num_minutes()))
        .await?;
    Ok(Json(crate::dto::SessionIdleDto {
        idle_minutes: minutes,
    }))
}

/// Every permission this role holds, walked off `Permission::ALL` and
/// answered by `can`. Walked rather than listed, so a fourteenth permission
/// reaches the screens without anybody editing a second list
/// (architecture.md rule 2).
fn held_by(role: dzpos_core::services::permissions::Role) -> Vec<PermissionDto> {
    Permission::ALL
        .into_iter()
        .filter(|p| permissions::can(role, *p))
        .map(PermissionDto::from)
        .collect()
}
