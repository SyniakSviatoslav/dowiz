//! WhatsApp/Instagram through their routes (W-COV C2): Meta's verification handshake, a signed
//! delivery stored in the venue's inbox, the owner's thread and reply (the Graph API is the
//! thread's outbound hook) -- and an unsigned or forged delivery refused.

use crate::edge::mem::{answer_outbound, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::wire::{Call, Reply};
use hmac::{Hmac, Mac};
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

fn deliver(site: &Site, raw: &str, sig: Option<String>) -> Reply {
    let mut c = Call::new(&at("/api/webhooks/meta"), worker::Method::Post).unwrap().with_body(raw.as_bytes().to_vec()).on("alpha");
    if let Some(s) = sig {
        c = c.with_header("x-hub-signature-256", &s);
    }
    site.run(crate::channels::webhook, c, &[])
}

fn sign(secret: &str, raw: &str) -> String {
    let mut m = Hmac::<sha2::Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    m.update(raw.as_bytes());
    format!("sha256={}", m.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>())
}

#[test]
fn a_signed_whatsapp_message_lands_in_the_inbox_and_the_owner_replies() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    setting(&site, &t, "notify.whatsapp.verify", "verify-me");
    let ok = site.run(crate::channels::webhook_verify, get(&at("/api/webhooks/meta?hub.verify_token=verify-me&hub.challenge=42")).on("alpha"), &[]);
    assert_eq!(ok.body_str(), "42");
    let bad = site.run(crate::channels::webhook_verify, get(&at("/api/webhooks/meta?hub.verify_token=nope&hub.challenge=42")).on("alpha"), &[]);
    assert_eq!(bad.status_code(), 403);

    let raw = json!({"object": "whatsapp_business_account", "entry": [{"changes": [{"value": {
        "messaging_product": "whatsapp", "contacts": [{"wa_id": "355691234567", "profile": {"name": "Ana"}}],
        "messages": [{"from": "355691234567", "id": "wamid.1", "timestamp": "1700000000", "type": "text", "text": {"body": "table for two?"}}]}}]}]})
    .to_string();
    assert_eq!(deliver(&site, &raw, None).status_code(), 401, "unsigned");
    let no_secret = deliver(&site, &raw, Some(sign("x", &raw)));
    assert_eq!(no_secret.body_value()["stored"], 0, "no app secret set: {}", no_secret.body_str());
    setting(&site, &t, "notify.meta.secret", "meta-secret");
    assert_eq!(deliver(&site, &raw, Some(sign("wrong", &raw))).status_code(), 401, "forged");
    let r = deliver(&site, &raw, Some(sign("meta-secret", &raw)));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["stored"], 1, "{}", r.body_str());

    let inbox = site.run(crate::channels::inbox, get(&at("/api/owner/inbox")).bearer(&t).on("alpha"), &[]);
    assert!(inbox.body_str().contains("table for two?") || inbox.body_str().contains("Ana"), "{}", inbox.body_str());
    let th = site.run(crate::channels::thread, get(&at("/api/owner/inbox/355691234567?channel=whatsapp")).bearer(&t).on("alpha"), &[("peer", "355691234567")]);
    assert_eq!(th.status_code(), 200, "{}", th.body_str());

    let empty = site.run(
        crate::channels::reply,
        post(&at("/api/owner/inbox/355691234567"), &json!({"channel": "whatsapp", "text": "  "})).bearer(&t).on("alpha"),
        &[("peer", "355691234567")],
    );
    assert_eq!(empty.status_code(), 400);
    let unset = site.run(
        crate::channels::reply,
        post(&at("/api/owner/inbox/355691234567"), &json!({"channel": "whatsapp", "text": "yes, 8pm"})).bearer(&t).on("alpha"),
        &[("peer", "355691234567")],
    );
    assert!(unset.status_code() >= 400, "WhatsApp not set up: {}", unset.body_str());
    setting(&site, &t, "notify.whatsapp.token", "wa-token");
    setting(&site, &t, "notify.whatsapp.phone_id", "1234");
    answer_outbound(|_| Reply::from_json(&json!({"messages": [{"id": "wamid.out"}]})));
    let r = site.run(
        crate::channels::reply,
        post(&at("/api/owner/inbox/355691234567"), &json!({"channel": "whatsapp", "text": "yes, 8pm"})).bearer(&t).on("alpha"),
        &[("peer", "355691234567")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let out = sent().into_iter().last().expect("a Graph call");
    assert!(out.url().unwrap().path().ends_with("/1234/messages"), "{}", out.url().unwrap());
}

#[test]
fn a_cloud_copy_goes_to_the_venues_bucket_and_integrations_are_checked() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    let unset = site.run(crate::cloud::push, post(&at("/api/owner/backup/cloud"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert!(unset.status_code() >= 400, "no bucket: {}", unset.body_str());
    for (k, v) in [("cloud.s3.endpoint", "https://s3.example"), ("cloud.s3.bucket", "venue-copies"), ("cloud.s3.key", "AKIA"), ("cloud.s3.secret", "shh")] {
        setting(&site, &t, k, v);
    }
    answer_outbound(|_| {
        let mut r = Reply::empty().unwrap();
        r.headers_mut().set("etag", "\"abc\"").unwrap();
        Ok(r)
    });
    let r = site.run(crate::cloud::push, post(&at("/api/owner/backup/cloud"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert!(r.status_code() == 200 || r.status_code() == 500, "{}", r.body_str());
    let puts: Vec<String> = sent().iter().filter(|c| c.method() == worker::Method::Put).map(|c| c.url().unwrap().to_string()).collect();
    assert!(puts.iter().any(|u| u.contains("venue-copies")), "the copy went to the venue's bucket: {puts:?}");
    let st = site.run(crate::cloud::status, get(&at("/api/owner/backup/cloud")).bearer(&t).on("alpha"), &[]);
    assert!(st.body_str().contains("venue-copies"), "{}", st.body_str());

    let s = site.run(crate::integrations::status, get(&at("/api/owner/integrations")).bearer(&t).on("alpha"), &[]);
    assert_eq!(s.status_code(), 200, "{}", s.body_str());
    // A check that cannot run answers 502 with `ok:false` and a named code (the
    // console shows the code); an unknown integration is the caller's error.
    for (which, code) in [("telegram", "no_token"), ("whatsapp", "no_phone"), ("instagram", "no_account")] {
        let r = site.run(crate::integrations::check, post(&at("/api/owner/integrations/check"), &json!({"which": which})).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 502, "{which}: {}", r.body_str());
        let v = r.body_value();
        assert_eq!((v["ok"].as_bool(), v["code"].as_str()), (Some(false), Some(code)), "{which}: {v}");
    }
    let r = site.run(crate::integrations::check, post(&at("/api/owner/integrations/check"), &json!({"which": "nonsense"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
}

#[test]
fn an_instagram_message_is_threaded_counted_unread_and_answered_through_graph() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    setting(&site, &t, "notify.meta.secret", "meta-secret");
    let dm = |mid: &str, text: &str, ts: i64, echo: bool| {
        json!({"object": "instagram", "entry": [{"messaging": [
            {"sender": {"id": "ig-777"}, "recipient": {"id": "ig-venue"}, "timestamp": ts,
             "message": {"mid": mid, "text": text, "is_echo": echo}},
            {"sender": {"id": "ig-777"}, "timestamp": ts, "message": {"mid": "sticker", "attachments": []}}
        ]}]})
        .to_string()
    };
    for (mid, text, ts) in [("m1", "do you deliver to Durrës?", 1_700_000_000_000), ("m2", "and on Sunday?", 1_700_000_060_000)] {
        let raw = dm(mid, text, ts, false);
        let r = deliver(&site, &raw, Some(sign("meta-secret", &raw)));
        assert_eq!(r.body_value()["stored"], 1, "a message without text is not stored: {}", r.body_str());
    }
    // An echo of the venue's own message is not the customer speaking.
    let raw = dm("m3", "yes we do", 1_700_000_120_000, true);
    assert_eq!(deliver(&site, &raw, Some(sign("meta-secret", &raw))).body_value()["stored"], 0);
    // The same delivery twice is one message.
    let raw = dm("m1", "do you deliver to Durrës?", 1_700_000_000_000, false);
    assert_eq!(deliver(&site, &raw, Some(sign("meta-secret", &raw))).body_value()["stored"], 0, "redelivered");

    let inbox = site.run(crate::channels::inbox, get(&at("/api/owner/inbox")).bearer(&t).on("alpha"), &[]).body_value();
    let row = inbox.to_string();
    assert!(row.contains("ig-777") && row.contains("instagram"), "{inbox}");
    assert!(row.contains("\"unread\":2"), "two unread: {inbox}");
    let th = site.run(crate::channels::thread, get(&at("/api/owner/inbox/ig-777?channel=instagram")).bearer(&t).on("alpha"), &[("peer", "ig-777")]);
    assert_eq!(th.status_code(), 200, "{}", th.body_str());
    assert!(th.body_str().contains("and on Sunday?"));
    // Reading the thread marks it read.
    let again = site.run(crate::channels::inbox, get(&at("/api/owner/inbox")).bearer(&t).on("alpha"), &[]).body_value();
    assert!(!again.to_string().contains("\"unread\":2"), "{again}");

    let reply = |text: &str| {
        site.run(
            crate::channels::reply,
            post(&at("/api/owner/inbox/ig-777"), &json!({"channel": "instagram", "text": text})).bearer(&t).on("alpha"),
            &[("peer", "ig-777")],
        )
    };
    assert!(reply("yes").status_code() >= 400, "Instagram is not set up");
    assert_eq!(
        site.run(crate::channels::reply, post(&at("/api/owner/inbox/ig-777"), &json!({"channel": "fax", "text": "x"})).bearer(&t).on("alpha"), &[("peer", "ig-777")]).status_code(),
        400
    );
    setting(&site, &t, "social.instagram.token", "ig-token");
    setting(&site, &t, "social.instagram.user_id", "1789");
    // Meta's own words come back when it refuses.
    answer_outbound(|_| Ok(Reply::from_json(&json!({"error": {"message": "(#10) outside the 24h window"}}))?.with_status(400)));
    let r = reply("yes, every day");
    assert!(r.status_code() >= 400 && r.body_str().contains("24h window"), "{}", r.body_str());
    answer_outbound(|_| Reply::from_json(&json!({"message_id": "mid.out"})));
    let r = reply("yes, every day");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let out = sent().into_iter().last().unwrap();
    assert!(out.url().unwrap().path().ends_with("/1789/messages"), "{}", out.url().unwrap());
    assert_eq!(out.headers().get("authorization").unwrap().as_deref(), Some("Bearer ig-token"));
    let body: serde_json::Value = serde_json::from_slice(&out.body_bytes()).unwrap();
    assert_eq!((body["recipient"]["id"].as_str(), body["message"]["text"].as_str()), (Some("ig-777"), Some("yes, every day")));
    let th = site.run(crate::channels::thread, get(&at("/api/owner/inbox/ig-777?channel=instagram")).bearer(&t).on("alpha"), &[("peer", "ig-777")]);
    assert!(th.body_str().contains("yes, every day"), "the venue's answer is in the thread: {}", th.body_str());
}

#[test]
fn each_integration_check_asks_its_provider_and_answers_in_the_providers_words() {
    let site = Site::with_secrets(&[("STRIPE_PUBLISHABLE_KEY", "pk_test_x")]);
    let t = site.venue("alpha", "a@x.test");
    let check = |which: &str| {
        let r = site.run(crate::integrations::check, post(&at("/api/owner/integrations/check"), &json!({"which": which})).bearer(&t).on("alpha"), &[]);
        (r.status_code(), r.body_value())
    };
    // Nothing set: each says what is missing, by code.
    for (which, code) in [("webhook", "no_verify"), ("cloud", "no_bucket"), ("stripe", "no_stripe"), ("ai", "ai_off")] {
        let (s, v) = check(which);
        assert_eq!((s, v["code"].as_str()), (502, Some(code)), "{which}: {v}");
    }
    assert!(check("stripe").1["error"].as_str().unwrap().contains("STRIPE_SECRET_KEY"), "names the missing key");
    let (s, v) = check("mcp");
    assert_eq!(s, 200, "{v}");
    assert!(v["detail"]["tools"].as_u64().unwrap() > 0 && v["detail"]["url"].as_str().unwrap().ends_with("/api/mcp"), "{v}");

    for (k, v) in [
        ("notify.telegram.token", "123:abc"), ("notify.whatsapp.token", "wa"), ("notify.whatsapp.phone_id", "555"),
        ("social.instagram.token", "ig"), ("social.instagram.user_id", "1789"),
        ("cloud.s3.endpoint", "https://s3.example"), ("cloud.s3.bucket", "copies"), ("cloud.s3.key", "AKIA"), ("cloud.s3.secret", "shh"),
        ("ai.endpoint", "https://llm.example/v1"), ("ai.model", "tiny"), ("ai.token", "sk-1"),
    ] {
        setting(&site, &t, k, v);
    }
    site.run(crate::services::venue::settings::set_feature, post(&at("/api/owner/features"), &json!({"key": "ai.enabled", "on": true})).bearer(&t).on("alpha"), &[]);
    // Every provider says yes.
    answer_outbound(|c| {
        let u = c.url().unwrap();
        match u.host_str().unwrap_or("") {
            "api.telegram.org" => Reply::from_json(&json!({"ok": true, "result": {"username": "alpha_bot"}})),
            "graph.facebook.com" if u.path().contains("/555") => Reply::from_json(&json!({"display_phone_number": "+355 69", "verified_name": "Alpha"})),
            "graph.facebook.com" => Reply::from_json(&json!({"username": "alpha.sushi", "followers_count": 12})),
            "llm.example" => Reply::from_json(&json!({"data": [{"id": "tiny"}, {"id": "big"}]})),
            _ => {
                let mut r = Reply::empty()?;
                r.headers_mut().set("etag", "\"p1\"")?;
                Ok(r)
            }
        }
    });
    let (s, v) = check("telegram");
    assert_eq!((s, v["detail"]["bot"].as_str()), (200, Some("alpha_bot")), "{v}");
    let (s, v) = check("whatsapp");
    assert_eq!(s, 200, "{v}");
    assert!(v.to_string().contains("+355 69"), "{v}");
    let (s, v) = check("instagram");
    assert_eq!(s, 200, "{v}");
    assert!(v.to_string().contains("alpha.sushi"), "{v}");
    let (s, v) = check("cloud");
    assert_eq!(s, 200, "{v}");
    assert!(sent().iter().any(|c| c.method() == worker::Method::Put && c.url().unwrap().path().contains("/copies/")), "a probe was written");
    let (s, v) = check("ai");
    assert_eq!((s, v["detail"]["models"].as_u64()), (200, Some(2)), "{v}");
    let models = sent().into_iter().filter(|c| c.url().unwrap().path().ends_with("/models")).last().unwrap();
    assert_eq!(models.headers().get("authorization").unwrap().as_deref(), Some("Bearer sk-1"));

    // And every provider says no, in its own words.
    answer_outbound(|c| {
        let u = c.url().unwrap();
        match u.host_str().unwrap_or("") {
            "api.telegram.org" => Reply::from_json(&json!({"ok": false, "description": "Unauthorized"})),
            "graph.facebook.com" => Ok(Reply::from_json(&json!({"error": {"message": "Invalid OAuth access token"}}))?.with_status(401)),
            "llm.example" => Ok(Reply::ok("rate limited")?.with_status(429)),
            _ => Ok(Reply::ok("<Error><Code>AccessDenied</Code></Error>")?.with_status(403)),
        }
    });
    for (which, words) in [("telegram", "Unauthorized"), ("whatsapp", "Invalid OAuth"), ("instagram", "Invalid OAuth"), ("ai", "429"), ("cloud", "AccessDenied")] {
        let (s, v) = check(which);
        assert_eq!(s, 502, "{which}: {v}");
        assert!(v["error"].as_str().unwrap_or("").contains(words), "{which}: {v}");
    }
}
