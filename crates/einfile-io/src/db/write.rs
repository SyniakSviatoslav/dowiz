//! `Db::write` (W-ES-P19b): a transaction as a closure, retried on `Conflict` with the policy of
//! es-p19.design §2.1. Split out of db.rs (the 300-line rule).

use super::{Db, Mode};
use crate::refusal::Refusal;
use crate::txn::WriteTxn;
use crate::{Device, Durability};

/// `Db::write`: retries on `Conflict` before the refusal comes back, and the conflict count
/// after which the run turns pessimistic (es-p19 §2.1: 16 and 4).
pub const MAX_RETRIES: usize = 16;
pub const PESSIMISTIC_AFTER: usize = 4;

impl<D: Device> Db<D> {
    /// RUN `f` AS A TRANSACTION, RETRYING ON `Conflict` (the policy of es-p19 §2.1): up to
    /// `MAX_RETRIES` with a bounded exponential backoff of 1, 2, 4 ... 256 us; after
    /// `PESSIMISTIC_AFTER` conflicts the transaction holds the commit section for its whole run
    /// (it then reads the current generation and cannot conflict), so no transaction starves.
    /// Any other refusal, and the closure's own, come back at once.
    pub fn write<T>(&self, d: Durability, mut f: impl FnMut(&mut WriteTxn<'_, D>) -> Result<T, Refusal>) -> Result<T, Refusal> {
        let mut backoff_us = 1u64;
        for conflicts in 0..=MAX_RETRIES {
            if conflicts >= PESSIMISTIC_AFTER {
                let mut w = self.lock_writer()?;
                let mut txn = self.begin_write()?;
                let out = f(&mut txn)?;
                txn.finish_in(&mut w, d, Mode::Delta)?;
                return Ok(out);
            }
            let mut txn = self.begin_write()?;
            let out = f(&mut txn)?;
            match txn.commit(d) {
                Ok(_) => return Ok(out),
                Err(Refusal::Conflict { .. }) => {
                    std::thread::sleep(std::time::Duration::from_micros(backoff_us));
                    backoff_us = (backoff_us * 2).min(256);
                }
                Err(e) => return Err(e),
            }
        }
        unreachable!("the pessimistic run commits or refuses")
    }

}
