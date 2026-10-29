//! `Kv::load_checked`: the KV half of the crc check on load (W-CRC). The module doc of the
//! whole check is bebop-image's `verify.rs`.

#[allow(unused_imports)]
use crate::prelude::*;
use crate::Kv;
use crate::Store;
pub use bebop_image::verify::BadCrc;

/// Why a KV image was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KvError {
    /// No KV root, or a slice in it does not fit its array (what `load` always said `None` to).
    NotKv,
    /// The root or one of its four arrays fails its crc.
    BadCrc(BadCrc),
    /// The root names a KV version this code does not know (W-ES-P8A: v4 is the B+tree; until a
    /// reader is taught it, it is REFUSED by name, never read as a v1-v3 base and never folded).
    UnknownVersion { v: i64 },
}

impl Kv {
    /// Read all entries out of a store, refusing an object whose crc does not match.
    pub fn load_checked(st: &Store) -> Result<Kv, KvError> {
        let root = st.root().ok_or(KvError::NotKv)?;
        st.check_obj(root).map_err(KvError::BadCrc)?;
        // W-ES-P8A: a version this code does not know is refused right after the root's own crc,
        // before its cells 1-4 are followed as v1-v3 arrays and before any crc of what they name.
        let v = Kv::version(st);
        if v > crate::delta::VERSION_DELTA {
            return Err(KvError::UnknownVersion { v });
        }
        for i in 1..=4 {
            if let Some(arr) = st.follow(root, i) {
                st.check_obj(arr).map_err(KvError::BadCrc)?;
            }
        }
        // W-DELTA: every record on the chain is checked too, before any is replayed.
        let chain = crate::delta::chain_in(st, root).ok_or(KvError::NotKv)?;
        crate::delta::check_chain(st, &chain).map_err(KvError::BadCrc)?;
        Kv::decode(st).ok_or(KvError::NotKv)
    }

    /// `load_checked` for a caller that only asks yes or no. `None` covers a crc
    /// mismatch too: it is never data.
    pub fn load(st: &Store) -> Option<Kv> {
        Self::load_checked(st).ok()
    }
}
