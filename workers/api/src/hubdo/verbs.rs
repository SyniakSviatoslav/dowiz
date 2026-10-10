//! THE COMMAND ROUTES' BODIES (split out of `hubdo.rs`, W-SPLIT): each parses
//! its body through `crate::body` (deny-unknown-fields, `strict-body.sh`), runs
//! the command in this turn, and answers the result or the refusal. The routes
//! are named in `hubdo/route.rs`; the commands live with their families.

use super::{forget, HubImages};
use crate::wire::{Call, Reply as Response};
use worker::*;

impl HubImages {
    /// A SUPPLIES / RECIPES SPREADSHEET, judged and applied in one turn (`hubdo/bulk.rs`).
    pub(super) async fn bulk_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        self.bulk_import(crate::body::parse(&mut req).await?).await
    }

    /// FORGET A CUSTOMER: `/fold/forget`
    pub(super) async fn forget_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let input: forget::ForgetIn = crate::body::parse(&mut req).await?;
        match self.forget(input).await? {
            Ok(out) => Response::from_json(&out),
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }

    /// REFUND AN ORDER: `/fold/refund`
    pub(super) async fn refund_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let crate::idempotency::commit::Claimed { input, idem } = crate::body::parse(&mut req).await?;
        match self.refund(input, idem).await? {
            Ok(out) => Response::from_json(&out),
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }

    /// AN AGGREGATOR ORDER ENTERED BY HAND: `/fold/aggregator`
    pub(super) async fn aggregator_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let input: crate::command::place::PlaceIn = crate::body::parse(&mut req).await?;
        match self.aggregator(input).await? {
            Ok(out) => Response::from_json(&out),
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }

    /// THE FOOD A REFUSED DELIVERY BROUGHT BACK: `/fold/returned`
    pub(super) async fn returned_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let input: crate::command::refund::returned::ReturnedIn = crate::body::parse(&mut req).await?;
        match self.returned(input).await? {
            Ok(out) => Response::from_json(&out),
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }

    /// A STOCK MOVEMENT (W0a): `/fold/stock_move`
    pub(super) async fn stock_move_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        self.stock_move(crate::body::parse(&mut req).await?).await
    }

    /// THE KITCHEN SAW THE TICKET: `/fold/kitchen_ack`
    pub(super) async fn kitchen_ack_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let input: crate::command::kitchen_ack::KitchenAckIn = crate::body::parse(&mut req).await?;
        match self.kitchen_ack(input).await? {
            Ok(out) => Response::from_json(&out),
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }

    /// PLACE AN ORDER: `/fold/place` (`hubdo/place.rs`), the generation in a header.
    pub(super) async fn place_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let crate::idempotency::commit::Claimed { input, idem } = crate::body::parse(&mut req).await?;
        match self.place(input, idem).await? {
            Ok(out) => {
                let mut res = Response::from_json(&out)?;
                res.headers_mut().set("x-generation", &out.generation.to_string())?;
                Ok(res)
            }
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }

    /// ADVANCE AN ORDER: `/fold/advance` (`hubdo/orders.rs`).
    pub(super) async fn advance_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let crate::idempotency::commit::Claimed { input, idem } = crate::body::parse(&mut req).await?;
        match self.advance(input, idem).await? {
            Ok(out) => Response::from_json(&out),
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }

    /// HAND AN ORDER TO A COURIER: `/fold/assign` (`hubdo/orders.rs`).
    pub(super) async fn assign_route(&self, req: Call) -> Result<Response> {
        let mut req = req;
        let input: crate::command::assign::AssignIn = crate::body::parse(&mut req).await?;
        match self.assign(input).await? {
            Ok(out) => Response::from_json(&out),
            Err(r) => Response::error(r.message().to_string(), r.status()),
        }
    }
}
