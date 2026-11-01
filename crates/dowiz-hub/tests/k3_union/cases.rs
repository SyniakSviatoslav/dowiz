//! The K3 cases, each number as measured. `what_holds_does_not_cover` and the wiped REVOKE pin
//! DEFECTS: when `verify` learns them these lines go red and are meant to be changed.
use super::*;

fn pin(name: &str, bytes: &[u8], want: (usize, usize, usize, usize, usize, usize), holds: bool) {
    let r = report(name, bytes);
    assert_eq!((sums(&r), r.holds()), (want, holds), "{name}: (records, broken, seals_ok, seals_bad, forks, admitted), holds");
}

/// `pin`, and how many records no signed tip reaches.
fn pin_u(name: &str, bytes: &[u8], want: (usize, usize, usize, usize, usize, usize), unsealed: usize) {
    pin(name, bytes, want, false);
    assert_eq!(report(name, bytes).counters.unsealed, unsealed, "{name}");
}

fn sign_seal(key: &DeviceKey, s: &Seal, prev_tip: &Id) -> Seal {
    let sig = key.sign(&message(&s.actor_id, s.seq, &s.tip, s.claimed_ms, prev_tip));
    Seal { sig, prev: None, ..s.clone() }
}

/// A record of `actor` at `seq` after `prev`, its id right, no key used.
fn forged(actor: Id, seq: u64, prev: Id, tag: u8) -> Record {
    let mut payload = vec![tag; 16];
    payload.push(EventKind::Placed as u8);
    payload.extend_from_slice(b"\x01xforged");
    let mut r = Record { id: [0; 32], prev, actor_pubkey: actor, actor_seq: seq, flags: FLAG_SHA256, payload };
    r.id = doc::Ids::chained(&HubIds, &r);
    r
}

#[test]
fn the_seals_survive_a_union() {
    let (a, b) = keys();
    let (fa, fb) = offline(&a, &b, 5, 4);
    pin("copy-a", &fa, (9, 0, 9, 0, 0, 2), true);
    pin("copy-b", &fb, (8, 0, 8, 0, 0, 2), true);
    let (sa, sb) = (side(&fa), side(&fb));
    let u = union(&sa, &sb);
    pin("union", &write(&u), (13, 0, 13, 0, 0, 2), true);
    pin("union-other-order", &write(&union(&sb, &sa)), (13, 0, 13, 0, 0, 2), true);
    // interleaved: the common prefix, then the two tails one by one
    let common = 4;
    let (ta, tb) = (&sa.recs[common..], &sb.recs[common..]);
    let mut mixed = sa.recs[..common].to_vec();
    for i in 0..ta.len().max(tb.len()) {
        mixed.extend(ta.get(i).cloned());
        mixed.extend(tb.get(i).cloned());
    }
    pin("union-interleaved", &write(&Side { recs: mixed, ..u.clone() }), (13, 0, 13, 0, 0, 2), true);
    // RED: a signature bit, a payload byte
    let mut bad = u.clone();
    bad.chains[0][1].sig[0] ^= 1;
    pin("red-sig-bit", &write(&bad), (13, 0, 12, 1, 0, 2), false);
    let mut bad = u.clone();
    bad.recs[6].payload[20] ^= 1;
    pin("red-payload-bit", &write(&bad), (13, 1, 13, 0, 0, 2), false);
}

#[test]
fn the_union_is_a_live_file() {
    let (a, b) = keys();
    let (fa, fb) = offline(&a, &b, 5, 4);
    let bytes = write(&union(&side(&fa), &side(&fb)));
    let mut sh = SealedHub::load(&bytes).unwrap();
    println!("K3 chain_check union: {:?}", sh.chain_check());
    sh.set_entropy([4; 32]);
    sh.append(EventKind::Placed, "after-a", MOVE, &a, 300).unwrap();
    sh.append(EventKind::Placed, "after-b", MOVE, &b, 301).unwrap();
    pin("union-then-commits", &sh.to_bytes_trimmed(), (15, 0, 15, 0, 0, 2), true);
    let n = sh.len();
    let archive = sh.cut(n - 2, &a, 302);
    println!("K3 cut below ADMIT keep={}: {:?}", n - 2, archive.as_ref().map(|v| v.len()));
    let mut sh = SealedHub::load(&bytes).unwrap();
    sh.set_entropy([5; 32]);
    let keep = sh.len() - 2;
    let cut = sh.cut(keep, &a, 303);
    println!("K3 cut keep={keep} of {}: {:?}", sh.len(), cut.as_ref().map(|v| v.len()));
    cut.unwrap();
    pin("union-cut", &sh.to_bytes_trimmed(), (12, 0, 12, 0, 0, 2), true);
    // the TRUNCATE signed by the ADMITTED device, not the genesis one: only a root cuts (`tam_tests::h1`)
    let mut sh = SealedHub::load(&bytes).unwrap();
    sh.set_entropy([5; 32]);
    let keep = sh.len() - 2;
    assert!(sh.cut(keep, &b, 304).is_err(), "an admitted key's cut declares nothing");
}

