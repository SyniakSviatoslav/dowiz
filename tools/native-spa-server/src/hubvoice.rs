//! The voice surface, for every role.
//!
//! ONE ENDPOINT, both roles, all three surfaces. The operator asked whether the
//! service can be driven by voice regardless of role and surface; this is the
//! seam that makes that one implementation rather than three.
//!
//! VOICE PROPOSES, A PERSON DISPOSES. A consequential command comes back as a
//! PROPOSAL carrying a signed token and a read-back line in the speaker's
//! language. Nothing moves until that token comes back. This is P64's rule and
//! the reason it exists is ordinary: a phone in a kitchen mishears, and
//! "cancel" and "confirm" sound alike in every language this speaks.
//!
//! THE TOKEN IS SIGNED AND SHORT-LIVED, and it binds the proposal to the exact
//! order and verb that were read back. Without that, a confirmation could be
//! replayed against a different order, or an attacker who could reach the
//! endpoint could skip the proposal step entirely and just send `confirm`.
//!
//! RECOGNITION HAPPENS IN THE BROWSER. The Web Speech API is already on the
//! phones this runs on, works in the user's own language, and costs nothing;
//! shipping audio to the hub would add a model, a dependency and a privacy
//! question for no gain. The hub receives TEXT and a confidence score.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use dowiz_core::order_machine::OrderStatus;
use dowiz_hub::token::{self, Claims, Role};
use dowiz_hub::voice::{self, Command, Speaker, Target};

use crate::hub::{now_ms, HubHttpError, Shared};
use crate::hubauth::Caller;

/// How long a spoken proposal stays confirmable.
///
/// Long enough to read it and say yes, short enough that a proposal cannot be
/// left lying around and confirmed into a different situation. An order that
/// was pending a minute ago may be rejected now.
const PROPOSAL_TTL_MS: i64 = 90_000;

#[derive(Deserialize)]
pub struct VoiceIn {
    #[serde(default)]
    pub transcript: String,
    /// The recogniser's own confidence, 0..1.
    #[serde(default = "one")]
    pub confidence: f64,
    #[serde(default = "yes")]
    pub is_final: bool,
    #[serde(default)]
    pub lang: Option<String>,
    /// A token from a previous proposal. Its presence is what turns a
    /// suggestion into an action.
    #[serde(default)]
    pub confirm: Option<String>,
}

fn one() -> f64 {
    1.0
}
fn yes() -> bool {
    true
}

/// Encode a proposal into the `scope` of a signed token.
///
/// Reusing the hub's own token machinery rather than inventing a second signed
/// format: it is already tested, already rotates with the signing key, and
/// already refuses a tampered payload byte by byte.
fn propose(st: &Shared, caller: &Caller, verb: &str, order_id: &str) -> String {
    let now = now_ms();
    token::mint(
        st.signing_key(),
        &Claims {
            role: caller.person.role,
            subject: caller.person.id.clone(),
            session: caller.session.clone(),
            scope: format!("voice:{verb}:{order_id}"),
            issued_ms: now,
            expires_ms: now + PROPOSAL_TTL_MS,
        },
    )
}

/// Read a proposal back, refusing anything that is not exactly what was signed.
fn accept(st: &Shared, caller: &Caller, tok: &str) -> Result<(String, String), HubHttpError> {
    let claims = token::verify(st.signing_key(), tok, now_ms())
        .map_err(|_| HubHttpError::Refused("that confirmation is no longer valid".into()))?;
    // BOUND TO THE SPEAKER. A proposal made to one person cannot be confirmed
    // by another, even one with the same role.
    if claims.subject != caller.person.id || claims.session != caller.session {
        return Err(HubHttpError::Refused("that confirmation belongs to someone else".into()));
    }
    let rest = claims
        .scope
        .strip_prefix("voice:")
        .ok_or_else(|| HubHttpError::Refused("not a voice confirmation".into()))?;
    let (verb, id) = rest
        .split_once(':')
        .ok_or_else(|| HubHttpError::Refused("malformed confirmation".into()))?;
    Ok((verb.to_string(), id.to_string()))
}

/// Which orders this speaker could possibly have meant.
async fn candidates(st: &Shared, caller: &Caller) -> Result<Vec<Value>, HubHttpError> {
    let hub = st.read_log()?;
    let me = caller.person.id.as_str();
    let mut out: Vec<Value> = hub
        .orders()
        .iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| {
            let s = o.get("status").and_then(Value::as_str).unwrap_or("");
            !crate::ostatus::is_terminal(s)
        })
        .filter(|o| match caller.person.role {
            // A courier can only ever mean one of their own runs.
            Role::Courier => o.get("courier_id").and_then(Value::as_str) == Some(me),
            _ => true,
        })
        .collect();
    out.sort_by_key(|o| o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0));
    Ok(out)
}

