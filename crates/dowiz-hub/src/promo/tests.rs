use super::*;

const HOUR: i64 = 3_600_000;
const NOW: i64 = 1_789_000_000_000;

fn pct(v: i64) -> Promo {
    Promo {
        code: "SAVE10".into(),
        kind: Kind::Percent,
        value: v,
        min_order: 0,
        from_ms: None,
        until_ms: None,
        max_uses: None,
        active: true,
    }
}

#[test]
fn a_code_is_the_same_code_however_the_customer_types_it() {
    for raw in ["save10", "SAVE10", " Save 10 ", "save-10", "save10\n"] {
        assert_eq!(normalise(raw), "SAVE10", "{raw:?}");
    }
}

#[test]
fn a_percentage_rounds_down_never_up() {
    // 33% of 1000 is 330.0; 33% of 1001 is 330.33, and the customer gets
    // 330. The venue is never short by the rounding.
    assert_eq!(pct(33).discount(1000), 330);
    assert_eq!(pct(33).discount(1001), 330);
    assert_eq!(pct(33).discount(1004), 331);
}

#[test]
fn a_discount_never_exceeds_the_food() {
    let flat = Promo { kind: Kind::Fixed, value: 5000, ..pct(10) };
    assert_eq!(flat.discount(2000), 2000, "a 5000 code on a 2000 basket takes 2000");
    assert_eq!(flat.discount(0), 0);
    assert_eq!(pct(100).discount(2650), 2650);
}

/// Release builds wrap on overflow. A hundred per cent of a basket near the
/// i64 ceiling must not come back negative -- a negative discount is money
/// ADDED to the bill.
#[test]
fn a_huge_basket_does_not_wrap_the_discount() {
    let d = pct(100).discount(i64::MAX);
    assert!(d >= 0 && d <= i64::MAX, "discount out of range: {d}");
    assert_eq!(d, i64::MAX);
}

#[test]
fn each_refusal_names_itself() {
    let base = Promo {
        from_ms: Some(NOW),
        until_ms: Some(NOW + HOUR),
        max_uses: Some(2),
        min_order: 1000,
        ..pct(10)
    };
    assert_eq!(base.redeem(2000, NOW - 1, 0), Err(Refusal::Scheduled));
    assert_eq!(base.redeem(2000, NOW + HOUR, 0), Err(Refusal::Expired));
    assert_eq!(base.redeem(2000, NOW + 1, 2), Err(Refusal::Exhausted));
    assert_eq!(base.redeem(999, NOW + 1, 0), Err(Refusal::BelowMinimum(1000)));
    assert_eq!(Promo { active: false, ..base.clone() }.redeem(2000, NOW + 1, 0),
               Err(Refusal::Inactive));
    assert_eq!(base.redeem(2000, NOW + 1, 1), Ok(200));
}

/// The window is half-open: the first millisecond counts, the last does
/// not. An "until midnight" code that still works at 00:00:00.000 the next
/// day is the classic off-by-one that shows up as a complaint.
#[test]
fn the_window_is_half_open() {
    let p = Promo { from_ms: Some(NOW), until_ms: Some(NOW + HOUR), ..pct(10) };
    assert_eq!(p.status(NOW - 1, 0), Status::Scheduled);
    assert_eq!(p.status(NOW, 0), Status::Active, "the first instant counts");
    assert_eq!(p.status(NOW + HOUR - 1, 0), Status::Active);
    assert_eq!(p.status(NOW + HOUR, 0), Status::Expired, "the last does not");
}

/// The owner's switch is read first: "I turned it off" is the answer they
/// are looking for, even when the code also expired.
#[test]
fn off_reads_as_off_even_when_it_also_expired() {
    let p = Promo { active: false, until_ms: Some(NOW - HOUR), ..pct(10) };
    assert_eq!(p.status(NOW, 0), Status::Inactive);
}

#[test]
fn a_promo_round_trips_through_json() {
    let p = Promo {
        code: "WELCOME".into(),
        kind: Kind::Fixed,
        value: 300,
        min_order: 1500,
        from_ms: Some(NOW),
        until_ms: Some(NOW + HOUR),
        max_uses: Some(50),
        active: false,
    };
    assert_eq!(Promo::parse(&p.to_json()), Some(p));
}

#[test]
fn an_unusable_promo_is_refused_at_parse_rather_than_applied() {
    for bad in [
        r#"{"code":"X","kind":"percent","value":10}"#,          // code too short
        r#"{"code":"SAVE","kind":"percent","value":0}"#,        // 0%
        r#"{"code":"SAVE","kind":"percent","value":101}"#,      // over 100%
        r#"{"code":"SAVE","kind":"percent","value":-10}"#,      // negative
        r#"{"code":"SAVE","kind":"fixed","value":0}"#,          // a code that does nothing
        r#"{"code":"SAVE","kind":"freebie","value":1}"#,        // unknown kind
        r#"{"code":"SAVE","value":10}"#,                        // no kind
    ] {
        assert_eq!(Promo::parse(bad), None, "accepted: {bad}");
    }
}

/// `maxUses: 0` would mean a code that can never be used, which nobody
/// creates on purpose -- it is what an empty form field serialises to. It
/// reads as no limit.
#[test]
fn a_zero_use_cap_reads_as_no_cap() {
    let p = Promo::parse(r#"{"code":"SAVE","kind":"percent","value":10,"maxUses":0}"#).unwrap();
    assert_eq!(p.max_uses, None);
    assert_eq!(p.status(NOW, 9_999), Status::Active);
}

#[test]
fn active_defaults_to_on_but_false_survives() {
    let on = Promo::parse(r#"{"code":"SAVE","kind":"percent","value":10}"#).unwrap();
    assert!(on.active);
    let off =
        Promo::parse(r#"{"code":"SAVE","kind":"percent","value":10,"active":false}"#).unwrap();
    assert!(!off.active);
}
