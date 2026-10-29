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
}

impl Kv {
    /// Read all entries out of a store, refusing an object whose crc does not match.
    pub fn load_checked(st: &Store) -> Result<Kv, KvError> {
        let root = st.root().ok_or(KvError::NotKv)?;
        st.check_obj(root).map_err(KvError::BadCrc)?;
        for i in 1..=4 {
            if let Some(arr) = st.follow(root, i) {
                st.check_obj(arr).map_err(KvError::BadCrc)?;
            }
        }
        // W-DELTA: every record on the chain is checked too, before any is replayed -- after a
        // version this code does not know is refused, so it never surfaces as a crc failure.
        if Kv::version(st) > crate::delta::VERSION_DELTA {
            return Err(KvError::NotKv);
        }
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
