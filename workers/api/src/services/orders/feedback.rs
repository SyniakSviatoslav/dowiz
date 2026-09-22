//! `POST /api/order/:id/feedback` — a sentence to the venue about one order.
//!
//! IT MOVED HERE WITH ITS DEFECT. The route asked whether the order was over
//! by writing `DELIVERED | REJECTED | CANCELLED` out by hand, and `PICKED_UP`
//! was not in that copy — so a customer who COLLECTED their order was told
//! "this order is still running" and could never leave a note. It now asks
//! `services::orders::status`, which asks the kernel.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::now_ms;

#[derive(Deserialize)]
struct FeedbackIn {
    text: String,
}

/// `POST /api/order/:id/feedback` — a sentence to the venue about one order.
///
/// NO STARS, NO SCORE, NOT ON ANYONE. A number attached to an order becomes a
/// number attached to whoever carried it the moment anybody joins the two, and
/// dowiz does not rank the people who work through it. A kitchen can act on "the
/// rice was cold"; it can do nothing with a three.
pub async fn feedback(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order", 400);
    };
    let body: FeedbackIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let text = body.text.trim().to_string();
    if text.is_empty() {
        return Response::error("say something, or say nothing", 400);
    }
    if text.chars().count() > 600 {
        return Response::error("that is longer than a note about an order", 400);
    }
    let db = ctx.d1("DB")?;
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The customer's own token for THIS order and nothing else. The owner's is
    // refused too: this is the customer's voice and not the venue's.
    match crate::auth::authenticate(&req, &ctx.env, &db, now_ms()).await {
        Ok(crate::auth::Principal::Customer { order_id, .. }) if order_id == id => {}
        Ok(_) => return Response::error("that link is not for this order", 401),
        Err(_) => return Response::error("this order needs the link you were given", 401),
    }

    let at = now_ms();
    let oid = id.clone();
    let outcome = crate::hubstore::append_for(&place, &oid, move |current| {
        let current = current.ok_or_else(|| Error::RustError("no such order".into()))?;
        let old: Value = serde_json::from_str(&current).unwrap_or(json!({}));
        if old.get("feedback").is_some() {
            return Err(Error::RustError("already".into()));
        }
        let status = old.get("status").and_then(Value::as_str).unwrap_or("");
        // THE KERNEL'S OWN LIST. This was `DELIVERED | REJECTED | CANCELLED`
        // spelled out here, and `PICKED_UP` was missing from it -- so a
        // customer who COLLECTED their order was told "this order is still
        // running" and could never leave a note.
        if !super::status::is_terminal(status) {
            return Err(Error::RustError("running".into()));
        }
        let mut o = old.clone();
        o["feedback"] = json!({ "text": text, "at": at });
        // `Noted`, not `Advanced`: a note is not a transition the order machine
        // decided, and writing it as one puts an edge in the log that does not
        // exist.
        let change = crate::fold::delta(&old, &o).to_string();
        Ok(Some((dowiz_hub::EventKind::Noted, change, json!(true))))
    })
    .await;
    match outcome {
        Ok(_) => Response::from_json(&json!({ "ok": true })),
        Err(e) if e.to_string().contains("already") => {
            Response::error("you have already left a note", 409)
        }
        Err(e) if e.to_string().contains("running") => Response::error(
            "this order is still running -- call the venue if something is wrong",
            409,
        ),
        Err(e) if e.to_string().contains("no such order") => Response::error("not found", 404),
        Err(e) => Err(e),
    }
}