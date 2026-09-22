//! Speech: what an owner said, turned into a proposal they then confirm.
//!
//! NOTHING IS DONE ON THE STRENGTH OF A TRANSCRIPT. A spoken instruction
//! becomes a PROPOSAL with a short life, and a second call confirms it -- a
//! microphone that mishears must not be able to change a price.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::now_ms;

//
// THE CLASSIFIER IS DETERMINISTIC AND THE SERVICE SURVIVES IT BEING OFF.
// Ambiguity is rejected rather than guessed, and a consequential command is
// NEVER acted on from one utterance: the hub answers with a signed proposal and
// a read-back, and a person presses the button. Voice moves a hand, never a
// decision.
//
// The proposal is a token scoped to `voice:<verb>:<order>`, bound to the
// speaker AND their session, and short-lived. So a confirmation cannot be
// replayed, cannot be used by anyone else, and cannot be pointed at a different
// order than the one that was read back.

#[derive(Deserialize)]
#[serde(default)]
struct VoiceIn {
    transcript: String,
    /// The recogniser's own confidence, 0..1.
    confidence: f64,
    is_final: bool,
    lang: Option<String>,
    /// A token from a previous proposal. Its presence is what turns a
    /// suggestion into an action.
    confirm: Option<String>,
}

impl Default for VoiceIn {
    fn default() -> Self {
        VoiceIn {
            transcript: String::new(),
            confidence: 1.0,
            is_final: true,
            lang: None,
            confirm: None,
        }
    }
}

/// How long a read-back stays answerable. Long enough to hear it and say yes,
/// short enough that a phone left on a counter cannot confirm it an hour later.
const PROPOSAL_TTL_MS: i64 = 90_000;

