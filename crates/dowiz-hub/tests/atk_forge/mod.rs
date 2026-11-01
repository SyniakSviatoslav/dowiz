//! W-MG-ATK: what a forger with the file and NO private key can write. `read` lifts every object
//! a verifier reads out of an image, `write` lays them into a fresh store with the public `Store`
//! calls, every crc and the anchor correct, as the file's holder can. A `Side` that is written
//! back unedited answers the same counters (`round_trip`): the test's own control.
#![allow(dead_code)]

use bebop_store::evlog::doc::anch::{read_anchors, Anchor};
use bebop_store::evlog::doc::keys::{key_rows, read_key, Key, DIGEST_KEY, DIGEST_KEYS};
use bebop_store::evlog::doc::seal::{message, read_heads, read_seal, Seal, DIGEST_HEADS, DIGEST_SEAL, SEAL_VERSION};
use bebop_store::evlog::doc::seg::{read_segs, Segment, DIGEST_SEG, SEG_VERSION};
use bebop_store::evlog::doc::{self, id_cells, Doc, Ids, Report, Trust, DIGEST_DOC, DOC_CELLS, ROOT_VERSION_DOC};
use bebop_store::evlog::{pack_payload, EvLog, Record, FLAG_SHA256};
use bebop_store::{Store, Tx};
use dowiz_hub::doc_seal::{DeviceKey, HubIds, MlDsa65};
use dowiz_hub::EventKind;
use std::collections::BTreeMap;

pub type Id = [u8; 32];

/// Every object as read. `chains` oldest seal first, one HEADS row per non-empty chain; `segs` and
/// `anchors` newest first, as the readers return them.
#[derive(Clone)]
pub struct Side {
    pub flags: i64,
    pub recs: Vec<Record>,
    pub keys: Vec<(Key, Option<Id>)>,
    pub chains: Vec<Vec<Seal>>,
    pub segs: Vec<Segment>,
    pub anchors: Vec<Anchor>,
    /// HEADS rows with no seal, added by hand.
    pub bare_rows: Vec<(Id, u64, Id)>,
    /// HEADS in reverse of sorted order.
    pub rows_reversed: bool,
    /// One object at DOC cell 2 (TREE): any cells, nothing reads them.
    pub tree: Option<Vec<i64>>,
}