#[test]
fn where_it_breaks() {
    let (a, b) = keys();
    let (fa, fb) = offline(&a, &b, 5, 4);
    let (sa, sb) = (side(&fa), side(&fb));
    // no dedupe: the shared prefix twice
    let mut twice = union(&sa, &sb);
    twice.recs = sa.recs.iter().chain(&sb.recs).cloned().collect();
    // A record at a forked seq declares nothing (H3), B's ADMIT among them: 4 of its seals fall with it.
    pin("no-dedupe", &write(&twice), (17, 1, 9, 4, 4, 1), false);
    // one device on both copies: a restored backup
    let (r1, r2) = {
        let mut x = SealedHub::load(&fa).unwrap();
        x.set_entropy([6; 32]);
        x.append(EventKind::Placed, "left", MOVE, &a, 400).unwrap();
        let mut y = SealedHub::load(&fa).unwrap();
        y.set_entropy([7; 32]);
        y.append(EventKind::Placed, "right", MOVE, &a, 401).unwrap();
        (side(&x.to_bytes_trimmed()), side(&y.to_bytes_trimmed()))
    };
    let mut seen = BTreeSet::new();
    let recs: Vec<Record> = r1.recs.iter().chain(&r2.recs).filter(|r| seen.insert(r.id)).cloned().collect();
    pin("fork-one-row", &write(&Side { recs: recs.clone(), keys: r1.keys.clone(), chains: r1.chains.clone() }), (11, 1, 10, 0, 1, 2), false);
    let two: Vec<Vec<Seal>> = r1.chains.iter().chain(&r2.chains).cloned().collect();
    pin("fork-two-rows", &write(&Side { recs, keys: r1.keys.clone(), chains: two }), (11, 1, 20, 0, 1, 2), false);
}

/// What a holder of the file adds or strips WITHOUT any private key: none of these may hold.
fn stranger() -> (DeviceKey, DeviceKey, Side) {
    let (a, b) = keys();
    let (fa, _) = offline(&a, &b, 5, 4);
    let s = side(&fa);
    (a, b, s)
}

#[test]
fn a_forged_tail_of_a_sealed_device_does_not_hold() {
    let (a, _, mut s) = stranger();
    let top = s.chains.last().unwrap().last().unwrap();
    let rec = forged(a.id(), top.seq + 1, top.tip, 0x55);
    s.recs.push(rec);
    pin_u("unsealed-tail-of-a", &write(&s), (10, 0, 9, 0, 0, 2), 1);
}

#[test]
fn a_chain_of_an_unknown_actor_does_not_hold() {
    let (_, _, mut s) = stranger();
    s.recs.push(forged([9; 32], 1, [0; 32], 0x66));
    pin_u("unsealed-unknown-actor", &write(&s), (10, 0, 9, 0, 0, 2), 1);
}

#[test]
fn an_unsealed_chain_of_an_admitted_device_does_not_hold() {
    let (_, b, mut s) = stranger();
    let rec = forged(b.id(), 1, s.recs[3].id, 0x77);
    s.recs.push(rec);
    pin_u("unsealed-chain-of-admitted-b", &write(&s), (10, 0, 9, 0, 0, 2), 1);
}

#[test]
fn a_dropped_heads_row_does_not_hold() {
    let (a, b) = keys();
    let (fa, fb) = offline(&a, &b, 5, 4);
    let mut u = union(&side(&fa), &side(&fb));
    u.chains.retain(|c| c[0].actor_id != b.id());
    pin_u("heads-row-dropped", &write(&u), (13, 0, 9, 0, 0, 2), 4);
}

