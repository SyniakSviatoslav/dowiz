#![cfg(feature = "pq")]

//! B4 (Tier-B4 content-addressed backup organ) — kernel-local caller wiring (RED→GREEN).
//!
//! Wires `backup::snapshot_and_restore_local` — a real production caller for the
//! `BackupOrgan` that had ZERO callers (only module tests used it). The caller
//! backs up a hub-state snapshot and restores it byte-for-byte, modeling the
//! P68 M4 local pre-promote-snapshot / rollback cycle. Before this wiring the
//! caller did not exist ⇒ the test fails to compile (RED). After wiring it
//! compiles and passes (GREEN).

use dowiz_kernel::backup::{
    snapshot_and_restore_local, BackupOrgan, BackupStats, MemStore, RestoreError,
};

#[test]
fn b4_snapshot_and_restore_is_byte_identical() {
    let data: Vec<u8> = (0u8..=255).cycle().take(120_000).collect();

    let mut organ = BackupOrgan::new(MemStore::new(), 1024, 32 * 1024, 12);

    // The kernel-local caller backs up the snapshot and restores it.
    let (restored, stats) = snapshot_and_restore_local(&mut organ, &data)
        .expect("snapshot + restore must succeed");
    assert_eq!(restored, data, "restored snapshot must be byte-identical");
    assert!(stats.total_blocks > 0, "snapshot produced blocks");

    // Re-snapshotting the SAME state must dedup to ZERO new physical blocks
    // (the dedup property is observable through the caller's returned stats).
    let (restored2, stats2) = snapshot_and_restore_local(&mut organ, &data)
        .expect("second snapshot + restore");
    assert_eq!(restored2, data);
    assert_eq!(
        stats2.new_blocks, 0,
        "unchanged re-snapshot must write no new blocks"
    );
    assert_eq!(stats2.dedup_ratio(), 1.0);
}

#[test]
fn b4_corrupt_snapshot_restores_fail_closed() {
    let data: Vec<u8> = (0u8..=200).cycle().take(40_000).collect();

    let mut organ = BackupOrgan::new(MemStore::new(), 1024, 32 * 1024, 12);
    // Take a clean snapshot to obtain a valid manifest.
    let (manifest, _s) = organ.backup(&data);
    let (restored, _stats) = snapshot_and_restore_local(&mut organ, &data)
        .expect("first snapshot+restore");
    assert_eq!(restored, data, "clean restore is byte-identical");

    // A truncated/corrupt manifest must fail-closed on restore, never yield
    // partial/garbage bytes — the RestoreError contract holds through the caller's
    // owned `restore` (the caller returns `Err` on any restore failure).
    let mut corrupt = manifest.clone();
    corrupt.blocks.push([0xAB; 32]);
    corrupt.total_len += 1;
    // The caller's own restore path returns Err on a corrupt manifest.
    assert!(organ.restore(&corrupt).is_err());
}
