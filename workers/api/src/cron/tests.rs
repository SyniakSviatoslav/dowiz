use super::*;

fn q(pairs: &[(&str, &str)]) -> Option<(String, i64)> {
    parse_runner_query(pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())))
}

#[test]
fn a_runner_is_never_a_venue() {
    assert_eq!(runner_name("dubin-durres"), "cron~dubin-durres");
    assert_ne!(runner_name("dubin-durres"), "dubin-durres");
}

#[test]
fn the_path_carries_the_venue_and_the_one_clock_read() {
    let p = runner_path("dubin durres&x", 1_790_000_000_000);
    assert_eq!(p, "https://cron/fold/cron?venue=dubin+durres%26x&now=1790000000000");
    let url = Url::parse(&p).unwrap();
    let back = parse_runner_query(url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())));
    assert_eq!(back, Some(("dubin durres&x".to_string(), 1_790_000_000_000)));
}

#[test]
fn a_runner_query_without_a_venue_or_a_clock_is_refused() {
    assert_eq!(q(&[("venue", "v"), ("now", "5")]), Some(("v".into(), 5)));
    assert_eq!(q(&[("now", "5")]), None, "no venue");
    assert_eq!(q(&[("venue", ""), ("now", "5")]), None, "empty venue");
    assert_eq!(q(&[("venue", "v")]), None, "no clock");
    assert_eq!(q(&[("venue", "v"), ("now", "soon")]), None, "a clock that is not a number");
}

/// THE NIGHTLY'S QUESTION goes to the venue's OWN object and carries its name
/// and the one clock read, which the object parses with the runner's parser.
#[test]
fn the_nightly_asks_the_venues_own_object_with_its_name_and_clock() {
    let p = timer_path("dubin-durres", 1_790_000_000_000);
    assert_eq!(p, "https://hub/fold/timer?venue=dubin-durres&now=1790000000000");
    let url = Url::parse(&p).unwrap();
    let back = parse_runner_query(url.query_pairs().map(|(k, v)| (k.to_string(), v.to_string())));
    assert_eq!(back, Some(("dubin-durres".to_string(), 1_790_000_000_000)));
}
