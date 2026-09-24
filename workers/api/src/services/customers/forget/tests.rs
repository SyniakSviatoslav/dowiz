//! §3.3 in the Worker's half: who the person is, what of them is emptied, and
//! the one failure that matters -- an archive write lost after the hot log
//! and its declaration landed -- converging on retry without double counting.

use super::*;
use crate::services::customers::handlers::customer_key;

const SECRET: &[u8] = b"test-secret-key";
const PHONE: &str = "+355691111111";
const LOC: &str = "venue_1";
const ACTOR: [u8; 32] = [0xA1; 32];

fn order(id: &str, phone: &str, status: &str) -> String {
    format!(
        r#"{{"id":"{id}","location_id":"{LOC}","status":"{status}","total":1800,"items":[{{"name":"Maki","qty":2}}],"contact":{{"name":"Arben {id}","phone":"{phone}"}},"fulfilment":{{"kind":"delivery","note":"ring twice","address":{{"line":"Rruga {id}","note":"floor 3","lat_udeg":41312345,"lon_udeg":19445678}},"fee":200}}}}"#
    )
}

/// Orders `pfx0..pfx2`; `pfx1` is the subject's (placed + advanced with the
/// contact, then a bare status delta), the others strangers'.
fn log(pfx: &str, seq0: u64) -> Hub {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    for i in 0..3u64 {
        let id = format!("{pfx}{i}");
        let phone = if i == 1 { PHONE.to_string() } else { format!("+3556900000{i:02}") };
        h.append(EventKind::Placed, &id, &order(&id, &phone, "PENDING"), seq0 + i * 3 + 1, ACTOR).unwrap();
        h.append(EventKind::Advanced, &id, &order(&id, &phone, "DELIVERED"), seq0 + i * 3 + 2, ACTOR).unwrap();
        h.append(EventKind::Advanced, &id, r#"{"_d":true,"status":"DELIVERED"}"#, seq0 + i * 3 + 3, ACTOR).unwrap();
    }
    h
}

fn views(h: &Hub) -> Vec<OrderView> {
    crate::hubstore::orders_state(h).into_iter().map(OrderView::of_event).collect()
}

fn key() -> String {
    customer_key(SECRET, PHONE)
}

fn act(k: &str) -> Act<'_> {
    Act { key: k, by: "owner_7", reason: "asked at the counter", now_ms: 1_800_000_000_000 }
}

fn has(bytes: &[u8], s: &str) -> bool {
    bytes.windows(s.len()).any(|w| w == s.as_bytes())
}

/// Law 9 across hot + archives.
fn law(hot: &Hub, archives: &[&Hub]) -> (usize, usize) {
    let t = hot.chain_check().redacted + archives.iter().map(|a| a.chain_check().redacted).sum::<usize>();
    (t, hot.declared())
}

