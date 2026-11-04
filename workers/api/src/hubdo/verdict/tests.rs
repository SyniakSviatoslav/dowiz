//! `POST /fold/verdict` (W-OWNER-CPU2): the platform object answers exactly the records the
//! Worker used to read out of the images it loaded; an owner route costs the Worker no
//! platform IMAGE any more, only the verdict's few bytes; authority is still live (a removed
//! owner, a revoked key, a gone staff membership: refused on the next call); and a venue's
//! object refuses to say who anyone is.

use crate::auth::verdict::{self, Answer, Ask};
use crate::edge::mem::{block_on, object_paths, object_traffic};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::identity_store as ids;
use crate::platform_store::PLATFORM;
use crate::wire::Call;
use serde_json::json;
use worker::Method;

fn venue_id(site: &Site, slug: &str) -> String {
    block_on(crate::hubstore::Place::of_slug(&site.ctx(&[]), slug)).expect("registry").venue
}

fn user_of(token: &str, site: &Site) -> String {
    match crate::auth::verify(&site.env(), token, site.now_ms).expect("verify") {
        crate::auth::Claims::Owner { user_id, .. } => user_id,
        crate::auth::Claims::Staff { sub, .. } | crate::auth::Claims::Courier { sub, .. } => sub,
        c => panic!("{c:?}"),
    }
}

fn jti_of(token: &str, site: &Site) -> String {
    match crate::auth::verify(&site.env(), token, site.now_ms).expect("verify") {
        crate::auth::Claims::Staff { jti, .. } | crate::auth::Claims::Courier { jti, .. } => jti,
        c => panic!("{c:?}"),
    }
}

/// The object requests `f` made, by path, and the answer.
fn paid<T>(f: impl FnOnce() -> T) -> (Vec<String>, Vec<(String, usize, usize)>, T) {
    let (from, tfrom) = (object_paths().len(), object_traffic().len());
    let r = f();
    (object_paths()[from..].to_vec(), object_traffic()[tfrom..].to_vec(), r)
}

fn categories(site: &Site, token: &str) -> crate::wire::Reply {
    site.run(crate::catalog_edit::list_categories, get(&format!("https://alpha.{PLATFORM_HOST}/api/owner/categories?location_id=alpha")).bearer(token).on("alpha"), &[])
}

