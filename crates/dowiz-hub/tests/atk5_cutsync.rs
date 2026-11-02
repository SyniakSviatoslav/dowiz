//! W-MG-ATK5, HOLE 13, CLOSED (W-MG-MERGE2): A CUT INSIDE ONE SYNC. `sync_stock` lays many records
//! under one seal over the newest (K2), and `cut` counts records, not seals. A cut that lands inside
//! a sync left that seal standing with its records half gone: `SealedHub::cut` answered Ok, the file
//! no longer held, and every merge of it was NOT_WHOLE. Now refused by name.
mod atk5_common;

use atk5_common::*;
use einstore::evlog::doc::{Doc, DocError, Signer};
use einstore::Store;
use dowiz_hub::doc_seal::{HubIds, SealedHub, MIN_SEALED_BYTES};

fn records(f: &[u8]) -> usize {
    SealedHub::load(f).unwrap().verify().unwrap().counters.records
}

/// The cut refuses, by name, and the file stands.
#[test]
fn a_cut_inside_one_sync_is_refused() {
    let s = genesis(&[received("salmon", 1), received("rice", 10)]);
    let mut sh = opened(&s, 2);
    let before = trimmed(&sh);
    let got = sh.cut(records(&s) - 1, &a(), 3).map(|_| ()).map_err(|e| format!("{e:?}"));
    assert!(got.as_ref().is_err_and(|e| e.contains("edge")), "HOLE 13: {got:?}, holds={}", holds(&trimmed(&sh)));
    assert_eq!(trimmed(&sh), before, "a refusal writes nothing");
}

/// The same on the library's own door, and on a merged file: a cut names the seal it would split.
#[test]
fn the_library_refuses_it_with_the_device_and_the_record() {
    let s = genesis(&[received("salmon", 1), received("rice", 10)]);
    let (st, n) = (Store::from_bytes(&s), records(&s));
    let mut fresh = Store::create_bytes(MIN_SEALED_BYTES).unwrap();
    let e = Doc::cut_into(&st, &mut fresh, n - 1, &HubIds).unwrap_err();
    assert_eq!(e, DocError::CutInsideSeal { actor: a().id(), seq: 1 });
}

/// THE TWIN: a cut on a seal's edge holds.
#[test]
fn a_cut_on_the_edge_of_a_sync_holds() {
    let s = genesis(&[received("salmon", 1), received("rice", 10)]);
    let mut sh = opened(&s, 2);
    sh.cut(records(&s) - 2, &a(), 3).unwrap();
    assert!(holds(&trimmed(&sh)));
}