/// Resolve what the speaker referred to, or say why it cannot be resolved.
///
/// AMBIGUITY IS AN ANSWER. "Confirm it" with three orders waiting is not a
/// command to confirm one of them at random; it is a question the person has to
/// finish. Guessing here is the single most expensive thing this module could
/// do.
fn resolve(target: &Target, mut pool: Vec<Value>) -> Result<Value, &'static str> {
    if pool.is_empty() {
        return Err("no open orders");
    }
    match target {
        Target::Newest => Ok(pool.pop().expect("non-empty")),
        Target::Oldest => Ok(pool.remove(0)),
        Target::Digits(d) => {
            let hits: Vec<Value> = pool
                .into_iter()
                .filter(|o| {
                    o.get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| id.ends_with(d.as_str()) || id.contains(d.as_str()))
                })
                .collect();
            match hits.len() {
                0 => Err("no order with that number"),
                1 => Ok(hits.into_iter().next().expect("one")),
                _ => Err("more than one order matches that number"),
            }
        }
        Target::Unsaid => {
            if pool.len() == 1 {
                Ok(pool.into_iter().next().expect("one"))
            } else {
                Err("which order?")
            }
        }
    }
}

/// `POST /api/voice`.
pub async fn endpoint(
    State(st): State<Shared>,
    caller: Caller,
    Json(body): Json<VoiceIn>,
) -> Result<Json<Value>, HubHttpError> {
    let lang = body.lang.clone().unwrap_or_else(|| "uk".into());

    // A confirmation arriving: act, and do not re-read the transcript. The
    // words were classified once, read back once, and agreed to once.
    if let Some(tok) = &body.confirm {
        let (verb, id) = accept(&st, &caller, tok)?;
        return run(&st, &caller, &verb, &id).await;
    }

    let who = match caller.person.role {
        Role::Courier => Speaker::Courier,
        _ => Speaker::Owner,
    };
    let cmd = voice::classify(&body.transcript, body.confidence, body.is_final, who);

    match &cmd {
        Command::Unclear(why) => Ok(Json(json!({
            "understood": false,
            "say": *why,
            // Echoed back so the surface can show what it thought it heard --
            // which is how a person learns to speak to it, instead of repeating
            // the same misheard phrase louder.
            "heard": body.transcript
        }))),

        // Read-only: run it now. Making someone confirm a question is the kind
        // of ceremony that makes people stop using voice.
        Command::Status => {
            let pool = candidates(&st, &caller).await?;
            let waiting = pool
                .iter()
                .filter(|o| o.get("status").and_then(Value::as_str) == Some(OrderStatus::Pending.as_str()))
                .count();
            Ok(Json(json!({
                "understood": true, "action": "status", "needsConfirmation": false,
                "open": pool.len(), "waiting": waiting,
                "oldestMinutes": pool.first()
                    .and_then(|o| o.get("created_at_ms").and_then(Value::as_i64))
                    .map(|c| (now_ms() - c) / 60_000)
            })))
        }

        Command::Ask(q) => Ok(Json(json!({
            "understood": true, "action": "ask", "needsConfirmation": false,
            // NOT answered here. The surface decides whether to send it to the
            // assistant, which may be switched off -- and voice must work when
            // it is. That is P64's "voice survives AiMode::Off", kept by having
            // no path from here to a model.
            "question": q
        }))),

        Command::Shift { open } => {
            let t = propose(&st, &caller, if *open { "shift_open" } else { "shift_close" }, "-");
            Ok(Json(json!({
                "understood": true, "action": "shift", "needsConfirmation": true,
                "readback": cmd.readback(&lang), "token": t
            })))
        }

        Command::Order { .. } | Command::Pickup { .. } | Command::Deliver { .. } => {
            // Destructured separately rather than in the pattern: the three
            // arms carry different fields, and one `verb` binding across them
            // would have to be invented for two of the three.
            let (verb, target) = match &cmd {
                Command::Order { verb, target } => (*verb, target),
                Command::Pickup { target } => ("pickup", target),
                Command::Deliver { target } => ("deliver", target),
                _ => unreachable!("guarded by the arm above"),
            };
            let pool = candidates(&st, &caller).await?;
            let order = match resolve(target, pool) {
                Ok(o) => o,
                Err(why) => {
                    return Ok(Json(json!({ "understood": false, "say": why, "heard": body.transcript })))
                }
            };
            let id = order.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
            let t = propose(&st, &caller, verb, &id);
            Ok(Json(json!({
                "understood": true, "action": verb, "needsConfirmation": true,
                "orderId": id,
                // The read-back names the order that was RESOLVED, not the words
                // that were said: "confirm the newest" becomes "confirm #4821",
                // so the person agrees to the actual thing.
                "readback": format!("{} · #{}", cmd.readback(&lang), &id[id.len().saturating_sub(4)..]),
                "token": t
            })))
        }
    }
}

