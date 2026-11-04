//! THE SIGN-IN DOORS, each run WHOLE by the PLATFORM object in ONE request (W-LOGIN-DO,
//! operator 2026-11-04: stay on the Workers Free plan and make sign-in reliable).
//!
//! WHAT IT REPLACED, MEASURED (wrangler tail, 2026-11-04, after 4bdaeed6): `POST /api/auth/login`
//! cost 48-62 ms of CPU in the STATELESS Worker against the Free plan's 10 ms per request --
//! argon2id (m=8 MiB, t=3) is 25-30 ms of it, the rest is the sessions image loaded,
//! crc-checked, changed and written back across the hop -- and about 40 % of logins were killed
//! (error 1102) or refused after the isolate's wasm reinit (1101). The same tail showed the
//! venue objects' alarm turns running 28-355 ms of CPU with outcome `ok`: a Durable Object's
//! budget is 30 s per request on every plan (developers.cloudflare.com/durable-objects/platform/
//! limits, read 2026-11-04: "CPU per request: 30 seconds by default"; the Free plan's 10 ms is
//! the Workers limits page's "CPU time per HTTP request", and the same page says "Move expensive
//! computation to Durable Objects").
//!
//! WHAT IT IS. The pattern is W-BN4B's (`hubdo/command.rs`): the Worker hands the platform object
//! the whole request and the object runs the very handler the Worker ran (`*_at`, unchanged)
//! with an `Env` that answers `__platform` in process (`edge::Own`). Every `identity_store` read
//! and write inside the handler becomes a function call in that turn; argon2 runs there; the
//! token is signed there with the same secret. The stateless Worker copies a body and encodes
//! one JSON. Nothing about the doors changed: the same argon2 parameters and stored hashes, the
//! same token formats and lifetimes, the same closed bodies (`crate::body`), the same refusals.
//!
//! WHAT IT COSTS. One object is one writer, so every sign-in on the platform is serialised
//! through `__platform` -- which `platform_store`'s header already names, and which every
//! `with_sessions` write already was. A courier's login still writes the venue's audit log
//! through that venue's own object: a real stub from inside the platform object, as before.
//!
//! THE MEMO (`auth/memo.rs`). A revocation a door writes (a password change ends the person's
//! sessions) now lands in the object's isolate; a Worker isolate that memoised the token learns
//! of it within `memo::TTL_MS` (60 s), the window documented for a write by another isolate.

use crate::edge::Ctx;
use crate::hubdo::command::Kind;
use crate::wire::{Call, Reply};
use std::{future::Future, pin::Pin};
use worker::Result;

/// THE WORKER'S HALF: the whole request to the PLATFORM object, which owns the identity,
/// sessions and couriers images. What the Worker spends is a body copy and one JSON encode.
pub(crate) async fn forward_platform(kind: Kind, req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    use crate::platform_store::{stub, PLATFORM};
    crate::hubdo::command::send_to(&stub(&ctx.env)?, PLATFORM, kind, &ctx, req).await
}

/// Does `kind` run in the platform object? A sign-in names no venue's object: its images are the
/// platform's, and a venue's object running it would reach them over a hop, from inside another
/// tenant's object; a venue's command never runs in the platform's (`command_route` refuses both).
pub(crate) fn at_platform(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::OwnerLogin | Kind::OwnerRefresh | Kind::OwnerLogout | Kind::CourierLogin | Kind::CourierClaim | Kind::CourierRefresh
            | Kind::StaffLogin | Kind::StaffClaim | Kind::StaffPassword | Kind::StaffRefresh | Kind::StaffReset
    )
}

/// A door's turn, boxed.
pub(crate) type Turn = Pin<Box<dyn Future<Output = Result<Reply>>>>;

/// THE OBJECT'S HALF (`hubdo/command.rs`): the handler body for a sign-in `Kind`, built here and
/// NOT in `command_route`'s own frame; any other kind is handed back untouched.
#[inline(never)]
pub(crate) fn in_object(kind: Kind, call: Call, ctx: Ctx<crate::Req>) -> std::result::Result<Turn, (Call, Ctx<crate::Req>)> {
    use crate::accounts::{self, session_renew};
    use crate::services::identity::staff;
    Ok(match kind {
        Kind::OwnerLogin => Box::pin(accounts::owner_login_at(call, ctx)),
        Kind::OwnerRefresh => Box::pin(accounts::owner_refresh_at(call, ctx)),
        Kind::OwnerLogout => Box::pin(accounts::owner_logout_at(call, ctx)),
        Kind::CourierLogin => Box::pin(accounts::courier_login_at(call, ctx)),
        Kind::CourierClaim => Box::pin(accounts::courier_claim_at(call, ctx)),
        Kind::CourierRefresh => Box::pin(session_renew::courier_refresh_at(call, ctx)),
        Kind::StaffLogin => Box::pin(staff::staff_login_at(call, ctx)),
        Kind::StaffClaim => Box::pin(staff::staff_claim_at(call, ctx)),
        Kind::StaffPassword => Box::pin(staff::staff_password_at(call, ctx)),
        Kind::StaffRefresh => Box::pin(session_renew::staff_refresh_at(call, ctx)),
        Kind::StaffReset => Box::pin(staff::password::owner_reset_at(call, ctx)),
        _ => return Err((call, ctx)),
    })
}

#[cfg(test)]
mod tests;
