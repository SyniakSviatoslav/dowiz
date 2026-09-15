//! Drafting and publishing what the venue says in public.
//!
//! THE MODEL PHRASES; THE HUB DECIDES WHAT ABOUT. Subjects come from a diff of
//! the venue's own catalogue and a count over its own order log. The assistant
//! is handed one fact and asked to write two sentences. It is never asked what
//! to promote, because a model given that freedom eventually announces a
//! discount the venue is not running, on the venue's account, in public.
//!
//! NOTHING PUBLISHES WITHOUT THE OWNER. Every draft waits. This is the same
//! shape as the voice surface and for the same reason: "the AI wrote it" is not
//! a defence anyone accepts for what appears under their name.
//!
//! ONE CHANNEL IS IMPLEMENTED -- a Telegram channel -- and that is a deliberate
//! choice over a row of greyed-out logos. The transport already exists here, the
//! API is free, and a venue can create a channel and add their bot to it in two
//! minutes. Instagram and Facebook need a Meta app, a review and page tokens;
//! X charges for write access. Declaring them and shipping nothing behind them
//! would be a worse answer than not declaring them.

use axum::extract::{Path as AxPath, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use dowiz_hub::post::{self, Channel, Post, State as PostState, Subject};

use crate::hub::{now_ms, HubHttpError, Shared};
use crate::hubauth::OwnerCaller;

fn new_post_id() -> String {
    let ms = now_ms();
    let r = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    format!("post_{ms:013}_{r:08x}")
}

/// `GET /api/owner/posts` — everything drafted, newest first.
pub async fn list(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let posts = st.read_posts()?;
    let rows: Vec<Value> = posts
        .all()
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id, "text": p.text, "state": p.state.as_str(),
                "channel": p.channel.as_str(), "about": p.subject_tag,
                "createdMs": p.created_ms, "error": p.error,
            })
        })
        .collect();
    let settings = st.read_settings()?;
    Ok(Json(json!({
        "posts": rows,
        "enabled": settings.flag("social.enabled"),
        // Told plainly rather than discovered at publish time.
        "channel": settings.known("social.telegram.channel"),
    })))
}

/// `POST /api/owner/posts/draft` — look for something worth saying, and say it.
pub async fn draft(
    State(st): State<Shared>,
    _who: OwnerCaller,
) -> Result<Json<Value>, HubHttpError> {
    let settings = st.read_settings()?;
    if !settings.flag("social.enabled") {
        return Err(HubHttpError::Refused("social drafting is switched off".into()));
    }
    let assistant = crate::ai::Assistant::from_settings(&settings)
        .map_err(|e| HubHttpError::Refused(format!("the assistant is needed to write a post: {e}")))?;

    // ── what is true right now ──
    let cat = st.read_catalog()?;
    let current: Vec<(String, String, bool)> = cat
        .products()
        .into_iter()
        .filter_map(|(id, j)| {
            let v: Value = serde_json::from_str(&j).ok()?;
            Some((
                id,
                v.get("name")?.as_str()?.to_string(),
                v.get("available").and_then(Value::as_bool).unwrap_or(false),
            ))
        })
        .collect();
    let venue_json = cat.location().unwrap_or_default();
    let venue: Value = serde_json::from_str(&venue_json).unwrap_or(json!({}));
    let venue_name = venue.get("name").and_then(Value::as_str).unwrap_or("the restaurant");
    let is_open = venue.get("status").and_then(Value::as_str) == Some("open");
    let lang = venue.get("default_locale").and_then(Value::as_str).unwrap_or("sq").to_string();

    // The week's most-ordered dish, counted from the log. The MODEL never
    // counts; it is handed the number.
    let hub = st.read_log()?;
    let week_ago = now_ms() - 7 * 24 * 60 * 60 * 1000;
    let mut tally: Vec<(String, i64)> = Vec::new();
    for ev in hub.orders() {
        let Ok(o) = serde_json::from_str::<Value>(&ev.order_json) else { continue };
        if o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0) < week_ago {
            continue;
        }
        // A rejected order is not a dish people wanted served.
        if matches!(o.get("status").and_then(Value::as_str), Some("REJECTED") | Some("CANCELLED")) {
            continue;
        }
        for item in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(pid) = item.get("product_id").and_then(Value::as_str) else { continue };
            let q = item.get("quantity").and_then(Value::as_i64).unwrap_or(1);
            match tally.iter_mut().find(|(p, _)| p == pid) {
                Some((_, n)) => *n += q,
                None => tally.push((pid.to_string(), q)),
            }
        }
    }
    // Ties break by product id, so the same week produces the same answer twice.
    tally.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let top = tally.first().cloned();

    let posts = st.read_posts()?;
    let previous = posts.catalogue_snapshot();
    let was_closed = st.was_closed();
    let subjects = post::derive_subjects(&previous, &current, top, was_closed, is_open, &|k| {
        posts.already_seen(k)
    });

    // The snapshot moves EVEN IF nothing is drafted. Otherwise a venue that
    // rejects every draft is re-offered the same ones on the next pass, because
    // the world never advanced.
    let snapshot: Vec<(String, bool)> =
        current.iter().map(|(id, _, av)| (id.clone(), *av)).collect();
    st.with_posts(move |p| {
        p.set_catalogue_snapshot(&snapshot);
        Ok(())
    })
    .await?;
    st.note_open_state(is_open).await;

    if subjects.is_empty() {
        return Ok(Json(json!({ "drafted": 0, "note": "nothing new to say" })));
    }

    // ── write them ──
    //
    // Capped at three. A venue handed twelve drafts approves none of them, and
    // a menu import that changes fifty dishes would otherwise produce fifty
    // calls to the owner's own AI account, which they pay for.
    let mut written = Vec::new();
    for subject in subjects.into_iter().take(3) {
        let prompt = post::prompt_for(&subject, venue_name, &lang);
        let text = match assistant.ask(post::SYSTEM_POST, &prompt, 60_000).await {
            Ok(t) => t,
            Err(e) => {
                // Loud, and it stops: if the model is unreachable the second
                // and third calls will fail the same way.
                eprintln!("post: drafting failed: {e}");
                break;
            }
        };
        // THE LAST GATE. A model that invented an offer does not get stored,
        // let alone published -- and the subject is not marked seen, so it can
        // be tried again later.
        if let Some(why) = post::unusable(&text) {
            eprintln!("post: refused a draft about {}: {why}", subject.key());
            continue;
        }
        let p = Post {
            id: new_post_id(),
            subject_key: subject.key(),
            subject_tag: subject.fact(),
            text,
            channel: Channel::Telegram,
            state: PostState::Draft,
            created_ms: now_ms(),
            decided_ms: 0,
            error: String::new(),
        };
        let stored = p.clone();
        st.with_posts(move |ps| {
            ps.put(&stored);
            Ok(())
        })
        .await?;
        written.push(json!({ "id": p.id, "text": p.text, "about": p.subject_tag }));
    }

    Ok(Json(json!({ "drafted": written.len(), "posts": written })))
}

