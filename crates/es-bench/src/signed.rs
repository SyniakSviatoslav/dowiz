//! THE SIGNED ROWS: what no other engine in this table has, so nothing here is compared. A DOC
//! file of N records from one device, one real ML-DSA-65 seal per `RECS_PER_SEAL` records (one
//! sync). Every timed row is a median with its spread; every row asserts what it read: a verify
//! that does not hold, or holds over the wrong counts, panics instead of getting faster.
//!
//! All writing is `Doc::*_bytes`, in memory, as the crate's writers do it: the seal rows do NOT
//! include a device write, and the note says so. `verify_signed` starts from the file on disk,
//! warm page cache: read + `verify_doc`, which is the library's entry for a stranger's file.
use std::fs;

use einstore::evlog::doc::{merge_body, Doc, DocError, Key, MergeView, Signer, SpecIn, Trust, Verifier, FLAG_SHA256_IDS, STATE_COMMUTES};
use einstore::evlog::{Record, FLAG_SHA256};
use einstore::iodev::docverify::{verify_doc, DocReport};
use einstore::iodev::Refusal;
use einstore::nodekey::sha256;
use einstore::Store;

use crate::archive::{self, READER_ABI};
use crate::data::*;
use crate::mldsa::{Counts, MlDsa, MlDsaVerifier, Sha256Ids};
use crate::work::{Cfg, BATCH};

pub const NAME: &str = "signed";
/// One seal per sync, the size of the KV rows' commit.
pub const RECS_PER_SEAL: usize = BATCH;
/// Samples of a single-operation row, per repetition.
const SAMPLES: usize = 20;
const CLOCK: i64 = 1_700_000_000_000;
const SPEC_ROOM: usize = 1 << 20;

/// One device: its key and its place in its own chain.
pub struct Device {
    pub key: MlDsa,
    pub seq: u64,
    pub tip: [u8; 32],
}

impl Device {
    pub fn new(name: u8) -> Device {
        Device { key: MlDsa::from_seed([name; 32]), seq: 0, tip: [0; 32] }
    }

    pub fn row(&self) -> Key {
        Key { actor_id: self.key.actor_id(), alg: self.key.alg(), valid_from: 0, revoked_at: 0, admit_rec: None, pk: self.key.public_key() }
    }

    /// The next record. A real nonce comes from the OS; a counter keeps a run's bytes repeatable.
    fn record(&mut self, body: &[u8]) -> Record {
        let seq = self.seq + 1;
        let payload = [format!("{seq:016x}").as_bytes(), body].concat();
        let mut rec = Record { id: [0; 32], prev: self.tip, actor_pubkey: self.key.actor_id(), actor_seq: seq, flags: FLAG_SHA256, payload };
        rec.id = einstore::evlog::doc::Ids::chained(&Sha256Ids, &rec);
        (self.seq, self.tip) = (seq, rec.id);
        rec
    }

    /// One sync: `bodies` under one seal.
    pub fn sync(&mut self, st: &mut Store, bodies: &[Vec<u8>]) {
        let recs: Vec<Record> = bodies.iter().map(|b| self.record(b)).collect();
        Doc::commit_many_bytes(st, &recs, &self.key, CLOCK + self.seq as i64).expect("commit_many_bytes");
    }

    /// One commit: one record, one seal.
    pub fn commit(&mut self, st: &mut Store, body: &[u8]) {
        let rec = self.record(body);
        Doc::commit_bytes(st, &rec, &self.key, CLOCK + self.seq as i64, None).expect("commit_bytes");
    }
}

/// `vs` bytes of data. The first byte's high bit is the tombstone mark (FORMAT §9.4), so it is ASCII.
pub fn body(i: u64, vs: usize, salt: u64) -> Vec<u8> {
    let mut b = val(i, vs, salt);
    b[0] = b'r';
    b
}

pub fn seals_for(n: u64) -> usize {
    (n as usize).div_ceil(RECS_PER_SEAL)
}

