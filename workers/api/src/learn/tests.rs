//! L7 (gated): who may watch which lesson, which keys are served, and Range -- each
//! refusal beside the positive twin that proves the rule is not "refuse everything".

use super::*;
use dowiz_hub::caps::{Cap, Caps};

fn owner() -> Principal {
    Principal::Owner { user_id: "u".into(), active_location_id: Some("dubin-durres".into()) }
}
fn platform() -> Principal {
    Principal::Owner { user_id: "u".into(), active_location_id: None }
}
fn staff() -> Principal {
    Principal::Staff { person_id: "p".into(), active_location_id: "v".into(), session_id: "s".into(), caps: Caps::of(&[Cap::TakeOrders]) }
}
fn courier() -> Principal {
    Principal::Courier { courier_id: "c".into(), active_location_id: "v".into(), session_id: "s".into() }
}
fn customer() -> Principal {
    Principal::Customer { customer_id: "k".into(), order_id: "o".into(), location_id: "v".into() }
}

#[test]
fn audience_admits_the_venues_people_and_refuses_the_rest() {
    assert_eq!(audience(&owner()), Ok(Audience::Owner));
    assert_eq!(audience(&staff()), Ok(Audience::Staff));
    assert_eq!(audience(&courier()), Ok(Audience::Courier));
    assert_eq!(audience(&customer()).unwrap_err().0, 403);
    assert_eq!(audience(&platform()).unwrap_err().0, 403);
}

#[test]
fn each_track_is_its_roles_and_the_owner_sees_all() {
    for id in ["O1a", "W3", "C5", "G1"] {
        assert!(may_see(Audience::Owner, id), "{id}");
    }
    assert!(may_see(Audience::Staff, "W3") && may_see(Audience::Staff, "G1"));
    assert!(!may_see(Audience::Staff, "O1a") && !may_see(Audience::Staff, "C5"));
    assert!(may_see(Audience::Courier, "C5") && may_see(Audience::Courier, "G1"));
    assert!(!may_see(Audience::Courier, "W3") && !may_see(Audience::Courier, "O1a"));
    assert!(!may_see(Audience::Owner, "X1") && !may_see(Audience::Owner, ""));
}

#[test]
fn media_key_serves_a_cuts_files_and_nothing_else() {
    assert_eq!(media_key("W3/sq/video.mp4"), Some(("W3".into(), "learn/W3/sq/video.mp4".into())));
    assert_eq!(media_key("/O1a/uk/subs_en.vtt").unwrap().1, "learn/O1a/uk/subs_en.vtt");
    assert!(media_key("C5/en/step-3.mp4").is_some());
    assert!(media_key("G1/sq/chapters.json").is_some());
    for bad in [
        "W3/sq/../../manifest.json", "W3/de/video.mp4", "W3/sq/video.exe", "W3/sq/Video.mp4", "W3/sq/.mp4",
        "w3/sq/video.mp4", "W/sq/video.mp4", "W3ab/sq/video.mp4", "W3/sq", "W3/sq/a/video.mp4", "",
        "W3/sq/video", "manifest.json",
    ] {
        assert_eq!(media_key(bad), None, "{bad}");
    }
}

#[test]
fn content_type_names_the_five_kinds() {
    assert_eq!(content_type("mp4"), Some("video/mp4"));
    assert_eq!(content_type("vtt"), Some("text/vtt; charset=utf-8"));
    assert_eq!(content_type("jpg"), Some("image/jpeg"));
    assert_eq!(content_type("webp"), Some("image/webp"));
    assert_eq!(content_type("json"), Some("application/json"));
    assert_eq!(content_type("html"), None);
}

#[test]
fn parse_range_reads_one_range_and_ignores_what_it_cannot() {
    assert_eq!(parse_range(Some("bytes=0-")), Some(ByteRange::From(0)));
    assert_eq!(parse_range(Some("bytes=100-199")), Some(ByteRange::Span(100, 199)));
    assert_eq!(parse_range(Some("bytes=-500")), Some(ByteRange::Suffix(500)));
    for bad in [None, Some("bytes=-0"), Some("bytes=5-1"), Some("bytes=0-1,5-9"), Some("items=0-1"), Some("bytes=-"), Some("bytes=a-b"), Some("bytes=1")] {
        assert_eq!(parse_range(bad), None, "{bad:?}");
    }
}

#[test]
fn resolve_clamps_to_the_object_and_refuses_past_its_end() {
    assert_eq!(resolve(ByteRange::From(0), 1000), Some((0, 999)));
    assert_eq!(resolve(ByteRange::Span(100, 5000), 1000), Some((100, 999)));
    assert_eq!(resolve(ByteRange::Suffix(300), 1000), Some((700, 999)));
    assert_eq!(resolve(ByteRange::Suffix(5000), 1000), Some((0, 999)));
    assert_eq!(resolve(ByteRange::From(1000), 1000), None);
    assert_eq!(resolve(ByteRange::Span(1000, 1001), 1000), None);
    assert_eq!(resolve(ByteRange::From(0), 0), None);
}

#[test]
fn the_manifest_is_narrowed_to_the_callers_track() {
    let m = serde_json::json!({ "version": 1, "lessons": { "O1a": {}, "W3": {}, "C5": {}, "G1": {} } });
    let keys = |a| {
        let v = filter_manifest(m.clone(), a);
        let mut k: Vec<String> = v["lessons"].as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    assert_eq!(keys(Audience::Owner), ["C5", "G1", "O1a", "W3"]);
    assert_eq!(keys(Audience::Staff), ["G1", "W3"]);
    assert_eq!(keys(Audience::Courier), ["C5", "G1"]);
    let odd = serde_json::json!({ "version": 1 });
    assert_eq!(filter_manifest(odd.clone(), Audience::Staff), odd);
}