/// THE ROLLBACK LIMIT, not closed here (`settle.rs`): a whole tail gone WITH its seals and
/// its row leaves a shorter true file. One file cannot know a newer one exists.
#[test]
fn limit_a_whole_tail_dropped_with_its_seals_still_holds() {
    let (a, b) = keys();
    let (fa, fb) = offline(&a, &b, 5, 4);
    let (sa, sb) = (side(&fa), side(&fb));
    let u = union(&sa, &sb);
    let mut gone = u.clone();
    gone.recs.retain(|r| r.actor_pubkey != b.id());
    gone.chains.retain(|c| c[0].actor_id != b.id());
    pin("limit-rollback", &write(&gone), (9, 0, 9, 0, 0, 2), true);
}

#[test]
fn one_seal_per_sync() {
    let (a, b) = keys();
    for n in [20u8, 100] {
        let (fa, _) = offline(&a, &b, n, 0);
        let s = side(&fa);
        report(&format!("per-move-{n}-as-written"), &fa);
        report(&format!("per-move-{n}-compact"), &write(&s));
        let mut one = s.clone();
        let chain = &mut one.chains[0];
        let kept = chain[3].clone(); // seq 4: the ADMIT, B needs it covered
        let top = chain.last().unwrap().clone();
        let last = sign_seal(&a, &top, &kept.tip);
        chain.truncate(4);
        chain.push(last);
        let r = report(&format!("per-sync-{n}"), &write(&one));
        assert_eq!((sums(&r), r.holds()), ((n as usize + 4, 0, 5, 0, 0, 2), true), "a seal over the tip alone: seq gaps between seals are fine");
    }
}

/// Per-sync sealing under the walk: the seal over the tip vouches for every record, and the
/// oldest record dropped from under it is a hole no seal is left to name.
#[test]
fn a_record_dropped_from_under_one_seal() {
    let (a, b) = keys();
    let (fa, _) = offline(&a, &b, 20, 0);
    let mut s = side(&fa);
    let top = s.chains[0].last().unwrap().clone();
    s.chains[0] = vec![sign_seal(&a, &top, &[0; 32])];
    pin("one-seal-whole", &write(&s), (24, 0, 1, 0, 0, 2), true);
    s.recs.remove(0);
    pin_u("one-seal-hole", &write(&s), (23, 0, 1, 0, 0, 2), 1);
}

/// A rotated hub under a DOC: its gaps were `broken` before this lane, nothing got worse.
#[test]
fn a_rotated_hub_sealed() {
    let (a, _) = keys();
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    (1..=4u64).for_each(|i| drop(hub.append(EventKind::Placed, &format!("o{i}"), MOVE, i, [0; 32]).unwrap()));
    hub.rotate(|o| o == "o4").unwrap();
    let mut sh = SealedHub::seal_hub(&hub.to_bytes_trimmed(), &a).unwrap();
    sh.append(EventKind::Placed, "o5", MOVE, &a, 1).unwrap();
    report("rotated-hub-sealed", &sh.to_bytes_trimmed());
}

/// §5.3's unproved claim: REVOKE is read from the table alone.
#[test]
fn a_revoke_on_one_side() {
    let (a, b) = keys();
    let (fa, fb) = offline(&a, &b, 0, 4);
    let mut sh = SealedHub::load(&fa).unwrap();
    sh.set_entropy([8; 32]);
    // A holds none of B's records, so it can name none: revoked after 0 (`tam_tests::h2`).
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 0, tip: [0; 32] }, &a, 500).unwrap();
    let u = union(&side(&sh.to_bytes_trimmed()), &side(&fb));
    let mut with = u.clone();
    with.keys = side(&sh.to_bytes_trimmed()).keys;
    pin("revoke-in-table", &write(&with), (9, 0, 5, 4, 0, 2), false);
    let mut wiped = with.clone();
    wiped.keys.iter_mut().for_each(|k| k.0.revoked_at = 0);
    pin("revoke-table-wiped", &write(&wiped), (9, 0, 5, 4, 0, 2), false);
}
