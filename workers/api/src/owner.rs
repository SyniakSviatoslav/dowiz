//! The owner's day of service: see the queue, move an order, stop a dish,
//! open or close the venue.
//!
//! Every status change goes through the kernel FSM. This module never contains a
//! list of statuses in order — the one in `web/src/app.js` is exactly the
//! duplicate authority the architecture forbids, and it is why an order there
//! could be walked anywhere. Here an illegal edge is the kernel's refusal.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::auth::{self, Principal};

pub(crate) fn now_ms() -> i64 {
    Date::now().as_millis() as i64
}

/// Authenticate, require the owner role, and confirm the membership covers this
/// location. The membership is read LIVE — an owner removed a moment ago is
/// refused here even holding a valid token.
///
/// ONE READ, NOT TWO. `authenticate` re-derives the owner's membership with a
/// SELECT of its own and this function then asked a second, narrower question
/// about the same rows: "is there a membership at all" followed by "is there
/// one HERE". The second answer implies the first, so the first query was a
/// round trip -- about sixty milliseconds -- bought nothing. Nine owner write
/// paths pay this on every call.
///
/// AN API KEY IS STILL A DIFFERENT CREDENTIAL. `dowiz_`-prefixed keys are not
/// JWTs and carry their own row, so they keep the full `authenticate` path;
/// they are the integration surface, not the console's hot path, and making
/// them cheap here would have meant making them wrong.
pub(crate) async fn owner_at(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    location_id: &str,
) -> std::result::Result<String, Response> {
    let raw = match auth::bearer(req) {
        Ok(r) => r,
        Err(e) => return Err(e.into_response().unwrap()),
    };
    let user_id = if raw.starts_with("dowiz_") {
        match auth::authenticate(req, &ctx.env, ctx.data.now_ms).await {
            // A KEY IS SCOPED TO THE VENUE IT WAS MINTED FOR, and that scope
            // used to be dropped on the floor here -- `active_location_id`
            // carries the key's own `location_id` (see `api_key_principal`)
            // and nothing compared it to the venue being written.
            //
            // An owner of two venues mints a key from A's console and hands it
            // to A's integrator; the integrator posts `{"location_id": "B"}`
            // and it is accepted, because the membership check only asks
            // whether the OWNER owns B. B's console never listed that key and
            // `revoke_api_key` is scoped by venue, so B cannot take it away
            // either. `mcp.rs` already honoured the scope; the REST surface
            // did not.
            Ok(Principal::Owner { user_id, active_location_id, .. }) => {
                if active_location_id.as_deref() != Some(location_id) {
                    return Err(Response::error("not found", 404).unwrap());
                }
                user_id
            }
            Ok(_) => return Err(Response::error("forbidden role", 403).unwrap()),
            Err(e) => return Err(e.into_response().unwrap()),
        }
    } else {
        // Pure HMAC, no I/O: a forged or expired token never reaches D1.
        match auth::verify(&ctx.env, &raw, ctx.data.now_ms) {
            Ok(auth::Claims::Owner { user_id, .. }) => user_id,
            Ok(_) => return Err(Response::error("forbidden role", 403).unwrap()),
            Err(e) => return Err(e.into_response().unwrap()),
        }
    };
    #[derive(Deserialize)]
    struct M {
        /// ONLY ITS EXISTENCE IS THE ANSWER: this asks whether the membership
        /// is there, not what it says.
        #[allow(dead_code)]
        id: String,
    }
    let m: std::result::Result<Option<M>, _> = crate::identity_store::identity(&ctx.env)
        .await
        .map(|t| {
            crate::identity_store::membership(&t, location_id, &user_id)
                .filter(|x| crate::identity_store::s_of(x, "role") == "owner")
                .map(|_| M { id: String::new() })
        });
    match m {
        Ok(Some(_)) => Ok(user_id),
        // Cross-tenant is 404, never 403: a 403 confirms the location exists.
        Ok(None) => Err(Response::error("not found", 404).unwrap()),
        Err(e) => Err(Response::error(format!("auth backend unavailable: {e}"), 503).unwrap()),
    }
}

/// Which venue, resolved rather than demanded.
///
/// A HUB IMAGE HOLDS EXACTLY ONE VENUE. The `location_id` query parameter is a
/// leftover from the multi-tenant table shape, and every console pane written
/// against the native adapter omits it -- which meant the owner surface on this
/// Worker answered 400 to its own admin pane for every route. When the caller
/// says nothing, the catalogue is asked.
/// Authenticate the owner AND resolve the venue, in one query.
///
/// Thirty-five handlers call `venue_of` and then `owner_at`, which were two D1
/// round trips -- one for the venue row, one for the membership -- before the
/// handler did anything at all. At roughly sixty milliseconds each that is an
/// eighth of a second every owner request spent asking two questions that one
/// query answers.
///
/// The membership is still re-derived from the database rather than trusted
/// from the token, which is the property that matters: an owner removed a
/// moment ago is refused here even holding a valid one.
pub(crate) async fn owner_and_venue(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
) -> std::result::Result<(String, String), Response> {
    // THE TOKEN IS VERIFIED WITHOUT TOUCHING THE DATABASE, and the membership
    // is checked by the JOIN below. `authenticate` would have run its own
    // membership SELECT first, so this path was asking the same question twice
    // -- once to decide the caller is an owner, once to decide which venue --
    // and paying two round trips for one answer.
    //
    // The property that matters is unchanged: authority is RE-DERIVED from the
    // database on every request, so an owner removed a moment ago is refused
    // here even holding a valid token. It is derived once instead of twice.
    let claims = match auth::verify(&ctx.env, &auth::bearer(req).map_err(|e| {
        e.into_response().unwrap()
    })?, ctx.data.now_ms) {
        Ok(c) => c,
        Err(e) => return Err(e.into_response().unwrap()),
    };
    let auth::Claims::Owner { user_id, active_location_id, .. } = claims else {
        return Err(Response::error("forbidden role", 403).unwrap());
    };
    #[derive(Deserialize)]
    struct Row {
        location_id: String,
    }
    // ── WHICH VENUE, AND WHY IT IS NOT A GUESS ──
    //
    // The caller's `?location_id=` first, then the venue their TOKEN says they
    // opened. Only if neither is given does this fall through to "the one
    // membership they have" -- which used to be the first row of an unordered
    // `LIMIT 1`.
    //
    // MEASURED, on this platform, 2026-09-21: the operator owns two venues, so
    // an owner signed into `sushi-durres` who called an owner route WITHOUT the
    // query parameter got `dubin-durres`. The courier invite is one such route,
    // so a code created from one venue's console produced a courier attached to
    // the OTHER venue -- and the courier app then showed them a queue that was
    // not their venue's. The console always sends the parameter, which is why
    // this survived; anything else calling the API did not.
    let wanted = location_of(req).or(active_location_id);
    // THE JOIN IS GONE AND WITH IT THE SECOND TABLE. `locations JOIN
    // memberships` existed to prove the venue exists AND that this person owns
    // it; the membership record only exists for a venue that does, so the
    // membership IS both halves of that question.
    let row: std::result::Result<Option<Row>, _> = crate::identity_store::identity(&ctx.env)
        .await
        .map(|t| match &wanted {
            Some(l) => crate::identity_store::membership(&t, l, &user_id)
                .filter(|x| crate::identity_store::s_of(x, "role") == "owner")
                .map(|_| Row { location_id: l.clone() }),
            // No venue named anywhere: the token carries none and the request
            // asked for none. Sorted rather than `LIMIT 1` over a join whose
            // order nobody specified -- an unordered LIMIT 1 is a coin toss the
            // query planner gets to make differently on any two days.
            None => {
                let mut owned: Vec<String> =
                    crate::identity_store::memberships_of(&t, &user_id)
                        .into_iter()
                        .filter(|(_, role)| role == "owner")
                        .map(|(loc, _)| loc)
                        .collect();
                owned.sort();
                owned.into_iter().next().map(|location_id| Row { location_id })
            }
        });
    match row {
        // Cross-tenant is 404, never 403: a 403 confirms the location exists.
        Ok(Some(r)) => Ok((user_id, r.location_id)),
        Ok(None) => Err(Response::error("not found", 404).unwrap()),
        Err(e) => Err(Response::error(format!("auth backend unavailable: {e}"), 503).unwrap()),
    }
}

