//! gen-vocab — blueprint P4: the clients' vocabulary, generated from the kernel.
//!
//! [`read`] asks `dowiz_core` (no source parsing anywhere in this crate);
//! [`emit`] turns the answers into `workers/api/public/lib/vocab.js`.
//! `tools/gates/vocab.sh` is the half that refuses a stale committed copy.

pub mod emit;
pub mod read;

#[cfg(test)]
mod tests;

/// The committed artefact's path, relative to the repository root. Named once
/// so the binary, the gate's usage line and the drift test cannot disagree.
pub const ARTEFACT: &str = "workers/api/public/lib/vocab.js";

/// Read the kernel and render the module, or say why not.
pub fn generate() -> Result<String, String> {
    read::read_kernel().map(|v| emit::render(&v))
}
