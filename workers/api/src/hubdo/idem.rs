//! THE PLACEMENT MARKS ITS CLAIM COMMITTED, in the turn that wrote the order
//! (W-FIX O2; the rule is `idempotency::commit`). One write to the `idem`
//! image, right after the log's. LOUD, NEVER FATAL: the order is in the log
//! whatever this does, and a mark that did not land is the behaviour before
//! it existed -- a retry that may run again -- said out loud.

use super::HubImages;
use crate::idempotency::{commit, IDEMPOTENCY_BYTES, IMAGE_IDEMPOTENCY};
use dowiz_hub::table::Table;
use worker::*;

impl HubImages {
    pub(super) async fn commit_claim(&self, claim: Option<&commit::Claim>, output: &str) {
        let Some(claim) = claim else { return };
        if let Err(e) = self.try_commit(claim, output).await {
            log_error!("idempotency: a command ran and its claim was NOT marked committed: {e}");
        }
    }

    async fn try_commit(&self, claim: &commit::Claim, output: &str) -> Result<()> {
        let (generation, loaded) = match self.image(IMAGE_IDEMPOTENCY).await? {
            Some((meta, bytes)) => (meta.generation, Table::load(&bytes, IDEMPOTENCY_BYTES)),
            None => (0, Table::create(IDEMPOTENCY_BYTES)),
        };
        let mut t = loaded.map_err(|e| Error::RustError(format!("idem image: {e:?}")))?;
        if !commit::commit(&mut t, claim, output)? {
            return Ok(());
        }
        let bytes = t.to_bytes().map_err(|e| Error::RustError(format!("idem image: {e:?}")))?;
        match self.put_image(IMAGE_IDEMPOTENCY, generation, &bytes).await? {
            Some(_) => Ok(()),
            None => Err(Error::RustError("the idem image's generation moved".into())),
        }
    }
}
