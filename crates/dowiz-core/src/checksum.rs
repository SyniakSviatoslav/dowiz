pub const CHECKSUM_MUL: u64 = 31;

/// FNV-1a-style folding checksum (pure, `core`-only) shared across the kernel.
/// Moved from `dowiz-kernel` root so no_std `dowiz-core` modules can hash payloads
/// without a std dependency.
pub fn checksum_fold(data: &[u8]) -> u64 {
    data.iter().fold(0u64, |acc, &b| acc.wrapping_mul(CHECKSUM_MUL).wrapping_add(b as u64))
}
