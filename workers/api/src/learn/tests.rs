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
        // No film is recorded in Russian (operator 2026-09-26): a ru cut is not a key.
        "W3/ru/video.mp4",
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

/// W-INT2 #31: the writer's manifest (`tools/learn/publish.mjs` -> the repo copy the R2 upload
/// carries) and this reader agree: every file the wiki will ask for is a key `media_key` serves,
/// under the lesson that lists it, and the narrowing keeps exactly the caller's track.
#[test]
fn every_file_the_published_manifest_names_is_a_key_this_route_serves() {
    let m: serde_json::Value = serde_json::from_str(include_str!("../../public/learn/media/manifest.json")).unwrap();
    let lessons = m["lessons"].as_object().expect("lessons");
    assert!(!lessons.is_empty());
    let mut files = 0;
    for (id, l) in lessons {
        for (_, cut) in l["cuts"].as_object().into_iter().flatten() {
            let mut urls: Vec<&str> = ["video", "poster", "chapters"].iter().filter_map(|k| cut[*k].as_str()).collect();
            urls.extend(cut["subs"].as_object().into_iter().flatten().filter_map(|(_, u)| u.as_str()));
            for u in urls {
                let raw = u.strip_prefix("/api/learn/media/").unwrap_or_else(|| panic!("{u} is not the gated route"));
                let (lesson, key) = media_key(raw).unwrap_or_else(|| panic!("{u} is not a key the route serves"));
                assert_eq!(&lesson, id, "{u}");
                assert_eq!(key, format!("{PREFIX}{raw}"));
                files += 1;
            }
        }
    }
    assert!(files >= lessons.len(), "{files}");
    let all = filter_manifest(m.clone(), Audience::Owner);
    assert_eq!(all["lessons"].as_object().unwrap().len(), lessons.len(), "the owner sees every published lesson");
    let rider = filter_manifest(m, Audience::Courier);
    assert!(rider["lessons"].as_object().unwrap().keys().all(|id| id.starts_with('C') || id.starts_with('G')));
}

/// The route itself, natively: the caller is judged before the bucket is touched, and a
/// deployment without the `LEARN` binding says so (503) instead of answering an empty list.
#[test]
fn the_manifest_route_judges_the_caller_and_names_a_missing_bucket() {
    use crate::edge::site::{get, As, Site, PLATFORM_HOST};
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    let url = format!("https://alpha.{PLATFORM_HOST}/api/learn/manifest");
    let r = site.run(super::manifest, get(&url).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "{}", r.body_str());
    // A platform token that owns no venue is refused by `auth::authenticate` itself (401, the
    // membership check) before `audience` is asked; `audience`'s own 403 for a token naming no
    // venue is `audience_admits_the_venues_people_and_refuses_the_rest`.
    let r = site.run(super::manifest, get(&url).bearer(&site.admin_token()).on("alpha"), &[]);
    assert_eq!(r.status_code(), 401, "a platform token: {}", r.body_str());
    assert!(r.body_str().contains("owner membership"), "{}", r.body_str());
    let r = site.run(super::manifest, get(&url).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 503, "{}", r.body_str());
}
