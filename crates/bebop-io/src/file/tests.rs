use super::*;
use crate::{Durability, Writer};
use crate::kv::delta::Op;
use crate::sim::testkit::{padded, replay, CAP};
use crate::sim::Cut;

fn tmp(name: &str) -> String {
    let p = std::env::temp_dir().join(format!("bebop_esp2_{name}_{}.store", std::process::id()));
    p.to_str().unwrap().to_string()
}

fn write_image(path: &str, st: &crate::Store) {
    std::fs::write(path, st.to_bytes()).unwrap();
}

/// AC-ES-11 (in-process half): the lock refuses a second handle; dropping the first frees it.
#[test]
fn a_second_open_is_locked_until_the_first_is_dropped() {
    let p = tmp("lock");
    let first = FileDevice::create(&p, CAP).unwrap();
    assert!(matches!(FileDevice::open(&p), Err(IoError::Locked { .. })));
    drop(first);
    assert!(FileDevice::open(&p).is_ok(), "free again");
    assert!(std::path::Path::new(&lock_path(&p)).exists());
}

/// pwrite + read back; past the end reads zero.
#[test]
fn cells_round_trip_through_the_file() {
    let p = tmp("rw");
    let mut d = FileDevice::create(&p, CAP).unwrap();
    d.write_cells(2000, &[42, -7]).unwrap();
    d.barrier().unwrap();
    assert_eq!(d.read_cells(2000, 2).unwrap(), vec![42, -7]);
    assert_eq!(d.len_cells().unwrap(), CAP / 8);
    assert_eq!(d.read_cells(CAP / 8 - 1, 3).unwrap(), vec![0, 0, 0]);
}

/// T-ES-24: a real file, the process "dying" after each of the commit's five operations in
/// turn (`Cut`): reopening the FILE always recovers k or k+1, replaying to before or after.
#[test]
fn a_file_cut_after_any_operation_reopens_at_k_or_k_plus_1() {
    let p = tmp("cut");
    let base = padded();
    let before = replay(&base);
    let mut gens = Vec::new();
    for budget in 0..=5 {
        write_image(&p, &base);
        let mut w = Writer::open(Cut { inner: FileDevice::open(&p).unwrap(), budget }).unwrap();
        let r = w.commit_delta(&[Op::Put("b", b"2")], Durability::Data);
        assert_eq!(r.is_ok(), budget == 5, "budget {budget}");
        drop(w);
        let w = Writer::open(FileDevice::open(&p).unwrap()).unwrap();
        let g = w.durable().gen;
        let want = if g == 1 { before.clone() } else { crate::sim::testkit::applied(&before, &[Op::Put("b", b"2")]) };
        assert_eq!(replay(w.store()), want, "budget {budget}");
        gens.push(g);
    }
    assert_eq!(gens, vec![1, 1, 1, 1, 2, 2], "the superblock write (op 4) is the commit point");
}

/// The first cells of the file at `path`, as the next opener would read them.
fn at_name(p: &str, n: usize) -> Vec<i64> {
    let b = std::fs::read(p).unwrap();
    b[..n * 8].chunks_exact(8).map(|w| i64::from_le_bytes(w.try_into().unwrap())).collect()
}

/// W-ES-HARD: a publish from a sibling that was NOT barriered renames NOTHING: the device reads
/// and writes the sibling, the store's name still holds the old cells (what a crash would leave),
/// a second deferred publish takes the other sibling name and removes the first, and the next
/// barrier names the newest one; a barriered sibling is renamed at once; a clean drop finishes a
/// pending publish.
#[test]
fn a_deferred_publish_leaves_the_old_file_at_the_name_until_the_next_barrier() {
    let p = tmp("publish");
    let mut d = FileDevice::create(&p, CAP).unwrap();
    d.write_cells(0, &[1, 2]).unwrap();
    d.barrier().unwrap();
    assert!(!d.pending_publish());
    let mut staged = d.fresh().unwrap();
    staged.write_cells(0, &[11, 12]).unwrap();
    d.publish(staged).unwrap();
    assert!(d.pending_publish(), "no barrier on the sibling: the rename waits for one");
    assert_eq!(d.read_cells(0, 2).unwrap(), vec![11, 12], "the device reads the sibling");
    assert_eq!(at_name(&p, 2), vec![1, 2], "the name still holds the durable cells");
    d.write_cells(0, &[13, 14]).unwrap();
    assert_eq!(at_name(&p, 2), vec![1, 2], "a later write goes to the sibling, not the old file");
    let mut staged2 = d.fresh().unwrap();
    assert_eq!(staged2.path(), format!("{p}.compact2"), "the other sibling name while the first is pending");
    staged2.write_cells(0, &[21, 22]).unwrap();
    d.publish(staged2).unwrap();
    assert!(d.pending_publish());
    assert!(!std::path::Path::new(&compact_path(&p)).exists(), "the superseded sibling is removed");
    assert_eq!(at_name(&p, 2), vec![1, 2]);
    d.barrier().unwrap();
    assert!(!d.pending_publish(), "the barrier synced the sibling, renamed it, fsynced the directory");
    assert_eq!(at_name(&p, 2), vec![21, 22]);
    assert!(!std::path::Path::new(&format!("{p}.compact2")).exists());
    let mut staged = d.fresh().unwrap();
    assert_eq!(staged.path(), compact_path(&p));
    staged.write_cells(0, &[31, 32]).unwrap();
    staged.barrier().unwrap();
    d.publish(staged).unwrap();
    assert!(!d.pending_publish(), "a barriered sibling is published synced");
    assert_eq!(at_name(&p, 2), vec![31, 32]);
    let mut staged = d.fresh().unwrap();
    staged.write_cells(0, &[41, 42]).unwrap();
    d.publish(staged).unwrap();
    assert!(d.pending_publish());
    drop(d);
    assert_eq!(FileDevice::open(&p).unwrap().read_cells(0, 2).unwrap(), vec![41, 42], "a clean drop finished the publish");
    assert!(!std::path::Path::new(&compact_path(&p)).exists());
}
