use super::super::config::{Provider, SMSGATE_URL};
use super::*;

const NOW: i64 = 20_000 * 86_400_000 + 3_600_000;

fn cfg(daily: u32) -> Result<Cfg, &'static str> {
    Ok(Cfg { provider: Provider::SmsGate, url: SMSGATE_URL.into(), user: "U".into(), secret: "P".into(), from: String::new(), daily })
}

fn entry(queued: i64) -> Entry {
    Entry::new("o1/sms/confirmed".into(), super::super::plan::KIND, "+355691234567".into(), r#"{"key":"k1","body":"hi"}"#.into(), queued)
}

#[test]
fn off_drops_incomplete_waits() {
    let h = Health::of(None, NOW);
    assert_eq!(gate(&Err("sms_off"), &entry(NOW), &h, NOW), Gate::Drop(None), "the owner turned texts off");
    assert_eq!(gate(&Err("sms_missing_secret"), &entry(NOW), &h, NOW), Gate::Wait, "not set up yet: wait, no try spent");
}

#[test]
fn a_stale_text_and_an_unreadable_one_are_dropped_and_said() {
    let h = Health::of(None, NOW);
    let Gate::Drop(Some(why)) = gate(&cfg(60), &entry(NOW - STALE_MS - 1), &h, NOW) else { panic!() };
    assert!(why.contains("two hours"), "{why}");
    let mut e = entry(NOW);
    e.text = "not json".into();
    assert!(matches!(gate(&cfg(60), &e, &h, NOW), Gate::Drop(Some(_))));
}

#[test]
fn the_daily_budget_is_a_hard_stop_and_resets_at_the_day() {
    let mut h = Health::of(None, NOW);
    h.sent = 2;
    let Gate::Drop(Some(why)) = gate(&cfg(2), &entry(NOW), &h, NOW) else { panic!("over budget must drop") };
    assert!(why.contains("daily SMS limit (2)"), "{why}");
    assert_eq!(gate(&cfg(3), &entry(NOW), &h, NOW), Gate::Send(Payload { key: "k1".into(), body: "hi".into() }));
    let rec = serde_json::to_string(&h).unwrap();
    let next_day = Health::of(Some(&rec), NOW + 86_400_000);
    assert_eq!(next_day.sent, 0, "a new day, a new budget");
    let same_day = Health::of(Some(&rec), NOW + 60_000);
    assert_eq!(same_day.sent, 2);
}

#[test]
fn health_keeps_the_last_failure_across_the_day() {
    let h = Health { day: NOW / 86_400_000 - 1, sent: 9, last_err: Some(Failure { at_ms: 1, code: 401, why: "sms_why_auth".into() }), ..Health::default() };
    let rolled = Health::of(Some(&serde_json::to_string(&h).unwrap()), NOW);
    assert_eq!((rolled.sent, rolled.last_err.map(|f| f.code)), (0, Some(401)));
}
