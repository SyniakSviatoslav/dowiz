//! `id_message`: `ID_TAG ‖ flags LE8 ‖ actor 32 ‖ seq LE8 ‖ prev 32 ‖ payload`.
//!
//! "A record" here is the five-tuple `(flags, actor_pubkey, actor_seq, prev, payload)`, which is the
//! whole `Record` but its `id`, the output of the hash. What a caller passes as `flags` is the
//! caller's: these harnesses stop at `id_message`'s own arguments.
//!
//! WHY: M0 hashed `prev ‖ payload` alone, and a cell outside the id is a cell a forger may change
//! (`atk_holes` holes 2, 9). What is signed must fix every cell, and two records must not collide
//! before the hash even starts.
//!
//! BOUNDS: flags, actor, seq, prev are UNBOUNDED (every value of each). The payload is bounded:
//! length <= 4 in the pairwise harness, <= 16 in the layout harness. The function has no branch on
//! the payload. That longer payloads behave alike is an argument from reading it, not a check.

use crate::common::{i64_at, id_at, u64_at, Bytes};
use bebop_log::doc::{id_message, ID_TAG};

const PAIR_PAYLOAD: usize = 4;
const LAYOUT_PAYLOAD: usize = 16;
/// `ID_TAG` is 12 bytes, the fixed fields 80.
const HEAD: usize = 92;

struct Rec<const P: usize> {
    flags: i64,
    actor: [u8; 32],
    seq: u64,
    prev: [u8; 32],
    payload: Bytes<P>,
}

fn any_rec<const P: usize>() -> Rec<P> {
    Rec { flags: kani::any(), actor: kani::any(), seq: kani::any(), prev: kani::any(), payload: Bytes::any() }
}

fn msg<const P: usize>(r: &Rec<P>) -> Vec<u8> {
    id_message(r.flags, &r.actor, r.seq, &r.prev, r.payload.as_slice())
}

/// PROPERTY: two different records never serialise alike. If two messages are equal, then flags,
/// actor, seq, prev and payload are all equal. Each pair of records within the payload bound.
#[kani::proof]
#[kani::unwind(102)] // past the longest message compared: 92 + 4
fn id_message_is_injective() {
    let (a, b) = (any_rec::<PAIR_PAYLOAD>(), any_rec::<PAIR_PAYLOAD>());
    let same = msg(&a) == msg(&b);
    kani::cover!(same, "two records can serialise alike only by being the same record");
    kani::cover!(!same && a.payload.as_slice().len() != b.payload.as_slice().len(), "payload lengths differ");
    if same {
        assert_eq!(a.flags, b.flags);
        assert_eq!(a.actor, b.actor);
        assert_eq!(a.seq, b.seq);
        assert_eq!(a.prev, b.prev);
        assert_eq!(a.payload.as_slice(), b.payload.as_slice());
    }
}

/// PROPERTY: the framing is fixed-width with the one variable field last, so each field reads back
/// from a fixed offset. This is the inverse of `id_message`, the structural reason for injectivity,
/// and it pins the documented order: a swap of `actor` and `prev` passes the harness above and fails
/// this one.
#[kani::proof]
#[kani::unwind(120)] // past the longest message read: 92 + 16
fn id_message_layout_is_as_documented() {
    let r = any_rec::<LAYOUT_PAYLOAD>();
    let m = msg(&r);
    assert_eq!(ID_TAG.len() + 80, HEAD);
    assert_eq!(m.len(), HEAD + r.payload.as_slice().len());
    assert_eq!(&m[..ID_TAG.len()], ID_TAG);
    assert_eq!(i64_at(&m, 12), r.flags);
    assert_eq!(id_at(&m, 20), r.actor);
    assert_eq!(u64_at(&m, 52), r.seq);
    assert_eq!(id_at(&m, 60), r.prev);
    assert_eq!(&m[HEAD..], r.payload.as_slice());
}
