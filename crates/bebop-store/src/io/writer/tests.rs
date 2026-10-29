use super::*;
use crate::kv::delta::{Appended, Op};
use crate::sim::testkit::{padded, replay};
use crate::sim::{SimDisk, SimOp, Via};

fn writer() -> Writer<SimDisk> {
    Writer::open(SimDisk::new(padded().cells)).unwrap()
}

/// T-ES-12 (AC-ES-12): a staged, unflushed commit writes nothing, and `abort` hands back
/// exactly the durable image.
#[test]
fn abort_leaves_every_byte_as_it_was() {
    let mut w = writer();
    let before = w.store().to_bytes_trimmed();
    crate::kv::delta::append_delta(w.stage().unwrap(), &[Op::Put("b", b"2")]).unwrap();
    assert_ne!(w.store().to_bytes_trimmed(), before, "staged");
    let w = w.abort().unwrap();
    assert_eq!(w.store().to_bytes_trimmed(), before);
    assert!(w.into_device().ops().is_empty(), "nothing reached the device");
}

/// T-ES-14 (AC-ES-14), the two named barriers: EIO at barrier 1 (before the superblock) and
/// at barrier 2 (the commit point). Either way the writer is POISONED -- stage and flush
/// refuse -- and `reopen` recovers k (barrier 1: no superblock was issued) or k+1 (barrier 2:
/// it was, and the cache holds it), after which a commit succeeds.
#[test]
fn a_failed_barrier_poisons_until_reopen() {
    for (n, gen) in [(0usize, 1i64), (1, 2)] {
        let base = padded();
        let mut w = Writer::open(SimDisk::new(base.cells).failing_barrier(n)).unwrap();
        crate::kv::delta::append_delta(w.stage().unwrap(), &[Op::Put("b", b"2")]).unwrap();
        let e = w.flush(Durability::Data).unwrap_err();
        assert!(matches!(e, IoError::Io(ref e) if e.raw_os_error() == Some(5)), "{e:?}");
        assert!(w.poisoned());
        assert!(matches!(w.stage(), Err(IoError::Poisoned)));
        assert!(matches!(w.flush(Durability::Data), Err(IoError::Poisoned)));
        assert!(matches!(w.sync(), Err(IoError::Poisoned)));
        let mut w = w.reopen().unwrap();
        assert!(!w.poisoned());
        assert_eq!(w.durable().gen, gen, "barrier {n}");
        assert_eq!(w.commit_delta(&[Op::Put("c", b"3")], Durability::Data).unwrap(), Appended::Delta(gen + 1));
    }
}

/// The twin: no fault, no poison; `commit_delta` is the durable delta commit P7/P3 call.
#[test]
fn commit_delta_writes_protocol_v2_and_reopens_at_the_new_generation() {
    let mut w = writer();
    assert_eq!(w.commit_delta(&[Op::Put("b", b"2")], Durability::Data).unwrap(), Appended::Delta(2));
    let dev = w.into_device();
    assert_eq!(dev.ops().iter().filter(|o| **o == SimOp::Barrier).count(), 2);
    let w = Writer::open(dev).unwrap();
    assert_eq!(w.durable().gen, 2);
    assert!(matches!(w.recovered().via, Via::Anchor { .. }));
    assert_eq!(replay(w.store()).unwrap().get("b").map(|v| v.as_slice()), Some(&b"2"[..]));
}

/// REPAIR: opening over a crc-valid superblock whose last commit is torn zeroes its magic ON
/// THE DEVICE, behind a barrier, before anything else; the next commit then lands cleanly.
#[test]
fn open_repairs_a_hidden_superblock_on_the_device() {
    let mut w = writer();
    w.commit_delta(&[Op::Put("b", b"2")], Durability::Data).unwrap();
    let mut img = w.into_device().cache();
    let mark = (img[crate::format::SB_ANC] as u64 >> 32) as usize;
    assert_eq!(Store { cells: img.clone() }.pick().unwrap().at, 0, "gen 2 lives in A");
    img[mark + 2] ^= 1;
    let w = Writer::open(SimDisk::new(img)).unwrap();
    assert_eq!(w.recovered().hidden, Some(0));
    assert_eq!(&w.into_device().ops()[..2], &[SimOp::Write { off: 0, cells: vec![0] }, SimOp::Barrier]);
}