/// A store with room for `recs` records and `seals` seals. Too small panics in the commit, by name.
fn room(recs: u64, seals: usize, vs: usize) -> Store {
    Store::create_bytes((1 << 20) + recs as usize * (vs + 1024) + seals * (8 << 10)).expect("create_bytes")
}

fn genesis(st: &mut Store, keys: &[Key]) {
    Doc::init_bytes(st, FLAG_SHA256_IDS, keys).expect("init_bytes");
}

fn write(dev: &mut Device, st: &mut Store, n: u64, vs: usize, salt: u64) {
    let mut i = 0;
    while i < n {
        let end = (i + RECS_PER_SEAL as u64).min(n);
        let bodies: Vec<Vec<u8>> = (i..end).map(|j| body(j, vs, salt)).collect();
        dev.sync(st, &bodies);
        i = end;
    }
}

/// The file the rows read: `n` records of device 1, room for `spare` more commits.
pub fn build(n: u64, vs: usize, spare: usize) -> (Device, Store) {
    let mut d = Device::new(1);
    let mut st = room(n + spare as u64, seals_for(n) + spare, vs);
    genesis(&mut st, &[d.row()]);
    write(&mut d, &mut st, n, vs, 0);
    (d, st)
}

pub fn check(bytes: &[u8], v: &dyn Verifier) -> Result<DocReport, Refusal> {
    verify_doc(bytes, &Sha256Ids, v, Trust::Genesis)
}

/// A verify that does not hold, or holds over other counts, is a failed run.
fn assert_whole(bytes: &[u8], records: usize, seals: usize, what: &str) {
    let r = check(bytes, &MlDsaVerifier).unwrap_or_else(|e| panic!("{what}: refused {e:?}"));
    let c = r.doc.counters;
    assert!(r.doc.holds(), "{what}: does not hold: {c:?}");
    assert_eq!((c.records, c.seals_ok), (records, seals), "{what}: counts");
}

fn arena(st: &Store) -> i64 {
    st.pick().expect("a committed store").arena_used
}

fn ms(t0: std::time::Instant) -> f64 {
    us(t0) / 1000.0
}

/// verify_signed, verify_signed_rate, verify_counted.
fn verify_rows(c: &Cfg, path: &std::path::Path, tag: &str) {
    let (n, seals) = (c.n as usize, seals_for(c.n));
    let (mut real, mut rate, mut count) = (vec![], vec![], vec![]);
    for _ in 0..c.reps {
        let t0 = t();
        let b = fs::read(path).unwrap();
        let r = check(&b, &MlDsaVerifier).expect("verify_doc");
        let dt = ms(t0);
        assert!(r.doc.holds() && r.doc.counters.records == n && r.doc.counters.seals_ok == seals, "verify_signed: {:?}", r.doc.counters);
        real.push(dt);
        rate.push(n as f64 / (dt / 1000.0));
        let t1 = t();
        let b = fs::read(path).unwrap();
        let r = check(&b, &Counts).expect("verify_doc, no signature code");
        count.push(ms(t1));
        assert_eq!((r.doc.counters.records, r.doc.counters.chained, r.doc.counters.broken), (n, n, 0), "verify_counted");
    }
    res("verify_signed", NAME, "ms", stats(real), &format!("{tag}; file read + verify_doc; records={n} seals={seals}, every seal ML-DSA-65 verified; warm page cache"));
    res("verify_signed_rate", NAME, "records/s", stats(rate), &format!("{tag}; the verify_signed row over {n} records; {RECS_PER_SEAL} records per seal, so the rate moves with that"));
    res("verify_counted", NAME, "ms", stats(count), &format!("{tag}; the same walk with no signature code behind the seam: every record and seal walked, none proved, `seals_bad` then only counts them"));
}