/// The owner, the venue AND the hub — with the two round trips OVERLAPPED.
///
/// `owner_and_venue` asks D1 who the caller is; `load_both` asks D1 for the
/// images. NEITHER NEEDS THE OTHER'S ANSWER — the hub image holds exactly one
/// venue and is fetched by a fixed id, not by anything the membership row says.
/// Run one after the other, as every handler did, and an owner request pays two
/// full round trips to a database that is roughly 150 ms away from this Worker.
/// Run them together and it pays the slower of the two.
///
/// THE TOKEN IS STILL CHECKED FIRST, and deliberately. `auth::verify` is pure
/// HMAC with no I/O, so it costs nothing to run before the join — and running
/// it first means a request with a forged or expired token never reaches D1 at
/// all. Starting the image load beside an unverified caller would hand anyone
/// on the internet a way to make this Worker do a megabyte of work per request.
///
/// Authority is unchanged: the membership is still re-derived from the database
/// on every request, so an owner removed a moment ago is refused even holding a
/// valid token. Nothing loaded here is disclosed before that check returns.
/// Takes the WORK rather than doing a fixed piece of it, because the handlers
/// do not agree on what they need: some want both images, some the catalogue
/// alone, some the order log, and the stock pane wants the catalogue and the
/// ledger. A helper per combination would be four helpers that each have to be
/// kept in step with this reasoning; a helper that takes a future is one.
pub(crate) async fn owner_beside<F, T>(
    req: &Request,
    ctx: &RouteContext<crate::Req>,
    place: &crate::hubstore::Place,
    work: F,
) -> std::result::Result<(String, String, T), Response>
where
    F: std::future::Future<Output = Result<T>>,
{
    // Pure, no I/O. A bad token stops here, before either query is issued.
    let bearer = auth::bearer(req).map_err(|e| e.into_response().unwrap())?;
    match auth::verify(&ctx.env, &bearer, ctx.data.now_ms) {
        Ok(auth::Claims::Owner { .. }) => {}
        Ok(_) => return Err(Response::error("forbidden role", 403).unwrap()),
        Err(e) => return Err(e.into_response().unwrap()),
    }

    let (who, done) = futures_util::future::join(owner_and_venue(req, ctx), work).await;

    // AUTHORISATION IS RESOLVED BEFORE THE WORK IS HANDED BACK, so a caller who
    // fails it gets their 401 or 404 and nothing else -- that the bytes were
    // already in memory is invisible to them.
    let (user_id, location_id) = who?;
    // AND THE VENUE THAT WAS READ IS THE VENUE THAT WAS AUTHORISED.
    //
    // The header above says "neither needs the other's answer", and that was
    // the defect: `work` reads an object chosen by the token's claim or the
    // Host, while the membership is checked against `?location_id=`. For an
    // owner of two venues those are not always the same venue, and every
    // handler on this path then answered -- or WROTE -- with the wrong one.
    // The overlap is kept, because the check is free once both have landed.
    place.must_be(&location_id)?;
    let out = done.map_err(|e| Response::error(format!("hub unavailable: {e}"), 503).unwrap())?;
    Ok((user_id, location_id, out))
}

// `venue_of` WAS HERE. It fell back to `SELECT id FROM locations LIMIT 1`
// under a comment saying "a hub image holds exactly one venue, so LIMIT 1 is
// not a guess about which; it is the only one there is" -- which stopped being
// true when this platform took its second venue. Nothing called it any more;
// `owner_and_venue` answers the same question and says which venue it means.

pub(crate) fn location_of(req: &Request) -> Option<String> {
    req.url()
        .ok()?
        .query_pairs()
        .find(|(k, _)| k == "location_id")
        .map(|(_, v)| v.to_string())
}

