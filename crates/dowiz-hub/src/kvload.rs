//! The one way a KV image of this crate is read from bytes (W-CRC, 2026-10-05).
//!
//! `Catalog`, `Settings`, `Posts`, `Table` and `Roster` each did
//! `Kv::load(&store).ok_or(HubError::NotAHub)`, so any refusal -- including the crc
//! mismatch `Kv::load` now detects -- read as "this is not a hub image". A changed byte
//! is a different failure with a different remedy (restore last night's copy, not
//! "create a fresh one"), so it keeps its name: `HubError::BadCrc`, naming the object.

use bebop_store::kv::Kv;
use bebop_store::verify::KvError;
use bebop_store::Store;

use crate::HubError;

pub(crate) fn kv_load(store: &Store) -> Result<Kv, HubError> {
    Kv::load_checked(store).map_err(|e| match e {
        KvError::NotKv => HubError::NotAHub,
        KvError::BadCrc(b) => HubError::BadCrc(b),
    })
}

#[cfg(test)]
mod tests;
