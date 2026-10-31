//! KILL TEST K3 (W-MG-K, docs/design/M2-merge-design.md §10): two copies of one DOC, each adds
//! sealed records offline, one file holds both chains. NOT M2: no merge code, no format change.
//! The union is laid by hand with the public `Store` calls and the layouts `keys.rs`/`seal.rs`/
//! `doc.rs` document, so what it pins is what TODAY's `verify` says about such a file.
//! `K_OUT=<dir>` also writes the files for the other readers.

use bebop_store::evlog::doc::keys::{key_rows, read_key, Key, DIGEST_KEY, DIGEST_KEYS};
use bebop_store::evlog::doc::seal::{message, read_heads, read_seal, Seal, DIGEST_HEADS, DIGEST_SEAL, SEAL_VERSION};
use bebop_store::evlog::doc::{self, id_cells, Doc, KeyOp, Report, Signer, Trust, DIGEST_DOC, DOC_CELLS, ROOT_VERSION_DOC};
use bebop_store::evlog::{pack_payload, EvLog, Record, FLAG_SHA256};
use bebop_store::nodekey::sha256;
use bebop_store::{Store, Tx};
use dowiz_hub::doc_seal::{DeviceKey, HubIds, MlDsa65, SealedHub};
use dowiz_hub::{EventKind, Hub};
use std::collections::{BTreeMap, BTreeSet};

type Id = [u8; 32];
const MOVE: &str = r#"{"item":"salmon","delta":-1,"unit":"portion"}"#;

/// A file's records (oldest first), keys with their ADMIT record named by id, seal chains.
#[derive(Clone)]
pub struct Side {
    recs: Vec<Record>,
    keys: Vec<(Key, Option<Id>)>,
    chains: Vec<Vec<Seal>>,
}

fn side(bytes: &[u8]) -> Side {
    let st = Store::from_bytes(bytes);
    let v = Doc::open(&st).unwrap();
    let id_at: BTreeMap<usize, Id> = EvLog::objs_at(&st, v.log).into_iter().zip(EvLog::walk_until_at(&st, v.log, |_| false).iter().map(|r| r.id)).collect();
    let mut recs = EvLog::walk_until_at(&st, v.log, |_| false);
    recs.reverse();
    let keys = key_rows(&st, v.keys).into_iter().map(|(_, at)| read_key(&st, at).unwrap()).map(|k| {
        let admit = k.admit_rec.map(|o| id_at[&o]);
        (k, admit)
    });
    let chains = read_heads(&st, v.heads).into_iter().map(|h| {
        let (mut chain, mut cur) = (Vec::new(), h.seal);
        while let Some(at) = cur {
            let s = read_seal(&st, at).unwrap();
            cur = s.prev;
            chain.push(s);
        }
        chain.reverse();
        chain
    });
    Side { recs, keys: keys.collect(), chains: chains.collect() }
}

fn put_id(st: &mut Store, o: usize, at: usize, id: &Id) {
    for (i, c) in id_cells(id).iter().enumerate() {
        st.put_cell(o, at + i, *c);
    }
}
fn put_link(st: &mut Store, o: usize, at: usize, t: Option<usize>) {
    match t {
        Some(t) => st.link(o, at, t),
        None => st.put_cell(o, at, 0),
    }
}
fn put_key(st: &mut Store, tx: &mut Tx, k: &Key, admit: Option<usize>) -> usize {
    let pk = pack_payload(&k.pk);
    let o = st.alloc(tx, 9 + pk.len() as i64, DIGEST_KEY).unwrap();
    put_id(st, o, 0, &k.actor_id);
    st.put_cell(o, 4, k.alg);
    st.put_cell(o, 5, k.valid_from);
    st.put_cell(o, 6, k.revoked_at);
    put_link(st, o, 7, admit);
    st.put_cell(o, 8, k.pk.len() as i64);
    pk.iter().enumerate().for_each(|(j, c)| st.put_cell(o, 9 + j, *c));
    st.seal(o);
    o
}
fn put_seal(st: &mut Store, tx: &mut Tx, s: &Seal, prev: Option<usize>) -> usize {
    let sig = pack_payload(&s.sig);
    let o = st.alloc(tx, 15 + sig.len() as i64, DIGEST_SEAL).unwrap();
    st.put_cell(o, 0, SEAL_VERSION);
    put_id(st, o, 1, &s.actor_id);
    st.put_cell(o, 5, s.seq as i64);
    put_id(st, o, 6, &s.tip);
    st.put_cell(o, 10, s.claimed_ms);
    put_link(st, o, 11, prev);
    st.put_cell(o, 12, 0);
    st.put_cell(o, 13, s.alg);
    st.put_cell(o, 14, s.sig.len() as i64);
    sig.iter().enumerate().for_each(|(j, c)| st.put_cell(o, 15 + j, *c));
    st.seal(o);
    o
}

