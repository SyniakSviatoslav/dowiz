//! THE ROUTES THAT STILL PULLED EVERY ORDER INTO THE WORKER, each now run WHOLE by the venue's
//! object in ONE request (W-OWNER-CPU2; the pattern is W-BN4B's, `hubdo/command.rs`).
//!
//! WHAT IT REPLACES. W-OWNER-CPU (c6b30337) relayed the owner console's big reads but left a
//! list of routes whose handlers fetched the venue's whole order log (1.76 MB on dubin-durres),
//! its people image or its till image across the hop and folded them in the stateless Worker --
//! the Free plan's 10 ms of CPU, which `exceededCpu` then charged to the NEXT request too (the
//! hardened shim refuses while the isolate reinitialises). The browser flows gate F4 failed on
//! qa-durres with `refund <order> from READY: 503 <!DOCTYPE html>` -- the refund route loaded
//! the identity image, then the idempotency table twice (guard, then done: load, crc walk,
//! serialise, write back), then parsed the merged order, all in the Worker.
//!
//! WHAT IT IS. Nothing in a handler changes: the Worker picks the object (`Place::of_any`:
//! the signed token's claim, else the Host; or the slug for a public slug route, as
//! `storefront::place` does) and hands it the whole request; the object runs the handler with
//! an `Env` that answers its own name in process, so every `load_*`, every `with_table`, every
//! `idempotency::guard` becomes a function call in that turn. The handler still authorises the
//! venue it names (a body naming another venue is acted on through a real stub, as before) and
//! its authorisation is now the verdict's few bytes (`auth/verdict.rs`). The answer is relayed:
//! its body never enters the Worker's wasm (`Stub::relay`).
//!
//! `routed(kind)` is what a `lib.rs` route line holds: the handler file keeps its name and body.

use super::{forward, send, Kind};
use crate::edge::Ctx;
use crate::wire::{Call, Reply};
use std::{future::Future, pin::Pin};
use worker::Result;

/// A turn, boxed.
pub(crate) type Turn = Pin<Box<dyn Future<Output = Result<Reply>>>>;

/// The route line's handler: the request forwarded to the venue's own object as `kind`.
pub(crate) fn routed(kind: Kind) -> impl FnOnce(Call, Ctx<crate::Req>) -> Turn {
    move |req, ctx| Box::pin(forward(kind, req, ctx))
}

/// The same for a public `/:slug/` route: the slug names the venue (`Place::of_slug`), as the
/// placement's does; the Host and a token are not consulted.
pub(crate) fn routed_slug(kind: Kind) -> impl FnOnce(Call, Ctx<crate::Req>) -> Turn {
    move |req, ctx| {
        Box::pin(async move {
            let Some(slug) = ctx.param("slug").cloned() else {
                return Reply::error("missing slug", 400);
            };
            let place = crate::hubstore::Place::of_slug(&ctx, &slug).await?;
            send(&place, kind, &ctx, req).await
        })
    }
}

/// The path parameters each of these kinds' handlers reads; `None` for any other kind.
pub(crate) fn params(kind: Kind) -> Option<&'static [&'static str]> {
    use Kind::*;
    Some(match kind {
        Refund | Returned | GuestConfirm | Sitting | CourierDetail | CampaignReport | CampaignPreview | CampaignSend | OptionBomRead | OptionBomWrite => &["id"],
        CustomerReveal | CustomerForget | CustomerRecord => &["key"],
        Welcome => &["slug"],
        ThreadMessages | ThreadSend => &["slug", "id"],
        StaffRoom | StaffFloor | TillTips | KitchenAssist | CourierAssist | Customers | Reveals | CustomerRekey | Reforget | CampaignList | CampaignDefine | Inbox
        | WalletLegs | WalletLegsRepair | Posts | DraftPost | Graph | OwnerAssist | AnalyticsKitchen | Exceptions | Health | BagCard | BagSet | BagQr | PromoCheck | Voice => &[],
        _ => return None,
    })
}

