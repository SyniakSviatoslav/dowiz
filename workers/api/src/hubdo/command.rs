//! ONE COMMAND, ONE OBJECT TURN (BN4 = arch-evo P1, R-GRAPH D3): `POST /fold/command`.
//!
//! WHAT IT REPLACED, MEASURED (`storefront/routes/tests.rs`, the test edge's
//! `object_requests`): a customer's placement cost TEN billed object requests --
//! the registry, then nine to the venue's own object: `/fold/basket`, `/img/people`,
//! `/img/settings`, `/fold/place`, `/img/people` twice, `/img/consent`, `/img/taste`
//! twice. Each was the Worker reading or writing one of the venue's images over the
//! hop, around the one turn (`/fold/place`) that decides.
//!
//! WHAT IT IS. The Worker resolves the venue (the registry: a different object,
//! A4's row) and hands the venue's object the WHOLE request -- its url, its headers
//! and its body, at the Worker's clock. The object runs the very handler the Worker
//! used to run (`storefront::place_at`) with an `Env` that answers THIS venue's name
//! in process (`edge::Own`, as the alarm's jobs run, W-LOOP): every one of those nine
//! reads and writes becomes a function call inside this turn. One request.
//!
//! WHAT DOES NOT MOVE. The decision is still `command::place::decide` inside
//! `/fold/place` (`hubdo/place.rs`), reached as a call: the decide/fold authority
//! (DECISIONS.md D0) and the idempotency claim are exactly where they were, and the
//! pricer is the same pure function. Only the hop is gone.
//!
//! WHO MAY SAY WHAT. The object is not reachable from the internet (`hubdo.rs`
//! `fetch`); the Worker has already chosen this object by the venue's name. The
//! venue in the body is still checked against the object's own (`is_me`): a body
//! naming another venue would make `Own` send that venue's reads to a real stub,
//! and the command would act on one venue from inside another (one-venue gate).
//!
//! THE REST OF THE ORDER'S LIFE (W-BN4B). The same move for every route an order passes
//! through (`Kind`): the console's moves and list, the kitchen's board and ack, the
//! courier's run, the customer's tracking, stamps and feedback, and the quote. Measured
//! over one delivery order (`storefront/routes/costs_tests.rs`): 51 venue-object requests -> 17,
//! one per step. A route that finds its own venue (`Place::of_any`: the signed token's
//! claim, else the Host) is `forward`ed to that object; its handler body (`*_at`) is
//! unchanged and still authorises the venue it names, so a body naming another venue is
//! acted on through a real stub exactly as the Worker would have. What stays a request:
//! the platform object's sign-in reads (identity, sessions, couriers) and the registry.

use super::HubImages;
use crate::wire::{Call, Reply, RequestInit};
use serde::{Deserialize, Serialize};
use worker::*;

/// The requests a venue's object can run whole. Each arm is one route's handler, run
/// unchanged inside the object (W-BN4B): the Worker's half only picks the object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// `POST /api/public/locations/:slug/orders` (`storefront::place_at`).
    Place,
    // ── the order's life after it is placed, each measured in `storefront/routes/costs_tests.rs` ──
    /// `POST /api/owner/orders/:id/action` (`owner::order_action_at`).
    OrderAction,
    /// `POST /api/owner/orders/:id/assign` (`owner::assign_courier_at`).
    Assign,
    /// `GET /api/owner/orders` (`owner::orders_at`): the console's list.
    ConsoleOrders,
    /// `POST /api/staff/orders/:id/kitchen-ack` (`kitchen_ack::kitchen_ack_at`).
    KitchenAck,
    /// `GET /api/staff/kitchen` (`kitchen_ack::board::kitchen_orders_at`).
    KitchenBoard,
    /// `GET /api/courier/tasks` (`courier::tasks_at`).
    CourierTasks,
    /// `POST /api/courier/orders/:id/accept` (`courier::accept_at`).
    CourierAccept,
    /// `POST /api/courier/orders/:id/pickup` (`courier::pickup_at`).
    CourierPickup,
    /// `POST /api/courier/orders/:id/deliver` (`courier::deliver_at`).
    CourierDeliver,
    /// `POST /api/courier/position` (`courier::position_at`).
    CourierPosition,
    /// `GET /api/order/:id` (`orders::read::order_at`): the customer's tracking read.
    Track,
    /// `GET /api/order/:id/stamps` (`loyalty::handlers::order_stamps_at`).
    Stamps,
    /// `POST /api/order/:id/feedback` (`orders::feedback::feedback_at`).
    Feedback,
    /// `POST /api/public/locations/:slug/eta` (`eta::quote_at`): products and the queue in one turn.
    Quote,
}

impl Kind {
    /// The path parameters this command's handler reads, by name.
    pub(crate) fn params(self) -> &'static [&'static str] {
        match self {
            Kind::Place | Kind::Quote => &["slug"],
            Kind::ConsoleOrders | Kind::KitchenBoard | Kind::CourierTasks | Kind::CourierPosition => &[],
            Kind::OrderAction
            | Kind::Assign
            | Kind::KitchenAck
            | Kind::CourierAccept
            | Kind::CourierPickup
            | Kind::CourierDeliver
            | Kind::Track
            | Kind::Stamps
            | Kind::Feedback => &["id"],
        }
    }
}