/// `GET /api/owner/orders?location_id=&status=`
pub async fn orders(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // ── A CONSOLE THAT ALREADY HAS A COPY ──
    //
    // `?since=<generation>` asks what CHANGED. The object answers from a small
    // window of recent events; past it -- a cold object, a long absence -- it
    // says so and this falls through to the whole list below. A client applies
    // the changes to the copy it holds, which is what lets a console keep
    // drawing a queue while the network is gone.
    //
    // THE FULL LIST IS STILL THE TRUTH. The catch-up is an optimisation over
    // it, in the snapshot-and-delta shape game netcode settled on decades ago:
    // when the delta cannot be trusted, send the snapshot.
    let since: Option<i64> = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "since").map(|(_, v)| v.to_string()))
        .and_then(|v| v.parse().ok());
    if let Some(since) = since {
        // Authorised exactly as the full read is, and before the object is
        // asked anything. The VENUE it answers with is kept: the delta path
        // has to filter by the same location the full path does, or a console
        // could be handed orders the full list would never have shown it.
        let want_loc = match owner_and_venue(&req, &ctx).await {
            Ok((_, l)) => l,
            Err(r) => return Ok(r),
        };
        // AND THE OBJECT BEING ASKED IS THAT VENUE'S. `place` was chosen by
        // the token's claim or the Host; a request that names one venue in its
        // query and another in its token is refused rather than answered from
        // whichever of the two the object happened to be.
        if let Err(r) = place.must_be(&want_loc) {
            return Ok(r);
        }
        // A STATUS FILTER IS NOT EXPRESSIBLE AS A DELTA. The full list drops
        // the orders that do not match; a change set says what MOVED, and an
        // order that moved out of the filtered status has to disappear from
        // the client's copy, which a change cannot say. So a filtered read is
        // always the whole list.
        let filtered = req
            .url()
            .ok()
            .and_then(|u| u.query_pairs().find(|(k, _)| k == "status").map(|_| ()))
            .is_some();
        if !filtered {
            if let Ok((generation, Some(changes))) =
                crate::hubstore::changes_since(&place, since).await
            {
                // THE SAME TENANCY TEST THE FULL PATH APPLIES. `place` comes
                // from the token's claim or the Host; `want_loc` is the venue
                // the membership was checked against. Where they differ, the
                // full list returns nothing and this must not return more.
                let mine: Vec<Value> = changes
                    .into_iter()
                    .filter(|c| {
                        serde_json::from_str::<Value>(&c.payload)
                            .ok()
                            .and_then(|v| {
                                v.get("location_id").and_then(Value::as_str).map(String::from)
                            })
                            // A delta carries only what changed, so most
                            // changes have no `location_id` at all; those are
                            // about an order this venue already holds, and the
                            // client keeps them only if it holds it.
                            .is_none_or(|l| l == want_loc)
                    })
                    .map(|c| json!({
                        "generation": c.generation,
                        "kind": c.kind,
                        "order_id": c.order_id,
                        "payload": c.payload,
                    }))
                    .collect();
                return Response::from_json(&json!({
                    "generation": generation,
                    "changes": mine,
                    "full": false,
                }));
            }
        }
        // Falls through: the object could not say, or the read is filtered,
        // so the whole list it is.
    }
    // The membership query and the image read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    // THE PROJECTION, NOT THE IMAGE. The object folds its own log once per
    // generation; a console polling every fifteen seconds used to be handed
    // every order the venue had ever taken so that this function could keep
    // the ones from today.
    let (_, loc, (generation, listed)) =
        match owner_beside(&req, &ctx, &place, crate::hubstore::orders_at(&place)).await {
            Ok(v) => v,
            Err(r) => return Ok(r),
        };

    let status = req
        .url()
        .ok()
        .and_then(|u| u.query_pairs().find(|(k, _)| k == "status").map(|(_, v)| v.to_string()));
    // The hub log is the source. Reading it folds every order to its newest
    // state, so the queue cannot show a status the events do not support.
    let mut out: Vec<Value> = listed
        .into_iter()
        .filter_map(|e| {
            let v: Value = serde_json::from_str(&e.order_json).ok()?;
            if v.get("location_id").and_then(|x| x.as_str()) != Some(loc.as_str()) {
                return None;
            }
            let st = v.get("status").and_then(|x| x.as_str()).unwrap_or("").to_string();
            if let Some(want) = &status {
                if &st != want {
                    return None;
                }
            }
            // THE ENVELOPE AS STORED, not a re-spelling of it.
            //
            // This projection sent `createdAtMs` and `courierId` while the
            // console reads `created_at_ms` and `courier_id`, and it dropped
            // `promo`, `discount`, `tip` and `feedback` entirely -- so every row
            // rendered without its money detail and with no time on it. That is
            // the same shape-mismatch that once left the courier app showing
            // "you are offline" for every courier: a hand-written projection is
            // a second contract, and the second contract is the one that drifts.
            //
            // Handing back the order itself removes the second contract. The
            // fields are the ones the log holds, which is what every other
            // reader already agrees on.
            let mut o = v.clone();
            o["id"] = json!(e.order_id);
            o["status"] = json!(st);
            Some(o)
        })
        .collect();
    // The time that is left on each live order, from where it is and where
    // the courier is, with one read of the map for the whole queue.
    if let Ok(loaded) = crate::hubstore::load_catalog(&place).await {
        crate::live_eta::attach_all(&place, &loaded, &mut out, ctx.data.now_ms).await;
    }
    // The generation travels with the list so a client can ask for changes
    // after it next time -- and it is the generation THIS list was folded
    // from, read off the same response, never asked for afterwards.
    Response::from_json(&json!({ "orders": out, "generation": generation, "full": true }))
}

/// `POST /api/owner/orders/:id/assign` -- the owner hands an order to a courier.
///
/// The same row and the same note the courier's own `accept` writes, so a
/// hand-off from the counter and a claim from the phone are one fact in one
/// place; the courier app shows it as theirs on its next read. An order that
/// already has a courier is refused rather than quietly reassigned.
pub async fn assign_courier(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        courier_id: String,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    // THE VENUE THAT WAS AUTHORISED, not the one the token happens to name --
    // see `Place::of_authorised`.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    // The courier must be this venue's and active.
    #[derive(Deserialize)]
    struct C {
        id: String,
    }
    let crew = crate::identity_store::couriers(&ctx.env).await?;
    // By id or by the phone the console displays, on THIS venue's roster, and
    // active. The roster key is the `JOIN courier_locations` and the status is
    // on the record.
    let known: Option<C> = [
        Some(body.courier_id.clone()),
        crate::identity_store::courier_id_for_phone(
            &crew,
            &crate::auth::sha256_hex(&body.courier_id),
        ),
    ]
    .into_iter()
    .flatten()
    .find(|cid| {
        crew.get(
            crate::identity_store::K_ROSTER,
            &crate::identity_store::roster_id(&body.location_id, cid),
        )
        .is_some()
            && crate::identity_store::rec(&crew, crate::identity_store::K_COURIER, cid)
                .map(|r| crate::identity_store::s_of(&r, "status") == "active")
                .unwrap_or(false)
    })
    .map(|id| C { id });
    let Some(known) = known else {
        return Response::error("no such courier at this venue", 404);
    };
    let courier_id = known.id;
    // THE ORDER IS NOT FETCHED HERE ANY MORE. This handler used to read it
    // across the hop to check the venue, the status and whether the payment was
    // cash -- three questions about a record the object is holding, asked from
    // the far side of a network, and then asked AGAIN by the writes that
    // followed. `command::assign` answers all three from the copy in its own
    // memory, so the round trip is gone with them.
    // ── THE ASSIGNMENT AND THE EVENT, IN ONE OBJECT TURN ──
    //
    // WHAT WAS HERE, and it had NO compensation at all: `with_table` wrote the
    // assignment record into the `ops` image, and then a SECOND round trip
    // appended the `Noted` event that puts `courier_id` on the order. The
    // assignment record is also the LOCK -- the refusal of a second courier is
    // `t.get("asg", id).is_some()` -- so it had to land first, and a lost
    // append left an invisible lock: the console showed an unassigned order,
    // the courier's list showed the job, and the next attempt answered "this
    // order already has a courier" naming nobody the owner could see.
    //
    // WHO THE COURIER IS STAYS HERE, deliberately. The roster lives in the
    // PLATFORM object, which the venue's object cannot read, so it is resolved
    // above -- the same division as pricing on a placement. The object decides
    // only what it alone can see: whose order this is, whether it is in a state
    // that can be handed out, and whether somebody already has it.
    let input = crate::command::assign::AssignIn {
        order_id: id.clone(),
        location_id: body.location_id.clone(),
        courier_id: courier_id.clone(),
        now_ms: ctx.data.now_ms,
    };
    let _: crate::command::assign::AssignOut =
        match crate::command::send(&place, "assign", &input).await {
            Ok(v) => v,
            Err((status, said)) => return Response::error(said, status),
        };
    Response::from_json(&json!({ "ok": true, "orderId": id, "courierId": courier_id }))
}

