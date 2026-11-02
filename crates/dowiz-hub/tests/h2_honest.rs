//! W-MG-HOLES2: the positive twins of the dense seq and the walk budget. Real files, written by the
//! real writers, each shape the two rules could have refused by mistake. Green is the point.
use einstore::evlog::doc::keys::KeyOp;
use einstore::evlog::doc::{self, Doc, Report, Trust};
use einstore::nodekey::sha256;
use einstore::Store;
use dowiz_hub::doc_seal::{DeviceKey, HubIds, MlDsa65, SealedHub};
use dowiz_hub::{EventKind, Hub};
use std::time::Instant;

const MOVE: &str = r#"{"item":"salmon","delta":-1,"unit":"portion"}"#;
const PHONE: &str = "+355691111111";

fn key(seed: u8) -> DeviceKey {
    DeviceKey::from_seed(&[seed; 32])
}
fn fresh(a: &DeviceKey) -> SealedHub {
    let mut sh = SealedHub::create(a).unwrap();
    sh.set_entropy([1; 32]);
    sh
}
fn orders(sh: &mut SealedHub, k: &DeviceKey, tag: &str, n: u32) {
    (0..n).for_each(|i| drop(sh.append(EventKind::Placed, &format!("{tag}{i}"), MOVE, k, i as i64 + 1).unwrap()));
}
fn holds(name: &str, bytes: &[u8], roots: &[[u8; 32]]) -> Report {
    let r = doc::verify(&Store::from_bytes(bytes), &HubIds, &MlDsa65, Trust::Only(roots)).unwrap_or_else(|e| panic!("{name}: refused {e:?}"));
    assert!(r.holds(), "{name}: {:?}", r.counters);
    r
}

/// Pre-seal records as a venue has them: legacy ones, then the flip, at the hub's own numbers,
/// actor 0^32. The device's first sealed record is seq 1 and chains onto them.
fn pre_seal() -> Vec<u8> {
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    hub.set_sha256(false);
    (1..=2u64).for_each(|i| drop(hub.append(EventKind::Placed, &format!("l{i}"), MOVE, i * 7, [0u8; 32]).unwrap()));
    hub.set_sha256(true);
    (3..=5u64).for_each(|i| drop(hub.append(EventKind::Placed, &format!("o{i}"), MOVE, i * 7, [0u8; 32]).unwrap()));
    hub.to_bytes_trimmed()
}

#[test]
fn a_sealed_hub_over_pre_seal_records_holds_through_forget_and_cut() {
    let a = key(0xA1);
    let mut sh = SealedHub::seal_hub(&pre_seal(), &a).unwrap();
    sh.set_entropy([1; 32]);
    let roots = [a.id()];
    orders(&mut sh, &a, "c", 3);
    let r = holds("sealed", &sh.to_bytes_trimmed(), &roots);
    assert_eq!((r.counters.legacy, r.counters.chained, r.counters.broken), (2, 6, 0), "{:?}", r.counters);
    sh.append(EventKind::Placed, "p1", &format!(r#"{{"phone":"{PHONE}"}}"#), &a, 9).unwrap();
    orders(&mut sh, &a, "z", 2);
    let n = sh.forget(|e| e.order_json.contains(PHONE).then(|| "{}".to_string()), &sha256(PHONE.as_bytes()), &a, 5).unwrap();
    assert_eq!(n, 1);
    assert_eq!(holds("forgotten", &sh.to_bytes_trimmed(), &roots).counters.redacted, 1);
    sh.cut(4, &a, 9).unwrap();
    holds("cut", &sh.to_bytes_trimmed(), &roots);
    orders(&mut sh, &a, "y", 2);
    holds("after the cut", &sh.to_bytes_trimmed(), &roots);
}

/// B's first record chains onto A's tip. The cut stops at B's ADMIT (`CutsAdmit`, the twin below);
/// A's kept chain continues its frontier row and B's own chain is untouched.
#[test]
fn two_devices_keep_their_seqs_across_cuts() {
    let (a, b) = (key(0xA1), key(0xB2));
    let mut sh = fresh(&a);
    orders(&mut sh, &a, "c", 2);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 2).unwrap();
    orders(&mut sh, &b, "b", 3);
    orders(&mut sh, &a, "d", 3);
    let roots = [a.id()];
    holds("two devices", &sh.to_bytes_trimmed(), &roots);
    assert!(sh.cut(3, &a, 9).is_err(), "a cut through B's ADMIT is refused");
    sh.cut(7, &a, 9).unwrap();
    holds("two devices cut", &sh.to_bytes_trimmed(), &roots);
    orders(&mut sh, &a, "e", 1);
    orders(&mut sh, &b, "f", 1);
    holds("both after", &sh.to_bytes_trimmed(), &roots);
}

/// A REVOKE after 2 with everything B sealed at or below it: the rule is about new records.
#[test]
fn a_revoked_device_keeps_what_it_sealed() {
    let (a, b) = (key(0xA1), key(0xB2));
    let mut sh = fresh(&a);
    orders(&mut sh, &a, "c", 1);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 2).unwrap();
    orders(&mut sh, &b, "b", 2);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 2, tip: sh.tip_at(&b.id(), 2) }, &a, 3).unwrap();
    holds("revoked", &sh.to_bytes_trimmed(), &[a.id()]);
}

/// The budget's positive twin: a real file past 1 MB, one seal per record, is read in full.
#[test]
fn a_real_file_past_a_megabyte_is_not_refused() {
    let a = key(0xA1);
    let mut sh = fresh(&a);
    orders(&mut sh, &a, "c", 340);
    let f = sh.to_bytes_trimmed();
    let t = Instant::now();
    let r = holds("big", &f, &[a.id()]);
    println!("H2 honest: file={} B records={} seals_ok={} verify={:.3}s", f.len(), r.counters.records, r.counters.seals_ok, t.elapsed().as_secs_f64());
    assert_eq!(r.counters.seals_ok, 340);
    assert!(f.len() > 1 << 20, "not past 1 MB: {}", f.len());
}

/// A segment is at least one record: `last < first` is no SEG, by name; its twin reads.
#[test]
fn an_inverted_segment_is_refused() {
    let a = key(0xA1);
    let mut sh = fresh(&a);
    orders(&mut sh, &a, "c", 4);
    sh.cut(2, &a, 9).unwrap();
    let f = sh.to_bytes_trimmed();
    let roots = [a.id()];
    holds("cut", &f, &roots);
    let v = Doc::open(&Store::from_bytes(&f)).unwrap();
    let mut st = Store::from_bytes(&f);
    let at = v.seg.unwrap();
    let first = st.get(at, 1);
    st.put_cell(at, 1, st.get(at, 2) + 1);
    st.seal(at);
    assert!(matches!(doc::verify(&st, &HubIds, &MlDsa65, Trust::Only(&roots)), Err(doc::DocError::Corrupt(_))), "first was {first}");
}
