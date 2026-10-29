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
