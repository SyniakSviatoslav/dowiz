//! KILL TEST K-T1 UNDER REAL SEAMS (docs/design/MESH-TRANSPORT-OVER-EINFILE.md §14): sha256 ids, real
//! ML-DSA-65, the hub's own framing. `einfile-log`'s `doc/tail/tests.rs` walks the fixtures with toys;
//! this is the same claim where a forged seal cannot pass: the bytes of `tail_into` are the bytes of
//! `merge_into`, or there are two doors into a file.

use einstore::evlog::doc::keys::KeyOp;
use einstore::evlog::doc::{Doc, DocError, Trust};
use einstore::Store;
use core::mem::discriminant;
use dowiz_hub::doc_seal::{DeviceKey, HubIds, MlDsa65, SealedHub};
use dowiz_hub::stock::StockEvent;

const SIZE: usize = 1 << 23;

fn keys() -> (DeviceKey, DeviceKey) {
    (DeviceKey::from_seed(&[0xA1; 32]), DeviceKey::from_seed(&[0xB2; 32]))
}
fn received(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.to_string(), qty }
}
fn served(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Served { item: item.to_string(), qty, order_id: order.to_string() }
}

fn genesis(a: &DeviceKey, b: &DeviceKey) -> Vec<u8> {
    let mut sh = SealedHub::create(a).unwrap();
    sh.set_entropy([1; 32]);
    sh.sync_stock(&[received("salmon", 1), received("rice", 10)], a, 1).unwrap();
    sh.commit_keyop(&KeyOp::Admit(b.key()), a, 2).unwrap();
    sh.to_bytes_trimmed()
}

fn offline(split: &[u8], seed: u8, key: &DeviceKey, moves: &[StockEvent]) -> Vec<u8> {
    let mut sh = SealedHub::load(split).unwrap();
    sh.set_entropy([seed; 32]);
    sh.sync_stock(moves, key, 100).unwrap();
    sh.to_bytes_trimmed()
}

fn one_door(ours: &[u8], theirs: &[u8], what: &str) {
    let (o, t) = (Store::from_bytes(ours), Store::from_bytes(theirs));
    let (mut fm, mut ft) = (Store::create_bytes(SIZE).unwrap(), Store::create_bytes(SIZE).unwrap());
    let um = Doc::merge_into(&o, &t, &mut fm, &HubIds, &MlDsa65, Trust::Genesis);
    let ut = Doc::front(&o, &HubIds).and_then(|at| Doc::tail_of(&t, &at, &HubIds)).and_then(|x| Doc::tail_into(&o, &x, &mut ft, &HubIds, &MlDsa65, Trust::Genesis));
    match (um, ut) {
        (Ok(m), Ok(x)) => {
            assert_eq!(m, x, "{what}: the Union");
            assert_eq!(fm.to_bytes(), ft.to_bytes(), "{what}: the bytes");
        }
        (Err(m), Err(x)) => assert_eq!(discriminant(&m), discriminant(&x), "{what}: {m:?} against {x:?}"),
        (m, x) => panic!("{what}: the doors differ: merge {m:?} tail {x:?}"),
    }
}

fn k_t1(a: &[u8], b: &[u8], what: &str) {
    one_door(a, b, what);
    one_door(b, a, what);
}

#[test]
fn k_t1_real_seals_two_sides_that_commute() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    let (x, y) = (offline(&s, 2, &a, &[served("rice", 2, "o1"), served("rice", 1, "o2")]), offline(&s, 3, &b, &[received("tuna", 4)]));
    k_t1(&x, &y, "two offline sides");
}

#[test]
fn k_t1_real_seals_the_last_salmon_sold_twice() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    let (x, y) = (offline(&s, 2, &a, &[served("salmon", 1, "o1")]), offline(&s, 3, &b, &[served("salmon", 1, "o2")]));
    k_t1(&x, &y, "a conflict is not this door's to name");
}

#[test]
fn k_t1_real_seals_one_side_wrote() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    k_t1(&s, &offline(&s, 3, &b, &[received("tuna", 4)]), "one history");
}

/// One device, two histories, under real ids. Same seed: same nonces would be one history, so two.
#[test]
fn k_t1_real_seals_a_fork() {
    let (a, b) = keys();
    let s = genesis(&a, &b);
    let (x, y) = (offline(&s, 2, &a, &[received("tuna", 1)]), offline(&s, 3, &a, &[received("eel", 1)]));
    k_t1(&x, &y, "a fork");
    let (o, t) = (Store::from_bytes(&x), Store::from_bytes(&y));
    let r = Doc::tail_of(&t, &Doc::front(&o, &HubIds).unwrap(), &HubIds);
    assert!(matches!(r, Err(DocError::Fork { .. })), "{r:?}");
}
