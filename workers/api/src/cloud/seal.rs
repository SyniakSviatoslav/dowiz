//! Sealing the nightly off-site copy to the platform's PUBLIC key.
//!
//! The bundle a venue's bucket receives is, after this, readable only by the
//! holder of the platform's backup SECRET key — which never exists in the
//! Worker. The Worker holds `BACKUP_SEAL_PK` (a Worker var or secret, text
//! `dwzseal-pk1:<hex>`, written by `tools/seal-open keygen`) and seals with
//! `dowiz_core::pq::backup_seal`: hybrid X25519 + ML-KEM-768 (both legs
//! mandatory, a degraded recipient key refused), AES-256-GCM, the byte layout
//! documented in that module's header. `tools/seal-open open` reverses it.
//!
//! THREE STATES, AND EACH ONE IS SAID OUT LOUD:
//!   * `Off`      — the var is absent: the copy goes up as plain gzip, as it
//!                  always did, and the push answer, the status route and the
//!                  error log all say `sealed: false`.
//!   * `On`       — the var parsed and passed both leg checks: the copy is
//!                  sealed and its key ends `.sealed`.
//!   * `Refused`  — the var is set but malformed: the push is REFUSED. The
//!                  operator asked for sealed copies; uploading the plaintext
//!                  instead would break that silently, and sealing to a key
//!                  nobody holds would write a file nobody can open.
//!
//! The witness (census of the chain tip) is NOT sealed: it is computed over the
//! plaintext log and uploaded beside the copy as before, so an audit can match
//! it against the image after opening.

use dowiz_core::pq::backup_seal::{self, SealEntropy, SealPublic};
use serde_json::{json, Value};

/// The Worker var (or secret) holding the recipient public key.
pub const PK_VAR: &str = "BACKUP_SEAL_PK";
/// Appended to the object key of a sealed copy: `<stamp>.json.gz.sealed`.
pub const SUFFIX: &str = ".sealed";
/// Header `kind` byte: what the sealed plaintext is.
pub const KIND_JSON: u8 = 0;
pub const KIND_GZIP: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealState {
    Off,
    On(SealPublic),
    Refused(String),
}

/// PURE: the state a given var value means.
pub fn state_from(text: Option<&str>) -> SealState {
    match text.map(str::trim) {
        None | Some("") => SealState::Off,
        Some(t) => match SealPublic::parse(t) {
            Ok(pk) => SealState::On(pk),
            Err(e) => SealState::Refused(format!("{PK_VAR} is set but refused: {e:?}")),
        },
    }
}

/// Read the var (a plain var first, then a secret of the same name).
pub fn state(env: &worker::Env) -> SealState {
    let text = env
        .var(PK_VAR)
        .map(|v| v.to_string())
        .or_else(|_| env.secret(PK_VAR).map(|v| v.to_string()))
        .ok();
    state_from(text.as_deref())
}

/// What health and the status route show.
pub fn describe(s: &SealState) -> Value {
    match s {
        SealState::Off => json!({ "sealed": false, "why": format!("{PK_VAR} is not set: copies go up as plain gzip") }),
        SealState::On(_) => json!({ "sealed": true, "scheme": "X25519+ML-KEM-768 / AES-256-GCM, dwzseal v1" }),
        SealState::Refused(e) => json!({ "sealed": false, "refused": e }),
    }
}

/// Per-seal entropy from the platform RNG (`crypto.getRandomValues` on wasm32
/// via getrandom's `js` feature). A failure refuses the seal — never zeros.
fn entropy() -> Result<SealEntropy, String> {
    let mut buf = [0u8; 76];
    getrandom::getrandom(&mut buf).map_err(|e| format!("no entropy for the seal: {e}"))?;
    Ok(SealEntropy {
        m: buf[..32].try_into().unwrap(),
        eph: buf[32..64].try_into().unwrap(),
        nonce: buf[64..].try_into().unwrap(),
    })
}

/// PURE: seal a body with the given entropy.
pub fn seal_with(pk: &SealPublic, gzipped: bool, body: &[u8], e: &SealEntropy) -> Result<Vec<u8>, String> {
    let kind = if gzipped { KIND_GZIP } else { KIND_JSON };
    backup_seal::seal(pk, kind, body, e).map_err(|e| format!("seal refused: {e:?}"))
}

/// Apply the state to one body: `(bytes to upload, sealed?)`, or a refusal.
pub fn apply(s: &SealState, gzipped: bool, body: Vec<u8>) -> Result<(Vec<u8>, bool), String> {
    match s {
        SealState::Off => Ok((body, false)),
        SealState::On(pk) => Ok((seal_with(pk, gzipped, &body, &entropy()?)?, true)),
        SealState::Refused(e) => Err(e.clone()),
    }
}

#[cfg(test)]
mod tests;
