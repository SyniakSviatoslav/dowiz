//! The owner's order routes, each run WHOLE by the venue's object in ONE request
//! (W-BN4B, `hubdo/command.rs`): the Worker picks the object (`Place::of_any`, the
//! signed token's claim) and the object runs the handler -- `*_at` in `owner.rs`,
//! unchanged -- in its own turn, where every read and write of its images is a call.

use crate::edge::Ctx;
use crate::hubdo::command::{forward, Kind};
use crate::wire::{Call, Reply};
use worker::Result;

/// `GET /api/owner/orders` (`owner::orders_at`).
pub async fn orders(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::ConsoleOrders, req, ctx).await
}

/// `POST /api/owner/orders/:id/assign` (`owner::assign_courier_at`).
pub async fn assign_courier(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::Assign, req, ctx).await
}

/// `POST /api/owner/orders/:id/action` (`owner::order_action_at`).
pub async fn order_action(req: Call, ctx: Ctx<crate::Req>) -> Result<Reply> {
    forward(Kind::OrderAction, req, ctx).await
}
