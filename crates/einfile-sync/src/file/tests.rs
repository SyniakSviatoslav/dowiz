//! KILL TEST K-T2 (§14): three devices meet only by `.eintail` files on a real disk, in fixed orders,
//! with duplicates and lost files: every front equal, MERGEs at most the meetings. No fuzzer: the
//! orders are a list, and a failure names its row. Then one damaged byte in every section.

use super::*;
use crate::session::Peer;
use crate::toys::*;
use crate::Budget;
use einfile_image::Store;
use einfile_log::doc::{Doc, KIND_PLATFORM_TSA};
use std::path::PathBuf;

const NOW: i64 = 1_000;
const TTL: i64 = 100;

#[derive(Clone, Copy, PartialEq)]
enum Fate {
    Arrives,
    /// The file lands twice.
    Twice,
    /// The reply never comes back.
    LostReply,
}

struct Disk(PathBuf);

impl Disk {
    fn new(row: usize) -> Disk {
        let d = std::env::temp_dir().join(format!("eintail-{}-{row}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        Disk(d)
    }
    /// Every byte crosses a file, as it does on a phone.
    fn carry(&self, name: &str, bytes: &[u8]) -> Vec<u8> {
        let p = self.0.join(name);
        std::fs::write(&p, bytes).unwrap();
        std::fs::read(&p).unwrap()
    }
}

impl Drop for Disk {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// `peer_seen`: per ordered pair, what `i` last knew of `j`.
struct World<'a> {
    peers: Vec<Peer<'a>>,
    seen: Vec<Vec<Front>>,
    meetings: usize,
}

impl<'a> World<'a> {
    fn new(g: &[u8], devices: &'a [Toy]) -> Self {
        let peers: Vec<Peer<'a>> = devices.iter().map(|d| peer(g, d)).collect();
        let base = peers[0].front().unwrap();
        let seen = vec![vec![base; peers.len()]; peers.len()];
        World { peers, seen, meetings: 0 }
    }

    /// `i` packs for `j`, `j` reads it and packs back, and `i` reads that.
    fn meet(&mut self, i: usize, j: usize, fate: Fate, disk: &Disk) {
        self.meetings += 1;
        let out = self.peers[i].write_tail(&self.seen[i][j], NOW + TTL).unwrap();
        let there = disk.carry("out.eintail", &out);
        let got = self.peers[j].read_tail(&there, NOW, TTL).unwrap();
        self.seen[j][i] = got.seen;
        if fate == Fate::Twice {
            let again = self.peers[j].read_tail(&there, NOW, TTL).unwrap();
            assert!(!again.adopted.wrote, "the same file twice writes nothing");
        }
        let Some(reply) = got.reply.filter(|_| fate != Fate::LostReply) else { return };
        let back = disk.carry("back.eintail", &reply);
        let got = self.peers[i].read_tail(&back, NOW, TTL).unwrap();
        self.seen[i][j] = got.seen;
    }

    fn merges(&self) -> usize {
        self.peers.iter().map(|p| report(&p.store).counters.merges).max().unwrap()
    }
}

const ORDERS: [[(usize, usize, Fate); 6]; 6] = {
    use Fate::{Arrives as N, LostReply as L, Twice as T};
    [
        [(0, 1, N), (1, 2, N), (2, 0, N), (0, 1, N), (1, 2, N), (2, 0, N)],
        [(2, 1, N), (0, 2, N), (1, 0, N), (2, 1, N), (0, 2, N), (1, 0, N)],
        [(0, 1, T), (0, 1, T), (1, 2, T), (2, 0, T), (0, 1, N), (1, 2, N)],
        [(0, 1, L), (0, 1, N), (1, 2, L), (1, 2, N), (2, 0, L), (2, 0, N)],
        [(1, 2, L), (2, 0, T), (0, 1, L), (1, 2, N), (2, 0, N), (0, 1, N)],
        [(2, 0, T), (2, 1, L), (0, 1, N), (2, 0, L), (1, 2, N), (0, 2, N)],
    ]
};

#[test]
fn k_t2_three_devices_meet_by_file() {
    let devices = [toy(A), toy(B), toy(C)];
    let g = genesis(&devices.iter().collect::<Vec<_>>());
    for (row, order) in ORDERS.iter().enumerate() {
        let disk = Disk::new(row);
        let mut w = World::new(&g, &devices);
        for (k, d) in devices.iter().enumerate() {
            write(&mut w.peers[k].store, d, format!("offline {k}").as_bytes(), None);
        }
        for (i, j, fate) in order {
            w.meet(*i, *j, *fate, &disk);
        }
        // The list has ended and they still differ: one round each way ends it, and only that.
        for (i, j) in [(0, 1), (1, 2), (2, 0), (0, 1), (1, 2), (2, 0)] {
            w.meet(i, j, Fate::Arrives, &disk);
        }
        let front = w.peers[0].front().unwrap();
        for (k, p) in w.peers.iter().enumerate() {
            assert_eq!(p.front().unwrap(), front, "row {row}, device {k}");
            assert!(report(&p.store).holds(), "row {row}, device {k}: {:?}", report(&p.store).counters);
        }
        assert!(w.merges() <= w.meetings, "row {row}: {} merges in {} meetings", w.merges(), w.meetings);
    }
}

fn one_file() -> (Vec<u8>, Tail, Front) {
    let (a, b) = (toy(A), toy(B));
    let g = genesis(&[&a, &b]);
    let base = Doc::front(&Store::from_bytes(&g), &ToyIds).unwrap();
    let mut st = Store::from_bytes(&g);
    write(&mut st, &b, b"one", None);
    write(&mut st, &b, b"two", None);
    let over = einfile_log::EvLog::tip_at(&st, Doc::open(&st).unwrap().log).unwrap();
    Doc::anchor_bytes(&mut st, KIND_PLATFORM_TSA, &over, 9, b"proof").unwrap();
    let t = Doc::tail_of(&st, &base, &ToyIds).unwrap();
    (pack(&t, NOW + TTL, &Budget::default(), &ToyIds).unwrap(), t, base)
}

/// ONE DAMAGED BYTE IN EVERY SECTION, walked over all of them: `BadSum`, and the twin reads.
#[test]
fn k_t2_one_damaged_byte_anywhere_is_named() {
    let (f, t, _) = one_file();
    let b = Budget::default();
    assert_eq!(unpack(&f, NOW, &b, &ToyIds).map(|x| x.recs.len()), Ok(t.recs.len()));
    for at in FILE_MARK.len()..f.len() {
        let mut x = f.clone();
        x[at] ^= 1;
        assert_eq!(unpack(&x, NOW, &b, &ToyIds).err(), Some(SyncError::BadSum), "byte {at}");
    }
    let mut x = f;
    x[0] ^= 1;
    assert_eq!(unpack(&x, NOW, &b, &ToyIds).err(), Some(SyncError::Mark("file")));
}

fn landed(t: &Tail) -> Result<crate::Adopted, SyncError> {
    let (a, b) = (toy(A), toy(B));
    let mut p = peer(&genesis(&[&a, &b]), &a);
    p.adopt(t)
}

/// PAST THE SUM, a forger: what is signed is refused by the door, each by name. Anchors are unsigned
/// and carried like any anchor (`anch.rs`): an adapter judges those, and this pins that it is so.
#[test]
fn a_forger_who_fixes_the_sum_meets_the_door() {
    let (_, t, _) = one_file();
    assert!(landed(&t).unwrap().wrote, "the twin");
    let mut x = t.clone();
    x.recs[0].payload[0] ^= 1;
    x.recs[0].id = ToyIds.chained(&x.recs[0]);
    assert_eq!(landed(&x), Err(SyncError::Doc(einfile_log::doc::DocError::Fork { actor: B, seq: 1, ours: x.recs[0].id, theirs: t.recs[0].id })));
    let mut x = t.clone();
    x.seals[0].sig[0] ^= 1;
    assert!(matches!(landed(&x), Err(SyncError::Doc(einfile_log::doc::DocError::SealFork { .. } | einfile_log::doc::DocError::UnionNotWhole))));
    let mut x = t.clone();
    x.front.places[0].row.seq ^= 1;
    assert!(landed(&x).is_err());
    let mut x = t.clone();
    x.anchors[0].proof[0] ^= 1;
    assert!(landed(&x).unwrap().wrote, "ANCHORS ARE UNSIGNED: an adapter judges it, not this door");
}

#[test]
fn a_file_past_its_time_is_refused_and_the_clock_is_the_callers() {
    let (f, ..) = one_file();
    let b = Budget::default();
    assert!(unpack(&f, NOW + TTL, &b, &ToyIds).is_ok());
    assert_eq!(unpack(&f, NOW + TTL + 1, &b, &ToyIds).err(), Some(SyncError::Expired { expires_ms: NOW + TTL, now_ms: NOW + TTL + 1 }));
}

#[test]
fn a_file_too_small_or_too_big_is_refused_by_name() {
    let (f, ..) = one_file();
    let b = Budget::default();
    assert_eq!(unpack(FILE_MARK, NOW, &b, &ToyIds).err(), Some(SyncError::Short("file")));
    let tight = Budget { message: 8, ..Budget::default() };
    assert!(matches!(unpack(&f, NOW, &tight, &ToyIds), Err(SyncError::Over { what: "file", .. })));
}