/// `POST /api/owner/orders/:id/action` — `{location_id, action, reason?}`
///
/// `action` names an intent, never a target status. The mapping from intent to
/// status lives in one place and the FSM decides whether the edge is legal.
pub async fn order_action(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        action: String,
        #[serde(default)]
        reason: Option<String>,
    }
    // READ ONCE, because the idempotency layer fingerprints the raw body: rule
    // 3 ("same key, different body is a 409") cannot be answered from a parsed
    // struct that has already dropped what else was sent.
    let raw_body = req.text().await.unwrap_or_default();
    let body: In = match serde_json::from_str(&raw_body) {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing order id", 400);
    };
    // THE VENUE THAT WAS AUTHORISED, not the one the token happens to name --
    // see `Place::of_authorised`.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let who = match owner_at(&req, &ctx, &body.location_id).await {
        Ok(user_id) => user_id,
        Err(r) => return Ok(r),
    };
    // ONE CLOCK READ FOR THE WHOLE REQUEST: the idempotency window and the
    // transition's own stamp are the same instant.
    let now = ctx.data.now_ms;
    // ── THE OWNER'S TAP IS RETRIED TOO ──
    //
    // A console on a patchy connection repeats a "confirm"; the FSM refuses the
    // second one as an illegal edge, which is SAFE and reads on screen as "that
    // did not work" for something that did. The key makes the second answer the
    // first one's. Scoped to the OWNER, so two people at one venue tapping the
    // same order are still two calls -- and the one who loses gets the kernel's
    // real refusal rather than a replay of somebody else's success.
    let idem = match crate::idempotency::guard(
        &place,
        req.headers().get("idempotency-key").ok().flatten(),
        &who,
        "owner.order_action",
        &format!("{id}:{raw_body}"),
        now,
    )
    .await
    {
        Ok(g) => g,
        Err(r) => return Ok(r),
    };

    let next = match body.action.as_str() {
        "confirm" => "CONFIRMED",
        "reject" => "REJECTED",
        "preparing" => "PREPARING",
        "ready" => "READY",
        // THE END OF A COLLECTION ORDER, which the product had no way to
        // reach. `allowed_next` has offered Ready -> PickedUp since the FSM
        // was written and no route ever emitted it, so a customer who chose to
        // collect left an order sitting at READY for ever: the owner's five
        // actions could not end it and a courier is the wrong answer. PickedUp
        // is terminal, and the ingredients were already consumed at PREPARING.
        "collected" => "PICKED_UP",
        "cancel" => "CANCELLED",
        other => return Response::error(format!("unknown action: {other}"), 400),
    };

    // ── THE TRANSITION AND THE SHELF, IN ONE OBJECT TURN ──
    //
    // WHAT WAS HERE: `append_for` to move the order, then a SECOND round trip
    // through `with_stock` to settle the ledger -- PREPARING consumes what was
    // held, REJECTED and CANCELLED release it. The settlement's failure was
    // logged and deliberately did not fail the transition, because the order
    // had already moved and the customer had already been told. That reasoning
    // was right and the situation it reasoned about should not exist: both
    // outcomes are an order and a ledger that disagree, and the code only got
    // to choose which way. `command::advance` decides both against copies in
    // memory and writes them once both have succeeded.
    //
    // THE KERNEL STILL DECIDES, and it decides inside the object now. An
    // illegal edge is still its refusal and still reaches the owner as a 409
    // carrying its own words.
    let input = crate::command::advance::AdvanceIn {
        order_id: id.clone(),
        location_id: body.location_id.clone(),
        next: next.to_string(),
        reason: body.reason.clone(),
        now_ms: now,
    };
    let advanced: crate::command::advance::AdvanceOut =
        match crate::command::send(&place, "advance", &input).await {
            Ok(v) => v,
            Err((status, said)) => return Response::error(said, status),
        };
    let merged: Value = serde_json::from_str(&advanced.merged)
        .map_err(|e| Error::RustError(format!("hub answered unreadable json: {e}")))?;

    idem.done(&place, 200, &merged.to_string()).await;
    Response::from_json(&merged)
}

/// `GET /api/owner/dashboard?location_id=` — the numbers an owner looks at
/// between orders, computed from the log rather than kept as a running total
/// that can drift.
pub async fn dashboard(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;
    // The membership query and the image read do not depend on each other, so
    // `owner_beside` runs them together. The token is still verified before
    // either is issued -- see it for why that order matters.
    // THE ORDERS, FOLDED, AND NOTHING ELSE. This route fetched the catalogue
    // beside the log "because the readiness count needs it" -- and then never
    // touched it: the tally below reads only the orders. So the dashboard was
    // paying for a whole second image on every poll to satisfy a comment.
    // The orders and the venue's own record, together. The record is ~1 KB from
    // `/fold/venue` -- NOT the catalogue image it lives in -- so this is a
    // second round trip in parallel rather than a second image in series. It is
    // here because the day boundary below needs the venue's time zone and
    // getting that wrong is what this whole change is about.
    let (_, loc, (listed, venue)) = match owner_beside(
        &req,
        &ctx,
        &place,
        futures_util::future::try_join(
            crate::hubstore::orders(&place),
            crate::hubstore::venue_record(&place),
        ),
    )
    .await
    {
        Ok(v) => v,
        Err(r) => return Ok(r),
    };

    // "Today" starts at local midnight FOR THE VENUE.
    //
    // This was `let tz_offset_ms: i64 = 2 * 60 * 60 * 1000; // Europe/Tirane,
    // standard time` -- and Europe/Tirane's standard time is UTC+1. The
    // constant was the SUMMER offset, so from 01:00 UTC on 25 October 2026 this
    // owner's day would have begun an hour early, every day, until the last
    // Sunday of March. `tz::start_of_local_day_ms` also handles the two days a
    // year when the offset in force now is not the offset that was in force at
    // midnight; see its comment for why one pass and two passes are both wrong.
    let zone = crate::hubstore::zone_of(venue.as_ref());
    let now = ctx.data.now_ms;
    let day_start = dowiz_hub::tz::start_of_local_day_ms(zone, now);

    let (mut count, mut revenue, mut pending, mut active) = (0i64, 0i64, 0i64, 0i64);
    for e in listed {
        let Ok(v) = serde_json::from_str::<Value>(&e.order_json) else { continue };
        // THE SAME TENANCY RULE THE ANALYTICS USE. This was `!= Some(loc)`,
        // which drops an order whose `location_id` is absent, while
        // `orders_of` kept it -- so the takings tile and the analytics pane on
        // the same screen could disagree about the same order. One rule now,
        // in `services::orders::mine`, with a test saying which way it goes
        // and why.
        if !crate::services::orders::mine::belongs_to(&v, &loc) {
            continue;
        }
        if (e.seq as i64) < day_start {
            continue;
        }
        count += 1;
        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("");
        if status == "PENDING" {
            pending += 1;
        } else if crate::services::orders::status::is_active(status) {
            active += 1;
        }
        // ── ONE DEFINITION OF TODAY'S TAKINGS ──
        //
        // This counted DELIVERED only while the native adapter and the
        // analytics count every order that was not REFUSED -- so the same
        // product showed an owner two different numbers depending on which
        // deployment they opened, and the dashboard disagreed with its own
        // analytics pane on the same screen. The rule is the analytics one,
        // because that is what the copy on both panes describes: money the
        // venue took, and a rejected order is not that.
        //
        // The tip is subtracted wherever the venue's money is counted: it is
        // the courier's, passing through.
        revenue += crate::services::orders::status::venue_took(
            v.get("total").and_then(|t| t.as_i64()).unwrap_or(0),
            v.get("tip").and_then(|t| t.as_i64()).unwrap_or(0),
            status,
        );
    }
    Response::from_json(&json!({
        "todayOrders": count, "todayRevenue": revenue,
        "pending": pending, "active": active, "dayStartMs": day_start
    }))
}

