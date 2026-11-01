//! W-MG-ATK: THE TABLE, every attack that is NOT a hole. ADD / REMOVE / REORDER / DUPLICATE /
//! REPLAY / SWAP per object, no private key, `Trust::Only`, real ML-DSA-65. `caught` = holds false
//! or a refusal, and names the counter; `inert` = holds stays true and the forger gained nothing
//! `verify` promises; `limit` = holds stays true, stated in FORMAT §9.4/§9.5, pinned here.
mod atk_forge;

use atk_forge::*;
use bebop_store::evlog::doc::keys::{KeyOp, ADMIT_MARK};
use bebop_store::evlog::doc::Report;
use bebop_store::evlog::{Record, FLAG_NONCE_DERIVED};
use dowiz_hub::doc_seal::{DeviceKey, SealedHub};
use dowiz_hub::EventKind;

const MOVE: &str = r#"{"item":"salmon","delta":-1,"unit":"portion"}"#;

fn key(seed: u8) -> DeviceKey {
    DeviceKey::from_seed(&[seed; 32])
}
fn orders(sh: &mut SealedHub, key: &DeviceKey, tag: &str, n: u8) {
    (0..n).for_each(|i| drop(sh.append(EventKind::Placed, &format!("{tag}{i}"), MOVE, key, i as i64 + 1).unwrap()));
}

/// A: two orders, ADMIT B; B: three orders; A: one order. All sealed.
fn base(entropy: u8) -> (DeviceKey, DeviceKey, Vec<u8>) {
    let (a, b) = (key(0xA1), key(0xB2));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([entropy; 32]);
    orders(&mut sh, &a, "c", 2);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 3).unwrap();
    orders(&mut sh, &b, "b", 3);
    orders(&mut sh, &a, "d", 1);
    let bytes = sh.to_bytes_trimmed();
    (a, b, bytes)
}

fn caught(name: &str, s: &Side, roots: &[Id], by: impl Fn(&Report) -> bool) {
    match only(name, &write(s), roots) {
        Ok(r) => assert!(!r.holds() && by(&r), "{name}: {:?}", r.counters),
        Err(e) => panic!("{name}: refused {e:?}, this row names a counter"),
    }
}
fn refused(name: &str, s: &Side, roots: &[Id]) {
    assert!(only(name, &write(s), roots).is_err(), "{name}: not refused");
}
fn stays(name: &str, s: &Side, roots: &[Id]) -> Report {
    let r = only(name, &write(s), roots).unwrap();
    assert!(r.holds(), "{name}: {:?}", r.counters);
    r
}