/// THE ORACLE: the Worker's old reads -- the whole image, then one record -- give the same
/// records the object answers, present and absent, for every ask.
#[test]
fn the_object_answers_the_records_the_worker_used_to_read_out_of_the_images() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let staff = site.staff("alpha", &owner, "w@x.test", "kitchen");
    let (rider, rider_id) = site.courier("alpha", &owner, "+355694444444");
    let (alpha, env) = (venue_id(&site, "alpha"), site.env());
    let (owner_id, staff_id) = (user_of(&owner, &site), user_of(&staff, &site));
    let (sjti, cjti) = (jti_of(&staff, &site), jti_of(&rider, &site));
    // The images, read as the Worker read them.
    let ident = block_on(ids::identity(&env)).expect("identity");
    let sess = block_on(ids::sessions(&env)).expect("sessions");
    let crew = block_on(ids::couriers(&env)).expect("couriers");

    let m = block_on(verdict::member(&env, &alpha, &owner_id)).expect("ask");
    assert_eq!(m, ids::membership(&ident, &alpha, &owner_id));
    assert_eq!(ids::s_of(m.as_ref().unwrap(), "role"), "owner", "not vacuous");
    assert_eq!(block_on(verdict::member(&env, &alpha, "nobody")).expect("ask"), None);
    assert_eq!(block_on(verdict::member(&env, "beta", &owner_id)).expect("ask"), None, "another venue");

    let owned = block_on(verdict::owned(&env, &owner_id)).expect("ask");
    assert_eq!(owned, ids::memberships_of(&ident, &owner_id));
    assert_eq!(owned, vec![(alpha.clone(), "owner".to_string())]);

    let (sm, ss) = block_on(verdict::staff(&env, &alpha, &staff_id, &sjti)).expect("ask");
    assert_eq!(sm, ids::membership(&ident, &alpha, &staff_id));
    assert_eq!(ss, ids::rec(&sess, crate::services::identity::staff_rules::K_SSESSION, &sjti));
    assert!(sm.is_some() && ss.is_some(), "not vacuous");

    let (cs, on) = block_on(verdict::courier(&env, &alpha, &rider_id, &cjti)).expect("ask");
    assert_eq!(cs, ids::rec(&sess, ids::K_CSESSION, &cjti));
    assert!(cs.is_some() && on, "not vacuous");
    assert!(crew.get(ids::K_ROSTER, &ids::roster_id(&alpha, &rider_id)).is_some());
    // Somebody else's session row is nobody's.
    assert_eq!(block_on(verdict::courier(&env, &alpha, "c_other", &cjti)).expect("ask"), (None, false));

    // THE SIZE IS THE POINT: each answer is a few hundred bytes, the images are not.
    for ask in [Ask::Member { venue: alpha.clone(), user: owner_id.clone() }, Ask::Owned { user: owner_id.clone() }, Ask::Staff { venue: alpha.clone(), user: staff_id, jti: sjti }, Ask::Courier { venue: alpha.clone(), courier: rider_id, jti: cjti }] {
        let a = block_on(verdict::ask(&env, &ask)).expect("ask");
        let n = super::bytes_of(&serde_json::to_value(&a).unwrap());
        assert!(n < 600, "{ask:?}: {n} bytes");
    }
    let image = block_on(ids::identity(&env)).expect("identity").to_bytes().expect("bytes").len();
    assert!(image > 4_000, "the identity image is {image} bytes: the thing the Worker no longer loads");
}

/// RED on 4e42be75: an owner route that is already a relay still fetched `/img/identity` (the
/// whole image) into the Worker to check the membership. Now the platform object is asked once
/// (`/fold/verdict`, a few bytes) and no platform image crosses.
#[test]
fn an_owner_route_costs_the_worker_a_verdict_and_no_platform_image() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let (paths, traffic, r) = paid(|| categories(&site, &owner));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(paths.contains(&"/fold/verdict".to_string()), "{paths:?}");
    assert!(!paths.iter().any(|p| p.starts_with("/img/identity") || p.starts_with("/img/sessions") || p.starts_with("/img/couriers")), "a platform image crossed: {paths:?}");
    let (_, sent, answered) = traffic.iter().find(|(p, _, _)| p == "/fold/verdict").expect("the verdict's bytes");
    assert!(sent + answered < 600, "{sent} + {answered} bytes crossed for the verdict");
    // And the staff path, through the same route (MENU): one verdict, no image.
    let staff = site.staff("alpha", &owner, "k@x.test", "kitchen");
    let (paths, _, r) = paid(|| categories(&site, &staff));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(paths.contains(&"/fold/verdict".to_string()) && !paths.iter().any(|p| p.starts_with("/img/")), "{paths:?}");
}

/// THE SECURITY PROPERTY, unchanged: authority is re-derived live. A membership removed between
/// two calls refuses the second, with the token still valid; the staff's and the owner's.
#[test]
fn a_removed_owner_and_a_removed_staff_member_are_refused_at_once_holding_valid_tokens() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let staff = site.staff("alpha", &owner, "w@x.test", "kitchen");
    let alpha = venue_id(&site, "alpha");
    assert_eq!(categories(&site, &owner).status_code(), 200);
    assert_eq!(categories(&site, &staff).status_code(), 200);
    for who in [user_of(&owner, &site), user_of(&staff, &site)] {
        let (a, w) = (alpha.clone(), who.clone());
        block_on(ids::with_identity(&site.env(), move |t| {
            assert!(t.remove(ids::K_MEMBER, &ids::member_id(&a, &w)), "the membership was there");
            Ok(())
        }))
        .expect("remove");
    }
    let r = categories(&site, &owner);
    assert_eq!(r.status_code(), 404, "a removed owner: {}", r.body_str());
    assert!(block_on(crate::auth::authenticate(&get("https://x/").bearer(&owner), &site.env(), site.now_ms)).is_err());
    let r = categories(&site, &staff);
    assert_eq!(r.status_code(), 401, "a removed cook: {}", r.body_str());
    assert!(r.body_str().contains("staff membership is gone"), "{}", r.body_str());
    // A token still verifies: the refusal is the live record's, not the signature's.
    assert!(crate::auth::verify(&site.env(), &owner, site.now_ms).is_ok());
}

