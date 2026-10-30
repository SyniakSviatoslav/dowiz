//! Opening an image the caller holds (`Db::from_bytes`, dowiz's Durable Object) or a store file
//! by path, and naming a refused torn commit (W-ES-P3). Split out of `db.rs` (the 300-line rule).

use super::{Db, OnTorn, Options};
use crate::refusal::Refusal;
use crate::{Store, Writer};

/// A refused torn last commit, NAMED where it can be (dowiz's `HubError::BadCrc` names the
/// object, W-CRC): the anchor says only that `[mark, used)` changed, so the objects of that range
/// are walked and the first whose payload fails its crc is named. When none does (the anchor
/// cell itself, or a header, changed), the refusal stays `LastCommit`. The walk is
/// `bebop_image::verify::last_commit_in` -- THE rule of FORMAT §7 phase P2 (W-ES-P10), the one
/// every reader of bytes applies, so this surface and the four others cannot drift apart.
fn name_torn(bytes: &[u8], e: Refusal) -> Refusal {
    let Refusal::LastCommit(crate::sim::Why::Anchor(crate::format::Anchor::Torn { .. })) = e else { return e };
    let st = Store::from_bytes(bytes);
    match crate::format::pick_checked_in(&st).map(|sb| bebop_image::verify::last_commit_in(&st, &sb)) {
        Ok(Err(bebop_image::verify::LastCommit::Object(b))) => Refusal::BadCrc(b),
        _ => e,
    }
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