#[test]
fn records() {
    let (a, b, bytes) = base(1);
    let roots = [a.id()];
    let s = round_trip(&bytes, &roots);
    let mut x = s.clone();
    let top = x.chains[0].last().unwrap().clone();
    x.recs.push(forged(top.actor_id, top.seq + 1, top.tip, 0x55));
    caught("rec-add-tail", &x, &roots, |r| r.counters.unsealed == 1);
    let mut x = s.clone();
    let r3 = x.recs[1].clone();
    x.recs.insert(2, forged(r3.actor_pubkey, r3.actor_seq + 1, r3.id, 0x56));
    caught("rec-add-middle", &x, &roots, |r| r.counters.broken >= 1);
    let mut x = s.clone();
    x.recs.remove(1);
    caught("rec-remove-middle", &x, &roots, |r| r.counters.broken >= 1);
    let mut x = s.clone();
    x.recs.pop();
    caught("rec-remove-newest-seal-kept", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.recs.swap(0, 1);
    caught("rec-reorder-one-device", &x, &roots, |r| r.counters.broken >= 1);
    let mut x = s.clone();
    let d = x.recs[1].clone();
    x.recs.insert(2, d);
    caught("rec-duplicate", &x, &roots, |r| r.counters.forks == 1);
    let mut x = s.clone();
    x.recs.last_mut().unwrap().actor_pubkey = b.id();
    caught("rec-relabel-sealed-actor", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.recs.last_mut().unwrap().actor_seq += 1;
    caught("rec-reseq-sealed", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.recs[0].flags |= FLAG_NONCE_DERIVED;
    caught("rec-flag-derived-set", &x, &roots, |r| r.counters.broken >= 1);
    // SWAP: the same position, another file of the same owner (other nonces, other ids).
    let (_, _, other) = base(2);
    let o = read(&other);
    let mut x = s.clone();
    x.recs[0] = o.recs[0].clone();
    caught("rec-swap-from-other-file", &x, &roots, |r| r.counters.broken >= 1 || r.counters.seals_bad >= 1);
    // LIMIT: the file's order across devices is signed by nobody (`settle.rs`, K3 interleaved).
    let mut x = s.clone();
    let (i, j) = (x.recs.len() - 2, x.recs.len() - 1);
    x.recs.swap(i, j);
    let f = write(&x);
    let names = |by: &[u8]| SealedHub::load(by).unwrap().events().iter().map(|e| e.order_id.clone()).collect::<Vec<_>>();
    assert_ne!(names(&bytes), names(&f), "the fold reads another order");
    stays("rec-reorder-across-devices LIMIT", &x, &roots);
}

#[test]
fn seals() {
    let (a, _b, bytes) = base(1);
    let roots = [a.id()];
    let s = round_trip(&bytes, &roots);
    let i = s.chains.iter().position(|c| c[0].actor_id == a.id()).unwrap();
    let mut x = s.clone();
    let d = x.chains[i].last().unwrap().clone();
    x.chains[i].push(d);
    caught("seal-duplicate-top", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.chains[i].remove(1);
    caught("seal-remove-middle", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.chains[i].remove(0);
    caught("seal-remove-oldest", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.chains[i].swap(0, 1);
    caught("seal-reorder", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.chains[i].pop();
    caught("seal-replay-older-top-records-kept", &x, &roots, |r| r.counters.unsealed >= 1);
    let mut x = s.clone();
    x.chains[i].last_mut().unwrap().claimed_ms += 1;
    caught("seal-claimed_ms", &x, &roots, |r| r.counters.seals_bad >= 1);
    let (_, _, other) = base(2);
    let mut x = s.clone();
    let o = read(&other);
    x.chains[i] = o.chains[i].clone();
    caught("seal-swap-from-other-file", &x, &roots, |r| r.counters.seals_bad >= 1 || r.counters.unsealed >= 1);
}

#[test]
fn heads_and_keys() {
    let (a, b, bytes) = base(1);
    let roots = [a.id()];
    let s = round_trip(&bytes, &roots);
    let mut x = s.clone();
    x.chains.retain(|c| c[0].actor_id != b.id());
    caught("heads-remove-row", &x, &roots, |r| r.counters.unsealed >= 1);
    let mut x = s.clone();
    x.bare_rows.push(([0x33; 32], 9, [0x44; 32]));
    let r = stays("heads-add-bare-row INERT", &x, &roots);
    assert_eq!(r.heads.len(), 3, "the row is listed, and proves nothing");
    let mut x = s.clone();
    let d = x.chains[0].clone();
    x.chains.push(d);
    stays("heads-duplicate-row INERT (seals_ok counts it twice)", &x, &roots);
    let mut x = s.clone();
    x.rows_reversed = true;
    stays("heads-reorder INERT", &x, &roots);

    let mut x = s.clone();
    x.keys.retain(|k| k.0.actor_id != b.id());
    caught("key-remove-admitted", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.keys.retain(|k| k.0.actor_id != a.id());
    caught("key-remove-root", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    let f = key(0xF0);
    x.keys.push((f.key(), None));
    let r = stays("key-add-unadmitted INERT", &x, &roots);
    assert_eq!((r.keys, r.admitted), (3, 2));
    // An ADMITTED key's pk IS bound: the ADMIT record names it. (A root's is not: HOLE 1.)
    let mut x = s.clone();
    let i = x.keys.iter().position(|k| k.0.actor_id == b.id()).unwrap();
    x.keys[i].0.pk = f.public_key().to_vec();
    caught("key-swap-pk-of-admitted", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    let at = pos_of(&x, ADMIT_MARK);
    x.keys[i].1 = Some(x.recs[at + 1].id);
    caught("key-repoint-admit-rec", &x, &roots, |r| r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.keys[i].0.revoked_at = 1;
    caught("key-add-revocation", &x, &roots, |r| r.counters.keys_disagree == 1);
}

#[test]
fn replay_keys_from_an_older_copy() {
    let (a, b, bytes) = base(1);
    let roots = [a.id()];
    let old = read(&bytes);
    let mut sh = SealedHub::load(&bytes).unwrap();
    sh.set_entropy([3; 32]);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 3, tip: sh.tip_at(&b.id(), 3) }, &a, 9).unwrap();
    let mut s = round_trip(&sh.to_bytes_trimmed(), &roots);
    s.keys = old.keys.clone();
    caught("keys-replay-pre-revoke", &s, &roots, |r| r.counters.keys_disagree == 1);
}

/// The erasure un-done from a copy before it, alone: law 9 sees it (HOLE 3 is the swap).
#[test]
fn replay_a_record_from_before_its_erasure() {
    let a = key(0xA1);
    let roots = [a.id()];
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "p", 1);
    orders(&mut sh, &a, "z", 1);
    let old = read(&sh.to_bytes_trimmed());
    assert_eq!(sh.forget(|e| (e.order_id == "p0").then(|| "{}".to_string()), &[1; 32], &a, 3).unwrap(), 1);
    let bytes = sh.to_bytes_trimmed();
    let mut s = round_trip(&bytes, &roots);
    s.recs[0] = old.recs[0].clone();
    caught("rec-replay-unerased", &s, &roots, |r| r.counters.redacted == 0 && r.counters.declared == 1 && r.counters.tombs_foreign == 1);
    // The cells a tombstone's id no longer checks: the FORGOTTEN's digest does.
    let mut s = read(&bytes);
    s.recs[0].actor_seq += 9;
    caught("tomb-seq-moved", &s, &roots, |r| r.counters.tombs_foreign == 1);
}

/// An OLDER segment's counts, moved together so law 9 still balances: only the TRUNCATE's hash
/// over the chain says so. Twice cut; the first TRUNCATE left with the second cut.
#[test]
fn an_older_segments_balanced_counts_are_signed() {
    let a = key(0xA1);
    let roots = [a.id()];
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 5);
    sh.cut(3, &a, 6).unwrap();
    sh.cut(1, &a, 7).unwrap();
    let mut s = round_trip(&sh.to_bytes_trimmed(), &roots);
    s.segs[1].redacted += 1;
    s.segs[1].declared += 1;
    refused("seg-older-counts-moved", &s, &roots);
}

/// A REVOKE of a key that never sealed, its KEY row gone: the REVOKE names nothing, no seal needed it.
#[test]
fn a_revoked_keys_row_removed() {
    let (a, b, bytes) = base(1);
    let c = key(0xC3);
    let roots = [a.id()];
    let mut sh = SealedHub::load(&bytes).unwrap();
    sh.set_entropy([3; 32]);
    sh.commit_keyop(&KeyOp::Admit(c.key()), &a, 8).unwrap();
    sh.commit_keyop(&KeyOp::Revoke { actor_id: c.id(), after_seq: 0, tip: sh.tip_at(&c.id(), 0) }, &a, 9).unwrap();
    let mut s = round_trip(&sh.to_bytes_trimmed(), &roots);
    s.keys.retain(|k| k.0.actor_id != c.id());
    stays("key-remove-revoked-never-sealed INERT", &s, &roots);
    let _ = b;
}

#[test]
fn segments() {
    let a = key(0xA1);
    let roots = [a.id()];
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 5);
    let whole = sh.to_bytes_trimmed();
    sh.cut(3, &a, 6).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let s = round_trip(&bytes, &roots);
    let mut x = s.clone();
    x.segs.clear();
    caught("seg-remove", &x, &roots, |r| r.counters.unsealed >= 1 || r.counters.seals_bad >= 1);
    let mut x = s.clone();
    x.segs[0].last += 1;
    refused("seg-count-moved", &x, &roots);
    let mut x = s.clone();
    x.segs[0].declared += 1;
    refused("seg-declared-moved (direct)", &x, &roots);
    let mut x = s.clone();
    let d = x.segs[0].clone();
    x.segs.push(d);
    refused("seg-duplicate", &x, &roots);
    let mut x = read(&whole);
    x.segs = s.segs.clone();
    refused("seg-add-to-uncut", &x, &roots);
    let mut x = s.clone();
    x.segs[0].frontier[0].scheme = 0;
    refused("seg-scheme-flipped", &x, &roots);
    let mut x = s.clone();
    x.segs[0].frontier[0].tip = [9; 32];
    refused("seg-row-to-nowhere", &x, &roots);
}

#[path = "atk_caught/limits.rs"]
mod limits;
