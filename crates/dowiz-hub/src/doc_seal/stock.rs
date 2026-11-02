//! STOCK UNDER A DOC (W-MG-M2, M2 design §1.1, §4): the shortest bridge between the two worlds. The
//! seal lived on the order log and the gluing on the stock log, and the two never met. A movement
//! here is a record of kind `KIND_STOCK` in the device's own chain behind M0's nonce: `seq` per
//! device, `actor` the device, the body `stock::encode`'s line. `StockLog`, the tablets and the
//! Worker are not touched: nothing live writes this yet (operator 2026-10-31, the lane's card).
//!
//! ONE SEAL PER SYNC (K2: 3 448 B a seal): `sync_stock` lays a device's movements since its last
//! sync under one seal over the newest (`Doc::commit_many_bytes`).
//!
//! THERE IS NO WRITE DOOR HERE: `sync_stock` does not fold the shelf. A device's ledger is the
//! tablet's and stays there; after an obstructed merge the union has no one fold, and a person
//! decides. What a merge reads is each SIDE's fold (`merge.rs`).

use bebop_store::evlog::doc::Doc;
use bebop_store::evlog::{EvLog, Record};

use super::{DeviceKey, SealedHub};
use crate::chain::{flags_for, record_id, with_nonce};
use crate::stock::{decode, encode, StockEvent};
use crate::{e_is_full, HubError};

/// The kind byte of a stock movement: `b'S'` (0x53), outside `EventKind`'s 1..=8 like `KIND_KEYOP`
/// and `KIND_DECL`, so `events()` drops it as no order and `HubIds::body` answers empty for it.
pub const KIND_STOCK: u8 = b'S';

/// The movement `r` carries, or `None` for any other record.
pub(super) fn stock_event(r: &Record) -> Option<StockEvent> {
    let (kind, line) = crate::body_of(r).split_first()?;
    if *kind != KIND_STOCK {
        return None;
    }
    decode(std::str::from_utf8(line).ok()?)
}

impl SealedHub {
    /// One sync of `key`'s device: every movement a record, one seal over the newest, one
    /// transaction. An empty sync is refused: a seal over nothing moves nothing.
    pub fn sync_stock(&mut self, moves: &[StockEvent], key: &DeviceKey, claimed_ms: i64) -> Result<i64, HubError> {
        let mut prev = self.device_prev(key);
        let mut seq = self.next_seq(key);
        let mut recs = Vec::with_capacity(moves.len());
        for ev in moves {
            let mut body = vec![KIND_STOCK];
            body.extend_from_slice(encode(ev).as_bytes());
            let (nonce, derived) = self.entropy.nonce(&prev, &body);
            let payload = with_nonce(&nonce, &body);
            let mut rec = Record { id: [0u8; 32], prev, actor_pubkey: key.id(), actor_seq: seq, flags: flags_for(derived), payload };
            rec.id = record_id(&rec);
            prev = rec.id;
            seq += 1;
            recs.push(rec);
        }
        match Doc::commit_many_bytes(&mut self.store, &recs, key, claimed_ms) {
            Ok(gen) => Ok(gen),
            Err(bebop_store::evlog::doc::DocError::Store(e)) if e_is_full(&e) => {
                self.grow()?;
                Doc::commit_many_bytes(&mut self.store, &recs, key, claimed_ms).map_err(super::sealed::doc_err)
            }
            Err(e) => Err(super::sealed::doc_err(e)),
        }
    }

    /// Every movement this build can read, oldest first in file order. After a merge that is one
    /// order of many: the order across devices is signed by nobody. `doc_verify` prints how many
    /// (`merge_report`), and how many it could not read.
    pub fn stock_events(&self) -> Vec<StockEvent> {
        let mut recs = self.records();
        recs.reverse();
        recs.iter().filter_map(stock_event).collect()
    }

    /// Every record, newest first.
    pub(super) fn records(&self) -> Vec<Record> {
        let log = Doc::open(&self.store).ok().and_then(|v| v.log);
        EvLog::walk_until_at(&self.store, log, |_| false)
    }
}
