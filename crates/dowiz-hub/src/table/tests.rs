use super::*;

const CEIL: usize = 256 * 1024;

fn idx(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

#[test]
fn a_record_comes_back_as_it_went_in() {
    let mut t = Table::create(CEIL).unwrap();
    t.put("user", "u1", r#"{"email":"a@b.c"}"#, &idx(&[("user.email/a@b.c", "u1")]), &[])
        .unwrap();
    assert_eq!(t.get("user", "u1").as_deref(), Some(r#"{"email":"a@b.c"}"#));
    assert_eq!(t.lookup("user.email/a@b.c").as_deref(), Some("u1"));
    assert!(t.has("user", "u1"));
    assert!(!t.has("user", "u2"));
}

#[test]
fn it_survives_a_round_trip_through_bytes() {
    let mut t = Table::create(CEIL).unwrap();
    t.put("user", "u1", r#"{"n":1}"#, &idx(&[("user.email/a", "u1")]), &[]).unwrap();
    let bytes = t.to_bytes().unwrap();
    let back = Table::load(&bytes, CEIL).unwrap();
    assert_eq!(back.get("user", "u1").as_deref(), Some(r#"{"n":1}"#));
    assert_eq!(back.lookup("user.email/a").as_deref(), Some("u1"));
    assert_eq!(back.root(), t.root(), "the fold is the same on both sides");
}

/// The defect this whole file is shaped to prevent.
#[test]
fn replacing_a_record_drops_the_keys_the_old_one_owned() {
    let mut t = Table::create(CEIL).unwrap();
    t.put("user", "u1", r#"{"email":"old@x"}"#, &idx(&[("user.email/old@x", "u1")]), &[])
        .unwrap();
    t.put("user", "u1", r#"{"email":"new@x"}"#, &idx(&[("user.email/new@x", "u1")]), &[])
        .unwrap();
    assert_eq!(t.lookup("user.email/new@x").as_deref(), Some("u1"));
    assert_eq!(t.lookup("user.email/old@x"), None, "the old email still finds the user");
}

#[test]
fn removing_a_record_removes_every_key_that_found_it() {
    let mut t = Table::create(CEIL).unwrap();
    t.put(
        "user",
        "u1",
        "{}",
        &idx(&[("user.email/a", "u1"), ("member.by_user/u1/v1", "owner")]),
        &[],
    )
    .unwrap();
    assert!(t.remove("user", "u1"));
    assert_eq!(t.get("user", "u1"), None);
    assert_eq!(t.lookup("user.email/a"), None);
    assert_eq!(t.lookup("member.by_user/u1/v1"), None);
    assert!(!t.remove("user", "u1"), "removing twice is not a second removal");
}

#[test]
fn a_unique_key_refuses_a_second_holder_and_allows_the_first_one_back() {
    let mut t = Table::create(CEIL).unwrap();
    let k = "user.email/a@b.c";
    t.put("user", "u1", "{}", &idx(&[(k, "u1")]), &[k]).unwrap();
    // The same record re-saved under its own key: allowed. This is the half
    // an ON CONFLICT clause always had to be read twice to be sure of.
    t.put("user", "u1", r#"{"n":2}"#, &idx(&[(k, "u1")]), &[k]).unwrap();
    let err = t.put("user", "u2", "{}", &idx(&[(k, "u2")]), &[k]).unwrap_err();
    assert_eq!(err, TableError::Taken { key: k.into(), held_by: "u1".into() });
    assert_eq!(t.get("user", "u2"), None, "a refused write leaves nothing behind");
}

/// A prefix scan IS the composite index, and the venue is in the key.
#[test]
fn a_prefix_scan_is_the_venue_filter_that_cannot_be_forgotten() {
    let mut t = Table::create(CEIL).unwrap();
    for (id, venue, role) in
        [("m1", "dubin", "owner"), ("m2", "sushi", "owner"), ("m3", "dubin", "staff")]
    {
        t.put(
            "member",
            id,
            "{}",
            &idx(&[(&format!("member/{venue}/{id}"), role)]),
            &[],
        )
        .unwrap();
    }
    let dubin = t.scan("member/dubin/");
    assert_eq!(dubin.len(), 2);
    assert!(dubin.iter().all(|(k, _)| k.starts_with("member/dubin/")));
    // Sorted, because the layout keeps its keys sorted -- this is ORDER BY.
    assert_eq!(dubin[0].0, "member/dubin/m1");
    assert_eq!(dubin[1].0, "member/dubin/m3");
    assert_eq!(t.scan("member/sushi/").len(), 1);
    assert_eq!(t.scan("member/nowhere/").len(), 0);
}

#[test]
fn all_lists_one_kind_and_not_another() {
    let mut t = Table::create(CEIL).unwrap();
    t.put("user", "u1", r#"{"a":1}"#, &[], &[]).unwrap();
    t.put("user", "u2", r#"{"a":2}"#, &[], &[]).unwrap();
    t.put("org", "o1", r#"{"a":3}"#, &[], &[]).unwrap();
    let users = t.all("user");
    assert_eq!(users.len(), 2);
    assert_eq!(users[0], ("u1".to_string(), r#"{"a":1}"#.to_string()));
    assert_eq!(t.all("org").len(), 1);
    assert_eq!(t.all("nothing").len(), 0);
}

/// F27: rebuilding the index from the records alone changes nothing.
#[test]
fn rebuilding_the_index_from_the_records_changes_nothing() {
    let mut t = Table::create(CEIL).unwrap();
    let index = |_kind: &str, id: &str, json: &str| -> Vec<(String, String)> {
        let email = json.split('"').nth(3).unwrap_or("");
        vec![(format!("user.email/{email}"), id.to_string())]
    };
    for (id, email) in [("u1", "a@x"), ("u2", "b@x")] {
        let json = format!(r#"{{"email":"{email}"}}"#);
        let ix = index("user", id, &json);
        t.put("user", id, &json, &ix, &[]).unwrap();
    }
    let before = t.root();
    t.rebuild_index(&["user"], index);
    assert_eq!(t.root(), before, "a rebuild moved the fold: a writer and the indexer disagree");
}

/// And it FAILS when they disagree, which is the half that makes it a gate.
#[test]
fn rebuilding_catches_an_index_a_writer_wrote_by_hand() {
    let mut t = Table::create(CEIL).unwrap();
    t.put("user", "u1", r#"{"email":"a@x"}"#, &idx(&[("user.email/WRONG", "u1")]), &[])
        .unwrap();
    let before = t.root();
    t.rebuild_index(&["user"], |_k, id, json| {
        let email = json.split('"').nth(3).unwrap_or("");
        vec![(format!("user.email/{email}"), id.to_string())]
    });
    assert_ne!(t.root(), before, "a hand-written index survived a rebuild unnoticed");
    assert_eq!(t.lookup("user.email/a@x").as_deref(), Some("u1"));
    assert_eq!(t.lookup("user.email/WRONG"), None);
}

#[test]
fn the_fold_does_not_depend_on_the_order_things_were_written() {
    let mut a = Table::create(CEIL).unwrap();
    let mut b = Table::create(CEIL).unwrap();
    a.put("user", "u1", "{}", &idx(&[("k/1", "u1")]), &[]).unwrap();
    a.put("user", "u2", "{}", &idx(&[("k/2", "u2")]), &[]).unwrap();
    b.put("user", "u2", "{}", &idx(&[("k/2", "u2")]), &[]).unwrap();
    b.put("user", "u1", "{}", &idx(&[("k/1", "u1")]), &[]).unwrap();
    assert_eq!(a.root(), b.root());
}

/// The gauge rule from `bebop-ceiling-not-capacity`: a reading must never
/// fall as the image grows, which is what `used/capacity` did.
#[test]
fn the_usage_reading_never_falls_as_the_image_grows() {
    let mut t = Table::create(CEIL).unwrap();
    let mut worst = 0;
    for n in 0..200 {
        t.put("user", &format!("u{n:04}"), &format!(r#"{{"n":{n},"pad":"{}"}}"#, "x".repeat(64)), &[], &[])
            .unwrap();
        // Re-load through bytes so the capacity really is re-chosen, which
        // is where the sawtooth came from.
        let bytes = t.to_bytes().unwrap();
        t = Table::load(&bytes, CEIL).unwrap();
        let u = t.usage();
        // Against the CEILING, which is the refusal point, never against
        // the capacity the doubling loop last happened to pick.
        let per_mille = u.used_cells * 1000 / u.ceiling_cells.max(1);
        assert!(per_mille >= worst, "the reading fell from {worst} to {per_mille} at {n}");
        worst = per_mille;
    }
    assert!(worst > 0, "200 records and the gauge never moved");
}
