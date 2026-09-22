//! `POST /api/promo/check` — what a code would take off this basket.
//!
//! ORCHESTRATION ONLY. It loads the two images, prices the basket with the
//! shared pricer and asks the hub's promo rules; every number in the answer is
//! decided somewhere that has tests.

use serde::Deserialize;
use serde_json::json;
use worker::*;

use super::pricing::{price_basket, Want};

#[derive(Deserialize)]
struct PromoCheckIn {
    code: String,
    items: Vec<crate::storefront::LineIn>,
}

/// `POST /api/promo/check`
///
/// The basket arrives as product ids, never as a subtotal: the hub prices it
/// with the catalogue, which is the same source the order uses. A preview that
/// trusted a number from the browser would quote whatever the browser asked for.
///
/// It is a PREVIEW and says so: the order re-checks under the write, so a code
/// on its last use can be quoted here and refused at checkout. That is the right
/// way round -- the alternative gives the same last use away twice.
pub async fn promo_check(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let body: PromoCheckIn = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    // NO `ctx.d1("DB")` here: this route held a D1 handle it never used, left
    // behind by the cutover. A binding that is opened and not read is what
    // makes "remove the D1 binding" look harder than it is.
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // TWO IMAGES, one hub: the log and the catalogue have different roots and
    // cannot share one, so a route that reads both loads both.
    // ONE ROUND TRIP for both images: see `load_both`.
    let (listed, cat) = futures_util::future::try_join(
        crate::hubstore::orders(&place),
        crate::hubstore::load_catalog(&place),
    )
    .await?;
    let cat = cat.catalog;
    let code = dowiz_hub::promo::normalise(&body.code);
    let Some(p) = cat.promo(&code).as_deref().and_then(dowiz_hub::promo::Promo::parse)
    else {
        return Response::error(dowiz_hub::promo::Refusal::Unknown.as_str(), 400);
    };

    // THE SAME PRICER THE CHECKOUT USES. This loop was a second implementation
    // that refused only unknown products: it quoted unavailable dishes, read a
    // missing price as free, dropped a paid option whenever the selection was
    // one the checkout REFUSES, and clamped an impossible quantity into range.
    // A preview that prices a basket the order will not accept is worse than
    // no preview.
    let basket = match price_basket(
        |id| cat.product(id),
        body.items.iter().map(|it| Want {
            product_id: &it.product_id,
            modifier_ids: &it.modifier_ids,
            quantity: it.quantity,
        }),
    ) {
        Ok(b) => b,
        Err(r) => return Response::error(r.text(), r.status()),
    };
    let subtotal = basket.subtotal;

    let used = crate::hubstore::promo_uses_in(&listed, &code);
    match p.redeem(subtotal, crate::owner::now_ms(), used) {
        Ok(cut) => Response::from_json(&json!({
            "code": p.code, "discount": cut, "subtotal": subtotal, "total": subtotal - cut
        })),
        Err(r) => Response::error(r.as_str(), 409),
    }
}
