//! A person decides. Approving PUBLISHES; rejecting keeps the draft so the
//! venue can see what it was offered and why it said no.

use serde::Deserialize;
use serde_json::{json, Value};
use worker::*;

use crate::owner::{now_ms, owner_and_venue};

#[derive(Deserialize)]
#[serde(default)]
struct ApproveIn {
    /// The owner's own words, if they edited the draft. An assistant that
    /// cannot be overruled is one that gets switched off.
    text: Option<String>,
}

impl Default for ApproveIn {
    fn default() -> Self {
        ApproveIn { text: None }
    }
}

/// `POST /api/owner/posts/:id/approve` — publish it, in the owner's words.
pub async fn approve_post(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::post::State as PostState;

    let body: ApproveIn = req.json().await.unwrap_or_default();
    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing post", 400);
    };
    let posts = crate::hubstore::load_posts(&place).await?.posts;
    let Some(mut p) = posts.get(&id) else {
        return Response::error("not found", 404);
    };
    if p.state == PostState::Published {
        return Response::error("that post is already out", 409);
    }
    if let Some(t) = body.text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        p.text = t.to_string();
    }

    let s = crate::hubstore::load_settings(&place).await?.settings;
    let channel = s.known("social.telegram.channel");
    let token = crate::notify::bot_token(&ctx.env, &s);
    let (state, error) = match (token, channel.trim().is_empty()) {
        (None, _) => (PostState::Failed, Some("no Telegram bot is configured".to_string())),
        (_, true) => (PostState::Failed, Some("no channel is set".to_string())),
        (Some(token), false) => {
            let url = format!("https://api.telegram.org/bot{token}/sendMessage");
            let headers = Headers::new();
            headers.set("content-type", "application/json")?;
            let payload =
                json!({ "chat_id": channel, "text": p.text, "disable_web_page_preview": true });
            let r = Request::new_with_init(
                &url,
                RequestInit::new()
                    .with_method(Method::Post)
                    .with_headers(headers)
                    .with_body(Some(payload.to_string().into())),
            )?;
            let mut res = Fetch::Request(r).send().await?;
            if res.status_code() < 400 {
                (PostState::Published, None)
            } else {
                // Telegram's own words. "Publishing failed" sends an owner to a
                // forum; "bot is not a member of the channel chat" sends them to
                // the channel's admin list, which is where the fix is.
                let body = res.text().await.unwrap_or_default();
                (PostState::Failed, Some(body[..body.len().min(200)].to_string()))
            }
        }
    };
    // ── Instagram, when the venue connected one ──
    //
    // A post about a dish carries that dish's photo; Instagram has no
    // text-only posts, so a post whose subject has no photo is Telegram-only
    // and says so. The two channels are judged together: published if either
    // took it, and the error names the one that did not.
    let (state, error) = match crate::channels::instagram_cfg(&s) {
        None => (state, error),
        Some(ig) => {
            let subject = p.subject_key.split_once(':').map(|(_, v)| v).unwrap_or(&p.subject_key).to_string();
            let origin = req.url().map(|u| u.origin().ascii_serialization()).unwrap_or_default();
            let photo = crate::hubstore::load_catalog(&place).await.ok().and_then(|c| {
                c.catalog.products().into_iter().find_map(|(id, pj)| {
                    let v: Value = serde_json::from_str(&pj).ok()?;
                    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
                    if id != subject && name != subject {
                        return None;
                    }
                    let url = v.get("imageUrl").and_then(Value::as_str)?;
                    Some(if url.starts_with("http") { url.to_string() } else { format!("{origin}{url}") })
                })
            });
            let ig_result = match photo {
                None => Err("Instagram: the post's dish has no photo".to_string()),
                Some(url) => crate::channels::instagram_publish(&ig, &url, &p.text).await.map_err(|e| format!("Instagram: {e}")),
            };
            match (state, ig_result) {
                (PostState::Published, Ok(_)) => (PostState::Published, None),
                (PostState::Published, Err(e)) => (PostState::Published, Some(e)),
                (_, Ok(_)) => (PostState::Published, error.map(|e| format!("Telegram: {e}"))),
                (st, Err(e)) => (st, Some(format!("{} · {e}", error.unwrap_or_default()))),
            }
        }
    };
    p.state = state;
    p.error = error.clone().unwrap_or_default();
    p.decided_ms = now_ms();
    let stored = p.clone();
    crate::hubstore::with_posts(&place, move |ps| {
        ps.put(&stored);
        Ok(())
    })
    .await?;
    if p.state == PostState::Failed {
        return Response::error(error.unwrap_or_else(|| "publishing failed".into()), 502);
    }
    Response::from_json(&json!({ "ok": true, "state": p.state.as_str() }))
}

/// `POST /api/owner/posts/:id/reject` — not this one.
pub async fn reject_post(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    use dowiz_hub::post::State as PostState;

    let db = ctx.d1("DB")?;
    let loc = match owner_and_venue(&req, &ctx, &db).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let Some(id) = ctx.param("id").cloned() else {
        return Response::error("missing post", 400);
    };
    let posts = crate::hubstore::load_posts(&place).await?.posts;
    let Some(mut p) = posts.get(&id) else {
        return Response::error("not found", 404);
    };
    if p.state == PostState::Published {
        return Response::error("that post is already out", 409);
    }
    p.state = PostState::Rejected;
    crate::hubstore::with_posts(&place, move |ps| {
        ps.put(&p);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "ok": true }))
}
