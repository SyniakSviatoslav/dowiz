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
