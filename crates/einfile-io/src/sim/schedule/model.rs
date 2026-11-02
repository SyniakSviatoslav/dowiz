//! THE SPACE AND THE MODEL of `schedule` (W-ES-HARD): the transactions of each conflict pattern,
//! every order, every cut, and the serial model written by hand -- a `BTreeMap` and the one
//! validation rule -- plus the two mutant models the RED proofs use. Nothing here touches the
//! code under test.

use std::collections::BTreeMap;

use crate::sim::testkit::Map;

/// The six-key universe.
pub const KEYS: [&str; 6] = ["a", "b", "c", "d", "e", "f"];

/// One writer: the keys it reads (`get`), then its writes (`Some` puts, `None` removes).
#[derive(Clone, Copy, Debug)]
pub struct Txn {
    pub reads: &'static [&'static str],
    pub writes: &'static [(&'static str, Option<&'static str>)],
}

pub(crate) const fn t(reads: &'static [&'static str], writes: &'static [(&'static str, Option<&'static str>)]) -> Txn {
    Txn { reads, writes }
}

/// The conflict patterns of es-p19 §3 step 1 for N in 2..=4 writers, 1-2 ops each: disjoint keys
/// (all accepted), one key every writer reads (all accepted: a shared read is no conflict), one
/// key every writer writes blind (all accepted, the last in the order wins), a read-write cycle
/// of two (the second of the pair is refused) and, from N = 3, of three.
pub fn patterns(n: usize) -> Vec<(&'static str, Vec<Txn>)> {
    assert!((2..=4).contains(&n), "N={n}: the table covers 2..=4");
    let disjoint = [
        t(&["a"], &[("a", Some("A"))]),
        t(&["b"], &[("b", None), ("e", Some("E"))]),
        t(&["c"], &[("c", Some("C"))]),
        t(&["d"], &[("d", None), ("f", Some("F"))]),
    ];
    let shared_read = [
        t(&["a"], &[("b", Some("B0"))]),
        t(&["a"], &[("c", Some("C1"))]),
        t(&["a"], &[("d", None)]),
        t(&["a"], &[("e", Some("E3"))]),
    ];
    let shared_write = [
        t(&[], &[("a", Some("w0"))]),
        t(&[], &[("a", Some("w1")), ("b", None)]),
        t(&[], &[("a", Some("w2"))]),
        t(&[], &[("a", None)]),
    ];
    let cycle2 = [
        t(&["a"], &[("b", Some("B"))]),
        t(&["b"], &[("a", Some("A"))]),
        t(&["c"], &[("c", Some("C"))]),
        t(&["d"], &[("d", None)]),
    ];
    let cycle3 = [
        t(&["a"], &[("b", Some("B"))]),
        t(&["b"], &[("c", Some("C"))]),
        t(&["c"], &[("a", None)]),
        t(&["d"], &[("d", Some("D"))]),
    ];
    let mut v = vec![
        ("disjoint", disjoint[..n].to_vec()),
        ("one shared read", shared_read[..n].to_vec()),
        ("one shared write", shared_write[..n].to_vec()),
        ("read-write cycle of 2", cycle2[..n].to_vec()),
    ];
    if n >= 3 {
        v.push(("read-write cycle of 3", cycle3[..n].to_vec()));
    }
    v
}

/// Every order of `n` transactions (lexicographic).
pub fn permutations(n: usize) -> Vec<Vec<usize>> {
    fn go(rest: &mut Vec<usize>, cur: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if rest.is_empty() {
            out.push(cur.clone());
            return;
        }
        for i in 0..rest.len() {
            let x = rest.remove(i);
            cur.push(x);
            go(rest, cur, out);
            cur.pop();
            rest.insert(i, x);
        }
    }
    let mut out = Vec::new();
    go(&mut (0..n).collect(), &mut Vec::new(), &mut out);
    out
}

/// Every way to cut `n` commits into flush groups, as group sizes (2^(n-1) compositions).
pub fn cuts(n: usize) -> Vec<Vec<usize>> {
    (0..1u32 << (n - 1))
        .map(|mask| {
            let mut sizes = vec![];
            let mut size = 0;
            for i in 0..n {
                size += 1;
                if i + 1 == n || mask >> i & 1 == 1 {
                    sizes.push(size);
                    size = 0;
                }
            }
            sizes
        })
        .collect()
}

/// What the serial model says: per position in the order, accepted or refused; and the map after
/// each accepted commit (`maps[0]` is the base, `maps[j]` the j-th accepted).
pub struct Serial {
    pub accepted: Vec<bool>,
    pub maps: Vec<Map>,
}

/// A model: the base map and the order -> `Serial`.
pub type ModelFn = dyn Fn(&Map, &[Txn]) -> Serial;

fn with_writes(m: &Map, t: &Txn) -> Map {
    let mut m = m.clone().unwrap_or_default();
    for (k, v) in t.writes {
        match v {
            Some(v) => m.insert(k.to_string(), v.as_bytes().to_vec()),
            None => m.remove(*k),
        };
    }
    Some(m)
}

/// THE MODEL: backward validation by hand. A transaction's read set is `reads` plus every key it
/// removes; it is refused iff an earlier accepted transaction wrote one of them.
pub fn serial(base: &Map, order: &[Txn]) -> Serial {
    let mut s = Serial { accepted: vec![], maps: vec![base.clone()] };
    let mut written: Vec<&str> = vec![];
    for t in order {
        let removes = t.writes.iter().filter(|(_, v)| v.is_none()).map(|(k, _)| k);
        let ok = !t.reads.iter().chain(removes).any(|r| written.contains(r));
        s.accepted.push(ok);
        if ok {
            s.maps.push(with_writes(s.maps.last().unwrap(), t));
            written.extend(t.writes.iter().map(|(k, _)| *k));
        }
    }
    s
}

/// Mutant models, for the RED proofs: a section that validates nothing (every transaction
/// accepted) and one that replays each commit onto the transaction's BASE (the earlier commits'
/// keys lost). The harness must refuse both on a named schedule.
pub fn serial_blind(base: &Map, order: &[Txn]) -> Serial {
    let maps = std::iter::once(base.clone()).chain(order.iter().scan(base.clone(), |m, t| {
        *m = with_writes(m, t);
        Some(m.clone())
    }));
    Serial { accepted: vec![true; order.len()], maps: maps.collect() }
}
pub fn serial_onto_base(base: &Map, order: &[Txn]) -> Serial {
    let s = serial(base, order);
    let maps: Vec<Map> = std::iter::once(base.clone()).chain(order.iter().zip(&s.accepted).filter(|(_, ok)| **ok).map(|(t, _)| with_writes(base, t))).collect();
    Serial { accepted: s.accepted, maps }
}

/// A map literal for the tests.
pub fn map(kvs: &[(&str, &str)]) -> Map {
    Some(kvs.iter().map(|(k, v)| (k.to_string(), v.as_bytes().to_vec())).collect::<BTreeMap<_, _>>())
}
