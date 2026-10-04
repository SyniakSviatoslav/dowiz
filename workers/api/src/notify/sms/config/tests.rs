use super::*;
use std::collections::HashMap;

struct M(HashMap<&'static str, &'static str>);
impl Read for M {
    fn val(&self, key: &str) -> String {
        self.0.get(key).copied().unwrap_or("").to_string()
    }
}
fn m(kv: &[(&'static str, &'static str)]) -> M {
    M(kv.iter().copied().collect())
}

fn body(on: bool, provider: &str) -> CfgIn {
    CfgIn { on, provider: provider.into(), url: None, user: None, secret: None, from: None, daily: None }
}

#[test]
fn off_or_incomplete_is_not_a_configuration() {
    assert_eq!(of(&m(&[])), Err("sms_off"));
    assert_eq!(of(&m(&[(KEY_ON, "on")])), Err("sms_missing_secret"));
    assert_eq!(of(&m(&[(KEY_ON, "on"), (KEY_SECRET, "pw")])), Err("sms_missing_user"), "SMSGate needs its login");
    assert_eq!(of(&m(&[(KEY_ON, "on"), (KEY_PROVIDER, "twilio"), (KEY_USER, "AC1"), (KEY_SECRET, "t")])), Err("sms_missing_from"));
    assert!(of(&m(&[(KEY_ON, "on"), (KEY_PROVIDER, "textbee"), (KEY_SECRET, "k")])).is_ok(), "textbee needs only its key");
}

#[test]
fn the_default_is_the_venues_own_phone_through_smsgate() {
    let c = of(&m(&[(KEY_ON, "on"), (KEY_USER, "ABCDEF"), (KEY_SECRET, "pw")])).unwrap();
    assert_eq!((c.provider, c.url.as_str(), c.daily), (Provider::SmsGate, SMSGATE_URL, DAILY_DEFAULT));
}

#[test]
fn the_view_never_shows_the_secret() {
    let v = view(&m(&[(KEY_ON, "on"), (KEY_USER, "ABCDEF"), (KEY_SECRET, "hunter2-password")]));
    assert!(!v.to_string().contains("hunter2"), "{v}");
    assert_eq!(v["secret_set"], true);
    assert_eq!(v["missing"], Value::Null);
    assert!(dowiz_hub::settings::is_secret(KEY_SECRET), "the generic settings read redacts it too");
    let v = view(&m(&[(KEY_ON, "on")]));
    assert_eq!(v["missing"], "sms_missing_secret");
}

#[test]
fn an_edit_is_checked_whole() {
    assert!(edit(&body(true, "smsgate"), false).unwrap_err().contains("sms_missing_secret"));
    let mut b = body(true, "smsgate");
    b.user = Some("ABCDEF".into());
    assert!(edit(&b, true).is_ok(), "a kept secret counts");
    b.secret = Some(String::new());
    assert!(edit(&b, true).is_err(), "forgetting the secret while switching on is refused");
    assert!(edit(&body(false, "smsgate"), false).is_ok(), "switching off needs nothing");
    assert!(edit(&body(true, "carrier-pigeon"), true).unwrap_err().contains("unknown SMS provider"));
}

#[test]
fn the_address_is_https_to_a_host_and_twilios_is_fixed() {
    let mut b = body(false, "smsgate");
    for bad in ["http://10.0.0.5:8080/message", "https://", "https://user:pw@evil.example/x", "ftp://x"] {
        b.url = Some(bad.into());
        assert!(edit(&b, false).is_err(), "{bad}");
    }
    b.url = Some("https://sms.my-venue.al/3rdparty/v1/messages".into());
    assert!(edit(&b, false).is_ok(), "a private SMSGate server");
    let mut t = body(false, "twilio");
    t.url = Some("https://api.twilio.com/x".into());
    assert!(edit(&t, false).is_err());
}

#[test]
fn the_daily_limit_and_sender_are_checked() {
    let mut b = body(false, "twilio");
    b.daily = Some(0);
    assert!(edit(&b, false).is_err());
    b.daily = Some(DAILY_MAX + 1);
    assert!(edit(&b, false).is_err());
    b.daily = Some(30);
    b.from = Some("069 123 4567".into());
    assert!(edit(&b, false).is_err(), "a national sender number is ambiguous");
    b.from = Some("+355691234567".into());
    let w = edit(&b, false).unwrap();
    assert!(w.contains(&(KEY_DAILY, "30".into())) && w.contains(&(KEY_FROM, "+355691234567".into())));
    assert!(!w.iter().any(|(k, _)| *k == KEY_SECRET), "no secret typed = the stored one is kept");
    assert_eq!(daily_of(&m(&[(KEY_DAILY, "0")])), DAILY_DEFAULT);
}

/// Twilio's Account SID is a path segment of its URL: a `/` or `?` in it would
/// steer the request to another resource on Twilio's host.
#[test]
fn a_twilio_sid_is_letters_and_digits() {
    let mut b = body(true, "twilio");
    b.secret = Some("token".into());
    b.from = Some("+355691234567".into());
    for bad in ["AC12/../Calls", "AC12?x=1", "AC 12"] {
        b.user = Some(bad.into());
        assert!(edit(&b, false).is_err(), "{bad}");
    }
    b.user = Some("AC0123456789abcdef0123456789abcdef".into());
    assert!(edit(&b, false).is_ok());
}
