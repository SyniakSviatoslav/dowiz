//! A file, no key: revocation and erasure undone.
use super::*;
use einstore::evlog::doc::keys::{KeyOp, REVOKE_MARK};
use einstore::evlog::FLAG_SHA256;
use einstore::nodekey::sha256;

const PHONE: &str = "+355691111111";

/// Owner A admits B; B seals twice; A revokes B after seq 1: B's seq 2 is a bad seal.
fn revoked(a: &DeviceKey, b: &DeviceKey) -> SealedHub {
    let mut sh = SealedHub::create(a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, a, "c", 1);
    sh.commit_keyop(&KeyOp::Admit(b.key()), a, 2).unwrap();
    orders(&mut sh, b, "b", 2);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 1, tip: sh.tip_at(&b.id(), 1) }, a, 3).unwrap();
    sh
}

/// HOLE 1. `Trust::Only` lists actor ids; the public key comes from the unsigned KEY row and
/// nothing compares sha256(pk) with the id. The forger swaps the root's pk for his own, signs
/// the whole file again, and adds a record in the owner's name.
#[test]
fn hole_1_a_root_key_row_is_not_bound_to_the_actor_it_names() {
    let (a, forger) = (key(0xA1), key(0xF0));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 3);
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    let top = s.chains[0].last().unwrap().clone();
    let rec = forged(a.id(), top.seq + 1, top.tip, 0x55);
    s.chains[0].push(einstore::evlog::doc::Seal { seq: top.seq + 1, tip: rec.id, ..top });
    s.recs.push(rec);
    s.keys[0].0.pk = forger.public_key().to_vec();
    resign(&mut s.chains[0], &forger, [0; 32]);
    must_not_hold("h1-root-pk-swapped", &write(&s), &roots);
}

/// HOLE 2. `flags` is outside the id, and it decides where the body starts. FLAG_SHA256 cleared
/// on the REVOKE: the id still chains, the body no longer parses, the REVOKE says nothing.
#[test]
fn hole_2_a_stripped_flag_erases_a_revoke() {
    let (a, b) = (key(0xA1), key(0xB2));
    let bytes = revoked(&a, &b).to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(!only("h2-control", &bytes, &roots).unwrap().holds(), "B's seq 2 is revoked");
    let at = pos_of(&s, REVOKE_MARK);
    s.recs[at].flags &= !FLAG_SHA256;
    zero_table(&mut s);
    must_not_hold("h2-flag-stripped-revoke", &write(&s), &roots);
}

/// HOLE 2, its other face: the same strip makes any order vanish from `events()`, holds true.
#[test]
fn hole_2b_a_stripped_flag_hides_an_order() {
    let a = key(0xA1);
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    orders(&mut sh, &a, "c", 3);
    let bytes = sh.to_bytes_trimmed();
    let mut s = round_trip(&bytes, &[a.id()]);
    s.recs[1].flags &= !FLAG_SHA256;
    let f = write(&s);
    let seen = SealedHub::load(&f).unwrap();
    println!("ATK h2b: events before=3 after={} chain_check={:?}", seen.events().len(), seen.chain_check());
    must_not_hold("h2b-flag-stripped-order", &f, &[a.id()]);
}

/// HOLE 3. A FORGOTTEN is a count, a tombstone names no declaration. With a copy from before the
/// erasure the forger gives the person's record back (id chains again) and tombstones the REVOKE
/// in its place: same count, same author, the seals never covered the payload's survival.
#[test]
fn hole_3_one_forgotten_and_an_old_copy_swap_the_erased_record() {
    let (a, b) = (key(0xA1), key(0xB2));
    let mut sh = revoked(&a, &b);
    sh.append(EventKind::Placed, "p1", &format!(r#"{{"phone":"{PHONE}"}}"#), &a, 4).unwrap();
    orders(&mut sh, &a, "z", 1);
    let old = read(&sh.to_bytes_trimmed());
    let n = sh.forget(|e| e.order_json.contains(PHONE).then(|| "{}".to_string()), &sha256(PHONE.as_bytes()), &a, 5).unwrap();
    assert_eq!(n, 1);
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(!only("h3-control", &bytes, &roots).unwrap().holds());
    let at = pos_of(&old, REVOKE_MARK);
    tombstone(&mut s.recs[at]);
    let p = old.recs.iter().position(|r| String::from_utf8_lossy(&r.payload).contains(PHONE)).unwrap();
    s.recs[p] = old.recs[p].clone();
    zero_table(&mut s);
    let f = write(&s);
    let back = SealedHub::load(&f).unwrap().events().iter().any(|e| e.order_json.contains(PHONE));
    println!("ATK h3: erased phone back in events: {back}");
    must_not_hold("h3-tomb-swapped", &f, &roots);
}

/// HOLE 3b. A tombstone's bytes past the kind bit are bound by nothing: `redacted` reads one bit,
/// the id no longer verifies them by design. Any tombstone is a slate: here it comes back from
/// `events()` as an order the forger wrote, no copy needed, the count and the author unchanged.
#[test]
fn hole_3b_a_tombstone_is_a_slate() {
    let a = key(0xA1);
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    sh.append(EventKind::Placed, "p1", &format!(r#"{{"phone":"{PHONE}"}}"#), &a, 1).unwrap();
    orders(&mut sh, &a, "z", 1);
    sh.forget(|e| e.order_json.contains(PHONE).then(|| "{}".to_string()), &sha256(PHONE.as_bytes()), &a, 5).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(only("h3b-control", &bytes, &roots).unwrap().holds());
    let mut p = s.recs[0].payload[..17].to_vec();
    p.extend_from_slice(b"\x02p1{\"total\":999999}");
    s.recs[0].payload = p;
    let f = write(&s);
    let seen: Vec<String> = SealedHub::load(&f).unwrap().events().iter().map(|e| e.order_json.clone()).collect();
    println!("ATK h3b: events={seen:?}");
    must_not_hold("h3b-tomb-rewritten", &f, &roots);
}

/// HOLE 4. `covered` is per actor, `unsealed` per tip, and they disagree over a device's first
/// record, which chains onto the log's tip. Rotation is exactly that shape: A's REVOKE, then
/// C's first commit. Row A rolled back one seal: the REVOKE stays in the file under C's signed
/// tip, so no counter moves, and no seal of A covers it, so it does nothing.
#[test]
fn hole_4_a_rolled_back_row_disowns_a_revoke_another_tip_still_covers() {
    let (a, b, c) = (key(0xA1), key(0xB2), key(0xC3));
    let mut sh = SealedHub::create(&a).unwrap();
    sh.set_entropy([1; 32]);
    sh.commit_keyop(&KeyOp::Admit(b.key()), &a, 1).unwrap();
    sh.commit_keyop(&KeyOp::Admit(c.key()), &a, 2).unwrap();
    orders(&mut sh, &b, "b", 2);
    sh.commit_keyop(&KeyOp::Revoke { actor_id: b.id(), after_seq: 1, tip: sh.tip_at(&b.id(), 1) }, &a, 3).unwrap();
    orders(&mut sh, &c, "n", 2);
    let bytes = sh.to_bytes_trimmed();
    let roots = [a.id()];
    let mut s = round_trip(&bytes, &roots);
    assert!(!only("h4-control", &bytes, &roots).unwrap().holds());
    chain_of(&mut s, &a.id()).pop();
    zero_table(&mut s);
    must_not_hold("h4-row-rolled-back", &write(&s), &roots);
}
