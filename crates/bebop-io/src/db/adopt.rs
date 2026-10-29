//! Opening an image the caller holds (`Db::from_bytes`, dowiz's Durable Object) or a store file
//! by path, and naming a refused torn commit (W-ES-P3). Split out of `db.rs` (the 300-line rule).

use super::{Db, OnTorn, Options};
use crate::refusal::Refusal;
use crate::{Store, Writer};

/// A refused torn last commit, NAMED where it can be (dowiz's `HubError::BadCrc` names the
/// object, W-CRC): the anchor says only that `[mark, used)` changed, so walk the objects of that
/// range and name the first whose payload fails its crc. When none does (the anchor cell itself,
/// or a header, changed), the refusal stays `LastCommit`.
fn name_torn(bytes: &[u8], e: Refusal) -> Refusal {
    let Refusal::LastCommit(crate::sim::Why::Anchor(crate::format::Anchor::Torn { mark, used, .. })) = e else { return e };
    let st = Store::from_bytes(bytes);
    let (mut i, end) = (mark.max(0) as usize, (used.max(0) as usize).min(st.cells.len()));
    while i < end {
        if let Err(b) = st.check_obj(i) {
            return Refusal::BadCrc(b);
        }
        i = i.saturating_add(2).saturating_add(st.obj_len(i).max(0) as usize);
    }
    e
}

impl Db<crate::Detached> {
    /// Adopt an image the caller holds (dowiz's Durable Object). `opts` normally
    /// `Options::detached()`: a torn last commit is refused, not fallen back past.
    pub fn from_bytes(bytes: &[u8], opts: Options) -> Result<Db<crate::Detached>, Refusal> {
        let w = match Writer::adopt(crate::Detached, Store::from_bytes(bytes), opts.on_torn == OnTorn::Fallback) {
            Ok(w) => w,
            Err(e) => return Err(name_torn(bytes, e.into())),
        };
        Db::over(w, opts)
    }

    /// A fresh, empty KV database whose bytes the caller will persist.
    pub fn create_detached(opts: Options) -> Result<Db<crate::Detached>, Refusal> {
        Db::create(crate::Detached, opts)
    }
}

#[cfg(all(unix, not(target_arch = "wasm32")))]
impl Db<crate::FileDevice> {
    /// Take `<path>.lock` and open the store file.
    pub fn open_path(path: &str, opts: Options) -> Result<Self, Refusal> {
        Db::open(crate::FileDevice::open(path)?, opts)
    }

    /// Create the store file (fsynced, and its directory) and the empty KV schema in it.
    pub fn create_path(path: &str, opts: Options) -> Result<Self, Refusal> {
        Db::create(crate::FileDevice::create(path, opts.initial_bytes)?, opts)
    }
}
