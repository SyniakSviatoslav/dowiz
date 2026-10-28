//! The Lean model's theorems on the REAL code (LV-INC-D hand-back, 2026-10-28).
//!
//! bebop-lang/formal/Bebop/ZSet.lean proves, over an ABSTRACT KV (a map, put/remove batches):
//!   kv_chain_is_zset   the state after c ++ [d], as a Z-set, = the state after c + delta_of d
//!   step_is_zset       delta_of d = +(k, new) - (k, old) per put, -(k, old) per remove, old read from
//!                      the state before that op; a remove of an absent key contributes nothing
//! This test is the bridge on ONE named chain (no fuzzing: operator rule): c0 = [[put a 1, put b 2]],
//! d1 = [put a 3, del b, del zz, put c 4], run through the real writer (`append_delta_with`) and the
//! real reader (`from_kv_chain`, `from_kv`, `Kv::load`). The weights are the theorem's, written out.
use super::*;
use crate::kv::delta::{append_delta_with, Appended, Op, MAX_DELTAS};
use crate::kv::Kv;
use crate::{root_in, Store};

fn row(k: &str, v: &str) -> KvRow {
    (k.as_bytes().to_vec(), v.as_bytes().to_vec())
}

#[test]
fn kv_chain_is_zset_on_the_named_chain() {
    let mut st = Store::from_bytes(&Kv { entries: Vec::new() }.compacted_bytes(1 << 20).unwrap());
    let c0 = [Op::Put("a", b"1"), Op::Put("b", b"2")];
    let d1 = [Op::Put("a", b"3"), Op::Remove("b"), Op::Remove("zz"), Op::Put("c", b"4")];
    assert!(matches!(append_delta_with(&mut st, &c0, MAX_DELTAS, 0).unwrap(), Appended::Delta(_)));
    let before = from_kv(&st, root_in(&st).unwrap()).unwrap();
    assert!(matches!(append_delta_with(&mut st, &d1, MAX_DELTAS, 0).unwrap(), Appended::Delta(_)));
    let root = root_in(&st).unwrap();
    // step_is_zset: delta_of d1, read from the image.
    let d = from_kv_chain(&st, root, d1.len()).unwrap();
    assert_eq!(d.weight(&row("a", "1")), -1);
    assert_eq!(d.weight(&row("a", "3")), 1);
    assert_eq!(d.weight(&row("b", "2")), -1);
    assert_eq!(d.weight(&row("c", "4")), 1);
    assert!(d.rows().iter().all(|(r, _)| r.0 != b"zz"), "a remove of an absent key is absent (weight 0)");
    assert_eq!(d.len(), 4);
    // kv_chain_is_zset: zset(replay (c0 ++ [d1])) = zset(replay c0) + delta_of d1, and = Kv::load.
    let after = from_kv(&st, root).unwrap();
    assert_eq!(after, before.add(&d));
    let loaded = Kv::load(&st).unwrap().entries.into_iter().map(|(k, v)| ((k.into_bytes(), v), 1)).collect();
    assert_eq!(after, ZSet::from_pairs(loaded));
}
