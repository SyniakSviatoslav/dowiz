//! What the order log refuses, and the one question every caller asks of a
//! refusal: is the image simply full?

use bebop_store::StoreError;

#[derive(Debug)]
pub enum HubError {
    Store(StoreError),
    /// The image is not a hub store, or carries no log root.
    NotAHub,
    /// An order id that is not in the log.
    UnknownOrder,
    /// An order id longer than the record header allows.
    OrderIdTooLong,
    /// A LOADED IMAGE DISAGREES WITH ITSELF: the chain does not deliver the
    /// records the root claims, or a record in it cannot be read back.
    ///
    /// This is the failure worth having an error for. An image cut short, or
    /// one byte of it changed in flight, used to LOAD -- and then answer
    /// `len() == 40` while handing back two events. A venue whose log silently
    /// drops its thirty-eight most recent orders has no way to notice, and
    /// every reply built on it is confidently wrong. Refusing is the only
    /// answer a caller can act on: it can re-fetch, restore, or alert.
    ///
    /// `chained` is `None` when the chain never ended -- a corrupted `next` ref
    /// can point backwards, and a reader that followed it would walk for ever.
    Corrupt { claimed: usize, chained: Option<usize> },
    /// AN OBJECT'S PAYLOAD DOES NOT MATCH THE CRC ITS HEADER CARRIES (W-CRC, 2026-10-05):
    /// one changed byte of a value or a record, which used to load and be served as the
    /// truth. Names the object (its cell index) and both numbers; see `bebop_store::verify`.
    BadCrc(bebop_store::BadCrc),
    /// THE ROOT NAMES A LOG VERSION THIS BUILD DOES NOT KNOW (W-MG-M1B): a DOC root
    /// (`bebop_store::evlog::doc`, root-version 5) under a loader that reads EvLog v1-v2.
    /// Refused BY NAME before the walk, as every reader of the format must (FORMAT §9: an
    /// unknown version is a refusal, never a guess) -- read as a log, a DOC's refs would have
    /// been walked as records and the answer would have been `Corrupt`, which names damage,
    /// not a newer file. `doc_seal::SealedHub` is the loader that knows a DOC.
    UnknownVersion(i64),
}

impl HubError {
    /// Is this "the image has no room left", and how much was wanted?
    ///
    /// Exposed as a method rather than left to the caller to pattern-match,
    /// because `bebop_store` is this crate's dependency and not everybody
    /// else's -- and a caller reduced to matching on a Debug string would be
    /// one rename away from silently losing the case.
    pub fn arena_full(&self) -> Option<(i64, i64)> {
        match self {
            HubError::Store(StoreError::ArenaFull { need, capacity }) => Some((*need, *capacity)),
            _ => None,
        }
    }
}

/// The library's refusals as this crate's errors (W-ES-P3), with NO new variant, so every
/// caller's match (workers/api) stands: a changed byte keeps its name (`BadCrc`), "this is not a
/// hub image" stays `NotAHub`, a full image stays `Store(ArenaFull)` (`arena_full`, `e_is_full`).
impl From<bebop_store::Refusal> for HubError {
    fn from(e: bebop_store::Refusal) -> Self {
        use bebop_store::sim::Why;
        use bebop_store::format::Anchor;
        use bebop_store::Refusal as R;
        match e {
            R::BadCrc(b) => HubError::BadCrc(b),
            // A torn last commit in an image whose writes are atomic is a changed byte: name the
            // cell where the proof failed and both numbers, as BadCrc does for an object.
            R::LastCommit(Why::Anchor(Anchor::Torn { mark, want, got, .. })) => {
                HubError::BadCrc(bebop_store::BadCrc { obj: mark.max(0) as usize, want, got })
            }
            R::LastCommit(Why::Walk { at }) | R::ObjectBad { at } => HubError::BadCrc(bebop_store::BadCrc { obj: at, want: 0, got: None }),
            R::LastCommit(_) | R::Pick(_) | R::Recover { .. } | R::NotKv | R::UnknownVersion(_) => HubError::NotAHub,
            R::ArenaFull { need, capacity } => HubError::Store(StoreError::ArenaFull { need, capacity }),
            R::Limit { what, got, max } => HubError::Store(StoreError::Limit { what, got, max }),
            R::KeyTooLong { len, max } => HubError::Store(StoreError::Limit { what: "key bytes", got: len as i64, max: max as i64 }),
            R::ValueTooLong { len, max } => HubError::Store(StoreError::Limit { what: "value bytes", got: len as i64, max: max as i64 }),
            R::TooSmall { bytes, floor } => HubError::Store(StoreError::TooSmall { bytes, floor }),
            R::Corrupt(what) | R::Plan(what) => HubError::Store(StoreError::Corrupt(what)),
            // Cannot happen over `Detached` (one owner, no device): named, never silent.
            R::WriterBusy => HubError::Store(StoreError::Corrupt("the image's writer is busy")),
            R::Poisoned => HubError::Store(StoreError::Corrupt("the image's writer is poisoned")),
            R::Locked { .. } => HubError::Store(StoreError::Corrupt("the image is locked by another process")),
            R::Exists => HubError::Store(StoreError::Corrupt("the image already holds a KV")),
            R::Io { .. } => HubError::Store(StoreError::Corrupt("the image's device failed")),
            R::Durability { .. } => HubError::Store(StoreError::Corrupt("the image's device cannot give that durability")),
            _ => HubError::Store(StoreError::Corrupt("the image was refused")),
        }
    }
}

impl From<StoreError> for HubError {
    fn from(e: StoreError) -> Self {
        HubError::Store(e)
    }
}

/// Is this "the image has no room left"? Matched through the public shape
/// rather than a Debug string, for the reason `HubError::arena_full` gives.
pub(crate) fn e_is_full(e: &StoreError) -> bool {
    matches!(e, StoreError::ArenaFull { .. })
}
