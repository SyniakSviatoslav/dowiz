//! Speech: what an owner, a courier or a waiter said, turned into a proposal
//! they then confirm.
//!
//! NOTHING IS DONE ON THE STRENGTH OF A TRANSCRIPT. A spoken instruction
//! becomes a PROPOSAL with a short life, and a second call confirms it -- a
//! microphone that mishears must not be able to change a price.
//!
//! THE CLASSIFIER IS DETERMINISTIC AND THE SERVICE SURVIVES IT BEING OFF. Ambiguity is
//! rejected rather than guessed, and a consequential command is NEVER acted on from one
//! utterance: the hub answers with a signed proposal and a read-back, and a person presses
//! the button. Voice moves a hand, never a decision. The proposal is a token scoped to
//! `voice:<verb>:<arg>`, bound to the speaker, their role AND short-lived: a confirmation
//! cannot be replayed, used by anyone else, or pointed at another order than the one read back.
//!
//! THREE GRAMMARS. The hub's (`dowiz_hub::voice`) reads the owner's order verbs and the
//! courier's run; `grammar` adds the owner's stop-list and venue state and the waiter's room
//! (open, add, send, paid). They resolve against `dish` (the menu), `room` (the open
//! sittings) and `decide` (who may).

mod decide;
mod dish;
mod grammar;
mod menu;
mod room;
mod say;
mod scope;
mod words;

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

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
    /// The waiter's round being built on the phone ("send the round").
    draft: Option<DraftIn>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct DraftIn {
    table: String,
    items: Vec<DraftLine>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct DraftLine {
    product_id: String,
    quantity: u32,
}

impl Default for VoiceIn {
    fn default() -> Self {
        VoiceIn { transcript: String::new(), confidence: 1.0, is_final: true, lang: None, confirm: None, draft: None }
    }
}

/// How long a read-back stays answerable. Long enough to hear it and say yes,
/// short enough that a phone left on a counter cannot confirm it an hour later.
const PROPOSAL_TTL_MS: i64 = 90_000;

/// Who is speaking, and what that lets them say.
enum Who {
    Hub(dowiz_hub::voice::Speaker),
    Waiter(dowiz_hub::caps::Caps),
}

/// A confirmation: the token's own instruction, handed back to the surface,
/// which calls the same route the button calls.
fn confirmed(claims: &dowiz_hub::token::Claims, who: &str, role: dowiz_hub::token::Role) -> Value {
    let no = |say: &str| json!({ "understood": false, "say": say });
    if claims.subject != who || claims.role != role {
        return no("це підтвердження не ваше");
    }
    let Some((verb, rest)) = claims.scope.strip_prefix("voice:").and_then(|r| r.split_once(':')) else {
        return no("не та відповідь");
    };
    let mut out = json!({ "understood": true, "needsConfirmation": false, "action": "do", "verb": verb });
    if scope::is_ours(verb) {
        match scope::decode(verb, rest) {
            Some(args) => out["args"] = args,
            None => return no("не та відповідь"),
        }
    } else {
        out["orderId"] = json!(rest);
    }
    out
}

/// `POST /api/voice`
pub async fn voice(mut req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    use dowiz_hub::token::Role;
    use dowiz_hub::voice::{classify, Command, Speaker, Target, MIN_CONFIDENCE};

    let body: VoiceIn = req.json().await.unwrap_or_default();

    // WHICH VOCABULARY APPLIES IS DECIDED BY THE TOKEN, never by what the
    // caller says they are: a courier claiming to be an owner would otherwise
    // reach the owner's commands by typing a word.
    let (speaker, who, loc, role) = match crate::auth::authenticate(&req, &ctx.env, ctx.data.now_ms).await {
        Ok(crate::auth::Principal::Owner { user_id, active_location_id }) => {
            (Who::Hub(Speaker::Owner), user_id, active_location_id.unwrap_or_default(), Role::Owner)
        }
        Ok(crate::auth::Principal::Courier { courier_id, active_location_id, .. }) => {
            (Who::Hub(Speaker::Courier), courier_id, active_location_id, Role::Courier)
        }
        Ok(crate::auth::Principal::Staff { person_id, active_location_id, caps, .. }) => {
            (Who::Waiter(caps), person_id, active_location_id, Role::Staff)
        }
        Ok(_) => return Response::error("forbidden role", 403),
        Err(e) => return e.into_response(),
    };
    let lang = body.lang.clone().unwrap_or_else(|| "uk".into());

    // A confirmation carries its own instruction; nothing is classified again.
    if let Some(tok) = &body.confirm {
        return match dowiz_hub::token::verify(crate::auth::signing_key(&ctx.env).as_slice(), tok, ctx.data.now_ms) {
            Ok(c) => Response::from_json(&confirmed(&c, &who, role)),
            Err(_) => Response::from_json(&json!({ "understood": false, "say": "та відповідь уже не дійсна" })),
        };
    }

    let propose = |verb: &str, arg: &str| -> String {
        let now = ctx.data.now_ms;
        dowiz_hub::token::mint(
            crate::auth::signing_key(&ctx.env).as_slice(),
            &dowiz_hub::token::Claims {
                role,
                subject: who.clone(),
                session: String::new(),
                scope: format!("voice:{verb}:{arg}"),
                caps: String::new(),
                issued_ms: now,
                expires_ms: now + PROPOSAL_TTL_MS,
            },
        )
    };
    let answer = |out: decide::Out| -> Value {
        match out {
            decide::Out::Now(mut v) => {
                v["understood"] = json!(true);
                v["needsConfirmation"] = json!(false);
                v
            }
            decide::Out::Propose { verb, arg, readback, mut extra } => {
                extra["understood"] = json!(true);
                extra["needsConfirmation"] = json!(true);
                extra["verb"] = json!(verb);
                extra["readback"] = json!(readback);
                extra["token"] = json!(propose(verb, &arg));
                extra
            }
            decide::Out::Refuse(say) => json!({ "understood": false, "say": say, "heard": body.transcript }),
        }
    };

    // The recogniser's own doubt, before any grammar reads the words: the
    // hub's gate, applied to the new grammars too.
    let gate = if !body.is_final {
        Some("interim")
    } else if body.confidence < MIN_CONFIDENCE {
        Some("not sure I heard that")
    } else {
        None
    };

    // ── THE WAITER ──
    if let Who::Waiter(caps) = speaker {
        if let Some(why) = gate {
            return Response::from_json(&json!({ "understood": false, "say": why, "heard": body.transcript }));
        }
        let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
        let said = grammar::waiter(&body.transcript);
        let mine: Vec<_> = crate::hubstore::orders(&place)
            .await?
            .into_iter()
            .filter(|o| {
                serde_json::from_str::<Value>(&o.order_json)
                    .is_ok_and(|v| v.get("location_id").and_then(Value::as_str) == Some(loc.as_str()))
            })
            .collect();
        let sittings = crate::command::sitting::room(&mine);
        let needs_menu = matches!(said, grammar::Said::Add { .. } | grammar::Said::Send { .. });
        let dishes = if needs_menu { menu::load(&place, &lang).await? } else { Vec::new() };
        let draft = body.draft.as_ref().map(|d| decide::Draft {
            table: d.table.clone(),
            items: d.items.iter().map(|l| (l.product_id.clone(), l.quantity)).collect(),
        });
        let out = decide::waiter(
            &said,
            &decide::Room { lang: &lang, caps, sittings: &sittings, menu: &dishes, draft: draft.as_ref() },
        );
        return Response::from_json(&answer(out));
    }
    let Who::Hub(speaker) = speaker else { return Response::error("forbidden role", 403) };
    let place = crate::hubstore::Place::of_any(&req, &ctx).await?;

    // ── THE OWNER'S STOP-LIST AND VENUE STATE ──
    if speaker == Speaker::Owner && gate.is_none() {
        if let Some(said) = grammar::owner(&body.transcript) {
            let dishes = match said {
                grammar::Said::DishSale { .. } => menu::load(&place, &lang).await?,
                _ => Vec::new(),
            };
            return Response::from_json(&answer(decide::owner(&said, &lang, &dishes)));
        }
    }

    let cmd = classify(&body.transcript, body.confidence, body.is_final, speaker);

    // The orders this speaker may act on. An owner sees the venue's live queue;
    // a courier sees only their own run -- so a misheard number can never reach
    // somebody else's delivery.
    let pool: Vec<Value> = crate::hubstore::orders(&place)
        .await?
        .into_iter()
        .filter_map(|e| serde_json::from_str::<Value>(&e.order_json).ok())
        .filter(|o| o.get("location_id").and_then(Value::as_str).map(|l| l == loc).unwrap_or(true))
        .filter(|o| {
            let st = o.get("status").and_then(Value::as_str).unwrap_or("");
            let live = matches!(st, "PENDING" | "CONFIRMED" | "PREPARING" | "READY" | "IN_DELIVERY");
            match speaker {
                Speaker::Owner => live,
                Speaker::Courier => live && o.get("courier_id").and_then(Value::as_str) == Some(who.as_str()),
            }
        })
        .collect();

    let resolve = |t: &Target| -> std::result::Result<String, &'static str> {
        let id = |o: &Value| o.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        let at = |o: &Value| o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0);
        match t {
            Target::Newest => pool.iter().max_by_key(|o| at(o)).map(id).ok_or("зараз немає замовлень"),
            Target::Oldest => pool.iter().min_by_key(|o| at(o)).map(id).ok_or("зараз немає замовлень"),
            Target::Digits(d) => {
                let hits: Vec<String> = pool.iter().filter(|o| id(o).ends_with(d.as_str())).map(id).collect();
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
    let order_verb = |verb: &'static str, target: &Target| -> decide::Out {
        match resolve(target) {
            Err(why) => decide::Out::Refuse(why.to_string()),
            Ok(order) => decide::Out::Propose {
                verb,
                arg: order.clone(),
                readback: cmd.readback(&lang),
                extra: json!({ "orderId": order }),
            },
        }
    };

    let out = match &cmd {
        Command::Unclear(why) => decide::Out::Refuse((*why).to_string()),
        Command::Status => {
            let waiting = pool.iter().filter(|o| o.get("status").and_then(Value::as_str) == Some("PENDING")).count();
            // READ-ONLY, so it runs at once. Making somebody confirm a question
            // is what turns voice into a form.
            decide::Out::Now(json!({ "action": "status", "open": pool.len(), "waiting": waiting }))
        }
        Command::Ask(q) => decide::Out::Now(json!({ "action": "ask", "question": q })),
        // An owner has no shift; the hub's grammar reads the words for anyone.
        Command::Shift { .. } if speaker == Speaker::Owner => decide::Out::Refuse(say::line("shift_owner", &lang).into()),
        Command::Shift { open } => decide::Out::Propose {
            verb: if *open { "shift_open" } else { "shift_close" },
            arg: "-".into(),
            readback: cmd.readback(&lang),
            extra: json!({}),
        },
        Command::Order { verb, target } => order_verb(verb, target),
        Command::Pickup { target } => order_verb("pickup", target),
        Command::Deliver { target } => order_verb("deliver", target),
    };
    Response::from_json(&answer(out))
}

#[cfg(test)]
mod tests;