/// The request as the Worker received it, and what the Worker already decided:
/// which venue (by the registry) and when (the request's one clock).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    pub kind: Kind,
    /// The venue's id: the registry's answer to the slug, or the signed token's claim.
    pub venue: String,
    /// The path parameters the handler reads (`Kind::params`), as the router matched them.
    pub params: Vec<(String, String)>,
    /// The method the client used: a read runs as a GET, a write as what it was sent as.
    pub method: String,
    /// The Worker's clock for this request (`crate::Req`), never the object's.
    pub now_ms: i64,
    pub url: String,
    /// Every header the client sent, in order (the idempotency key, a waiter's token).
    pub headers: Vec<(String, String)>,
    /// The body as text: the handler fingerprints exactly what was sent.
    pub body: String,
}

/// THE WORKER'S HALF, for a route whose handler finds its own venue (`Place::of_any`: the
/// signed token's claim, else the Host): the same object that handler would have asked,
/// asked once with the whole request.
pub(crate) async fn forward(kind: Kind, req: Call, ctx: crate::edge::Ctx<crate::Req>) -> Result<Reply> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    send(&place, kind, &ctx, req).await
}

/// THE WORKER'S HALF: the whole request to the venue's object, in ONE request, and its
/// answer back as it is -- status, headers (`idempotent-replay`) and body.
pub(crate) async fn send(place: &crate::hubstore::Place, kind: Kind, ctx: &crate::edge::Ctx<crate::Req>, mut req: Call) -> Result<Reply> {
    let body = match req.text().await {
        Ok(t) => t,
        Err(e) => return Reply::error(format!("bad request body: {e}"), 400),
    };
    let params = kind.params().iter().filter_map(|k| ctx.param(k).map(|v| (k.to_string(), v.clone()))).collect();
    let cmd = Command {
        kind,
        venue: place.venue.clone(),
        params,
        method: req.method().to_string(),
        now_ms: ctx.data.now_ms,
        url: req.url()?.to_string(),
        headers: req.headers().entries().cloned().collect(),
        body,
    };
    let json = serde_json::to_string(&cmd).map_err(|e| Error::RustError(format!("command: {e}")))?;
    let mut init = RequestInit::new();
    init.with_method(Method::Post).with_body(Some(json.into_bytes()));
    let mut call = Call::new_with_init("https://hub/fold/command", &init)?;
    call.headers_mut()?.set("content-type", "application/json")?;
    place.stub()?.fetch_with_request(call).await
}

/// The request the Worker received, rebuilt for the handler.
pub(crate) fn rebuilt(cmd: &Command) -> Result<Call> {
    let method = Method::from(cmd.method.to_ascii_uppercase());
    let mut init = RequestInit::new();
    // A GET carries no body (the platform refuses one).
    let body = (method != Method::Get && method != Method::Head).then(|| cmd.body.clone().into_bytes());
    init.with_method(method).with_body(body);
    let mut call = Call::new_with_init(&cmd.url, &init)?;
    let h = call.headers_mut()?;
    for (k, v) in &cmd.headers {
        h.append(k, v)?;
    }
    Ok(call)
}

impl HubImages {
    /// THE OBJECT'S HALF: `POST /fold/command`, the handler run in this turn.
    pub(super) async fn command_route(&self, mut req: Call) -> Result<Reply> {
        let cmd: Command = crate::body::parse(&mut req).await?;
        if !self.state.is_me(&cmd.venue) {
            return Reply::error(format!("command: this object is not {}", cmd.venue), 409);
        }
        let Some(base) = self.state.base_env() else {
            return Reply::error("command: no platform to run a command against", 500);
        };
        let env = self.own_env(&base, &cmd.venue);
        let call = rebuilt(&cmd)?;
        let params: Vec<(&str, &str)> = cmd.params.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let ctx = crate::edge::Ctx::given(crate::Req { now_ms: cmd.now_ms }, env, &params);
        // BOXED: the handler can reach this object again through `Stub::Own` -> `route`,
        // and an async cycle needs one heap hop to have a size.
        match cmd.kind {
            Kind::Place => {
                let place = crate::hubstore::Place::of_authorised(&ctx, &cmd.venue)?;
                Box::pin(crate::storefront::place_at(place, call, ctx)).await
            }
            Kind::OrderAction => Box::pin(crate::owner::order_action_at(call, ctx)).await,
            Kind::Assign => Box::pin(crate::owner::assign_courier_at(call, ctx)).await,
            Kind::ConsoleOrders => Box::pin(crate::owner::orders_at(call, ctx)).await,
            Kind::KitchenAck => Box::pin(crate::services::orders::kitchen_ack::kitchen_ack_at(call, ctx)).await,
            Kind::KitchenBoard => Box::pin(crate::services::orders::kitchen_ack::board::kitchen_orders_at(call, ctx)).await,
            Kind::CourierTasks => Box::pin(crate::courier::tasks_at(call, ctx)).await,
            Kind::CourierAccept => Box::pin(crate::courier::accept_at(call, ctx)).await,
            Kind::CourierPickup => Box::pin(crate::courier::pickup_at(call, ctx)).await,
            Kind::CourierDeliver => Box::pin(crate::courier::deliver_at(call, ctx)).await,
            Kind::CourierPosition => Box::pin(crate::courier::position_at(call, ctx)).await,
            Kind::Track => Box::pin(crate::services::orders::read::order_at(call, ctx)).await,
            Kind::Stamps => Box::pin(crate::services::loyalty::handlers::order_stamps_at(call, ctx)).await,
            Kind::Feedback => Box::pin(crate::services::orders::feedback::feedback_at(call, ctx)).await,
            Kind::Quote => {
                let place = crate::hubstore::Place::of_authorised(&ctx, &cmd.venue)?;
                Box::pin(crate::eta::quote_at(place, call, ctx)).await
            }
        }
    }
}

#[cfg(test)]
mod tests;
