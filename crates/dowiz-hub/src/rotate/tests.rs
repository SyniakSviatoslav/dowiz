//! W-MG-M3: the checkpoint names the archive's RFC 9162 root, and the archive recomputes it.

use super::segment_root;
use crate::{EventKind, Hub};
use bebop_store::evlog::EvLog;

const ACTOR: [u8; 32] = [0xA1; 32];

fn twelve_orders() -> Hub {
    let mut h = Hub::create_sized(1 << 20).unwrap();
    let mut seq = 0;
    for o in 0..12 {
        let id = format!("ord_{o}");
        for s in ["PENDING", "DELIVERED"] {
            seq += 1;
            let kind = if s == "PENDING" { EventKind::Placed } else { EventKind::Advanced };
            h.append(kind, &id, &format!(r#"{{"id":"{id}","status":"{s}","total":900}}"#), seq, ACTOR).unwrap();
        }
    }
    h
}

/// The value after `root=` in the newest checkpoint.
fn checkpoint_root(h: &Hub) -> String {
    let mark = h.checkpoints().into_iter().next().expect("a checkpoint");
    mark.split(' ').find_map(|kv| kv.strip_prefix("root=")).expect("root=").to_string()
}

/// The archive's ids, oldest first, climb to the root the checkpoint names; one id fewer, or
/// one bit of one id, climbs elsewhere. The archive is read back exactly as a cold reader
/// would: `Hub::load` of the bytes the rotation handed back.
#[test]
fn the_checkpoint_root_is_recomputed_from_the_archive() {
    let mut h = twelve_orders();
    let archived = h.rotate(|id| id.ends_with('1')).unwrap();
    let named = checkpoint_root(&h);
    assert_eq!(named.len(), 64, "{named}");

    let cold = Hub::load(&archived).unwrap();
    let mut ids: Vec<[u8; 32]> = EvLog::walk(&cold.store).iter().map(|r| r.id).collect();
    ids.reverse();
    assert_eq!(ids.len(), 24);
    assert_eq!(crate::crypto::hex(&segment_root(&ids)), named, "the archive climbs to the named root");

    assert_ne!(crate::crypto::hex(&segment_root(&ids[..23])), named, "one record fewer");
    ids[7][0] ^= 1;
    assert_ne!(crate::crypto::hex(&segment_root(&ids)), named, "one bit of one id");

    // Everything the rotation promised before still holds: kept orders, chain, no quarantine.
    assert_eq!(h.orders().len(), 2, "ord_1 and ord_11");
    assert!(h.chain_check().intact());
    assert!(h.quarantined().is_empty());
}

/// A second rotation names a different root: the roots chain through the archives, newest
/// checkpoint first, as `Hub::checkpoints` lists them.
#[test]
fn each_rotation_names_its_own_archive() {
    let mut h = twelve_orders();
    h.rotate(|_| true).unwrap();
    let first = checkpoint_root(&h);
    h.append(EventKind::Placed, "ord_99", r#"{"id":"ord_99","status":"PENDING","total":100}"#, 99, ACTOR).unwrap();
    let second_archive = h.rotate(|_| false).unwrap();
    let second = checkpoint_root(&h);
    assert_ne!(first, second);
    let cold = Hub::load(&second_archive).unwrap();
    let mut ids: Vec<[u8; 32]> = EvLog::walk(&cold.store).iter().map(|r| r.id).collect();
    ids.reverse();
    assert_eq!(crate::crypto::hex(&segment_root(&ids)), second);
    assert_eq!(h.checkpoints().len(), 1, "an earlier checkpoint lives in the archive, not here");
}