#[derive(Deserialize, Default)]
pub struct EditIn {
    /// The owner's own words, if they changed it. An assistant that cannot be
    /// overruled is one that gets switched off.
    #[serde(default)]
    pub text: Option<String>,
}

/// `POST /api/owner/posts/{id}/approve` — publish it.
pub async fn approve(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
    body: Option<Json<EditIn>>,
) -> Result<Json<Value>, HubHttpError> {
    let edit = body.map(|b| b.0).unwrap_or_default();
    let posts = st.read_posts()?;
    let mut p = posts.get(&id).ok_or(HubHttpError::NotFound("post"))?;
    if p.state == PostState::Published {
        return Err(HubHttpError::Conflict("that post is already out".into()));
    }
    if let Some(t) = edit.text {
        let t = t.trim().to_string();
        // The owner's own text is checked too. They are unlikely to write
        // "50% off" by accident -- but if they do, they mean it, and this is
        // the wrong tool for an offer the rest of the system knows nothing about.
        if let Some(why) = post::unusable(&t) {
            return Err(HubHttpError::Invalid(format!("{why}; post that one yourself")));
        }
        p.text = t;
    }

    let settings = st.read_settings()?;
    let channel = settings.known("social.telegram.channel");
    if channel.trim().is_empty() {
        return Err(HubHttpError::Invalid(
            "no Telegram channel is configured in settings".into(),
        ));
    }
    let Some(tg) = crate::notify::Telegram::from_env() else {
        return Err(HubHttpError::Refused("this hub has no bot token".into()));
    };

    match tg.send(&channel, &p.text).await {
        Ok(()) => {
            p.state = PostState::Published;
            p.decided_ms = now_ms();
            p.error.clear();
        }
        Err(e) => {
            // FAILED is a state, not a silence. The commonest cause is the bot
            // not being an administrator of the channel, and the owner needs to
            // read that rather than wonder why nothing appeared.
            p.state = PostState::Failed;
            p.decided_ms = now_ms();
            p.error = format!("{e:?}");
        }
    }

    let stored = p.clone();
    st.with_posts(move |ps| {
        ps.put(&stored);
        Ok(())
    })
    .await?;

    if p.state == PostState::Failed {
        return Err(HubHttpError::Io(format!("the channel refused it: {}", p.error)));
    }
    Ok(Json(json!({ "id": p.id, "state": p.state.as_str(), "channel": channel })))
}

/// `POST /api/owner/posts/{id}/reject`.
pub async fn reject(
    State(st): State<Shared>,
    _who: OwnerCaller,
    AxPath(id): AxPath<String>,
) -> Result<Json<Value>, HubHttpError> {
    let posts = st.read_posts()?;
    let mut p = posts.get(&id).ok_or(HubHttpError::NotFound("post"))?;
    if p.state == PostState::Published {
        return Err(HubHttpError::Conflict("that one is already out".into()));
    }
    p.state = PostState::Rejected;
    p.decided_ms = now_ms();
    let stored = p.clone();
    st.with_posts(move |ps| {
        ps.put(&stored);
        Ok(())
    })
    .await?;
    Ok(Json(json!({ "id": p.id, "state": "rejected" })))
}

pub fn routes(state: Shared) -> Router {
    Router::new()
        .route("/api/owner/posts", get(list))
        .route("/api/owner/posts/draft", post(draft))
        .route("/api/owner/posts/{id}/approve", post(approve))
        .route("/api/owner/posts/{id}/reject", post(reject))
        .with_state(state)
}
