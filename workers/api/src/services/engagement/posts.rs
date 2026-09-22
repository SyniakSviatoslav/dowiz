//! The venue's public voice: a post is DRAFTED by the model and PUBLISHED by
//! a person.
//!
//! NOTHING REACHES A CUSTOMER WITHOUT AN OWNER SAYING SO. The draft route
//! writes a proposal and nothing else; approving is the act that publishes,
//! and rejecting keeps the draft so the venue can see what it was offered.

use serde_json::{json, Value};
use worker::*;

use crate::owner::{owner_and_venue};
use crate::services::orders::mine::of_venue as orders_of;

//
// NOTHING IS PUBLISHED WITHOUT A PERSON. The assistant drafts; the owner reads,
// edits and approves. That is the whole shape, and it is the reason the drafts
// are stored at all rather than posted as they are written.

/// `GET /api/owner/posts`
pub async fn posts(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    // Two small images, still two queries. They are 16 KB and 64 KB, so the
    // round trip is the cost rather than the bytes -- and this pane is opened
    // rarely enough that a third query for a pane nobody has open would be the
    // worse trade.
    let posts = crate::hubstore::load_posts(&place).await?.posts;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    Response::from_json(&json!({
        "posts": posts.all().into_iter().map(|p| json!({
            "id": p.id, "text": p.text, "state": p.state.as_str(),
            "channel": p.channel.as_str(), "about": p.subject_tag,
            "createdMs": p.created_ms, "error": p.error,
        })).collect::<Vec<_>>(),
        "enabled": s.flag("social.enabled"),
        // Told plainly rather than discovered at publish time.
        "channel": s.known("social.telegram.channel"),
    }))
}

/// `POST /api/owner/posts/draft` — look for something worth saying, and say it.
///
/// THE SUBJECTS ARE DERIVED FROM FACTS, never invented: a dish that came back,
/// a dish that went off, the week's most-ordered plate counted from the log, a
/// venue that reopened. The model writes the sentence; it never decides what is
/// true. A draft the checker finds unusable -- an invented discount, a
/// manufactured urgency -- is dropped and its subject is NOT marked seen, so it
/// can be tried again.
pub async fn draft_post(req: Request, ctx: RouteContext<crate::Req>) -> Result<Response> {
    use dowiz_hub::post::{self, Post, State as PostState};

    let loc = match owner_and_venue(&req, &ctx).await {
        Ok((_, l)) => l,
        Err(r) => return Ok(r),
    };
    // The venue this caller was authorised for, and no other.
    let place = crate::hubstore::Place::of_authorised(&ctx, &loc)?;
    let s = crate::hubstore::load_settings(&place).await?.settings;
    if !s.flag("social.enabled") {
        return Response::error("social drafting is switched off", 409);
    }

    let cat = crate::hubstore::load_catalog(&place).await?.catalog;
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
    let venue: Value =
        cat.location().and_then(|j| serde_json::from_str(&j).ok()).unwrap_or(json!({}));
    let venue_name = venue.get("name").and_then(Value::as_str).unwrap_or("the restaurant");
    let is_open = venue.get("status").and_then(Value::as_str) == Some("open");
    let lang = venue.get("default_locale").and_then(Value::as_str).unwrap_or("sq").to_string();

    // The week's most-ordered dish, COUNTED HERE. The model never counts; it is
    // handed the number.
    let listed = crate::hubstore::orders(&place).await?;
    let week_ago = ctx.data.now_ms - 7 * 24 * 60 * 60 * 1000;
    let mut tally: Vec<(String, i64)> = Vec::new();
    for o in orders_of(listed, &loc) {
        if o.get("created_at_ms").and_then(Value::as_i64).unwrap_or(0) < week_ago {
            continue;
        }
        // A rejected order is not a dish people wanted served.
        if !crate::services::orders::status::took_money(
            o.get("status").and_then(Value::as_str).unwrap_or(""),
        ) {
            continue;
        }
        for it in o.get("items").and_then(Value::as_array).into_iter().flatten() {
            let Some(pid) = it.get("product_id").and_then(Value::as_str) else { continue };
            let q = it.get("quantity").and_then(Value::as_i64).unwrap_or(1);
            match tally.iter_mut().find(|t| t.0 == pid) {
                Some(t) => t.1 += q,
                None => tally.push((pid.to_string(), q)),
            }
        }
    }
    tally.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let top = tally.first().cloned();

    let posts_img = crate::hubstore::load_posts(&place).await?.posts;
    let previous = posts_img.catalogue_snapshot();
    // A Worker holds no memory between requests, so "was the venue closed last
    // time we looked" cannot be an in-process flag as it is natively. The
    // snapshot in the posts image is the only thing that persists, and a
    // reopening is announced from the venue's current status alone -- which
    // means it can be drafted once per snapshot rather than once per reopening.
    let was_closed = previous.is_empty();
    let subjects = post::derive_subjects(&previous, &current, top, was_closed, is_open, &|k| {
        posts_img.already_seen(k)
    });

    let snapshot: Vec<(String, bool)> =
        current.iter().map(|(id, _, a)| (id.clone(), *a)).collect();
    if subjects.is_empty() {
        let snap = snapshot.clone();
        crate::hubstore::with_posts(&place, move |p| {
            p.set_catalogue_snapshot(&snap);
            Ok(())
        })
        .await?;
        return Response::from_json(&json!({ "drafted": 0, "note": "nothing new to say" }));
    }

    let mut written = Vec::new();
    for subject in &subjects {
        let prompt = post::prompt_for(subject, venue_name, &lang);
        let mut res = crate::assist::ask(&place, post::SYSTEM_POST, json!({}), &prompt).await?;
        if res.status_code() >= 400 {
            let why = res.text().await.unwrap_or_default();
            return Response::error(
                format!("the assistant is needed to write a post: {why}"),
                409,
            );
        }
        let v: Value = res.json().await?;
        let text = v.get("answer").and_then(Value::as_str).unwrap_or("").to_string();
        // A draft that fails the checker is DROPPED and its subject is not
        // marked seen, so nothing invented reaches the owner and the subject can
        // be tried again.
        if post::unusable(&text).is_some() {
            continue;
        }
        let Some(id) = crate::edge_id() else {
            return Response::error("no platform CSPRNG", 500);
        };
        let p = Post {
            id,
            text,
            subject_tag: subject.fact(),
            subject_key: subject.key(),
            state: PostState::Draft,
            channel: dowiz_hub::post::Channel::Telegram,
            created_ms: ctx.data.now_ms,
            decided_ms: 0,
            error: String::new(),
        };
        let stored = p.clone();
        // Storing the draft IS marking the subject seen: `already_seen` reads
        // the posts themselves, so a second list of keys would be a second fact
        // that could disagree with them.
        crate::hubstore::with_posts(&place, move |posts| {
            posts.put(&stored);
            Ok(())
        })
        .await?;
        written.push(json!({ "id": p.id, "text": p.text, "about": p.subject_tag }));
    }
    let snap = snapshot.clone();
    crate::hubstore::with_posts(&place, move |p| {
        p.set_catalogue_snapshot(&snap);
        Ok(())
    })
    .await?;
    Response::from_json(&json!({ "drafted": written.len(), "posts": written }))
}