/// `PATCH /api/owner/products/:id` — the stop-list and the price.
pub async fn update_product(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        #[serde(default)]
        available: Option<bool>,
        #[serde(default)]
        unavailable_note: Option<String>,
        #[serde(default)]
        price: Option<i64>,
        /// The fourteen declarable allergens. AN EMPTY ARRAY IS A CLAIM --
        /// "none of the fourteen" -- and absent is not, which is why this is
        /// `Option<Vec<_>>` all the way from the wire.
        #[serde(default)]
        allergens: Option<Vec<String>>,
        /// The dish's real widest dimension, in centimetres. Without it the
        /// storefront shows no AR button, which is correct: a guessed size
        /// answers the customer's question wrongly.
        #[serde(default)]
        size_cm: Option<i64>,
        /// How long THIS dish takes in the kitchen, in minutes.
        ///
        /// The venue's own figure, not a guess and not a platform average. It
        /// is what `dowiz_kernel::eta` uses to quote a delivery time, so a
        /// coffee and a slow roast stop sharing one published estimate. Absent
        /// means the venue has not said, and the estimate falls back to the
        /// venue's default rather than treating the dish as instant.
        #[serde(default)]
        cooking_min: Option<i64>,
        /// WHAT IS IN THE DISH, as the venue declares it.
        ///
        /// A customer choosing food asks three things a price cannot answer:
        /// what is in it, how much of it there is, and what it does to their
        /// day. `ingredients` is the venue's own list, `weight_g` the served
        /// weight, `nutrition` the per-portion figures it publishes
        /// (`{"kcal":..,"protein":..,"fat":..,"carbs":..}`, each optional).
        ///
        /// ALL THREE ARE OPTIONAL AND ABSENT IS NOT ZERO. A dish with no
        /// declared protein must render as "not declared", never as "0 g" --
        /// the same rule the allergen list already follows, for the same
        /// reason: a made-up number about food is worse than no number.
        #[serde(default)]
        ingredients: Option<Vec<String>>,
        #[serde(default)]
        weight_g: Option<i64>,
        #[serde(default)]
        nutrition: Option<serde_json::Map<String, Value>>,
        /// The dish's name and description in the venue's OTHER languages:
        /// `{"uk": {"name": "...", "description": "..."}, "en": {...}}`.
        ///
        /// The read side of this has existed since the first catalogue
        /// migration (`content_i18n`) and had no write side at all, which is
        /// why every menu was served in the venue's own language whatever the
        /// customer chose. A locale that is not supplied is left alone; an
        /// EMPTY STRING deletes that translation rather than storing a blank
        /// one, because a dish whose Ukrainian name is "" must fall back to the
        /// venue's own name, not render nameless.
        #[serde(default)]
        translations: Option<std::collections::HashMap<String, std::collections::HashMap<String, String>>>,
        /// What the dish IS, as the venue files it: `salmon`, `hot`,
        /// `vegetarian`, `popular`. The storefront's filter rail is built from
        /// these. Lower-case slugs; an empty list clears them.
        #[serde(default)]
        tags: Option<Vec<String>>,
        /// One portion's recipe: `[{supply, qty}]`. An empty list clears it.
        #[serde(default)]
        bom: Option<Vec<crate::recipe::BomLineIn>>,
        /// Five axes, levels 1…3; absent = not declared.
        #[serde(default)]
        taste: Option<serde_json::Map<String, Value>>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing product id", 400);
    };
    // THE VENUE THAT WAS AUTHORISED, not the one the token happens to name --
    // see `Place::of_authorised`.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    if let Some(p) = body.price {
        // Money is integer minor units and never negative. Refuse rather than clamp.
        if p < 0 {
            return Response::error("price must be >= 0", 400);
        }
    }

    // Checked before the write, so a refused list never reaches the record. A
    // dish tagged `shelfish` would match no filter, so the customer who
    // filtered for shellfish would be shown it as safe.
    let allergens = match &body.allergens {
        None => None,
        Some(raw) => match dowiz_hub::allergens::validate(raw) {
            Ok(v) => Some(v),
            Err(e) => return Response::error(e, 400),
        },
    };
    if let Some(cm) = body.size_cm {
        if !(3..=120).contains(&cm) {
            return Response::error("a dish is between 3 and 120 cm across", 400);
        }
    }
    if let Some(m) = body.cooking_min {
        // A bound, not a clamp. Zero would read as "instant" and four hours is
        // not a dish a delivery app can quote; both are refused so the venue
        // corrects the number rather than the number silently correcting itself.
        if !(1..=240).contains(&m) {
            return Response::error("cooking time is between 1 and 240 minutes", 400);
        }
    }

    if let Some(lines) = &body.bom {
        for l in lines {
            if l.supply.trim().is_empty() || l.qty <= 0 || l.qty > crate::recipe::QTY_MAX {
                return Response::error("a recipe line is a supply and a positive quantity", 400);
            }
        }
    }
    let taste = match &body.taste {
        None => None,
        Some(m) => match crate::recipe::validate_taste(m) {
            Ok(t) => Some(t),
            Err(e) => return Response::error(e, 400),
        },
    };
    let bom = body.bom.clone();
    let want_id = id.clone();
    let price = body.price;
    let available = body.available;
    let note = body.unavailable_note.clone();
    let size_cm = body.size_cm;
    let cooking_min = body.cooking_min;
    let translations = body.translations.clone();
    let ingredients = body.ingredients.clone();
    let weight_g = body.weight_g;
    let nutrition = body.nutrition.clone();
    let tags = match &body.tags {
        None => None,
        Some(list) => {
            let clean: Vec<String> = list
                .iter()
                .map(|t| t.trim().to_lowercase())
                .filter(|t| !t.is_empty())
                .collect();
            if clean.iter().any(|t| t.len() > 32 || !t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')) {
                return Response::error("a tag is a short lower-case slug", 400);
            }
            Some(clean)
        }
    };
    // THE GATE FOLLOWS THE FEATURE. A venue that switched the allergen filter
    // off (this one did, 2026-09-18: no allergens anywhere on its storefront)
    // is not asked to declare what it no longer shows; with the filter on, the
    // refusal below stands, per dish, as before.
    let allergen_gate = match crate::hubstore::load_settings(&place).await {
        Ok(l) => dowiz_hub::features::is_on(&l.settings, "feature.allergen_filter"),
        Err(_) => true,
    };
    let written = crate::hubstore::with_catalog(&place, move |cat| {
        let Some(pj) = cat.product(&want_id) else {
            return Err(Error::RustError("unknown product".into()));
        };
        let mut p: Value = serde_json::from_str(&pj)
            .map_err(|e| Error::RustError(format!("catalogue product unreadable: {e}")))?;
        if let Some(v) = price {
            p["price"] = json!(v);
        }
        if let Some(cm) = size_cm {
            p["sizeCm"] = json!(cm);
        }
        if let Some(m) = cooking_min {
            p["cookingMin"] = json!(m);
        }
        if let Some(list) = &ingredients {
            p["ingredients"] = json!(list);
        }
        if let Some(g) = weight_g {
            p["weightG"] = json!(g);
        }
        if let Some(n) = &nutrition {
            p["nutrition"] = Value::Object(n.clone());
        }
        if let Some(list) = &tags {
            p["tags"] = if list.is_empty() { Value::Null } else { json!(list) };
        }
        if let Some(t) = &taste {
            p["taste"] = if t.is_empty() { Value::Null } else { Value::Object(t.clone()) };
        }
        // ── THE RECIPE, AND WHAT FOLLOWS FROM IT ──
        // Each line is snapshotted from the supply as it is now; the dish's
        // nutrition, ingredient list, weight and cost are summed from the
        // lines. A value the owner typed in the same request wins over the
        // sum, and stays marked as theirs.
        if let Some(lines) = &bom {
            let mut snap = Vec::with_capacity(lines.len());
            for l in lines {
                let Some(sj) = cat.supply(&l.supply) else {
                    return Err(Error::RustError(format!("unknown supply {}", l.supply)));
                };
                let sv: Value = serde_json::from_str(&sj).unwrap_or(json!({}));
                if snap.iter().any(|x: &crate::recipe::Line| x.supply == l.supply) {
                    continue; // one line per supply, as the old editor enforced
                }
                snap.push(crate::recipe::line_of(&l.supply, l.qty, &sv));
            }
            if snap.is_empty() {
                p["bom"] = Value::Null;
                p["nutritionDerived"] = Value::Null;
                p["cost"] = Value::Null;
            } else {
                let d = crate::recipe::derive(&snap);
                p["bom"] = crate::recipe::bom_json(&snap);
                if nutrition.is_none() && d.nutrition_complete {
                    p["nutrition"] = json!({ "kcal": d.kcal, "protein": d.protein, "fat": d.fat, "carbs": d.carbs, "approx": false });
                    p["nutritionDerived"] = json!(true);
                } else if nutrition.is_some() {
                    p["nutritionDerived"] = json!(false);
                }
                if weight_g.is_none() {
                    if let Some(w) = d.weight_g {
                        p["weightG"] = json!(w);
                    }
                }
                // The console always sends the ingredients box, empty or not;
                // an empty box beside a recipe means "use the recipe's names".
                if ingredients.as_ref().is_none_or(|l| l.is_empty()) && !d.ingredients.is_empty() {
                    p["ingredients"] = json!(d.ingredients);
                }
                p["cost"] = d.cost.map(|c| json!(c)).unwrap_or(Value::Null);
                p["nutritionComplete"] = json!(d.nutrition_complete);
            }
        }
        if let Some(list) = &allergens {
            p["allergens"] = json!(list);
        }
        // ── THE ALLERGEN PUBLISH GATE ──
        //
        // A dish nobody has declared cannot go on sale. Not a badge, a refusal:
        // a customer with an allergy cannot tell "we checked and it is clear"
        // from "nobody filled this in", and every surface renders both as no
        // warning. Declaring costs one action and "none of the fourteen" is a
        // valid answer; the gate is per DISH, so a venue is never blocked
        // wholesale.
        if available == Some(true) && allergen_gate {
            let decided = allergens.as_ref().map(|l| {
                if l.is_empty() {
                    dowiz_hub::allergens::Declaration::None
                } else {
                    dowiz_hub::allergens::Declaration::Contains(l.clone())
                }
            });
            let state = decided.unwrap_or_else(|| dowiz_hub::allergens::read(&pj));
            if !state.is_declared() {
                return Err(Error::RustError("undeclared".into()));
            }
        }
        if let Some(a) = available {
            p["available"] = json!(a);
            // Clearing the note when a dish comes back is the point: a stale
            // reason on an available dish reads as a contradiction.
            p["unavailableNote"] = if a { Value::Null } else { json!(note) };
        }
        cat.set_product(&want_id, &serde_json::to_string(&p).unwrap_or(pj));

        // Any catalogue write moves the menu version, which is how a client
        // notices its cart went stale.
        if let Some(lj) = cat.location() {
            if let Ok(mut l) = serde_json::from_str::<Value>(&lj) {
                let v = l.get("menu_version").and_then(|x| x.as_i64()).unwrap_or(1);
                l["menu_version"] = json!(v + 1);
                cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
            }
        }
        Ok(())
    })
    .await;
    // The gate's refusal is a 409 with the sentence that tells the owner what
    // to do, not a 500 with a marker word. The marker only exists because the
    // closure can only fail with `Error`.
    if let Err(e) = written {
        if e.to_string().contains("undeclared") {
            return Response::error(
                "declare this dish's allergens before putting it on sale; \
                 'none of the fourteen' is a valid answer, an empty field is not",
                409,
            );
        }
        if e.to_string().starts_with("unknown supply") {
            return Response::error(e.to_string(), 400);
        }
        if e.to_string().contains("unknown product") {
            return Response::error("not found", 404);
        }
        return Err(e);
    }

    // ── The other languages ──
    //
    // Written AFTER the catalogue, and never inside it: a translation is not
    // part of the priced record, and a locale table that failed must not take
    // a price change down with it. Each row is upserted on its own primary key
    // so re-saving a dish is idempotent, and an empty string DELETES rather
    // than storing a blank the menu would then serve as the dish's name.
    if let Some(by_locale) = &translations {
        // CHECKED FIRST, ALL OF THEM, THEN WRITTEN ONCE. A per-entry write was
        // an image read and write each; worse, a refusal half way through left
        // the earlier languages saved and the later ones not.
        let mut batch = Vec::new();
        for (locale, fields) in by_locale {
            for (field, value) in fields {
                match i18n_check("product", locale, field, value) {
                    Ok(loc) => batch.push((
                        "product".to_string(),
                        id.clone(),
                        loc,
                        field.clone(),
                        value.clone(),
                    )),
                    Err(e) => return Response::error(e, 400),
                }
            }
        }
        apply_i18n(&place, batch).await?;
    }

    Response::from_json(&json!({ "ok": true, "id": id }))
}

