use super::super::config::{Cfg, Provider, SMSGATE_URL, TEXTBEE_URL};
use super::*;
use serde_json::Value;

fn cfg(p: Provider) -> Cfg {
    let url = match p {
        Provider::SmsGate => SMSGATE_URL,
        Provider::TextBee => TEXTBEE_URL,
        Provider::Twilio => "",
    };
    Cfg { provider: p, url: url.into(), user: "USER1".into(), secret: "pa:ss".into(), from: "+15005550006".into(), daily: 60 }
}

fn header<'a>(r: &'a Req, k: &str) -> &'a str {
    r.headers.iter().find(|(n, _)| *n == k).map(|(_, v)| v.as_str()).unwrap_or("")
}

#[test]
fn smsgate_gets_its_documented_shape() {
    let r = request(&cfg(Provider::SmsGate), "+355691234567", "Hi", "ord1/sms/confirmed");
    assert_eq!(r.url, "https://api.sms-gate.app/3rdparty/v1/messages");
    assert_eq!(header(&r, "authorization"), format!("Basic {}", B64.encode("USER1:pa:ss")));
    let b: Value = serde_json::from_str(&r.body).unwrap();
    assert_eq!(b["textMessage"]["text"], "Hi");
    assert_eq!(b["phoneNumbers"], serde_json::json!(["+355691234567"]));
    assert_eq!(b["ttl"], TTL_SECS);
    let id = b["id"].as_str().unwrap();
    assert!(id.len() <= 36, "SMSGate's id is at most 36 characters");
    assert_eq!(id, message_id("ord1/sms/confirmed"), "stable per entry, so a retry is a 409");
    assert_ne!(id, message_id("ord1/sms/ready"));
}

#[test]
fn textbee_and_twilio_get_theirs() {
    let r = request(&cfg(Provider::TextBee), "+355691234567", "Hi", "x");
    assert_eq!((r.url.as_str(), header(&r, "x-api-key")), (TEXTBEE_URL, "pa:ss"));
    let b: Value = serde_json::from_str(&r.body).unwrap();
    assert_eq!((b["recipients"][0].as_str(), b["message"].as_str()), (Some("+355691234567"), Some("Hi")));

    let r = request(&cfg(Provider::Twilio), "+355691234567", "Ready & hot", "x");
    assert_eq!(r.url, "https://api.twilio.com/2010-04-01/Accounts/USER1/Messages.json");
    assert_eq!(r.body, "To=%2B355691234567&From=%2B15005550006&Body=Ready%20%26%20hot");
    assert_eq!(header(&r, "content-type"), "application/x-www-form-urlencoded");
}

#[test]
fn answers_are_classified() {
    for p in [Provider::SmsGate, Provider::TextBee, Provider::Twilio] {
        assert_eq!(classify(p, 202), Outcome::Sent);
        assert_eq!(classify(p, 201), Outcome::Sent);
        assert_eq!(classify(p, 400), Outcome::Refused);
        assert_eq!(classify(p, 401), Outcome::Failed, "wrong credentials end loud after six tries");
        assert_eq!(classify(p, 503), Outcome::Failed);
        assert_eq!(classify(p, 429), Outcome::Failed);
    }
    assert_eq!(classify(Provider::SmsGate, 409), Outcome::Sent, "already queued under this id");
    assert_eq!(classify(Provider::Twilio, 409), Outcome::Failed);
    assert_eq!(why(401), "sms_why_auth");
    assert_eq!(why(503), "sms_why_offline");
    assert_eq!(why(0), "sms_why_network");
}
