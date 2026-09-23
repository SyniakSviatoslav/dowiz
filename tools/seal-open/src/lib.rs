//! seal-open — the native half of the sealed nightly backup.
//!
//! `keygen <secret-key-file>` draws 64 bytes from `/dev/urandom`, writes the
//! secret-key file (mode 0600, refusing to overwrite) and prints the public key
//! text for the Worker var `BACKUP_SEAL_PK`.
//! `pubkey <secret-key-file>` prints that text again.
//! `open <secret-key-file> <sealed-file> <out-file>` writes the plaintext (the
//! `.json.gz` or `.json` bundle the venue would have got unsealed), refusing to
//! overwrite, and writes NOTHING when the file does not open.
//!
//! Format and crypto: `dowiz_core::pq::backup_seal` (layout in its header).

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use dowiz_core::pq::backup_seal::{self, keygen, secret_from_file};

/// Write `bytes` to a NEW file (never overwrite) with the given unix mode.
fn write_new(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    f.write_all(bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Create a keypair from 64 seed bytes; write the secret file; return the public key text.
pub fn keygen_to(sk_path: &Path, seed: &[u8; 64]) -> Result<String, String> {
    let (file, pk) = keygen(seed[..32].try_into().unwrap(), seed[32..].try_into().unwrap());
    write_new(sk_path, &file, 0o600)?;
    Ok(pk.encode())
}

/// 64 bytes from the OS. A short read is refused, never padded.
pub fn os_seed() -> Result<[u8; 64], String> {
    let mut seed = [0u8; 64];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut seed))
        .map_err(|e| format!("/dev/urandom: {e}"))?;
    Ok(seed)
}

/// The public key text for an existing secret-key file.
pub fn pubkey_of(sk_path: &Path) -> Result<String, String> {
    let file = fs::read(sk_path).map_err(|e| format!("{}: {e}", sk_path.display()))?;
    let kp = secret_from_file(&file).map_err(|e| format!("{}: {e:?}", sk_path.display()))?;
    Ok(backup_seal::SealPublic { x_pk: kp.x_pk, kem_pk: kp.kem_pk }.encode())
}

/// Open a sealed file to `out`. Returns the header's content kind (0 json, 1 gzip).
pub fn open_file(sk_path: &Path, sealed_path: &Path, out: &Path) -> Result<u8, String> {
    let file = fs::read(sk_path).map_err(|e| format!("{}: {e}", sk_path.display()))?;
    let kp = secret_from_file(&file).map_err(|e| format!("{}: {e:?}", sk_path.display()))?;
    let sealed = fs::read(sealed_path).map_err(|e| format!("{}: {e}", sealed_path.display()))?;
    let (kind, plain) =
        backup_seal::open(&kp, &sealed).map_err(|e| format!("{}: refused: {e:?}", sealed_path.display()))?;
    write_new(out, &plain, 0o600)?;
    Ok(kind)
}

#[cfg(test)]
mod tests;