/// The fields a translation may carry, and the shape each must have.
/// `ingredients` is a JSON array of strings, checked before it is stored so
/// the menu route never has to guess what it is reading back.
fn i18n_value_ok(field: &str, value: &str) -> std::result::Result<(), String> {
    match field {
        "name" | "description" => Ok(()),
        "ingredients" => match serde_json::from_str::<Value>(value) {
            Ok(Value::Array(a)) if a.iter().all(|x| x.is_string()) => Ok(()),
            _ => Err("ingredients must be a JSON array of strings".into()),
        },
        other => Err(format!("{other:?} is not a translatable field")),
    }
}

/// One translation row: upserted on its own key, DELETED when the value is
/// empty rather than stored as a blank the menu would serve as the dish's
/// name. Products and categories are the two things a customer reads.
/// What a translation must be, decided with no I/O at all.
///
/// LIFTED OUT OF THE WRITE. It used to live inside an `async fn` that needed a
/// database handle, so none of these rules could be tested and the whole
/// function had to be reasoned about rather than exercised. This is the shape
/// the verification blueprint's L1 asks for: a decision as a function of its
/// arguments.
fn i18n_check(
    entity_type: &str,
    locale: &str,
    field: &str,
    value: &str,
) -> std::result::Result<String, String> {
    if !matches!(entity_type, "product" | "category") {
        return Err(format!("{entity_type:?} is not translatable"));
    }
    let locale = locale.trim().to_lowercase();
    if locale.len() != 2 || !locale.chars().all(|c| c.is_ascii_lowercase()) {
        return Err(format!("{locale:?} is not a locale"));
    }
    if !value.trim().is_empty() {
        i18n_value_ok(field, value)?;
    }
    Ok(locale)
}

