//! THE JOURNAL KEPT IN HAND BETWEEN CATALOGUE WRITES (W-LOOPB, R-LOOPS row 5).
//!
//! Every catalogue write re-did two things the write before it had just done: it re-read the
//! journal image (`LogImage::load` = copy + a crc of every record, ~0.65 ms at 522 records)
//! to append one record, and it decoded the catalogue it was replacing to take its `State`
//! (~0.6 ms at 165 dishes) -- the very `State` the previous write had decoded as its `after`.
//! Measured on the release bench: 2265 -> 925 us per write.
//!
//! So the write that LANDED leaves both behind here: the journal `LogImage` after its append
//! and the catalogue `State` after it, each tagged with the generation AND the length of the
//! image in `mem` it describes. The next write uses a copy only when `mem` still holds exactly
//! that image (generation and length both equal); anything else -- another generation, a
//! journal written by a route, an evicted `mem` entry, a shrink, a failed or partial write, a
//! write that journaled nothing -- leaves the desk EMPTY, and that write reads and decodes as
//! it always did. The copy is TAKEN at the start of every catalogue write and put back only by
//! `write_image` after its storage write returned Ok: an error can never leave one standing.
//! (memory: write-only-what-changed-makes-memory-load-bearing.)

use crate::hubstore::Edited;
use dowiz_hub::catalog::edits::State;
use dowiz_hub::logimage::LogImage;

/// The `edit` cell of the object: the signer of the write under way, and the resident copy.
#[derive(Default)]
pub(in crate::hubdo) struct Desk {
    /// Who signed the catalogue write under way (W-PITR2); never outlives the call.
    pub(in crate::hubdo) stamp: Option<Edited>,
    pub(super) resident: Option<Resident>,
    /// Tests: never use the resident copy (the twin that IS the old path).
    #[cfg(test)]
    pub(in crate::hubdo) off: bool,
    /// Tests: how many writes used it, journal and `before` counted apart.
    #[cfg(test)]
    pub(in crate::hubdo) hits: (usize, usize),
}

/// What the last landed catalogue write left: (generation, length) of each image it describes.
pub(super) struct Resident {
    pub(super) journal: (i64, usize),
    pub(super) log: LogImage,
    pub(super) catalogue: (i64, usize),
    pub(super) after: State,
}

impl Desk {
    /// The resident copy, taken out (the desk is empty until a landed write refills it).
    pub(super) fn take(&mut self) -> Option<Resident> {
        let r = self.resident.take();
        #[cfg(test)]
        if self.off {
            return None;
        }
        r
    }

    #[cfg(test)]
    pub(super) fn hit(&mut self, journal: bool, before: bool) {
        self.hits.0 += journal as usize;
        self.hits.1 += before as usize;
    }
}