/// Carry out a confirmed proposal, through the same handlers every other
/// surface uses.
async fn run(
    st: &Shared,
    caller: &Caller,
    verb: &str,
    id: &str,
) -> Result<Json<Value>, HubHttpError> {
    match verb {
        "shift_open" | "shift_close" => {
            if caller.person.role != Role::Courier {
                return Err(HubHttpError::Refused("only a courier has a shift".into()));
            }
            st.set_shift(&caller.person.id, verb == "shift_open").await;
            Ok(Json(json!({ "done": true, "action": verb })))
        }
        "pickup" | "deliver" => {
            if caller.person.role != Role::Courier {
                return Err(HubHttpError::Refused("that is the courier's to say".into()));
            }
            let target = if verb == "pickup" { OrderStatus::InDelivery } else { OrderStatus::Delivered };
            crate::hubcourier::advance_confirmed(st, caller.person.id.clone(), id.to_string(), target)
                .await
                .map(|v| Json(json!({ "done": true, "action": verb, "order": v })))
        }
        other => {
            if caller.person.role != Role::Owner {
                return Err(HubHttpError::Refused("that is the kitchen's to decide".into()));
            }
            // A rejection needs a reason, and voice has not got one. Saying so
            // is better than inventing "rejected by voice" and showing it to a
            // customer.
            if other == "reject" {
                return Err(HubHttpError::Invalid(
                    "a rejection needs a reason; do that one on screen".into(),
                ));
            }
            crate::hubowner::apply_owner_action(st, &caller.person.id, id, other, None)
                .await
                .map(|v| Json(json!({ "done": true, "action": other, "order": v })))
        }
    }
}

pub fn routes(state: Shared) -> Router {
    Router::new().route("/api/voice", post(endpoint)).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn order(id: &str, at: i64) -> Value {
        json!({ "id": id, "status": "PENDING", "created_at_ms": at })
    }

    #[test]
    fn newest_and_oldest_pick_the_ends() {
        let pool = vec![order("ord_a", 1), order("ord_b", 2), order("ord_c", 3)];
        assert_eq!(resolve(&Target::Newest, pool.clone()).unwrap()["id"], "ord_c");
        assert_eq!(resolve(&Target::Oldest, pool).unwrap()["id"], "ord_a");
    }

    #[test]
    fn digits_match_the_tail_of_an_id() {
        let pool = vec![order("ord_1111_4821", 1), order("ord_2222_9999", 2)];
        assert_eq!(resolve(&Target::Digits("4821".into()), pool.clone()).unwrap()["id"], "ord_1111_4821");
        assert_eq!(resolve(&Target::Digits("7777".into()), pool), Err("no order with that number"));
    }

    /// The expensive mistake this refuses to make.
    #[test]
    fn an_unqualified_reference_with_several_orders_is_a_question() {
        let pool = vec![order("ord_a", 1), order("ord_b", 2)];
        assert_eq!(resolve(&Target::Unsaid, pool), Err("which order?"));
        // With exactly one candidate there is nothing to guess.
        assert_eq!(resolve(&Target::Unsaid, vec![order("ord_a", 1)]).unwrap()["id"], "ord_a");
    }

    #[test]
    fn digits_matching_two_orders_are_refused() {
        let pool = vec![order("ord_4821", 1), order("ord_x4821", 2)];
        assert_eq!(
            resolve(&Target::Digits("4821".into()), pool),
            Err("more than one order matches that number")
        );
    }

    #[test]
    fn nothing_open_says_so() {
        assert_eq!(resolve(&Target::Newest, vec![]), Err("no open orders"));
        assert_eq!(resolve(&Target::Unsaid, vec![]), Err("no open orders"));
    }
}
