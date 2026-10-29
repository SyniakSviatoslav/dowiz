//! `Bytes`: a key or a value read out of an image (W-ES-P3, R-EMBED-STORE §3.2).
//!
//! BORROWED wherever the bytes lie contiguous in what the reader holds -- a v2/v3 blob or a delta
//! record in a byte image (`View`, the library's snapshot), or a write transaction's own batch --
//! and OWNED only where they do not: a v1 base keeps ONE BYTE PER CELL, so its values have to be
//! gathered. Either way a read allocates nothing beyond the value itself (AC-ES-51). A newtype
//! rather than a bare `Cow`, so the backing can change (P14's mapped file) without the API.

use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;
use core::ops::Deref;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes<'t>(Cow<'t, [u8]>);

impl<'t> Bytes<'t> {
    /// Bytes that stay where they are.
    pub fn borrowed(b: &'t [u8]) -> Bytes<'t> {
        Bytes(Cow::Borrowed(b))
    }
    /// Bytes that had to be gathered.
    pub fn owned(v: Vec<u8>) -> Bytes<'t> {
        Bytes(Cow::Owned(v))
    }
    /// Whether these bytes are read in place (T-ES-51 asserts it for every v2/v3 read).
    pub fn is_borrowed(&self) -> bool {
        matches!(self.0, Cow::Borrowed(_))
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn into_owned(self) -> Vec<u8> {
        self.0.into_owned()
    }
}

impl<'t> From<Cow<'t, [u8]>> for Bytes<'t> {
    fn from(c: Cow<'t, [u8]>) -> Bytes<'t> {
        Bytes(c)
    }
}

impl Deref for Bytes<'_> {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for Bytes<'_> {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<[u8]> for Bytes<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        *self.0 == *other
    }
}

impl PartialEq<&[u8]> for Bytes<'_> {
    fn eq(&self, other: &&[u8]) -> bool {
        *self.0 == **other
    }
}

impl fmt::Debug for Bytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"{}\"", self.0.escape_ascii())
    }
}