/// An API key: judged and `last_used_ms` stamped by the platform object; the Worker reads no
/// sessions image and writes none. A revoked key and a wrong secret refuse in the old words.
#[test]
fn an_api_key_is_judged_and_stamped_in_the_platform_object() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let at = |p: &str| format!("https://alpha.{PLATFORM_HOST}{p}");
    let r = site.run(crate::services::identity::keys::create_api_key, post(&at("/api/owner/apikeys?location_id=alpha"), &json!({"label": "robot"})).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let (key, id) = (r.body_value()["key"].as_str().unwrap().to_string(), r.body_value()["id"].as_str().unwrap().to_string());
    // THE KEY'S DOOR is `authenticate` (`owner_at`, the MCP surface); a key is not a JWT and
    // `owner_beside` does not take one, as before.
    let judge = |bearer: &str| block_on(crate::auth::authenticate(&get("https://alpha.dowiz.org/api/mcp").bearer(bearer), &site.env(), site.now_ms));
    let (paths, _, p) = paid(|| judge(&key));
    assert!(matches!(p, Ok(crate::auth::Principal::Owner { ref active_location_id, .. }) if active_location_id.as_deref() == Some("alpha")), "the key opens alpha");
    assert!(paths.contains(&"/fold/verdict".to_string()) && !paths.iter().any(|p| p.starts_with("/img/")), "{paths:?}");
    let sess = block_on(ids::sessions(&site.env())).expect("sessions");
    let rec = ids::rec(&sess, ids::K_APIKEY, &id).expect("the key's row");
    assert_eq!(ids::i_of(&rec, "last_used_ms"), site.now_ms, "stamped in the object: {rec}");
    // The wrong secret, the same id: "no such key", as before.
    let (kid, _) = key.strip_prefix("dowiz_").unwrap().split_once('.').unwrap();
    let r = judge(&format!("dowiz_{kid}.not-the-secret")).err().expect("refused").into_response().unwrap();
    assert_eq!((r.status_code(), r.body_str()), (401, "not authorised: no such key".to_string()));
    // Revoked: refused at once.
    let r = site.run(crate::services::identity::keys::revoke_api_key, post(&at("/api/owner/apikeys/revoke?location_id=alpha"), &json!({"id": id})).bearer(&owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = judge(&key).err().expect("refused").into_response().unwrap();
    assert_eq!((r.status_code(), r.body_str()), (401, "not authorised: that key was revoked".to_string()));
    // The pure judgement, at its edges.
    let row = json!({"owner_id": "o", "location_id": "l", "key_hash": crate::auth::hash_opaque("s"), "expires_at_ms": 10});
    assert_eq!(verdict::judge_api_key(Some(&row), "s", 9), Ok(("o".into(), "l".into())));
    assert_eq!(verdict::judge_api_key(Some(&row), "s", 10), Err(verdict::KEY_EXPIRED));
    assert_eq!(verdict::judge_api_key(Some(&row), "x", 9), Err(verdict::KEY_UNKNOWN));
    assert_eq!(verdict::judge_api_key(None, "s", 9), Err(verdict::KEY_UNKNOWN));
    assert_eq!(verdict::reason("something else"), verdict::KEY_UNKNOWN, "an unknown word fails closed");
}

/// Only the platform object knows who anyone is: a venue's object answers 409, not "nobody".
#[test]
fn a_venue_object_refuses_to_say_who_someone_is_and_a_closed_body_is_refused() {
    let site = Site::new();
    let owner = site.venue("alpha", "a@x.test");
    let alpha = venue_id(&site, "alpha");
    let ask = Ask::Member { venue: alpha.clone(), user: user_of(&owner, &site) };
    let r = block_on(site.object(&alpha).route(Call::new("https://hub/fold/verdict", Method::Post).unwrap().with_json(&ask))).expect("answered");
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    let r = block_on(site.object(PLATFORM).route(Call::new("https://hub/fold/verdict", Method::Post).unwrap().with_json(&ask))).expect("answered");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(r.body_value()["member"].is_object(), "{}", r.body_str());
    let mut v = serde_json::to_value(&ask).unwrap();
    v["extra"] = json!(1);
    assert!(block_on(site.object(PLATFORM).route(Call::new("https://hub/fold/verdict", Method::Post).unwrap().with_json(&v))).is_err(), "an unknown field is an error");
    // An absent field answers `None`, never a parse failure.
    let a: Answer = serde_json::from_str("{}").unwrap();
    assert_eq!(a, Answer::default());
}

/// WHERE THE MILLISECONDS WENT, measured natively (wasm is 2-4x this): `Table::load` of an
/// identity image with `n` people (crc walk of every object) against one `membership` lookup.
/// `cargo test --lib verdict::tests::measure -- --ignored --nocapture`.
#[test]
#[ignore]
fn measure_identity_image_load_against_one_record() {
    use dowiz_hub::table::Table;
    for n in [10usize, 50, 200, 1000] {
        let mut t = Table::create(crate::platform_store::IDENTITY_BYTES).unwrap();
        for i in 0..n {
            let (u, v) = (format!("usr_{i:04}"), format!("venue-{}", i % 7));
            let rec = json!({"id": u, "email": format!("person{i}@example.test"), "name": "Someone Somewhere", "password_hash": "$argon2id$v=19$m=8192,t=3,p=1$c2FsdHNhbHRzYWx0c2FsdA$aGFzaGhhc2hoYXNoaGFzaGhhc2hoYXNoaGFzaGhhc2g", "created_at_ms": 1_700_000_000_000i64 + i as i64}).to_string();
            t.put(ids::K_USER, &u, &rec, &[(ids::user_by_email(&format!("person{i}@example.test")), u.clone())], &[]).unwrap();
            let m = json!({"user_id": u, "location_id": v, "role": if i % 5 == 0 { "owner" } else { "kitchen" }, "status": "active", "created_at_ms": 1_700_000_000_000i64}).to_string();
            t.put(ids::K_MEMBER, &ids::member_id(&v, &u), &m, &[(ids::member_by_venue(&v, &u), ids::member_id(&v, &u)), (ids::member_by_user(&u, &v), ids::member_id(&v, &u))], &[]).unwrap();
        }
        let bytes = t.to_bytes().unwrap();
        let median = |f: &dyn Fn()| {
            let mut v: Vec<u128> = (0..9).map(|_| { let s = std::time::Instant::now(); f(); s.elapsed().as_nanos() }).collect();
            v.sort_unstable();
            v[4] as f64 / 1e3
        };
        let load = median(&|| { std::hint::black_box(Table::load(&bytes, crate::platform_store::IDENTITY_BYTES).unwrap()); });
        let loaded = Table::load(&bytes, crate::platform_store::IDENTITY_BYTES).unwrap();
        let lookup = median(&|| { std::hint::black_box(ids::membership(&loaded, "venue-3", "usr_0003")); });
        println!("VERDICT n={n}: image {} bytes | Table::load {load:.0} us | membership lookup {lookup:.1} us", bytes.len());
    }
}
