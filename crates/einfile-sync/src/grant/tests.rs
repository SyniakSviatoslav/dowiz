//! GRANT's fold, and the hole it exists for: a key `verify` calls valid may still not move money.
//! The gate that acts on this is P4's; K-T4 is not here.

use super::*;
use crate::toys::*;
use crate::Device;
use einfile_image::Store;
use einfile_log::doc::{Doc, KeyOp};
use einfile_log::EvLog;

const MONEY: Pair = (R_LEDGER, A_APPEND);
const MENU: Pair = (0x05, 0x02);
const KITCHEN: [u8; 32] = [0xD4; 32];

fn records(st: &Store) -> Vec<Record> {
    let mut v = EvLog::walk_until_at(st, Doc::open(st).unwrap().log, |_| false);
    v.reverse();
    v
}

fn keys(st: &Store) -> Vec<Key> {
    einfile_log::doc::keys::read_keys(st, Doc::open(st).unwrap().keys)
}

fn fold(st: &Store) -> Grants {
    Grants::fold(&records(st), &keys(st), &ROOTS, &ToyIds)
}

fn file() -> (Store, Toy, Toy) {
    let (a, b) = (toy(A), toy(B));
    (Store::from_bytes(&genesis(&[&a, &b])), a, b)
}

fn give(st: &mut Store, by: &Toy, to: [u8; 32], pairs: &[Pair]) {
    write(st, by, &grant_body(&to, pairs), None);
}

#[test]
fn a_grant_body_comes_back_and_a_damaged_one_does_not() {
    let b = grant_body(&B, &[MONEY, MENU]);
    assert_eq!(parse_grant(&b), Some((B, vec![MONEY, MENU])));
    for n in 0..b.len() {
        assert_eq!(parse_grant(&b[..n]), None, "cut at {n}");
    }
    let mut long = b.clone();
    long.push(0);
    assert_eq!(parse_grant(&long), None);
    let mut lying = b;
    lying[GRANT_MARK.len() + 32] = 9;
    assert_eq!(parse_grant(&lying), None, "a count that is not what the body holds");
}

/// THE HOLE: admitted, `holds`, and no pair at all. Deny by default.
#[test]
fn an_admitted_key_with_no_grant_may_do_nothing() {
    let (mut st, a, b) = file();
    write(&mut st, &b, b"anything", None);
    assert!(report(&st).holds(), "the file is whole");
    let g = fold(&st);
    assert!(!g.allows(&B, MONEY) && !g.allows(&B, MENU));
    assert!(g.allows(&A, MONEY), "the positive twin: a root");
    let _ = a;
}

#[test]
fn a_root_grants_and_it_is_exactly_what_was_granted() {
    let (mut st, a, _) = file();
    give(&mut st, &a, B, &[MENU]);
    let g = fold(&st);
    assert!(g.allows(&B, MENU) && !g.allows(&B, MONEY));
}

/// NARROW ONLY: B may pass on what it has, and not one pair more, and a too-wide GRANT gives nothing.
#[test]
fn a_grant_is_narrow_only() {
    let (mut st, a, b) = file();
    let op = KeyOp::Admit(key_of(KITCHEN));
    write(&mut st, &a, &op.body(), Some(&op));
    give(&mut st, &a, B, &[MENU]);
    give(&mut st, &b, KITCHEN, &[MENU]);
    assert!(fold(&st).allows(&KITCHEN, MENU));
    give(&mut st, &b, KITCHEN, &[MENU, MONEY]);
    let g = fold(&st);
    assert!(!g.allows(&KITCHEN, MONEY));
    assert!(!g.allows(&KITCHEN, MENU), "its newest GRANT is too wide: fail closed, not cut down");
}

#[test]
fn two_keys_granting_each_other_start_nothing() {
    let (mut st, a, b) = file();
    let c = toy(C);
    let op = KeyOp::Admit(key_of(C));
    write(&mut st, &a, &op.body(), Some(&op));
    give(&mut st, &b, C, &[MONEY]);
    give(&mut st, &c, B, &[MONEY]);
    let g = fold(&st);
    assert!(!g.allows(&B, MONEY) && !g.allows(&C, MONEY));
}

#[test]
fn n_zero_takes_a_grant_back_and_the_key_stays() {
    let (mut st, a, b) = file();
    give(&mut st, &a, B, &[MONEY]);
    assert!(fold(&st).allows(&B, MONEY));
    give(&mut st, &a, B, &[]);
    assert!(!fold(&st).allows(&B, MONEY));
    write(&mut st, &b, b"still a device", None);
    assert!(report(&st).holds());
}