#[test]
fn the_person_is_emptied_and_the_order_is_not() {
    let out = redact_order_json(&order("o1", PHONE, "DELIVERED")).expect("it names the person");
    let v: Value = serde_json::from_str(&out).unwrap();
    for (p, want) in [("/contact/phone", json!("")), ("/contact/name", json!("")), ("/fulfilment/note", json!("")),
        ("/fulfilment/address/line", json!("")), ("/fulfilment/address/lat_udeg", Value::Null)] {
        assert_eq!(v.pointer(p), Some(&want), "{p}");
    }
    for (p, want) in [("/total", json!(1800)), ("/status", json!("DELIVERED")), ("/fulfilment/fee", json!(200)),
        ("/items/0/name", json!("Maki"))] {
        assert_eq!(v.pointer(p), Some(&want), "{p} is the ORDER's and moved");
    }
    assert!(!out.contains(PHONE) && !out.contains("Arben") && !out.contains("Rruga"));
    // Nothing of theirs: a delta, and an event already emptied.
    assert_eq!(redact_order_json(r#"{"_d":true,"status":"DELIVERED"}"#), None);
    assert_eq!(redact_order_json(&out), None);
}

#[test]
fn find_keeps_the_keys_orders_of_this_venue_and_names_a_live_one() {
    let h = log("o", 0);
    let mut f = Found::default();
    find(&views(&h), LOC, &key(), SECRET, &mut f);
    assert_eq!(f.orders, BTreeSet::from(["o1".to_string()]));
    assert_eq!(f.phones, BTreeSet::from([PHONE.to_string()]));
    assert_eq!(f.live, None, "a delivered order is not live");

    let mut other_venue = Found::default();
    find(&views(&h), "venue_2", &key(), SECRET, &mut other_venue);
    assert!(other_venue.orders.is_empty(), "another venue's orders were taken");

    let mut live = Hub::create_sized(1 << 16).unwrap();
    live.append(EventKind::Placed, "o9", &order("o9", PHONE, "PENDING"), 1, ACTOR).unwrap();
    let mut f = Found::default();
    find(&views(&live), LOC, &key(), SECRET, &mut f);
    assert_eq!(f.live.as_deref(), Some("o9"));
}

#[test]
fn people_loses_the_card_and_the_legacy_card_and_nobody_elses() {
    let mut t = dowiz_hub::table::Table::create(1 << 20).unwrap();
    let kind = crate::services::customers::record::KIND;
    let legacy = legacy_ids(&BTreeSet::from([PHONE.to_string()]));
    for id in [key().as_str(), legacy[0].as_str(), "0123456789abcdef"] {
        t.put(kind, id, r#"{"note":"x"}"#, &[], &[]).unwrap();
    }
    assert_eq!(forget_people(&mut t, &[key()], &legacy), 2);
    assert!(t.get(kind, &key()).is_none() && t.get(kind, &legacy[0]).is_none());
    assert!(t.get(kind, "0123456789abcdef").is_some(), "a stranger's card went");
    assert_eq!(forget_people(&mut t, &[key()], &legacy), 0, "a second run removes nothing");
}

#[test]
fn hot_and_archive_are_redacted_declared_once_and_the_chain_holds() {
    let (mut hot, mut arch) = (log("o", 100), log("a", 0));
    let (tip_hot, tip_arch) = (hot.tip(), arch.tip());
    let orders = BTreeSet::from(["o1".to_string(), "a1".to_string()]);
    let k = key();
    let mut archives = vec![("log@1".to_string(), arch)];
    let e = erase(&act(&k), &orders, &mut hot, &mut archives).unwrap();
    arch = archives.pop().unwrap().1;

    assert_eq!((e.hot, e.archived.clone(), e.declared), (2, vec![("log@1".to_string(), 2)], 4));
    assert_eq!(law(&hot, &[&arch]), (4, 4), "tombstones == declarations across hot + archives");
    assert_eq!((hot.chain_check().broken, arch.chain_check().broken), (0, 0));
    assert_eq!(arch.tip(), tip_arch, "the archive's seal moved");
    assert_ne!(hot.tip(), tip_hot, "the declaration is a new record");
    for b in [hot.to_bytes(), arch.to_bytes()] {
        assert!(!has(&b, PHONE) && !has(&b, "Arben o1") && !has(&b, "Arben a1"), "the person is still in an image");
    }
    assert!(has(&hot.to_bytes(), "Arben o0"), "a stranger was redacted");

    // Idempotent: nothing left to redact, nothing new to declare.
    let before = hot.to_bytes();
    let mut archives = vec![("log@1".to_string(), arch)];
    assert_eq!(erase(&act(&k), &orders, &mut hot, &mut archives).unwrap().declared, 0);
    assert_eq!(hot.to_bytes(), before);
}

/// THE FAILURE THAT MATTERS: the hot log (and its declaration) landed, the
/// archive write did not. Law 9 is RED -- loudly, as a half-done erasure must
/// be -- and a retry that no fold can help (the hot phones are gone) finds the
/// orders in the declaration, finishes the archive and declares NOTHING more.
#[test]
fn an_archive_lost_after_the_log_is_red_until_the_retry_converges() {
    let mut hot = log("o", 100);
    let untouched_archive = log("a", 0).to_bytes();
    let k = key();
    let orders = BTreeSet::from(["o1".to_string(), "a1".to_string()]);
    let mut archives = vec![("log@1".to_string(), Hub::load(&untouched_archive).unwrap())];
    assert_eq!(erase(&act(&k), &orders, &mut hot, &mut archives).unwrap().declared, 4);

    // The archive's write is lost: storage still holds the old bytes.
    let arch = Hub::load(&untouched_archive).unwrap();
    let (t, d) = law(&hot, &[&arch]);
    assert_eq!((t, d), (2, 4), "a lost archive write must leave law 9 red");

    // The retry: the hot fold finds nobody; the declaration names the scope.
    let mut f = Found::default();
    find(&views(&hot), LOC, &k, SECRET, &mut f);
    assert!(f.orders.is_empty());
    let (declared_orders, declared_archives) = declared_scope(&hot, &k);
    assert_eq!(declared_archives, BTreeSet::from(["log@1".to_string()]));
    let mut archives = vec![("log@1".to_string(), arch)];
    let e = erase(&act(&k), &declared_orders, &mut hot, &mut archives).unwrap();
    assert_eq!((e.hot, e.archived[0].1, e.declared), (0, 2, 0), "the retry declared again");
    assert_eq!(law(&hot, &[&archives[0].1]), (4, 4));
}

#[test]
fn the_declaration_carries_no_contact_details() {
    let mut hot = log("o", 0);
    let k = key();
    erase(&act(&k), &BTreeSet::from(["o1".to_string()]), &mut hot, &mut []).unwrap();
    let d = hot.events().into_iter().find(|e| e.kind == EventKind::Forgotten).unwrap();
    assert_eq!(d.order_id, format!("cust:{k}"));
    assert!(!d.order_json.contains(PHONE) && !d.order_json.contains("Arben"));
    assert_eq!(hot.declared_for(&d.order_id), 2);
}

#[test]
fn only_a_customer_key_is_a_customer_key() {
    assert!(is_customer_key(&key()));
    for bad in ["", "0123456789ABCDEF", "0123456789abcde", "0123456789abcdefg", PHONE] {
        assert!(!is_customer_key(bad), "{bad}");
    }
}

/// G8: forget via ONE spelling finds the orders of the LINKED spelling. The
/// circle comes from the `alias` rows (`run::circle`), and `find_any` takes
/// the whole of it. The twin: the key alone finds only its own orders.
#[test]
fn forget_via_one_spelling_reaches_the_linked_spelling() {
    let national = "069 111 1111";
    let (k, k2) = (key(), customer_key(SECRET, national));
    let mut h = log("o", 0);
    h.append(EventKind::Placed, "n1", &order("n1", national, "DELIVERED"), 100, ACTOR).unwrap();
    let mut people = dowiz_hub::table::Table::create(1 << 16).unwrap();
    crate::services::customers::alias::link(&mut people, &k2, &k, crate::services::customers::alias::By::Rule, "rule", 1).unwrap();
    let aliases = crate::services::customers::alias::Aliases::of(&people);
    let circle = run::circle(&aliases, &k, &BTreeSet::new());
    assert_eq!(circle, BTreeSet::from([k.clone(), k2.clone()]));
    let mut f = Found::default();
    find_any(&views(&h), LOC, &circle, SECRET, &mut f);
    assert_eq!(f.orders, BTreeSet::from(["o1".to_string(), "n1".to_string()]));
    let mut alone = Found::default();
    find(&views(&h), LOC, &k, SECRET, &mut alone);
    assert_eq!(alone.orders, BTreeSet::from(["o1".to_string()]), "the twin: one key misses the linked spelling");
    // And the people image loses both cards AND the link between them.
    let card = crate::services::customers::record::KIND;
    people.put(card, &k, "{}", &[], &[]).unwrap();
    people.put(card, &k2, "{}", &[], &[]).unwrap();
    assert_eq!(forget_people(&mut people, &circle.iter().cloned().collect::<Vec<_>>(), &[]), 3);
    assert!(people.all(crate::services::customers::alias::KIND).is_empty(), "the link survives the person");
}

/// P3's CHECK. A bundle taken BEFORE an erasure is restored: the replay,
/// driven only by the register's entry (no alias rows, no people image), finds
/// the person in the restored log, redacts them, and the declared count is
/// the one the log had before the restore.
#[test]
fn a_restored_bundle_from_before_the_erasure_is_forgotten_again() {
    let bundle = log("o", 0).to_bytes();
    let k = key();
    // The erasure, as the route runs it: find, register, erase.
    let mut hot = Hub::load(&bundle).unwrap();
    let keys = BTreeSet::from([k.clone()]);
    let mut f = Found::default();
    find_any(&views(&hot), LOC, &keys, SECRET, &mut f);
    let entry = register::Entry::new(LOC, &k, &keys, &f.orders, 1_800_000_000_000);
    erase(&act(&k), &f.orders, &mut hot, &mut []).unwrap();
    let declared_before = hot.declared();
    assert!(declared_before > 0);

    // The restore: the old bytes are back, the phone with them.
    let mut restored = Hub::load(&bundle).unwrap();
    assert!(has(&restored.to_bytes(), PHONE));
    // The replay (`run::replay`): the register's keys and orders, the fold.
    let mut again = Found::default();
    find_any(&views(&restored), LOC, &run::circle(&Default::default(), &entry.key, &entry.keys), SECRET, &mut again);
    let orders: BTreeSet<String> = again.orders.union(&entry.orders).cloned().collect();
    erase(&act(&k), &orders, &mut restored, &mut []).unwrap();
    let bytes = restored.to_bytes();
    assert!(!has(&bytes, PHONE) && !has(&bytes, "Arben o1"), "the person came back with the backup");
    assert_eq!(restored.declared(), declared_before, "declared after the replay equals the pre-restore value");
    assert_eq!(law(&restored, &[]), (declared_before, declared_before));
    assert!(has(&bytes, "Arben o0"), "a stranger was redacted by the replay");
}

/// P3: the answer's promise is TRUE and said in the console's three
/// languages. "22 days" is derived from the rotation's own constant (the last
/// copy made before tonight's is at most 21 days old when it goes, plus the
/// night it was made), so shortening or lengthening the rotation without the
/// sentence turns this red.
#[test]
fn the_backup_promise_matches_the_rotation_in_three_languages() {
    let days = crate::cloud::KEEP_WEEKLY_MS / 86_400_000 + 1;
    for lang in ["sq", "en", "uk"] {
        let n = notice(lang);
        assert!(n.contains(&format!("{days} ")), "{lang}: {n}");
        assert!(!n.contains("month") && !n.contains("muaj") && !n.contains("місяц"), "{lang}: {n}");
    }
    assert_ne!(notice("sq"), notice("en"));
    assert_ne!(notice("uk"), notice("en"));
    assert_eq!(notice("xx"), notice("sq"), "an unknown language answers in the venue's own");
}