/// One file: `recs` verbatim in this order, these keys, these seal chains, one HEADS row per
/// chain. Each `ref` is re-pointed; no signature or id is touched.
fn write(s: &Side) -> Vec<u8> {
    let bytes: usize = s.recs.iter().map(|r| r.payload.len() + 512).sum::<usize>() + s.chains.iter().flatten().count() * 4096 + s.keys.len() * 4096;
    let mut st = Store::create_bytes((bytes * 2).max(1 << 20)).unwrap();
    let genesis: Vec<Key> = s.keys.iter().filter(|k| k.1.is_none()).map(|k| k.0.clone()).collect();
    Doc::init_over_bytes(&mut st, doc::FLAG_SHA256_IDS, &genesis, &s.recs).unwrap();
    let v = Doc::open(&st).unwrap();
    let at: BTreeMap<Id, usize> = EvLog::objs_at(&st, v.log).into_iter().zip(EvLog::walk_until_at(&st, v.log, |_| false).iter().map(|r| r.id)).map(|(o, id)| (id, o)).collect();
    let mut tx = st.begin().unwrap();
    let mut rows: Vec<(Id, usize)> = s.keys.iter().map(|(k, a)| (k.actor_id, put_key(&mut st, &mut tx, k, a.map(|id| at[&id])))).collect();
    rows.sort();
    let t = st.alloc(&mut tx, 1 + 5 * rows.len() as i64, DIGEST_KEYS).unwrap();
    st.put_cell(t, 0, rows.len() as i64);
    for (i, (actor, o)) in rows.iter().enumerate() {
        put_id(&mut st, t, 1 + i * 5, actor);
        st.link(t, 5 + i * 5, *o);
    }
    st.seal(t);
    let mut heads: Vec<(Id, u64, Id, usize)> = Vec::new();
    for chain in &s.chains {
        let mut prev = None;
        for x in chain {
            prev = Some(put_seal(&mut st, &mut tx, x, prev));
        }
        let top = chain.last().unwrap();
        heads.push((top.actor_id, top.seq, top.tip, prev.unwrap()));
    }
    heads.sort();
    let h = st.alloc(&mut tx, 1 + 10 * heads.len() as i64, DIGEST_HEADS).unwrap();
    st.put_cell(h, 0, heads.len() as i64);
    for (i, (actor, seq, tip, seal)) in heads.iter().enumerate() {
        let b = 1 + i * 10;
        put_id(&mut st, h, b, actor);
        st.put_cell(h, b + 4, *seq as i64);
        put_id(&mut st, h, b + 5, tip);
        st.link(h, b + 9, *seal);
    }
    st.seal(h);
    let d = st.alloc(&mut tx, DOC_CELLS, DIGEST_DOC).unwrap();
    (0..DOC_CELLS as usize).for_each(|i| st.put_cell(d, i, 0));
    st.put_cell(d, 0, v.flags);
    st.put_cell(d, 5, ROOT_VERSION_DOC);
    st.put_cell(d, 7, ROOT_VERSION_DOC);
    st.link(d, 1, v.log.unwrap());
    st.link(d, 3, h);
    st.link(d, 4, t);
    st.seal(d);
    st.commit_bytes(&tx, d);
    st.to_bytes_trimmed()
}

fn report(name: &str, bytes: &[u8]) -> Report {
    let r = doc::verify(&Store::from_bytes(bytes), &HubIds, &MlDsa65, Trust::Genesis).unwrap();
    let c = r.counters;
    println!(
        "K3 {name}: bytes={} records={} chained={} broken={} seals_ok={} seals_bad={} forks={} unsealed={} heads={} keys={} admitted={} holds={}",
        bytes.len(), c.records, c.chained, c.broken, c.seals_ok, c.seals_bad, c.forks, c.unsealed, r.heads.len(), r.keys, r.admitted, r.holds()
    );
    if let Ok(dir) = std::env::var("K_OUT") {
        std::fs::write(format!("{dir}/{name}.store"), bytes).unwrap();
    }
    r
}
fn sums(r: &Report) -> (usize, usize, usize, usize, usize, usize) {
    let c = r.counters;
    (c.records, c.broken, c.seals_ok, c.seals_bad, c.forks, r.admitted)
}

/// Genesis A, three records, ADMIT of B; the split; A adds `na`, B adds `nb`, offline.
fn offline(a: &DeviceKey, b: &DeviceKey, na: u8, nb: u8) -> (Vec<u8>, Vec<u8>) {
    let mut sh = SealedHub::create(a).unwrap();
    sh.set_entropy([1; 32]);
    (1..=3).for_each(|i| drop(sh.append(EventKind::Placed, &format!("c{i}"), MOVE, a, i).unwrap()));
    sh.commit_keyop(&KeyOp::Admit(b.key()), a, 4).unwrap();
    let split = sh.to_bytes_trimmed();
    let mut ca = SealedHub::load(&split).unwrap();
    ca.set_entropy([2; 32]);
    (0..na).for_each(|i| drop(ca.append(EventKind::Placed, &format!("a{i}"), MOVE, a, 100 + i as i64).unwrap()));
    let mut cb = SealedHub::load(&split).unwrap();
    cb.set_entropy([3; 32]);
    (0..nb).for_each(|i| drop(cb.append(EventKind::Placed, &format!("b{i}"), MOVE, b, 200 + i as i64).unwrap()));
    (ca.to_bytes_trimmed(), cb.to_bytes_trimmed())
}

/// The two sides as one: records deduplicated by id in the order `ours`-then-`theirs`.
fn union(ours: &Side, theirs: &Side) -> Side {
    let mut seen = BTreeSet::new();
    let recs: Vec<Record> = ours.recs.iter().chain(&theirs.recs).filter(|r| seen.insert(r.id)).cloned().collect();
    // HEADS as a pointwise maximum: per actor, the longer seal chain (design §3).
    let mut best: BTreeMap<Id, Vec<Seal>> = BTreeMap::new();
    for c in ours.chains.iter().chain(&theirs.chains) {
        let row = best.entry(c[0].actor_id).or_default();
        if c.len() > row.len() {
            *row = c.clone();
        }
    }
    let chains = best.into_values().collect();
    let keys = if ours.keys.len() >= theirs.keys.len() { ours.keys.clone() } else { theirs.keys.clone() };
    Side { recs, keys, chains }
}

fn keys() -> (DeviceKey, DeviceKey) {
    (DeviceKey::from_seed(&[0xA1; 32]), DeviceKey::from_seed(&[0xB2; 32]))
}

#[path = "k3_union/cases.rs"]
mod cases;
