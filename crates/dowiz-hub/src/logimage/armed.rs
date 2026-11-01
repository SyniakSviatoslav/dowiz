//! THE ARMED STATE TRAVELS WITH THE LOG (W-MG-TRANS): the doors that replace a log's image.

use super::LogImage;
use crate::logimage::DEFAULT_LOG_BYTES;
use crate::HubError;

impl LogImage {
    /// AN EMPTY LOG THAT REPLACES THIS ONE, armed as this one is (`Entropy::forward`): the only
    /// way to start a rebuild. `create` here would come back on the crate's defaults.
    pub fn fresh_sized(&self, bytes: usize) -> Result<Self, HubError> {
        let mut out = Self::create_sized(bytes)?;
        out.entropy = self.entropy.forward(&self.store);
        Ok(out)
    }

    /// `fresh_sized` at the default size.
    pub fn fresh(&self) -> Result<Self, HubError> {
        self.fresh_sized(DEFAULT_LOG_BYTES)
    }

    /// Take `bytes` as this log's image, keeping its armed state: a reload never resets it.
    pub(crate) fn reload(&mut self, bytes: &[u8]) -> Result<(), HubError> {
        let mut out = Self::load(bytes)?;
        out.entropy = self.entropy.forward(&self.store);
        *self = out;
        Ok(())
    }
}