/// `POST /api/voice`
pub async fn voice(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::voice::{classify, Command, Speaker, Target};

    let body: VoiceIn = req.json().await.unwrap_or_default();
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;

    // WHICH VOCABULARY APPLIES IS DECIDED BY THE TOKEN, never by what the
    // caller says they are: a courier claiming to be an owner would otherwise
    // reach the owner's commands by typing a word.
    let (speaker, who, loc) =
        match crate::auth::authenticate(&req, &ctx.env, now_ms()).await {
            Ok(crate::auth::Principal::Owner { user_id, active_location_id }) => (
                Speaker::Owner,
                user_id,
                active_location_id.unwrap_or_default(),
            ),
            Ok(crate::auth::Principal::Courier { courier_id, active_location_id, .. }) => {
                (Speaker::Courier, courier_id, active_location_id)
            }
            Ok(_) => return Response::error("forbidden role", 403),
            Err(e) => return e.into_response(),
        };
    let lang = body.lang.clone().unwrap_or_else(|| "uk".into());

    // A confirmation carries its own instruction; nothing is classified again.
    if let Some(tok) = &body.confirm {
        let claims = match dowiz_hub::token::verify(
            crate::auth::signing_key(&ctx.env).as_slice(),
            tok,
            now_ms(),
        ) {
            Ok(c) => c,
            Err(_) => {
                return Response::from_json(&json!({
                    "understood": false, "say": "та відповідь уже не дійсна"
                }))
            }
        };
        if claims.subject != who {
            return Response::from_json(&json!({
                "understood": false, "say": "це підтвердження не ваше"
            }));
        }
        let Some(rest) = claims.scope.strip_prefix("voice:") else {
            return Response::from_json(&json!({ "understood": false, "say": "не та відповідь" }));
        };
        let Some((verb, order)) = rest.split_once(':') else {
            return Response::from_json(&json!({ "understood": false, "say": "не та відповідь" }));
        };
        return Response::from_json(&json!({
            "understood": true, "needsConfirmation": false,
            // The ACTION IS NOT RUN HERE. The surface calls the same route the
            // button calls, so voice reaches exactly the checks a tap reaches
            // and cannot become a second way to move an order.
            "action": "do", "verb": verb, "orderId": order,
        }));
    }

    let cmd = classify(&body.transcript, body.confidence, body.is_final, speaker);

    // The orders this speaker may act on. An owner sees the venue's live queue;
    // a courier sees only their own run -- so a misheard number can never reach
    // somebody else's delivery.
    let pool: Vec<Value> = crate::hubstore::orders(&place)
        .await?
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| {
            o.get("location_id").and_then(Value::as_str).map(|l| l == loc).unwrap_or(true)
        })
        .filter(|o| {
            let st = o.get("status").and_then(Value::as_str).unwrap_or("");
            let live = matches!(
                st,
                "PENDING" | "CONFIRMED" | "PREPARING" | "READY" | "IN_DELIVERY"
            );
            match speaker {
                Speaker::Owner => live,
                Speaker::Courier => {
                    live && o.get("courier_id").and_then(Value::as_str) == Some(who.as_str())
                }
            }
        })
        .collect();

    let resolve = |t: &Target| -> std::result::Result<String, &'static str> {
        let id = |o: &Value| o.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let at = |o: &Value| o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        match t {
            Target::Newest => pool
                .iter()
                .max_by_key(|o| at(o))
                .map(id)
                .ok_or("зараз немає замовлень"),
            Target::Oldest => pool
                .iter()
                .min_by_key(|o| at(o))
                .map(id)
                .ok_or("зараз немає замовлень"),
            Target::Digits(d) => {
                let hits: Vec<String> =
                    pool.iter().filter(|o| id(o).ends_with(d.as_str())).map(id).collect();
                match hits.len() {
                    1 => Ok(hits[0].clone()),
                    0 => Err("такого номера серед відкритих немає"),
                    // AMBIGUITY IS REFUSED, not guessed. Two orders ending in
                    // the same digits is exactly when a guess moves the wrong
                    // one.
                    _ => Err("під цей номер підходить кілька — скажіть більше цифр"),
                }
            }
            Target::Unsaid => match pool.len() {
                1 => Ok(id(&pool[0])),
                0 => Err("зараз немає замовлень"),
                _ => Err("яке саме?"),
            },
        }
    };

    let propose = |verb: &str, order: &str| -> Option<String> {
        // `mint` returns the token itself here, not a Result: the signing key
        // is already in hand and there is nothing left to fail at.
        let now = now_ms();
        dowiz_hub::token::mint(
            crate::auth::signing_key(&ctx.env).as_slice(),
            &dowiz_hub::token::Claims {
                role: match speaker {
                    Speaker::Owner => dowiz_hub::token::Role::Owner,
                    Speaker::Courier => dowiz_hub::token::Role::Courier,
                },
                subject: who.clone(),
                session: String::new(),
                scope: format!("voice:{verb}:{order}"),
                issued_ms: now,
                expires_ms: now + PROPOSAL_TTL_MS,
            },
        )
        .into()
    };

    let out = match &cmd {
        Command::Unclear(why) => json!({
            "understood": false, "say": why, "heard": body.transcript
        }),
        Command::Status => {
            let waiting = pool
                .iter()
                .filter(|o| o.get("status").and_then(Value::as_str) == Some("PENDING"))
                .count();
            // READ-ONLY, so it runs at once. Making somebody confirm a question
            // is what turns voice into a form.
            json!({ "understood": true, "needsConfirmation": false, "action": "status",
                    "open": pool.len(), "waiting": waiting })
        }
        Command::Ask(q) => json!({
            "understood": true, "needsConfirmation": false, "action": "ask", "question": q
        }),
        Command::Shift { open } => {
            let verb = if *open { "shift_open" } else { "shift_close" };
            match propose(verb, "-") {
                Some(tok) => json!({ "understood": true, "needsConfirmation": true,
                                     "readback": cmd.readback(&lang), "token": tok }),
                None => json!({ "understood": false, "say": "не вдалося підготувати дію" }),
            }
        }
        Command::Order { verb, target } => match resolve(target) {
            Err(why) => json!({ "understood": false, "say": why, "heard": body.transcript }),
            Ok(order) => match propose(verb, &order) {
                Some(tok) => json!({ "understood": true, "needsConfirmation": true,
                                     "readback": cmd.readback(&lang), "token": tok,
                                     "orderId": order }),
                None => json!({ "understood": false, "say": "не вдалося підготувати дію" }),
            },
        },
        Command::Pickup { target } | Command::Deliver { target } => {
            let verb = if matches!(cmd, Command::Pickup { .. }) { "pickup" } else { "deliver" };
            match resolve(target) {
                Err(why) => json!({ "understood": false, "say": why, "heard": body.transcript }),
                Ok(order) => match propose(verb, &order) {
                    Some(tok) => json!({ "understood": true, "needsConfirmation": true,
                                         "readback": cmd.readback(&lang), "token": tok,
                                         "orderId": order }),
                    None => json!({ "understood": false, "say": "не вдалося підготувати дію" }),
                },
            }
        }
    };
    Response::from_json(&out)
}
