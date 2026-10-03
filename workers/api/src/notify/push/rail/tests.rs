//! The push service's answers, classified; each refusal with its positive twin.
use super::*;

#[test]
fn a_2xx_is_delivered_and_a_404_or_410_is_a_gone_device() {
    assert_eq!(classify(201), (Push::Done(true), false));
    assert_eq!(classify(200), (Push::Done(true), false));
    assert_eq!(classify(410), (Push::Drop, true));
    assert_eq!(classify(404), (Push::Drop, true));
}

#[test]
fn a_message_the_service_can_never_take_is_dropped_but_the_device_kept() {
    assert_eq!(classify(413), (Push::Drop, false));
    assert_eq!(classify(400), (Push::Drop, false));
}

#[test]
fn a_rate_limit_a_server_error_and_a_vapid_refusal_are_retried() {
    for s in [429, 500, 502, 503, 401, 403] {
        assert_eq!(classify(s), (Push::Done(false), false), "{s}");
    }
}

#[test]
fn the_topic_is_one_per_order_and_audience_within_32_safe_characters() {
    assert_eq!(topic("ord1/push/ready/pc/ord1/aaaa"), "ord1pc");
    assert_eq!(topic("ord1/push/confirmed/pc/ord1/aaaa"), topic("ord1/push/ready/pc/ord1/bbbb"), "a newer status replaces an older one");
    assert_ne!(topic("ord1/push/ready/pk/c1/x"), topic("ord1/push/ready/pc/ord1/x"));
    let long = topic("0123456789abcdef-0123456789abcdef-01/push/new/ps/x/y");
    assert!(long.len() <= 32 && long.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
}