/// seal_commit, seal_bytes, sign_one, verify_one.
fn seal_rows(c: &Cfg, d: &mut Device, st: &mut Store, tag: &str) {
    let samples = c.reps * SAMPLES;
    let (mut v, start) = (vec![], arena(st));
    for i in 0..samples {
        let b = body(i as u64, c.vs, 1);
        let t0 = t();
        d.commit(st, &b);
        v.push(ms(t0));
    }
    res("seal_commit", NAME, "ms/commit", stats(v), &format!("{tag}; one record, one real seal, into a file of N records; IN MEMORY, the device write is not in this row"));
    let each = (arena(st) - start) as f64 * 8.0 / samples as f64;
    one("seal_bytes", NAME, "bytes/commit", each, &format!("{tag}; what one such commit adds to the file, its {} byte record body included", c.vs));

    let msgs: Vec<Vec<u8>> = (0..samples).map(|i| sha256(&i.to_le_bytes()).to_vec()).collect();
    let (mut sv, mut vv, mut sigs) = (vec![], vec![], vec![]);
    for m in &msgs {
        let t0 = t();
        sigs.push(d.key.sign(m));
        sv.push(ms(t0));
    }
    let (alg, pk) = (d.key.alg(), d.key.public_key());
    for (m, s) in msgs.iter().zip(&sigs) {
        let t0 = t();
        assert!(MlDsaVerifier.verify(alg, &pk, m, s), "verify_one: its own signature");
        vv.push(ms(t0));
    }
    let of = format!("{samples} messages; through the seam, this provider's cost and no file's");
    res("sign_one", NAME, "ms", stats(sv), &of);
    res("verify_one", NAME, "ms", stats(vv), &of);
}

/// Two copies of one genesis, each device writing half the records.
pub fn two_sides(n: u64, vs: usize) -> (Device, Store, Store) {
    let (mut a, mut b) = (Device::new(1), Device::new(2));
    let (n_a, n_b) = (n / 2, n - n / 2);
    let keys = [a.row(), b.row()];
    let (mut ours, mut theirs) = (room(n_a, seals_for(n_a), vs), room(n_b, seals_for(n_b), vs));
    genesis(&mut ours, &keys);
    genesis(&mut theirs, &keys);
    write(&mut a, &mut ours, n_a, vs, 2);
    write(&mut b, &mut theirs, n_b, vs, 3);
    (a, ours, theirs)
}

/// `Doc::merge_into` (it verifies both sides and the union, every seal twice over) and the signed
/// MERGE it leaves owed: a merge, whole, in memory. Device `a` has moved one record on after it.
pub fn merge(a: &mut Device, ours: &Store, theirs: &Store, n: u64, vs: usize) -> Result<Vec<u8>, DocError> {
    let mut fresh = room(n + 1, seals_for(n / 2) + seals_for(n - n / 2) + 1, vs);
    let u = Doc::merge_into(ours, theirs, &mut fresh, &Sha256Ids, &MlDsaVerifier, Trust::Genesis)?;
    assert!(u.wrote && u.both_wrote, "merge: both sides wrote");
    let m = merge_body(&MergeView { ours: u.ours, theirs: u.theirs, state: STATE_COMMUTES, seg_root: u.seg_root, conflicts: vec![] });
    a.commit(&mut fresh, &m);
    Ok(fresh.to_bytes_trimmed())
}

fn merge_rows(c: &Cfg, tag: &str) {
    let (mut a, ours, theirs) = two_sides(c.n, c.vs);
    let seals = seals_for(c.n / 2) + seals_for(c.n - c.n / 2);
    let (seq, tip) = (a.seq, a.tip);
    let (mut v, mut file) = (vec![], vec![]);
    for _ in 0..c.reps {
        (a.seq, a.tip) = (seq, tip);
        let t0 = t();
        file = merge(&mut a, &ours, &theirs, c.n, c.vs).expect("merge_into");
        v.push(ms(t0));
    }
    assert_whole(&file, c.n as usize + 1, seals + 1, "merge: the union");
    assert_eq!(check(&file, &MlDsaVerifier).unwrap().doc.counters.merges, 1, "merge: one MERGE record");
    res("merge", NAME, "ms", stats(v), &format!("{tag}; two devices, {} records in all, {seals} seals; both sides and the union verified, a fresh file built, one MERGE sealed; in memory", c.n));
    one("merge_bytes", NAME, "bytes", file.len() as f64, &format!("{tag}; the merged file with its MERGE record"));
}