/// Apply a batch of translations to the venue's own image, in ONE write.
///
/// ONE IMAGE, ONE TRANSACTION. Per-entry writes would have been one image read
/// and one image write EACH -- a bulk save of a 165-dish menu in three
/// languages is fifteen hundred of them. They are also not atomic: a failure
/// half way would leave a dish named in one language and not the next.
///
/// An empty value REMOVES the entry rather than storing a blank the menu would
/// then serve as the dish's name -- the same rule the DELETE arm of the old
/// upsert had, now visible in one place.
async fn apply_i18n(
    place: &crate::hubstore::Place,
    entries: Vec<(String, String, String, String, String)>,
) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    crate::hubstore::with_table(
        place,
        crate::hubstore::IMAGE_I18N,
        crate::hubstore::I18N_BYTES,
        move |t| {
            for (entity_type, id, locale, field, value) in &entries {
                let key = crate::hubstore::i18n_key(locale, entity_type, id, field);
                if value.trim().is_empty() {
                    t.remove(crate::hubstore::I18N_KIND, &key);
                } else {
                    t.put(crate::hubstore::I18N_KIND, &key, value, &[], &[])
                        .map_err(|e| Error::RustError(format!("i18n: {e}")))?;
                }
            }
            Ok(())
        },
    )
    .await
}

/// `POST /api/owner/i18n` -- the venue's other languages, in bulk.
///
/// `{ "location_id": "...", "entries": [ { "entity": "category"|"product",
///   "id": "...", "locale": "uk", "field": "name", "value": "..." }, ... ] }`
///
/// The per-product route can carry a dish's own translations; a category
/// heading had no write side at all, so every heading stayed in the venue's
/// language whatever the customer chose. This is the one place both are
/// written, up to five hundred rows a call, each checked on its own so a bad
/// row is named rather than the whole batch silently half-applied.
pub async fn write_translations(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    struct Entry {
        entity: String,
        id: String,
        locale: String,
        field: String,
        #[serde(default)]
        value: String,
    }
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        entries: Vec<Entry>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    if body.entries.len() > 500 {
        return Response::error("at most 500 entries per call", 400);
    }
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    // Only ids the catalogue actually has: a translation of a dish that does
    // not exist is a row nothing will ever read.
    //
    // THE AUTHORISED VENUE, not the token's. Its four siblings were switched
    // to `of_authorised` when that helper was written and this one was missed:
    // `owner_at` authorises `body.location_id` while `of_any` resolved the
    // catalogue from the CLAIM, so for an owner of two venues the ids were
    // checked against the wrong restaurant. `content_i18n` has no venue column
    // (migration 0002), so this id check is the only thing keeping a
    // translation in its own venue.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    let loaded = crate::hubstore::load_catalog(&place).await?;
    let products: std::collections::BTreeSet<String> =
        loaded.catalog.products().into_iter().map(|(id, _)| id).collect();
    let categories: std::collections::BTreeSet<String> =
        loaded.catalog.categories().into_iter().map(|(id, _)| id).collect();
    let mut refused: Vec<Value> = Vec::new();
    let mut batch = Vec::new();
    for e in &body.entries {
        let known = match e.entity.as_str() {
            "product" => products.contains(&e.id),
            "category" => categories.contains(&e.id),
            _ => false,
        };
        if !known {
            refused.push(json!({ "id": e.id, "why": format!("unknown {}", e.entity) }));
            continue;
        }
        match i18n_check(&e.entity, &e.locale, &e.field, &e.value) {
            Ok(locale) => batch.push((
                e.entity.clone(),
                e.id.clone(),
                locale,
                e.field.clone(),
                e.value.clone(),
            )),
            Err(why) => refused.push(json!({ "id": e.id, "field": e.field, "why": why })),
        }
    }
    // The ids were checked against THE VENUE'S OWN catalogue above, and the
    // image this lands in is that venue's. Two independent reasons a
    // translation cannot reach another restaurant, where the table had none.
    let written = batch.len();
    apply_i18n(&place, batch).await?;
    Response::from_json(&json!({ "ok": refused.is_empty(), "written": written, "refused": refused }))
}