/// THE OBJECT'S HALF (`hubdo/command.rs`): the handler body for one of these kinds, built in
/// a frame of its own (see `command_route` on why); any other kind is handed back untouched.
#[inline(never)]
pub(crate) fn in_object(kind: Kind, call: Call, ctx: Ctx<crate::Req>) -> std::result::Result<Turn, (Call, Ctx<crate::Req>)> {
    use crate::services::{campaigns, courier, customers, engagement, loyalty, orders};
    use crate::services::orders::room;
    Ok(match kind {
        // ── the room and the till ──
        Kind::Refund => Box::pin(orders::refund::refund(call, ctx)),
        Kind::Returned => Box::pin(orders::refund::returned(call, ctx)),
        Kind::GuestConfirm => Box::pin(room::guest_round::confirm(call, ctx)),
        Kind::Sitting => Box::pin(room::guest_round::sitting_bill(call, ctx)),
        Kind::StaffRoom => Box::pin(room::handlers::room_view(call, ctx)),
        Kind::StaffFloor => Box::pin(room::floor::get(call, ctx)),
        Kind::TillTips => Box::pin(room::till::tips(call, ctx)),
        // ── the owner's people ──
        Kind::Customers => Box::pin(customers::handlers::customers(call, ctx)),
        Kind::CustomerReveal => Box::pin(customers::handlers::reveal_customer(call, ctx)),
        Kind::Reveals => Box::pin(customers::handlers::reveals(call, ctx)),
        Kind::CustomerForget => Box::pin(customers::forget::forget_customer(call, ctx)),
        Kind::CustomerRecord => Box::pin(customers::record_routes::put_record(call, ctx)),
        Kind::CustomerRekey => Box::pin(customers::record_routes::rekey(call, ctx)),
        Kind::Reforget => Box::pin(customers::forget::run::reforget(call, ctx)),
        Kind::CourierDetail => Box::pin(courier::console::courier_detail(call, ctx)),
        Kind::CampaignList => Box::pin(campaigns::handlers::list(call, ctx)),
        Kind::CampaignDefine => Box::pin(campaigns::handlers::define(call, ctx)),
        Kind::CampaignReport => Box::pin(campaigns::handlers::report(call, ctx)),
        Kind::CampaignPreview => Box::pin(campaigns::handlers::preview(call, ctx)),
        Kind::CampaignSend => Box::pin(campaigns::handlers::send_now(call, ctx)),
        Kind::Inbox => Box::pin(crate::social::inbox::list(call, ctx)),
        Kind::WalletLegs => Box::pin(orders::legs::audit(call, ctx)),
        Kind::WalletLegsRepair => Box::pin(orders::legs::repair(call, ctx)),
        // ── the owner's numbers and words ──
        Kind::Posts => Box::pin(engagement::posts::posts(call, ctx)),
        Kind::DraftPost => Box::pin(engagement::posts::draft_post(call, ctx)),
        Kind::Graph => Box::pin(engagement::assist::graph(call, ctx)),
        Kind::OwnerAssist => Box::pin(engagement::assist::owner_assist(call, ctx)),
        Kind::KitchenAssist => Box::pin(engagement::assist::kitchen::kitchen_assist(call, ctx)),
        Kind::CourierAssist => Box::pin(engagement::assist::courier_assist(call, ctx)),
        Kind::AnalyticsKitchen => Box::pin(crate::services::analytics::kitchen::kitchen(call, ctx)),
        Kind::Exceptions => Box::pin(crate::exceptions::exceptions(call, ctx)),
        Kind::Health => Box::pin(crate::services::operations::health(call, ctx)),
        Kind::OptionBomRead => Box::pin(crate::services::catalogue::option_bom::read(call, ctx)),
        Kind::OptionBomWrite => Box::pin(crate::services::catalogue::option_bom::write(call, ctx)),
        Kind::BagCard => Box::pin(loyalty::bag_routes::card(call, ctx)),
        Kind::BagSet => Box::pin(loyalty::bag_routes::set(call, ctx)),
        Kind::BagQr => Box::pin(loyalty::bag_routes::qr(call, ctx)),
        // ── the public ones ──
        Kind::PromoCheck => Box::pin(crate::services::ordering::preview::promo_check(call, ctx)),
        Kind::Voice => Box::pin(engagement::voice::voice(call, ctx)),
        Kind::Welcome => Box::pin(loyalty::bag_routes::public(call, ctx)),
        Kind::ThreadMessages => Box::pin(crate::social::messages(call, ctx)),
        Kind::ThreadSend => Box::pin(crate::social::send(call, ctx)),
        _ => return Err((call, ctx)),
    })
}

#[cfg(test)]
mod tests;
