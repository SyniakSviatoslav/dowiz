//! TOY SEAMS for this crate's tests, as in `bebop-log`'s `doc/tests.rs`: NOT CRYPTO. A 32-byte mix
//! and a "signature" anyone can forge. They prove the layout and the flow; real ML-DSA-65 goes through
//! the same door in `crates/dowiz-hub/tests/k_t1_tail.rs`.

use crate::{Budget, Device, Peer};
use bebop_image::Store;
use bebop_log::doc::seal::read_heads;
use bebop_log::doc::{id_message, verify, Doc, Ids, Key, KeyOp, Report, Signer, Trust, Union, Verifier};
use bebop_log::{EvLog, Record};

pub const ALG_TOY: i64 = 1;
pub const A: [u8; 32] = [0xA1; 32];
pub const B: [u8; 32] = [0xB2; 32];
pub const C: [u8; 32] = [0xC3; 32];
pub static ROOTS: [[u8; 32]; 1] = [A];
pub static IDS: ToyIds = ToyIds;
pub static V: ToyVerifier = ToyVerifier;

pub fn mix(parts: &[&[u8]]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ lane.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for p in parts {
            for b in p.iter() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100_0000_01b3);
            }
            h ^= 0xFF;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        out[lane as usize * 8..lane as usize * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}

pub struct ToyIds;
impl Ids for ToyIds {
    fn chained(&self, r: &Record) -> [u8; 32] {
        mix(&[b"c", &id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload)])
    }
    fn hash(&self, bytes: &[u8]) -> Option<[u8; 32]> {
        Some(mix(&[b"h", bytes]))
    }
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        Some(mix(&[b"t", &ids.concat()]))
    }
}

pub struct ToySigner {
    pub id: [u8; 32],
}
impl Signer for ToySigner {
    fn actor_id(&self) -> [u8; 32] {
        self.id
    }
    fn alg(&self) -> i64 {
        ALG_TOY
    }
    fn public_key(&self) -> Vec<u8> {
        self.id.to_vec()
    }
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        mix(&[b"toy", &self.id, msg]).to_vec()
    }
}

pub struct ToyVerifier;
impl Verifier for ToyVerifier {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        alg == ALG_TOY && sig == mix(&[b"toy", pk, msg])
    }
}

pub fn key_of(id: [u8; 32]) -> Key {
    Key { actor_id: id, alg: ALG_TOY, valid_from: 0, revoked_at: 0, admit_rec: None, pk: id.to_vec() }
}

/// A device: its next record is derived from the file it is given, so a retry on a larger image
/// asks again and gets the same answer. `named` is the conflict list it reports.
pub struct Toy {
    pub s: ToySigner,
    pub named: Vec<Vec<u8>>,
}

pub fn toy(id: [u8; 32]) -> Toy {
    Toy { s: ToySigner { id }, named: Vec::new() }
}

impl Device for Toy {
    fn signer(&self) -> &dyn Signer {
        &self.s
    }
    fn record(&self, union: &Store, body: &[u8]) -> Record {
        let view = Doc::open(union).unwrap();
        let head = read_heads(union, view.heads).into_iter().find(|h| h.actor_id == self.s.id);
        let (seq, prev) = match head {
            Some(h) => (h.seq + 1, h.tip),
            None => (1, EvLog::tip_at(union, view.log).unwrap_or([0u8; 32])),
        };
        let mut r = Record { id: [0u8; 32], prev, actor_pubkey: self.s.id, actor_seq: seq, flags: 0, payload: body.to_vec() };
        r.id = ToyIds.chained(&r);
        r
    }
    fn conflicts(&self, _union: &Store, _u: &Union) -> Vec<Vec<u8>> {
        self.named.clone()
    }
    fn claimed_ms(&self) -> i64 {
        1_700_000_000_000
    }
}

/// One device commit of `body`.
pub fn write(st: &mut Store, d: &Toy, body: &[u8], op: Option<&KeyOp>) {
    let r = d.record(st, body);
    Doc::commit_bytes(st, &r, &d.s, d.claimed_ms(), op).unwrap();
}

/// A file with `A` as its root and `B`, `C` admitted: the shared past of every test here.
pub fn genesis(devices: &[&Toy]) -> Vec<u8> {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    Doc::init_bytes(&mut st, 0, &[key_of(A)]).unwrap();
    let a = toy(A);
    write(&mut st, &a, b"first", None);
    for d in devices.iter().filter(|d| d.s.id != A) {
        let op = KeyOp::Admit(key_of(d.s.id));
        write(&mut st, &a, &op.body(), Some(&op));
    }
    st.to_bytes()
}

pub fn peer<'a>(bytes: &[u8], d: &'a Toy) -> Peer<'a> {
    Peer { store: Store::from_bytes(bytes), ids: &IDS, v: &V, roots: &ROOTS, device: d, budget: Budget::default() }
}

pub fn report(st: &Store) -> Report {
    verify(st, &ToyIds, &ToyVerifier, Trust::Only(&ROOTS)).unwrap()
}
