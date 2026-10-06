//! The assistant, social drafts and voice through their routes (W-COV C2). The model is the
//! thread's outbound hook: what the venue asked it is read back from the recorded call, and the
//! routes' refusals (switched off, http endpoint, the model failing) are each exercised.

use crate::edge::mem::{answer_outbound, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use crate::wire::Reply;
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn setting(site: &Site, t: &str, key: &str, value: &str) {
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&at("/api/owner/settings"), &json!({"key": key, "value": value})).bearer(t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{key}: {}", r.body_str());
}

fn ask(site: &Site, t: &str) -> Reply {
    site.run(crate::services::engagement::assist::owner_assist, post(&at("/api/owner/assist"), &json!({"question": "what sold best?"})).bearer(t).on("alpha"), &[])
}

#[test]
fn the_owner_assistant_is_off_until_configured_and_then_asks_the_venues_model() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    // W-TASTE2 (2026-10-06): ON by default, but with no endpoint and no binding there is no route,
    // and the refusal says WHY (not "off"); the owner's own off says "off".
    let r = ask(&site, &t);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert!(r.body_str().contains("workers-ai-unavailable") && !r.body_str().contains("is off"), "{}", r.body_str());
    let r = site.run(
        crate::services::venue::settings::set_feature,
        post(&at("/api/owner/features"), &json!({"key": "ai.enabled", "on": false})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = ask(&site, &t);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert!(r.body_str().contains("is off"), "the owner's off: {}", r.body_str());
    assert!(sent().is_empty(), "nothing was sent while off or without a route");
    let r = site.run(
        crate::services::venue::settings::set_feature,
        post(&at("/api/owner/features"), &json!({"key": "ai.enabled", "on": true})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    // An http endpoint is refused where it is written, not when it is used.
    let r = site.run(
        crate::services::venue::settings::set_setting,
        post(&at("/api/owner/settings"), &json!({"key": "ai.endpoint", "value": "http://insecure.example"})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
    assert!(r.body_str().contains("https"), "{}", r.body_str());
    setting(&site, &t, "ai.endpoint", "https://llm.example/v1");
    setting(&site, &t, "ai.model", "tiny");
    // The model fails: 502, said.
    answer_outbound(|_| Ok(Reply::error("overloaded", 503).unwrap()));
    assert_eq!(ask(&site, &t).status_code(), 502);
    answer_outbound(|_| Reply::from_json(&json!({"choices": [{"message": {"content": "Futomaki"}}]})));
    let r = ask(&site, &t);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["answer"], "Futomaki");
    let call = sent().into_iter().last().expect("the model was called");
    assert!(call.url().unwrap().as_str().ends_with("/chat/completions"));
    let g = site.run(crate::services::engagement::assist::graph, get(&at("/api/owner/graph")).bearer(&t).on("alpha"), &[]);
    assert_eq!(g.status_code(), 200, "{}", g.body_str());
}

#[test]
fn social_drafts_are_off_until_switched_on_then_drafted_and_judged() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let off = site.run(crate::services::engagement::posts::draft_post, post(&at("/api/owner/posts/draft"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert_eq!(off.status_code(), 409, "{}", off.body_str());
    site.run(
        crate::services::venue::settings::set_feature,
        post(&at("/api/owner/features"), &json!({"key": "social.enabled", "on": true})).bearer(&t).on("alpha"),
        &[],
    );
    let d = site.run(crate::services::engagement::posts::draft_post, post(&at("/api/owner/posts/draft"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert!(d.status_code() == 200 || d.status_code() == 409, "{}", d.body_str());
    let list = site.run(crate::services::engagement::posts::posts, get(&at("/api/owner/posts")).bearer(&t).on("alpha"), &[]);
    assert_eq!(list.status_code(), 200, "{}", list.body_str());
    if let Some(id) = list.body_value().to_string().split("\"id\":\"").nth(1).map(|s| s.split('"').next().unwrap().to_string()) {
        let r = site.run(
            crate::services::engagement::verdict::reject_post,
            post(&at(&format!("/api/owner/posts/{id}/reject")), &json!({})).bearer(&t).on("alpha"),
            &[("id", &id)],
        );
        assert!(r.status_code() < 500, "{}", r.body_str());
    }
    let ghost = site.run(
        crate::services::engagement::verdict::approve_post,
        post(&at("/api/owner/posts/ghost/approve"), &json!({})).bearer(&t).on("alpha"),
        &[("id", "ghost")],
    );
    assert!(ghost.status_code() >= 400, "{}", ghost.body_str());
}

#[test]
fn a_voice_command_never_acts_alone() {
    // Voice has no switch (voice.rs: "the service survives it being off" is the
    // classifier's, not a feature flag); what it guarantees is that a transcript
    // changes nothing by itself.
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let v = |body: serde_json::Value| site.run(crate::services::engagement::voice::voice, post(&at("/api/voice"), &body).bearer(&t).on("alpha"), &[]);
    let before = site.fold("alpha", "/fold/generation").body_value();
    let r = v(json!({"transcript": "two futomaki for table four", "confidence": 0.9, "is_final": true, "lang": "en"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let a = r.body_value();
    assert_eq!((a["understood"].as_bool(), a["action"].as_str(), a["needsConfirmation"].as_bool()), (Some(true), Some("ask"), Some(false)), "{a}");
    let r = v(json!({"transcript": "", "confidence": 0.1, "is_final": false}));
    assert_eq!(r.body_value()["understood"].as_bool(), Some(false), "{}", r.body_str());
    assert_eq!(site.fold("alpha", "/fold/generation").body_value(), before, "voice never writes an order by itself");
}

fn dish(site: &Site, t: &str, name: &str) -> String {
    let r = site.run(crate::catalog_edit::set_category, post(&at("/api/owner/categories"), &json!({"location_id": "alpha", "name": "Specials"})).bearer(t).on("alpha"), &[]);
    let cat = r.body_value()["id"].as_str().unwrap_or_else(|| panic!("{}", r.body_str())).to_string();
    let r = site.run(
        crate::catalog_edit::create_product,
        post(&at("/api/owner/products"), &json!({"location_id": "alpha", "category_id": cat, "name": name, "price": 1200})).bearer(t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    // Held off sale until its allergens are declared (audit D17); a new dish is news only on sale.
    let id = r.body_value()["id"].as_str().unwrap().to_string();
    let r = site.run(
        crate::owner::update_product,
        post(&at(&format!("/api/owner/products/{id}")), &json!({"location_id": "alpha", "allergens": [], "available": true})).bearer(t).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "on sale: {}", r.body_str());
    id
}

#[test]
fn a_new_dish_becomes_a_draft_a_person_publishes_or_rejects_it() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    for k in ["social.enabled", "ai.enabled"] {
        let r = site.run(crate::services::venue::settings::set_feature, post(&at("/api/owner/features"), &json!({"key": k, "on": true})).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{k}: {}", r.body_str());
    }
    setting(&site, &t, "ai.endpoint", "https://llm.example/v1");
    setting(&site, &t, "ai.model", "tiny");
    answer_outbound(|c| {
        let u = c.url().unwrap().to_string();
        if u.ends_with("/chat/completions") {
            Reply::from_json(&json!({"choices": [{"message": {"content": "Uramaki is here: rice outside, salmon inside. Come taste it."}}]}))
        } else if u.contains("api.telegram.org") {
            Reply::from_json(&json!({"ok": true, "result": {"message_id": 1}}))
        } else {
            Reply::error("unexpected", 500)
        }
    });
    let draft = || site.run(crate::services::engagement::posts::draft_post, post(&at("/api/owner/posts/draft"), &json!({})).bearer(&t).on("alpha"), &[]);
    let list = |tok: &str| site.run(crate::services::engagement::posts::posts, get(&at("/api/owner/posts")).bearer(tok).on("alpha"), &[]);
    let reject_as = |id: &str, tok: &str| {
        site.run(crate::services::engagement::verdict::reject_post, post(&at(&format!("/api/owner/posts/{id}/reject")), &json!({})).bearer(tok).on("alpha"), &[("id", id)])
    };
    // The first run says nothing about the dishes -- with no snapshot every dish would look new --
    // but an open venue with no snapshot is announced as reopened, once (`posts.rs`, `was_closed`).
    let r = draft();
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let first = r.body_value()["posts"].as_array().cloned().unwrap_or_default();
    assert!(first.iter().all(|p| p["about"] == "the restaurant is open again"), "{}", r.body_str());
    for p in &first {
        assert_eq!(reject_as(p["id"].as_str().unwrap(), &t).status_code(), 200);
    }
    // A dish that was not there before is something to say.
    dish(&site, &t, "Uramaki");
    let r = draft();
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["drafted"], 1, "{}", r.body_str());
    let ps: Vec<_> = list(&t).body_value()["posts"].as_array().cloned().unwrap().into_iter().filter(|p| p["state"] == "draft").collect();
    assert_eq!(ps.len(), 1, "{ps:?}");
    assert!(ps[0]["about"].as_str().unwrap().contains("Uramaki"), "{ps:?}");
    let id = ps[0]["id"].as_str().unwrap().to_string();
    let state_of = |id: &str| list(&t).body_value()["posts"].as_array().unwrap().iter().find(|p| p["id"] == id).unwrap()["state"].clone();
    // The same dish again is not news.
    assert_eq!(draft().body_value()["drafted"], 0);

    let approve = |id: &str, body: serde_json::Value| {
        site.run(crate::services::engagement::verdict::approve_post, post(&at(&format!("/api/owner/posts/{id}/approve")), &body).bearer(&t).on("alpha"), &[("id", id)])
    };
    // No bot: the post fails, and says why; it stays to be tried again.
    let r = approve(&id, json!({}));
    assert_eq!(r.status_code(), 502, "{}", r.body_str());
    assert!(r.body_str().contains("no Telegram bot"), "{}", r.body_str());
    assert_eq!(state_of(&id), "failed");
    setting(&site, &t, "notify.telegram.token", "123:abc");
    let r = approve(&id, json!({}));
    assert_eq!(r.status_code(), 502);
    assert!(r.body_str().contains("no channel"), "{}", r.body_str());
    setting(&site, &t, "social.telegram.channel", "@alpha_sushi");
    // The owner's edit is what goes out.
    let r = approve(&id, json!({"text": "  Uramaki, today only.  "}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let out = sent().into_iter().filter(|c| c.url().unwrap().as_str().contains("/sendMessage")).last().expect("published");
    let body: serde_json::Value = serde_json::from_slice(&out.body_bytes()).unwrap();
    assert_eq!((body["chat_id"].as_str(), body["text"].as_str()), (Some("@alpha_sushi"), Some("Uramaki, today only.")), "{body}");
    assert_eq!(state_of(&id), "published");
    // Published is final: neither approved again nor rejected.
    assert_eq!(approve(&id, json!({})).status_code(), 409);
    let reject = |id: &str, tok: &str| {
        site.run(crate::services::engagement::verdict::reject_post, post(&at(&format!("/api/owner/posts/{id}/reject")), &json!({})).bearer(tok).on("alpha"), &[("id", id)])
    };
    assert_eq!(reject(&id, &t).status_code(), 409);
    assert_eq!(reject("ghost", &t).status_code(), 404);

    // A second new dish, rejected: kept, marked, never sent.
    dish(&site, &t, "Nigiri");
    assert_eq!(draft().status_code(), 200);
    let ps = list(&t).body_value()["posts"].as_array().cloned().unwrap();
    let second = ps.iter().find(|p| p["state"] == "draft").expect("a second draft")["id"].as_str().unwrap().to_string();
    let before = sent().len();
    // Another venue's owner cannot decide it.
    let (beta, _) = open_venue(&site, "beta", "b@x.test");
    let r = site.run(
        crate::services::engagement::verdict::reject_post,
        post(&at(&format!("/api/owner/posts/{second}/reject?location_id=alpha")), &json!({})).bearer(&beta).on("alpha"),
        &[("id", &second)],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
    assert!(list(&beta).body_value()["posts"].as_array().map_or(true, |a| a.is_empty()), "beta sees no alpha drafts");
    assert_eq!(reject(&second, &t).status_code(), 200);
    let ps = list(&t).body_value()["posts"].as_array().cloned().unwrap();
    assert_eq!(ps.iter().find(|p| p["id"] == second.as_str()).unwrap()["state"], "rejected");
    assert_eq!(sent().len(), before, "a rejection sends nothing");
}

#[test]
fn an_approved_post_goes_to_instagram_with_its_dishs_photo_or_says_why_not() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    for k in ["social.enabled", "ai.enabled"] {
        site.run(crate::services::venue::settings::set_feature, post(&at("/api/owner/features"), &json!({"key": k, "on": true})).bearer(&t).on("alpha"), &[]);
    }
    for (k, v) in [("ai.endpoint", "https://llm.example/v1"), ("ai.model", "tiny"), ("social.instagram.token", "ig-token"), ("social.instagram.user_id", "1789")] {
        setting(&site, &t, k, v);
    }
    answer_outbound(|c| {
        let u = c.url().unwrap().to_string();
        if u.ends_with("/chat/completions") {
            Reply::from_json(&json!({"choices": [{"message": {"content": "Sashimi is on the menu now."}}]}))
        } else if u.ends_with("/1789/media") {
            Reply::from_json(&json!({"id": "container-1"}))
        } else if u.ends_with("/1789/media_publish") {
            Reply::from_json(&json!({"id": "ig-post-1"}))
        } else {
            Reply::error("unexpected", 500)
        }
    });
    let draft = || site.run(crate::services::engagement::posts::draft_post, post(&at("/api/owner/posts/draft"), &json!({})).bearer(&t).on("alpha"), &[]);
    let list = || site.run(crate::services::engagement::posts::posts, get(&at("/api/owner/posts")).bearer(&t).on("alpha"), &[]).body_value();
    let approve = |id: &str| site.run(crate::services::engagement::verdict::approve_post, post(&at(&format!("/api/owner/posts/{id}/approve")), &json!({})).bearer(&t).on("alpha"), &[("id", id)]);
    let drafts_about = |word: &str| -> String {
        list()["posts"].as_array().unwrap().iter().find(|p| p["state"] == "draft" && p["about"].as_str().unwrap_or("").contains(word)).unwrap_or_else(|| panic!("{}", list()))["id"].as_str().unwrap().to_string()
    };
    draft();
    let pid = dish(&site, &t, "Sashimi");
    assert_eq!(draft().status_code(), 200);
    // No Telegram, and the dish has no photo: Instagram is the only channel and cannot post.
    let id = drafts_about("Sashimi");
    let r = approve(&id);
    assert_eq!(r.status_code(), 502, "{}", r.body_str());
    assert!(r.body_str().contains("no photo"), "{}", r.body_str());

    // A photo for the dish: the post goes to Instagram, image and caption.
    let png = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==",
    )
    .unwrap();
    let up = site.run(
        crate::services::catalogue::media::set_product_image,
        crate::wire::Call::new(&at(&format!("/api/owner/products/{pid}/image")), worker::Method::Post).unwrap().with_body(png).bearer(&t).on("alpha"),
        &[("id", &pid)],
    );
    assert_eq!(up.status_code(), 200, "{}", up.body_str());
    let r = approve(&id);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["state"], "published");
    let media = sent().into_iter().filter(|c| c.url().unwrap().path().ends_with("/1789/media")).last().expect("a container");
    let body: serde_json::Value = serde_json::from_slice(&media.body_bytes()).unwrap();
    assert!(body["image_url"].as_str().unwrap().starts_with("https://alpha.") && body["image_url"].as_str().unwrap().contains("/media/"), "{body}");
    assert_eq!(body["caption"], "Sashimi is on the menu now.");
    assert!(sent().iter().any(|c| c.url().unwrap().path().ends_with("/1789/media_publish")));
}

#[test]
fn a_spoken_order_action_is_proposed_signed_and_done_only_when_its_own_speaker_confirms_in_time() {
    let mut site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let id = crate::storefront::route_tests::place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
    let say_as = |site: &Site, tok: &str, body: serde_json::Value| {
        let r = site.run(crate::services::engagement::voice::voice, post(&at("/api/voice"), &body).bearer(tok).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
        r.body_value()
    };
    // The owner: "confirm the last one" names the newest live order and asks to be confirmed.
    let p = say_as(&site, &t, json!({"transcript": "confirm the last one", "confidence": 0.95, "is_final": true, "lang": "en"}));
    assert_eq!((p["needsConfirmation"].as_bool(), p["verb"].as_str(), p["orderId"].as_str()), (Some(true), Some("confirm"), Some(id.as_str())), "{p}");
    let token = p["token"].as_str().unwrap().to_string();
    assert!(!p["readback"].as_str().unwrap_or("").is_empty(), "the readback is what the speaker hears: {p}");
    // Proposing changed nothing.
    let gen = site.fold("alpha", "/fold/generation").body_value();
    // Confirmed in time, by the speaker: the instruction comes back to the surface to perform.
    let ok = say_as(&site, &t, json!({"confirm": token, "lang": "en"}));
    assert_eq!((ok["action"].as_str(), ok["verb"].as_str(), ok["orderId"].as_str()), (Some("do"), Some("confirm"), Some(id.as_str())), "{ok}");
    assert_eq!(site.fold("alpha", "/fold/generation").body_value(), gen, "the surface performs it, not the voice route");
    // Someone else holding the token is not its speaker.
    let (rider, _) = site.courier("alpha", &t, "+355692220000");
    let theirs = say_as(&site, &rider, json!({"confirm": token, "lang": "en"}));
    assert_eq!(theirs["understood"], false, "{theirs}");
    // A garbled token and an old one are refused, by name.
    assert_eq!(say_as(&site, &t, json!({"confirm": "nope", "lang": "en"}))["understood"], false);
    site.now_ms += 5 * 60_000;
    let late = say_as(&site, &t, json!({"confirm": token, "lang": "en"}));
    assert_eq!(late["understood"], false, "a proposal expires: {late}");
    // A number nobody's order ends in is refused, not guessed.
    let miss = say_as(&site, &t, json!({"transcript": "confirm 9999", "confidence": 0.95, "is_final": true, "lang": "en"}));
    assert_eq!(miss["understood"], false, "{miss}");
    // The owner's status is answered at once.
    let st = say_as(&site, &t, json!({"transcript": "status", "confidence": 0.95, "is_final": true, "lang": "en"}));
    assert!(st.to_string().contains("open") || st["understood"] == true, "{st}");

    // A courier sees only their own run: the venue's order is not theirs to pick up.
    let c = say_as(&site, &rider, json!({"transcript": "picked up the last one", "confidence": 0.95, "is_final": true, "lang": "en"}));
    assert_eq!(c["understood"], false, "{c}");
    let shift = say_as(&site, &rider, json!({"transcript": "start shift", "confidence": 0.95, "is_final": true, "lang": "en"}));
    assert_eq!((shift["needsConfirmation"].as_bool(), shift["verb"].as_str()), (Some(true), Some("shift_open")), "{shift}");
    // The owner has no shift to start.
    assert_eq!(say_as(&site, &t, json!({"transcript": "start shift", "confidence": 0.95, "is_final": true, "lang": "en"}))["understood"], false);

    // A waiter speaks the room's grammar: opening a table is proposed, not done.
    let waiter = site.staff("alpha", &t, "w@x.test", "waiter");
    let w = say_as(&site, &waiter, json!({"transcript": "open table 4", "confidence": 0.95, "is_final": true, "lang": "en"}));
    assert!(w["understood"] == true || w["say"].is_string(), "{w}");
    let gate = say_as(&site, &waiter, json!({"transcript": "open table 4", "confidence": 0.2, "is_final": true, "lang": "en"}));
    assert_eq!(gate["understood"], false, "an unsure transcript is asked again: {gate}");
}
