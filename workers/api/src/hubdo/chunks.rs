//! Which chunks of an image a write must send (moved out of `hubdo.rs` by W-ZC, unchanged).

/// A stored chunk comes back as whatever the platform decided to hand us —
/// `Uint8Array` or the `ArrayBuffer` behind one. Accept both rather than assume,
/// because assuming is a corrupt image reported a long way from here.
/// Which chunks of `new` differ from `old`, by index. Without an old image
/// every chunk is new. A chunk past the end of the old image is new. A chunk
/// is unchanged only when the STORED chunk has the same length as the new
/// slice and the same bytes: a stored chunk that is longer (the image shrank
/// and the new tail is a prefix of the old one) must be rewritten, or the
/// meta's `len` and the bytes on disk disagree on the next cold load.
pub(super) fn changed_chunks(old: Option<&[u8]>, new: &[u8], chunk: usize) -> Vec<usize> {
    let chunks = new.len().div_ceil(chunk).max(1);
    (0..chunks)
        .filter(|&n| {
            let at = n * chunk;
            let end = (at + chunk).min(new.len());
            match old {
                Some(o) if (at + chunk).min(o.len()) == end => o[at..end] != new[at..end],
                _ => true,
            }
        })
        .collect()
}


#[cfg(test)]
mod tests;