/// `PATCH /api/owner/location` — open, close, go busy, pause delivery.
pub async fn update_location(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    #[derive(Deserialize)]
    struct In {
        location_id: String,
        #[serde(default)]
        status: Option<String>,
        #[serde(default)]
        delivery_paused: Option<bool>,
        /// The number a customer rings when the app cannot help them, and one
        /// of the two ways a venue satisfies the notifications leg.
        #[serde(default)]
        phone: Option<String>,
        /// Can a customer come and collect? The other half of fulfilment.
        #[serde(default)]
        pickup: Option<bool>,
        /// The venue's name as the customer should read it -- the one on its
        /// sign and its Google listing, not the slug the hub was created under.
        #[serde(default)]
        name: Option<String>,
        /// Delivery terms, in minor units. Each one optional; `null` for the
        /// threshold means "no free delivery".
        #[serde(default)]
        delivery_fee: Option<i64>,
        #[serde(default, deserialize_with = "deserialize_some")]
        free_delivery_threshold: Option<Option<i64>>,
        #[serde(default)]
        min_order: Option<i64>,
        /// The venue's IANA time zone, e.g. `Europe/Tirane`.
        ///
        /// THE ONLY PLACE LOCAL TIME COMES FROM. It used to be the constant
        /// `2 * 60 * 60 * 1000` in three separate files, one of them labelled
        /// "standard time" while carrying the summer offset. The name is
        /// refused here if this build does not know its rule, because a venue
        /// quietly keeping another country's hours is exactly the failure the
        /// constant already was.
        #[serde(default)]
        timezone: Option<String>,
        /// The venue's crypto wallets: `[{network, symbol, address, note?}]`.
        /// An empty list switches the rail off. Checked here so a wallet with
        /// no address never reaches a customer as a way to pay.
        #[serde(default)]
        crypto_wallets: Option<Vec<Value>>,
        /// THE VENUE'S STAGE: what its storefront puts around its mark.
        /// `{ "seal": "ドウビン", "motif": "leaf", "warm": "#e0754d", "sage": "#8a9a7b" }`
        /// -- a short seal text drawn as a vertical stamp beside the mark, an
        /// ornament for the rule, and two supporting colours taken from the
        /// mark itself. A storefront is one venue's, so these are the venue's
        /// to set; an absent block leaves the five brand tokens alone.
        #[serde(default)]
        stage: Option<Value>,
    }
    let body: In = match req.json().await {
        Ok(b) => b,
        Err(e) => return Response::error(format!("bad request body: {e}"), 400),
    };
    // THE VENUE THAT WAS AUTHORISED, not the one the token happens to name --
    // see `Place::of_authorised`.
    let place = crate::hubstore::Place::of_authorised(&ctx, &body.location_id)?;
    if let Err(r) = owner_at(&req, &ctx, &body.location_id).await {
        return Ok(r);
    }
    let stage = match &body.stage {
        None => None,
        Some(v) => match clean_stage(v) {
            Ok(s) => Some(s),
            Err(e) => return Response::error(e, 400),
        },
    };
    if let Some(n) = &body.name {
        let n = n.trim();
        if n.is_empty() || n.chars().count() > 80 {
            return Response::error("a venue name is 1 to 80 characters", 400);
        }
    }
    for (what, v) in [("delivery_fee", body.delivery_fee), ("min_order", body.min_order),
                      ("free_delivery_threshold", body.free_delivery_threshold.flatten())] {
        if let Some(v) = v {
            if v < 0 {
                return Response::error(format!("{what} must be >= 0"), 400);
            }
        }
    }
    if let Some(list) = &body.crypto_wallets {
        if list.len() > 12 {
            return Response::error("at most 12 wallets", 400);
        }
        for w in list {
            let s = |k: &str| w.get(k).and_then(Value::as_str).map(str::trim).unwrap_or("");
            if s("network").is_empty() || s("symbol").is_empty() || s("address").is_empty() {
                return Response::error("a wallet needs a network, a symbol and an address", 400);
            }
            if s("address").len() > 128 || s("symbol").len() > 12 || s("network").len() > 40 {
                return Response::error("a wallet field is too long", 400);
            }
        }
    }
    if let Some(st) = &body.status {
        if !matches!(st.as_str(), "open" | "closed" | "busy") {
            return Response::error("status must be open, closed or busy", 400);
        }
    }
    if let Some(tz) = &body.timezone {
        if dowiz_hub::tz::zone(tz.trim()).is_none() {
            return Response::error(
                format!(
                    "unknown time zone {:?}. Known: {}",
                    tz.trim(),
                    dowiz_hub::tz::NAMES.join(", ")
                ),
                400,
            );
        }
    }
    if let Some(ph) = &body.phone {
        // Empty CLEARS it -- a venue with no phone should be able to say so
        // rather than keep a number that no longer answers.
        if !ph.trim().is_empty() && ph.chars().filter(|c| c.is_ascii_digit()).count() < 8 {
            return Response::error("that does not look like a phone number", 400);
        }
    }
    let status = body.status.clone();
    let paused = body.delivery_paused;
    let phone = body.phone.clone();
    let pickup = body.pickup;
    let name = body.name.as_deref().map(str::trim).map(String::from);
    let timezone = body.timezone.as_deref().map(str::trim).map(String::from);
    let delivery_fee = body.delivery_fee;
    let free_th = body.free_delivery_threshold;
    let min_order = body.min_order;
    let wallets: Option<Vec<Value>> = body.crypto_wallets.as_ref().map(|list| {
        list.iter()
            .map(|w| {
                let s = |k: &str| w.get(k).and_then(Value::as_str).map(str::trim).unwrap_or("").to_string();
                json!({ "network": s("network"), "symbol": s("symbol").to_uppercase(),
                        "address": s("address"),
                        "note": w.get("note").and_then(Value::as_str).map(str::trim)
                            .filter(|n| !n.is_empty()).map(|n| json!(n)).unwrap_or(Value::Null) })
            })
            .collect()
    });
    // The registry record is a pointer with a name on it, and the platform
    // console lists venues from it: renamed in the same request so the two
    // never disagree about what the venue is called.
    if let Some(n) = &name {
        let loc_for_name = body.location_id.clone();
        let name_for_registry = n.clone();
        let _ = crate::identity_store::with_registry(&ctx.env, move |t| {
            if let Some(mut r) =
                crate::identity_store::rec(t, crate::identity_store::K_LOC, &loc_for_name)
            {
                r["name"] = json!(name_for_registry);
                r["updated_at_ms"] = json!(ctx.data.now_ms);
                let slug = crate::identity_store::s_of(&r, "slug");
                let index = vec![(
                    crate::identity_store::loc_by_slug(&slug),
                    loc_for_name.clone(),
                )];
                t.put(
                    crate::identity_store::K_LOC,
                    &loc_for_name,
                    &r.to_string(),
                    &index,
                    &[],
                )
                .map_err(|e| Error::RustError(format!("registry: {e}")))?;
            }
            Ok(())
        })
        .await;
    }
    crate::hubstore::with_catalog(&place, move |cat| {
        let Some(lj) = cat.location() else {
            return Err(Error::RustError("no venue in the catalogue".into()));
        };
        let mut l: Value = serde_json::from_str(&lj)
            .map_err(|e| Error::RustError(format!("catalogue location unreadable: {e}")))?;
        if let Some(st) = &status {
            l["status"] = json!(st);
        }
        if let Some(p) = paused {
            l["delivery_paused"] = json!(if p { 1 } else { 0 });
        }
        if let Some(ph) = &phone {
            l["phone"] = if ph.trim().is_empty() { Value::Null } else { json!(ph.trim()) };
        }
        if let Some(p) = pickup {
            l["pickup"] = json!(p);
        }
        if let Some(n) = &name {
            l["name"] = json!(n);
        }
        if let Some(tz) = &timezone {
            l["tz"] = json!(tz);
        }
        if let Some(f) = delivery_fee {
            l["delivery_fee"] = json!(f);
        }
        if let Some(th) = free_th {
            l["free_delivery_threshold"] = th.map(|v| json!(v)).unwrap_or(Value::Null);
        }
        if let Some(m) = min_order {
            l["min_order"] = json!(m);
        }
        if let Some(w) = &wallets {
            if !l.get("payments").map_or(false, Value::is_object) {
                l["payments"] = json!({});
            }
            l["payments"]["crypto"] = json!(w);
        }
        if let Some(s) = &stage {
            l["stage"] = s.clone();
        }
        // Delivery terms are read by every basket, so they move the menu
        // version the way a price does.
        if name.is_some() || delivery_fee.is_some() || free_th.is_some() || min_order.is_some() {
            let v = l.get("menu_version").and_then(|x| x.as_i64()).unwrap_or(1);
            l["menu_version"] = json!(v + 1);
        }
        cat.set_location(&serde_json::to_string(&l).unwrap_or(lj));
        Ok(())
    })
    .await?;

    Response::from_json(&json!({ "ok": true }))
}

/// The stage's bounds. A seal is a stamp, not a sentence; the motifs are the
/// ones the storefront can draw; a colour is six hex digits.
const STAGE_SEAL_MAX_CHARS: usize = 12;
const STAGE_MOTIFS: [&str; 3] = ["leaf", "wave", "none"];

fn hex_colour(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Only the four keys, each checked, so nothing reaches the storefront's CSS
/// that is not a short text, a named motif or a colour.
fn clean_stage(v: &Value) -> std::result::Result<Value, String> {
    let Some(o) = v.as_object() else { return Err("stage must be an object".into()) };
    let mut out = serde_json::Map::new();
    if let Some(seal) = o.get("seal").and_then(Value::as_str).map(str::trim) {
        if seal.chars().count() > STAGE_SEAL_MAX_CHARS {
            return Err(format!("a seal is at most {STAGE_SEAL_MAX_CHARS} characters"));
        }
        if !seal.is_empty() {
            out.insert("seal".into(), json!(seal));
        }
    }
    if let Some(m) = o.get("motif").and_then(Value::as_str).map(str::trim) {
        if !STAGE_MOTIFS.contains(&m) {
            return Err(format!("motif must be one of {}", STAGE_MOTIFS.join(", ")));
        }
        out.insert("motif".into(), json!(m));
    }
    for key in ["warm", "sage"] {
        if let Some(c) = o.get(key).and_then(Value::as_str).map(str::trim) {
            if !hex_colour(c) {
                return Err(format!("{key} must be a #rrggbb colour"));
            }
            out.insert(key.into(), json!(c.to_lowercase()));
        }
    }
    Ok(Value::Object(out))
}

/// `Option<Option<T>>`: absent means "leave it", `null` means "clear it".
fn deserialize_some<'de, T, D>(d: D) -> std::result::Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}
