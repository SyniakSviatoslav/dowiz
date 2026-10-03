//! What a subscription must be before the drain may trust it.
use super::*;

const P256DH: &str = "BCVxsr7N_eNgVRqvHtD0zTZsEc6-VV-JvLexhqUzORcxaOzi6-AYWXvTBHm4bjyPjs7Vd8pZGH6SRpkNtoIAiw4";
const AUTH: &str = "BTBZMqHH6r4Tts7J_aSIgg";

fn input(endpoint: &str, p256dh: &str, auth: &str, lang: Option<&str>) -> SubIn {
    SubIn { endpoint: endpoint.into(), keys: KeysIn { p256dh: p256dh.into(), auth: auth.into() }, lang: lang.map(str::to_string), _expiration_time: None }
}

#[test]
fn a_real_browser_subscription_is_accepted_for_each_push_service() {
    for ep in [
        "https://fcm.googleapis.com/fcm/send/dXy:APA91b",
        "https://updates.push.services.mozilla.com/wpush/v2/gAAAAAB",
        "https://web.push.apple.com/QGHv0",
        "https://wns2-par02p.notify.windows.com/w/?token=BQYAAA",
    ] {
        let s = input(ep, P256DH, AUTH, Some("uk")).check(7).unwrap_or_else(|e| panic!("{ep}: {e}"));
        assert_eq!((s.endpoint.as_str(), s.lang.as_str(), s.at_ms), (ep, "uk", 7));
    }
}

#[test]
fn an_endpoint_that_is_not_a_push_service_is_refused() {
    for ep in [
        "http://fcm.googleapis.com/fcm/send/x",
        "https://evil.example/fcm.googleapis.com/",
        "https://fcm.googleapis.com.evil.example/x",
        "https://user@fcm.googleapis.com/x",
        "https://localhost/x",
        "https://169.254.169.254/latest",
        "https://push.apple.com.evil/x",
    ] {
        assert!(input(ep, P256DH, AUTH, None).check(1).is_err(), "{ep} must be refused");
    }
    let long = format!("https://fcm.googleapis.com/{}", "a".repeat(ENDPOINT_MAX));
    assert_eq!(input(&long, P256DH, AUTH, None).check(1).err(), Some("that endpoint is too long"));
}

#[test]
fn keys_must_be_a_p256_point_and_a_16_byte_secret() {
    let ep = "https://fcm.googleapis.com/fcm/send/x";
    assert_eq!(input(ep, "not base64!", AUTH, None).check(1).err(), Some("p256dh is not base64url"));
    assert_eq!(input(ep, &P256DH[..40], AUTH, None).check(1).err(), Some("p256dh is not a P-256 public key"));
    assert_eq!(input(ep, P256DH, "AAAA", None).check(1).err(), Some("auth is not 16 bytes"));
    assert!(input(ep, P256DH, AUTH, None).check(1).is_ok());
}

#[test]
fn an_unknown_language_falls_back_to_english_and_a_known_one_is_kept() {
    let ep = "https://fcm.googleapis.com/fcm/send/x";
    assert_eq!(input(ep, P256DH, AUTH, Some("de")).check(1).unwrap().lang, "en");
    assert_eq!(input(ep, P256DH, AUTH, None).check(1).unwrap().lang, "en");
    assert_eq!(input(ep, P256DH, AUTH, Some("sq")).check(1).unwrap().lang, "sq");
}

#[test]
fn the_body_refuses_a_field_it_does_not_know() {
    let ok = r#"{"endpoint":"https://fcm.googleapis.com/x","expirationTime":null,"keys":{"p256dh":"a","auth":"b"},"lang":"en"}"#;
    assert!(serde_json::from_str::<SubIn>(ok).is_ok());
    let bad = r#"{"endpoint":"https://fcm.googleapis.com/x","keys":{"p256dh":"a","auth":"b"},"role":"owner"}"#;
    assert!(serde_json::from_str::<SubIn>(bad).is_err());
}

#[test]
fn one_device_is_one_record_and_two_devices_are_two() {
    assert_eq!(record_id("ord1", "https://a/1"), record_id("ord1", "https://a/1"));
    assert_ne!(record_id("ord1", "https://a/1"), record_id("ord1", "https://a/2"));
    assert!(record_id("ord1", "https://a/1").starts_with("ord1/"));
    assert_eq!(device("x").len(), 16);
}

fn rec(at: i64) -> String {
    serde_json::to_string(&Sub { endpoint: "e".into(), p256dh: "p".into(), auth: "a".into(), lang: "en".into(), at_ms: at }).unwrap()
}

#[test]
fn a_sixth_device_pushes_out_the_oldest_and_a_fifth_does_not() {
    let all: Vec<(String, String)> = (0..5).map(|i| (format!("c1/d{i}"), rec(100 - i))).collect();
    let mine = of_key(&all, "c1");
    assert_eq!(over_cap(&mine, "c1/new"), vec!["c1/d4".to_string()], "d4 is the oldest (at 96)");
    assert!(over_cap(&mine, "c1/d2").is_empty(), "re-subscribing a known device adds nothing");
    assert!(over_cap(&mine[..4], "c1/new").is_empty());
}

#[test]
fn of_key_does_not_mix_up_a_key_with_its_prefix() {
    let all = vec![("c1/x".to_string(), rec(1)), ("c10/y".to_string(), rec(1))];
    assert_eq!(of_key(&all, "c1").len(), 1);
}

#[test]
fn a_customer_record_older_than_two_days_is_stale_and_a_newer_one_is_not() {
    let now = 10 * 86_400_000;
    let all = vec![("o/a".to_string(), rec(now - CUSTOMER_KEEP_MS - 1)), ("o/b".to_string(), rec(now - CUSTOMER_KEEP_MS)), ("o/c".to_string(), "garbage".to_string())];
    assert_eq!(stale_customers(&all, now), vec!["o/a".to_string(), "o/c".to_string()]);
}