/// A REVOKE of the author ends what it grants after, and what stood before it stands.
#[test]
fn a_revoked_author_grants_nothing_after_its_revoke() {
    let (mut st, a, b) = file();
    let c = toy(C);
    let op = KeyOp::Admit(key_of(C));
    write(&mut st, &a, &op.body(), Some(&op));
    give(&mut st, &a, B, &[MENU, MONEY]);
    give(&mut st, &b, C, &[MENU]);
    let tip = Doc::tip_at(&st, &B, 1).unwrap();
    let op = KeyOp::Revoke { actor_id: B, after_seq: 1, tip };
    write(&mut st, &a, &op.body(), Some(&op));
    let g = fold(&st);
    assert!(g.allows(&C, MENU), "below the revoke");
    let mut recs = records(&st);
    let mut later = c.record(&st, b"unused");
    later.actor_pubkey = B;
    later.actor_seq = 2;
    later.payload = grant_body(&C, &[MONEY]);
    recs.push(later);
    assert!(!Grants::fold(&recs, &keys(&st), &ROOTS, &ToyIds).allows(&C, MONEY), "past it");
}

/// The file's order is signed by nobody: the same records in another order fold the same.
#[test]
fn the_files_order_plays_no_part() {
    let (mut st, a, b) = file();
    let op = KeyOp::Admit(key_of(C));
    write(&mut st, &a, &op.body(), Some(&op));
    give(&mut st, &b, C, &[MENU]);
    give(&mut st, &a, B, &[MENU]);
    let recs = records(&st);
    let mut back = recs.clone();
    back.reverse();
    let ks = keys(&st);
    assert_eq!(Grants::fold(&recs, &ks, &ROOTS, &ToyIds), Grants::fold(&back, &ks, &ROOTS, &ToyIds));
    assert!(Grants::fold(&back, &ks, &ROOTS, &ToyIds).allows(&C, MENU));
}

#[test]
fn a_damaged_grant_is_counted_and_gives_nothing() {
    let (mut st, a, _) = file();
    let mut body = grant_body(&B, &[MONEY]);
    body.push(0);
    write(&mut st, &a, &body, None);
    let g = fold(&st);
    assert_eq!(g.bad, 1);
    assert!(!g.allows(&B, MONEY));
}

/// An author with no key in the table grants nothing, not even one the caller names as a root:
/// nothing could have checked its seal.
#[test]
fn a_stranger_grants_nothing() {
    let (st, ..) = file();
    let mut recs = records(&st);
    let mut r = recs[0].clone();
    r.actor_pubkey = [0xEE; 32];
    r.actor_seq = 1;
    r.payload = grant_body(&B, &[MONEY]);
    recs.push(r);
    assert!(!Grants::fold(&recs, &keys(&st), &[[0xEE; 32]], &ToyIds).allows(&B, MONEY));
}

#[test]
fn the_red_lines_are_the_ones_named() {
    for (p, want) in [(MONEY, Some(RedLine::Money)), ((R_ORDER, A_CREATE_ORDER), Some(RedLine::Money)), ((R_CLAIM, 0x0E), Some(RedLine::Money)), ((R_AUTH, 1), Some(RedLine::Auth)), ((R_SECRET, 1), Some(RedLine::Secrets)), ((R_MIGRATION, 1), Some(RedLine::Migrations)), (MENU, None), ((R_LEDGER, 0x02), None)] {
        assert_eq!(is_red_line(p), want, "{p:?}");
    }
}

/// A tombstone says nothing: an erased GRANT gives nothing, whatever its bytes still read.
#[test]
fn a_tombstone_grants_nothing() {
    struct Tombs;
    impl Ids for Tombs {
        fn chained(&self, r: &Record) -> [u8; 32] {
            r.id
        }
        fn redacted(&self, _flags: i64, _payload: &[u8]) -> bool {
            true
        }
    }
    let (mut st, a, _) = file();
    give(&mut st, &a, B, &[MONEY]);
    assert!(Grants::fold(&records(&st), &keys(&st), &ROOTS, &ToyIds).allows(&B, MONEY), "the twin");
    assert!(!Grants::fold(&records(&st), &keys(&st), &ROOTS, &Tombs).allows(&B, MONEY));
}

/// A later GRANT that is lower in the file's order still takes back the earlier one.
#[test]
fn the_newest_by_seq_stands_wherever_it_lies() {
    let (mut st, a, _) = file();
    give(&mut st, &a, B, &[MONEY]);
    give(&mut st, &a, B, &[]);
    let mut back = records(&st);
    back.reverse();
    assert!(!Grants::fold(&back, &keys(&st), &ROOTS, &ToyIds).allows(&B, MONEY));
}