pub fn read(bytes: &[u8]) -> Side {
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
    Side {
        flags: v.flags,
        recs,
        keys: keys.collect(),
        chains: chains.collect(),
        segs: read_segs(&st, v.seg, EvLog::step_cap(&st)).unwrap(),
        anchors: read_anchors(&st, v.anch),
        bare_rows: Vec::new(),
        rows_reversed: false,
        tree: None,
    }
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
fn put_seg(st: &mut Store, tx: &mut Tx, s: &Segment, prev: Option<usize>) -> usize {
    let o = st.alloc(tx, 12 + 10 * s.frontier.len() as i64, DIGEST_SEG).unwrap();
    st.put_cell(o, 0, SEG_VERSION);
    st.put_cell(o, 1, s.first as i64);
    st.put_cell(o, 2, s.last as i64);
    put_id(st, o, 3, &s.root);
    put_link(st, o, 7, prev);
    st.put_cell(o, 8, 0);
    st.put_cell(o, 9, s.redacted as i64);
    st.put_cell(o, 10, s.declared as i64);
    st.put_cell(o, 11, s.frontier.len() as i64);
    for (i, f) in s.frontier.iter().enumerate() {
        let b = 12 + i * 10;
        put_id(st, o, b, &f.actor_id);
        st.put_cell(o, b + 4, f.seq as i64);
        put_id(st, o, b + 5, &f.tip);
        st.put_cell(o, b + 9, f.scheme);
    }
    st.seal(o);
    o
}

/// One file from `s`. Refs re-pointed; no signature touched unless the caller changed one.
pub fn write(s: &Side) -> Vec<u8> {
    let bytes: usize = s.recs.iter().map(|r| r.payload.len() + 512).sum::<usize>() + s.chains.iter().flatten().count() * 4096 + s.keys.len() * 4096 + s.anchors.len() * 8192 + s.segs.len() * 4096;
    let mut st = Store::create_bytes((bytes * 2).max(1 << 20)).unwrap();
    let genesis: Vec<Key> = s.keys.iter().filter(|k| k.1.is_none()).map(|k| k.0.clone()).collect();
    Doc::init_over_bytes(&mut st, s.flags, &genesis, &s.recs).unwrap();
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
    let mut heads: Vec<(Id, u64, Id, Option<usize>)> = s.bare_rows.iter().map(|(a, q, t)| (*a, *q, *t, None)).collect();
    for chain in s.chains.iter().filter(|c| !c.is_empty()) {
        let mut prev = None;
        for x in chain {
            prev = Some(put_seal(&mut st, &mut tx, x, prev));
        }
        let top = chain.last().unwrap();
        heads.push((top.actor_id, top.seq, top.tip, prev));
    }
    heads.sort();
    if s.rows_reversed {
        heads.reverse();
    }
    let h = st.alloc(&mut tx, 1 + 10 * heads.len() as i64, DIGEST_HEADS).unwrap();
    st.put_cell(h, 0, heads.len() as i64);
    for (i, (actor, seq, tip, seal)) in heads.iter().enumerate() {
        let b = 1 + i * 10;
        put_id(&mut st, h, b, actor);
        st.put_cell(h, b + 4, *seq as i64);
        put_id(&mut st, h, b + 5, tip);
        put_link(&mut st, h, b + 9, *seal);
    }
    st.seal(h);
    let mut seg = None;
    for x in s.segs.iter().rev() {
        seg = Some(put_seg(&mut st, &mut tx, x, seg));
    }
    let tree = s.tree.as_ref().map(|cells| {
        let o = st.alloc(&mut tx, cells.len() as i64, 1).unwrap();
        cells.iter().enumerate().for_each(|(i, c)| st.put_cell(o, i, *c));
        st.seal(o);
        o
    });
    let d = st.alloc(&mut tx, DOC_CELLS, DIGEST_DOC).unwrap();
    (0..DOC_CELLS as usize).for_each(|i| st.put_cell(d, i, 0));
    st.put_cell(d, 0, s.flags);
    st.put_cell(d, 5, ROOT_VERSION_DOC);
    st.put_cell(d, 7, ROOT_VERSION_DOC);
    st.link(d, 1, v.log.unwrap());
    if let Some(o) = tree {
        st.link(d, 2, o);
    }
    st.link(d, 3, h);
    st.link(d, 4, t);
    if let Some(o) = seg {
        st.link(d, 9, o);
    }
    st.seal(d);
    st.commit_bytes(&tx, d);
    for a in s.anchors.iter().rev() {
        Doc::anchor_bytes(&mut st, a.kind, &a.over, a.time_ms, &a.proof).unwrap();
    }
    st.to_bytes_trimmed()
}

/// `verify` under `Trust::Only(roots)` with real ML-DSA-65; the counters as one line.
pub fn only(name: &str, bytes: &[u8], roots: &[Id]) -> Result<Report, doc::DocError> {
    if let Ok(dir) = std::env::var("ATK_OUT") {
        std::fs::write(format!("{dir}/{}.store", name.split(' ').next().unwrap()), bytes).unwrap();
    }
    let r = doc::verify(&Store::from_bytes(bytes), &HubIds, &MlDsa65, Trust::Only(roots));
    match &r {
        Ok(r) => {
            let c = r.counters;
            println!(
                "ATK {name}: holds={} records={} chained={} legacy={} redacted={} broken={} seals_ok={} seals_bad={} forks={} segments={} cut={} cut_redacted={} declared={} keys_disagree={} unsettled={} tombs_foreign={} unsealed={} heads={} keys={} admitted={}",
                r.holds(), c.records, c.chained, c.legacy, c.redacted, c.broken, c.seals_ok, c.seals_bad, c.forks, c.segments, c.cut, c.cut_redacted, c.declared, c.keys_disagree, c.revokes_unsettled, c.tombs_foreign, c.unsealed, r.heads.len(), r.keys, r.admitted
            );
        }
        Err(e) => println!("ATK {name}: REFUSED {e:?}"),
    }
    r
}

/// The control every attack starts from: an unedited `read` + `write` answers what the file does.
pub fn round_trip(bytes: &[u8], roots: &[Id]) -> Side {
    let s = read(bytes);
    let want = doc::verify(&Store::from_bytes(bytes), &HubIds, &MlDsa65, Trust::Only(roots)).unwrap();
    let got = only("round-trip", &write(&s), roots).unwrap();
    assert_eq!((want.counters, want.keys, want.admitted), (got.counters, got.keys, got.admitted), "the forge's own control");
    s
}

/// A record of `actor` at `seq` after `prev`, its id right, no key used.
pub fn forged(actor: Id, seq: u64, prev: Id, tag: u8) -> Record {
    let mut payload = vec![tag; 16];
    payload.push(EventKind::Placed as u8);
    payload.extend_from_slice(b"\x01xforged");
    let mut r = Record { id: [0; 32], prev, actor_pubkey: actor, actor_seq: seq, flags: FLAG_SHA256, payload };
    r.id = HubIds.chained(&r);
    r
}

/// The record whose body behind nonce and kind byte starts with `mark`.
pub fn pos_of(s: &Side, mark: &[u8]) -> usize {
    s.recs.iter().position(|r| r.payload.get(17..).is_some_and(|b| b.starts_with(mark))).expect("the marked record")
}

/// The stale table a forger leaves behind a vanished REVOKE: every `revoked_at` zeroed.
pub fn zero_table(s: &mut Side) {
    s.keys.iter_mut().for_each(|k| k.0.revoked_at = 0);
}

/// `payload` as a tombstone: nonce erased, the kind byte's high bit set (`forget.rs`).
pub fn tombstone(r: &mut Record) {
    let kind = r.payload.get(16).copied().unwrap_or(1);
    r.payload = b"0000000000000000".to_vec();
    r.payload.push(kind | 0x80);
    r.payload.push(0);
}

pub fn chain_of<'a>(s: &'a mut Side, actor: &Id) -> &'a mut Vec<Seal> {
    s.chains.iter_mut().find(|c| c.first().is_some_and(|x| x.actor_id == *actor)).expect("the device's seal chain")
}

/// Every seal of `chain` signed again by `key` (the actor cell stays what it says).
pub fn resign(chain: &mut [Seal], key: &DeviceKey, first_prev: Id) {
    use bebop_store::evlog::doc::Signer;
    let mut prev = first_prev;
    for s in chain.iter_mut() {
        s.sig = key.sign(&message(&s.actor_id, s.seq, &s.tip, s.claimed_ms, &prev));
        prev = s.tip;
    }
}
