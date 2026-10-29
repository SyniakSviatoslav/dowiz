use super::*;

/// The record keeps every write and barrier in issue order; the cache applies them.
#[test]
fn the_sim_disk_records_in_order_and_the_cache_applies() {
    let mut d = SimDisk::new(vec![0; 4]);
    d.write_cells(1, &[7, 8]).unwrap();
    d.barrier().unwrap();
    d.write_cells(5, &[9]).unwrap();
    assert_eq!(d.ops(), &[SimOp::Write { off: 1, cells: vec![7, 8] }, SimOp::Barrier, SimOp::Write { off: 5, cells: vec![9] }]);
    assert_eq!(d.cache(), vec![0, 7, 8, 0, 0, 9], "grown with zeros");
    assert_eq!(d.base(), &[0; 4], "the durable base is untouched");
    assert_eq!(d.read_cells(4, 3).unwrap(), vec![0, 9, 0], "past the end reads 0");
    d.settle();
    assert_eq!((d.base().len(), d.ops().len()), (6, 0));
}

/// FaultDevice: the named barrier fails ONCE with EIO; its twin, the next one, succeeds.
#[test]
fn a_failing_barrier_fails_once_with_eio() {
    let mut d = SimDisk::new(vec![]).failing_barrier(1);
    assert!(d.barrier().is_ok());
    assert_eq!(d.barrier().unwrap_err().raw_os_error(), Some(5));
    assert!(d.barrier().is_ok());
    assert_eq!(d.ops(), &[SimOp::Barrier, SimOp::Barrier], "the failed barrier is not recorded as one");
}

/// `Cut` lets exactly `budget` operations through, then refuses every one.
#[test]
fn a_cut_device_stops_after_its_budget() {
    let mut c = Cut { inner: SimDisk::new(vec![]), budget: 2 };
    c.write_cells(0, &[1]).unwrap();
    c.barrier().unwrap();
    assert!(c.write_cells(1, &[2]).is_err());
    assert!(c.barrier().is_err());
    assert_eq!(c.inner.ops().len(), 2);
}
