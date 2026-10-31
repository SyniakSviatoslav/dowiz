//! W-MG-M3b END TO END, on REAL hub bytes and real ML-DSA-65: seal, anchor, forget, cut; the
//! cut file verifies with no archive bytes and holds; the anchor over a cut record is the
//! archive's to place, and the archive places it; a revoked device's FORGOTTEN declares
//! nothing; every reader written before refuses the sealed bytes by name.

use super::tests::{device, real_hub, JSON};
use super::tsa::{self, PlatformTsa};
use super::{HubIds, SealedHub, KIND_DECL};
use crate::{Event, EventKind, Hub, HubError};
use bebop_store::evlog::doc::seg::{parse_truncate, read_seg};
use bebop_store::evlog::doc::{AnchorCounts, Doc, Ids, KeyOp, Verdict, KIND_PLATFORM_TSA};
use bebop_store::evlog::EvLog;
use bebop_store::nodekey::sha256;

const PHONE: &str = "+355691111111";

fn order(id: &str, phone: &str) -> String {
    format!(r#"{{"id":"{id}","status":"new","total":1800,"contact":{{"name":"N {id}","phone":"{phone}"}}}}"#)
}

fn redactor(e: &Event) -> Option<String> {
    e.order_json.contains(PHONE).then(|| e.order_json.replace(PHONE, ""))
}

fn ids_oldest_first(sh: &SealedHub) -> Vec<[u8; 32]> {
    let log = Doc::open(&sh.store).unwrap().log;
    let mut v: Vec<[u8; 32]> = EvLog::walk_until_at(&sh.store, log, |_| false).iter().map(|r| r.id).collect();
    v.reverse();
    v
}

/// Six real records, then the person's two orders and one other, an anchor over that tip,
/// two more orders; FORGET the person (2 redacted, 1 FORGOTTEN); CUT all but the newest 3.
/// The cut image holds under ML-DSA-65 from its own bytes; the archive recomputes the root
/// and holds the early anchor's id; anchoring the cut file still works.
#[test]
fn a_real_hub_is_sealed_anchored_forgotten_cut_and_still_holds() {
    let key = device(7);
    let platform = device(42);
    let mut sh = SealedHub::seal_hub(&real_hub(), &key).unwrap();
    sh.append(EventKind::Placed, "p1", &order("p1", PHONE), &key, 1).unwrap();
    sh.append(EventKind::Placed, "p2", &order("p2", PHONE), &key, 2).unwrap();
    sh.append(EventKind::Placed, "o7", JSON, &key, 3).unwrap();
    let early = sh.tip().unwrap();
    sh.anchor(KIND_PLATFORM_TSA, &early, 1_730_000_000_000, &tsa::mark(&platform, &early, 1_730_000_000_000)).unwrap();
    sh.append(EventKind::Placed, "o8", JSON, &key, 4).unwrap();
    sh.append(EventKind::Placed, "o9", JSON, &key, 5).unwrap();
    let before = ids_oldest_first(&sh);
    assert_eq!(before.len(), 11);

    // FORGET: two tombstones and one signed FORGOTTEN; the phone is gone from every event.
    let subject = sha256(PHONE.as_bytes());
    assert_eq!(sh.forget(redactor, &subject, &key, 6).unwrap(), 2);
    assert_eq!(sh.forget(redactor, &subject, &key, 6).unwrap(), 0, "a second erasure writes nothing");
    let r = sh.verify().unwrap();
    let c = r.counters;
    assert_eq!((c.records, c.legacy, c.chained, c.redacted, c.declared, c.broken, c.seals_ok), (12, 1, 9, 2, 2, 0, 6), "{c:?}");
    assert!(r.holds(), "{c:?}");
    assert!(sh.events().iter().all(|e| !e.order_json.contains(PHONE)), "the phone left the bytes");
    assert_eq!(sh.events().len(), 11, "the FORGOTTEN is not an order event");
    assert_eq!(ids_oldest_first(&sh)[..11], before[..], "redaction in place: every id kept");

    // CUT: the six pre-seal records, the two tombstones and o7 leave under one segment.
    let archive = sh.cut(3, &key, 7).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let sh = SealedHub::load(&bytes).unwrap();
    let r = sh.verify().unwrap();
    let c = r.counters;
    assert_eq!((c.records, c.legacy, c.chained, c.broken, c.seals_ok, c.seals_bad), (4, 0, 4, 0, 4, 0), "{c:?}");
    assert_eq!((c.segments, c.cut, c.cut_redacted, c.redacted, c.declared), (1, 9, 2, 0, 2), "{c:?}");
    assert!(r.holds(), "the cut file holds from its own bytes: {c:?}");
    assert_eq!(sh.events().len(), 2, "o8 and o9 stay; the declarations are skipped by name");
    assert_eq!(sh.len(), 4);
    let seg_at = Doc::open(&sh.store).unwrap().seg.unwrap();
    let seg = read_seg(&sh.store, seg_at).unwrap();
    let truncate = EvLog::walk_until_at(&sh.store, Doc::open(&sh.store).unwrap().log, |_| false)
        .iter()
        .find_map(|r| (crate::body_of(r).first() == Some(&KIND_DECL)).then(|| parse_truncate(HubIds.body(r.flags, &r.payload))).flatten())
        .expect("the TRUNCATE record");
    assert!(seg.is_named_by(&truncate));
    assert_eq!((truncate.first, truncate.last, truncate.redacted, truncate.declared), (0, 8, 2, 0));

    // THE ARCHIVE places what the file cannot: the early anchor's `over` is a cut id, the cut
    // ids climb to the root the TRUNCATE signed, and the archive itself still holds.
    let arch = SealedHub::load(&archive).unwrap();
    assert!(arch.verify().unwrap().holds());
    let ids = ids_oldest_first(&arch);
    assert_eq!(ids.len(), 12);
    assert!(ids[..9].contains(&early));
    assert_eq!(HubIds.tree_root(&ids[..9]), Some(seg.root));
    assert_eq!(crate::rotate::segment_root(&ids[..9]), seg.root, "one root for an archive and a segment");

    // THE ANCHORS: the early one is under the cut -- unchecked, counted, not bad; a new one
    // over the cut file's tip checks under the platform key.
    let keys = [platform.public_key().to_vec()];
    let checked = sh.anchors(&PlatformTsa { keys: &keys }).unwrap();
    assert_eq!(AnchorCounts::of(&checked), AnchorCounts { anchors: 1, ok: 0, bad: 0, unchecked: 1, cut: 1 }, "{checked:?}");
    assert!(checked[0].under_cut && !checked[0].names_record);
    assert!(matches!(&checked[0].verdict, Verdict::Unchecked(w) if w.contains("1 segment")), "{:?}", checked[0].verdict);
    let mut sh = sh;
    let tip = sh.tip().unwrap();
    sh.anchor(KIND_PLATFORM_TSA, &tip, 1_730_000_010_000, &tsa::mark(&platform, &tip, 1_730_000_010_000)).unwrap();
    let checked = sh.anchors(&PlatformTsa { keys: &keys }).unwrap();
    assert_eq!(AnchorCounts::of(&checked), AnchorCounts { anchors: 2, ok: 1, bad: 0, unchecked: 1, cut: 1 }, "{checked:?}");
    assert!(sh.verify().unwrap().holds());

    // EVERY READER WRITTEN BEFORE refuses the cut, sealed bytes by name.
    assert!(matches!(Hub::load(&sh.to_bytes_trimmed()), Err(HubError::UnknownVersion(5))));
}

/// KILL (M4 on the hub): B redacts and declares -- holds. The owner revokes B after seq 2;
/// B's FORGOTTEN was seq 3: `declared` falls to 0, `redacted` stays 1, the file no longer
/// holds, and B's next forget is refused with the hub left as it was. The owner's REVOKE
/// chains onto the owner's own ADMIT (`broken` 0), not onto B's tip.
#[test]
fn a_forgotten_by_a_revoked_device_declares_nothing_on_a_sealed_hub() {
    let owner = device(7);
    let b = device(8);
    let subject = sha256(PHONE.as_bytes());
    let mut sh = SealedHub::create(&owner).unwrap();
    sh.commit_keyop(&KeyOp::Admit(b.key()), &owner, 1).unwrap();
    sh.append(EventKind::Placed, "p1", &order("p1", PHONE), &b, 2).unwrap();
    sh.append(EventKind::Placed, "o2", JSON, &b, 3).unwrap();
    assert_eq!(sh.forget(redactor, &subject, &b, 4).unwrap(), 1);
    let r = sh.verify().unwrap();
    assert_eq!((r.counters.records, r.counters.redacted, r.counters.declared, r.counters.broken, r.counters.seals_ok), (4, 1, 1, 0, 4), "{:?}", r.counters);
    assert!(r.holds(), "{:?}", r.counters);

    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 2 }, &owner, 5).unwrap();
    let r = sh.verify().unwrap();
    assert_eq!((r.counters.records, r.counters.redacted, r.counters.declared, r.counters.broken, r.counters.seals_bad), (5, 1, 0, 0, 1), "{:?}", r.counters);
    assert!(!r.holds(), "a revoked key's FORGOTTEN declares nothing");

    let bytes = sh.to_bytes_trimmed();
    let e = sh.forget(|e| (e.order_id == "o2").then(|| JSON.replace("new", "gone")), &subject, &b, 6).unwrap_err();
    assert!(format!("{e:?}").contains("revoked"), "{e:?}");
    assert_eq!(sh.to_bytes_trimmed(), bytes, "a refused declaration leaves the hub as it was");
}
