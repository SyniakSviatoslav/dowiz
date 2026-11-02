// SPDX-License-Identifier: MIT OR Apache-2.0
//! A STRANGER: this file links only MIT OR Apache-2.0 code (no dowiz-hub, no dowiz-core) and
//! writes a sealed DOC, checks it, and watches a changed tip go bad. It is what `crates/einstore`'s
//! README says a user with no dowiz code can do.

use einfile_image::Store;
use einfile_log::doc::{id_message, verify, Doc, Ids, Signer as _, Trust};
use einfile_log::{Record, FLAG_SHA256};
use einfile_sig_mldsa::{actor_id, DeviceKey, MlDsa65};

/// A stranger's id scheme: sha256 over `id_message`.
struct Sha;
impl Ids for Sha {
    fn chained(&self, r: &Record) -> [u8; 32] {
        actor_id(&id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload))
    }
}

fn sealed(key: &DeviceKey, n: u64) -> Store {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    Doc::init_bytes(&mut st, 0, &[key.key()]).unwrap();
    let mut tip = [0u8; 32];
    for seq in 1..=n {
        let payload = format!("nonce-{seq:0>10}event-{seq}").into_bytes();
        let id = actor_id(&id_message(FLAG_SHA256, &key.actor_id(), seq, &tip, &payload));
        let r = Record { id, prev: tip, actor_pubkey: key.actor_id(), actor_seq: seq, flags: FLAG_SHA256, payload };
        Doc::commit_bytes(&mut st, &r, key, 1_700_000_000_000 + seq as i64, None).unwrap();
        tip = id;
    }
    st
}

#[test]
fn a_stranger_seals_and_checks_with_no_dowiz_code() {
    let key = DeviceKey::from_seed(&[5u8; 32]);
    let mut st = sealed(&key, 3);
    let rep = verify(&st, &Sha, &MlDsa65, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.chained, rep.counters.broken), (3, 0), "{rep:?}");
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad), (3, 0), "{rep:?}");
    assert!(rep.holds(), "{rep:?}");

    let heads = Doc::open(&st).unwrap().heads.unwrap();
    st.put_cell(heads, 1 + 5, st.get(heads, 1 + 5) ^ 1);
    st.seal(heads);
    let rep = verify(&st, &Sha, &MlDsa65, Trust::Genesis).unwrap();
    assert_eq!(rep.counters.seals_bad, 1, "a changed head tip under a real signature: {rep:?}");
    assert!(!rep.holds(), "{rep:?}");
}