/// archive, archive_added, size_compact. Pushes `archive_formula` twice into `sums`, the measured
/// and FORMAT §9.5's: `main` compares a row's values, so a difference is a CHECK mismatch and rc 1.
fn archive_rows(c: &Cfg, raw: &[u8], tag: &str, sums: &mut Vec<(String, u64)>) {
    let (wasm, text) = archive::inputs();
    let src = Store::from_bytes(raw);
    let mut plain = Store::create_bytes(raw.len() + SPEC_ROOM).expect("create_bytes");
    Doc::copy_bytes(&src, &mut plain).expect("copy_bytes");
    let plain = plain.to_bytes_trimmed();
    let mut dst = Store::create_bytes(raw.len() + wasm.len() + text.len() + SPEC_ROOM).expect("create_bytes");
    let spec = SpecIn { wasm: &wasm, wasm_sha256: sha256(&wasm), text: &text, abi: READER_ABI };
    Doc::archive_bytes(&src, &mut dst, &spec).expect("archive_bytes");
    let arch = dst.to_bytes_trimmed();
    assert_whole(&arch, c.n as usize, seals_for(c.n), "archive");

    let added = arch.len() as u64 - plain.len() as u64;
    let want = archive::formula(wasm.len(), text.len());
    let of = format!("{tag}; reader {} bytes, format text {} bytes", wasm.len(), text.len());
    one("archive", NAME, "bytes", arch.len() as f64, &format!("{of}; the working file is {} bytes, its copy with no reader {}", raw.len(), plain.len()));
    one("archive_added", NAME, "bytes", added as f64, &format!("{of}; archive minus the copy; FORMAT §9.5 says {want}"));
    one("size_compact", NAME, "bytes/rec", plain.len() as f64 / c.n as f64, &format!("{tag}; the copy `Doc::copy_bytes` writes, dropping every superseded root"));
    sums.push(("archive_formula".into(), added));
    sums.push(("archive_formula".into(), want));
}

pub fn run(c: &Cfg) -> Vec<(String, u64)> {
    let mut sums = vec![];
    let groups = ["load_signed", "size", "verify_signed", "seal", "merge", "archive"];
    if !groups.iter().any(|g| c.want(g)) {
        return sums;
    }
    assert!(c.n >= 2 && c.vs > 0, "signed: needs N >= 2, VS >= 1");
    let tag = format!("N={} VS={} seal every {RECS_PER_SEAL} records", c.n, c.vs);
    let dir = c.dir.join(NAME);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("doc.ein");
    let spare = c.reps * SAMPLES;

    let mut built = None;
    let mut v = vec![];
    for _ in 0..c.reps {
        let t0 = t();
        built = Some(build(c.n, c.vs, spare));
        v.push(ms(t0));
    }
    let (mut d, mut st) = built.expect("reps >= 1");
    res("load_signed", NAME, "ms", stats(v), &format!("{tag}; keygen + {} real seals, in memory", seals_for(c.n)));
    let raw = st.to_bytes_trimmed();
    write_atomic(&path, &raw, true);
    assert_whole(&raw, c.n as usize, seals_for(c.n), "the file");
    one("size", NAME, "bytes/rec", raw.len() as f64 / c.n as f64, &format!("{tag}; the working file: {} bytes", raw.len()));

    if c.want("verify_signed") {
        verify_rows(c, &path, &tag);
    }
    if c.want("seal") {
        seal_rows(c, &mut d, &mut st, &tag);
    }
    if c.want("merge") {
        merge_rows(c, &tag);
    }
    if c.want("archive") {
        archive_rows(c, &raw, &tag, &mut sums);
    }
    let _ = fs::remove_dir_all(&dir);
    sums
}
