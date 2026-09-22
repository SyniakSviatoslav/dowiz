//! One socket per client, instead of a question every twelve seconds.
//!
//! WHAT THIS REPLACES. A venue with its console open, one courier on shift and
//! three customers watching their orders asked this platform about ten thousand
//! questions a day, nearly all of which were answered "nothing has changed".
//! Polling is 65 % of every request this system serves, and the object knows
//! the moment something happens.
//!
//! THE SOCKET IS NOT A SECOND API. Everything that changes an order still goes
//! through a Worker route, where the caller is authenticated per request and
//! the kernel decides. What travels down the socket is what the object already
//! wrote: the event, as it landed. A client applies it and keeps its own copy;
//! `?since=` on the ordinary endpoints is still there for a client that missed
//! something, and polling still works for one that has no socket at all.
//!
//! WHO MAY HEAR WHAT is decided HERE, once, at connect time, and travels to the
//! object as a tag. The object is not reachable from the internet, so the tag
//! is the Worker's word rather than the client's claim -- which is what makes
//! "a customer hears about their own order and nothing else" a property rather
//! than a hope.

use worker::*;

use crate::auth::{self, Principal};

/// `GET /api/live` — upgrade to a socket, tagged by who is asking.
///
/// A customer's token names ONE order; an owner's names the venue; a courier's
/// names the courier. Those are the three tags, and nothing else is accepted:
/// an unauthenticated upgrade is refused before the object is reached.
pub async fn connect(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // CASE-INSENSITIVE, because the header is: a client may send "WebSocket",
    // and over HTTP/2 there is no `Upgrade` header at all -- which is why a
    // curl probe on h2 reads 426 while a browser on wss:// gets through.
    let upgrading = req
        .headers()
        .get("Upgrade")
        .ok()
        .flatten()
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"));
    if !upgrading {
        return Response::error("this route is a websocket upgrade", 426);
    }
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    let now = Date::now().as_millis() as i64;
    // THE TOKEN ARRIVES AS A SUBPROTOCOL, because a browser cannot set a header
    // on a WebSocket and a token in the URL is a token in every log and every
    // history. `Sec-WebSocket-Protocol: bearer, <token>` is the standard way
    // round it; the chosen protocol is echoed below, or the browser fails the
    // handshake on its own side.
    let offered = req.headers().get("Sec-WebSocket-Protocol").ok().flatten().unwrap_or_default();
    let token = offered.split(',').nth(1).map(str::trim).unwrap_or("").to_string();
    if token.is_empty() {
        return Response::error("a socket needs its token as the second subprotocol", 401);
    }
    // ── THE PRINCIPAL MUST BELONG TO THIS VENUE ──
    //
    // This was a cross-tenant hole and it was mine. `Place::of_any` resolves
    // the venue from the token's claim FIRST and the Host second -- but a
    // WebSocket carries no `Authorization` header, so the claim is never seen
    // here and the venue is always the HOST. Choosing the tag from the ROLE
    // alone then meant: an owner (or a courier) of venue B, connecting to
    // venue A's host with their own valid token, was handed venue A's console
    // stream -- every order, every address, every rejection reason -- and, on
    // the courier tag, the ability to write positions into venue A's map.
    //
    // `authenticate`'s owner check is deliberately NOT location-scoped ("is
    // this user an owner somewhere"), so it cannot answer this question; the
    // claim's `active_location_id` can, and it is what every other hub route
    // resolves the venue from. A principal whose location is not this hub is
    // refused with 404, the same answer a cross-tenant read gets everywhere
    // else -- a 403 would confirm the venue exists.
    let tag = match auth::authenticate_token(&token, &ctx.env, now).await {
        Ok(Principal::Owner { active_location_id, .. }) => {
            if active_location_id.as_deref() != Some(place.venue.as_str()) {
                return Response::error("not found", 404);
            }
            crate::hubdo::TAG_CONSOLE.to_string()
        }
        Ok(Principal::Courier { courier_id, active_location_id, .. }) => {
            if active_location_id != place.venue {
                return Response::error("not found", 404);
            }
            // THE COURIER'S ID TRAVELS IN THE TAG, not in the messages they
            // send. A GPS frame used to name its own courier, so one courier's
            // socket could move another's pin -- and the object, which cannot
            // see a token, had no way to know better. A tag is attached here,
            // by the Worker, from the verified claim.
            crate::hubdo::tag_courier(&courier_id)
        }
        Ok(Principal::Customer { order_id, location_id, .. }) => {
            if location_id != place.venue {
                return Response::error("not found", 404);
            }
            crate::hubdo::tag_order(&order_id)
        }
        Err(e) => return e.into_response(),
    };

    // The upgrade is forwarded to the object with the tag this Worker decided.
    // The headers travel too, because the runtime needs the `Upgrade` and the
    // key to complete the handshake with the client on the other side.
    let stub = place.stub()?;
    let mut init = RequestInit::new();
    init.with_method(Method::Get).with_headers(req.headers().clone());
    // THE CHOSEN SUBPROTOCOL TRAVELS WITH THE TAG. The object builds the 101
    // and is the only place that can carry a header into it: a response that
    // has already crossed a fetch has immutable headers, and setting one threw
    // -- which is what made this handshake answer 500 in production.
    let out = Request::new_with_init(
        &format!("https://hub/fold/socket?tag={}&proto=bearer", crate::mcp::enc(&tag)),
        &init,
    )?;
    stub.fetch_with_request(out).await
}

#[cfg(test)]
mod tests {
    /// The tags are a contract between this module and the object: the Worker
    /// decides them and the object enforces them, so a typo on either side is
    /// a customer hearing somebody else's order.
    #[test]
    fn an_order_tag_names_exactly_one_order() {
        assert_eq!(crate::hubdo::tag_order("ord_1"), "order:ord_1");
        assert_ne!(crate::hubdo::tag_order("ord_1"), crate::hubdo::tag_order("ord_2"));
        assert_ne!(crate::hubdo::tag_order("ord_1"), crate::hubdo::TAG_CONSOLE);
        assert_ne!(crate::hubdo::TAG_CONSOLE, crate::hubdo::TAG_COURIER);
    }
}
