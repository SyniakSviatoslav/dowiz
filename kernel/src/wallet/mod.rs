//! BLUEPRINT-P66 — offline data wallet + single-writer LWW drafts + Signal-style
//! QR wallet transfer (self-custody, no dowiz account, query-before-replay reconnect).
//!
//! This module is the on-device client-side wallet. It is PURE LOGIC (no tauri /
//! idb / reqwest) — the platform storage + QR-scan live OUT-OF-CORE behind the
//! `WalletStore` / `QrScanPort` ports (the P60 `payment-adapters` split, mirrored).
//!
//! GREP-GATES (structural, run in `cargo test --lib`):
//!   * [`no_card_data_in_wallet`]  — no PAN/CVV/expiry field may exist in this module (PCI red-line).
//!   * [`no_break_glass_in_wallet`] — no recovery/escrow/dowiz-recipient symbol (self-custody absolute, §4-B).
//!
//! Reuse-first (X6 / §2): the idempotency contract is P60's — [`record`]/[`draft`] re-export
//! `IdempotencyKey`/`PaymentStatus` from `crate::ports::payment_provider`; the transfer crypto
//! reuses `crate::pq::{x25519, keccak::shake256}` + `aes-gcm` (zero new crypto deps). Money is
//! `crate::money::Money` (integer minor units). NO card data is EVER stored — only a
//! [`PaymentMethodRef`] (opaque provider id, e.g. `pm_…`).
//!
//! NO CRDT — single-writer LWW is strictly correct (R4 §3.1). NO tombstones. NO break-glass.

pub mod draft;
pub mod outbox;
pub mod record;
/// `transfer` reuses the `pq` crypto primitives (x25519 / shake256 / aes-gcm) — compiled only
/// under that feature so the DEFAULT kernel build stays serde-free and crypto-light.
#[cfg(feature = "pq")]
pub mod transfer;

// ─────────────────────────────────────────────────────────────────────────────
// Structural grep-gates (the PCI + self-custody red-line teeth).
//
// The forbidden tokens are assembled via `concat!` so the gate source NEVER
// contains a contiguous forbidden literal — otherwise `!contains` would be a
// vacuous/always-false check. Mirrors `kernel/tests/no_card_data.rs` + P60 §4.1.
// ─────────────────────────────────────────────────────────────────────────────

/// Scan this module's own sources for card-data identifiers. Adding a `pan:` /
/// `cvv:` / `card_number:` field anywhere under `kernel/src/wallet` makes this
/// test FAIL — the no-PAN guarantee is enforced by construction plus this CI teeth.
#[cfg(test)]
mod firewall {
    const SELF_DIR: &str = "src/wallet";

    const FORBIDDEN_CARD_TOKENS: &[&str] = &[
        concat!("card_", "number"),
        concat!("card", "number"),
        concat!("card_", "holder"),
        concat!("card", "holder"),
        concat!("exp_", "month"),
        concat!("exp_", "year"),
        concat!("c", "vv"),
        concat!("c", "vc"),
        concat!("p", "an"),
    ];

    const FORBIDDEN_BREAK_GLASS_TOKENS: &[&str] = &[
        concat!("break_", "glass"),
        concat!("break", "glass"),
        concat!("es", "crow"),
        concat!("recovery_", "key"),
        concat!("dowiz_", "recipient"),
        concat!("backup_", "to_", "dowiz"),
    ];

    /// Collect every `.rs` file under `kernel/src/wallet`.
    fn wallet_src_files() -> Vec<String> {
        let mut out = Vec::new();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SELF_DIR);
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                        out.push(p.to_string_lossy().into_owned());
                    }
                }
            }
        }
        out
    }

    /// Strip Rust comments + string/char literals so doc prose naming a token is harmless.
    fn strip_comments(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let b = src.as_bytes();
        let mut i = 0;
        let mut in_block = false;
        while i < b.len() {
            if in_block {
                if i + 1 < b.len() && b[i] == b'*' && b[i + 1] == b'/' {
                    in_block = false;
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'/' {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            if i + 1 < b.len() && b[i] == b'/' && b[i + 1] == b'*' {
                in_block = true;
                i += 2;
                continue;
            }
            if b[i] == b'"' || b[i] == b'\'' {
                let q = b[i];
                out.push(' ');
                i += 1;
                while i < b.len() && b[i] != q {
                    if b[i] == b'\\' {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                i += 1;
                out.push(' ');
                continue;
            }
            out.push(b[i] as char);
            i += 1;
        }
        out
    }

    #[test]
    fn no_card_data_in_wallet() {
        let files = wallet_src_files();
        assert!(!files.is_empty(), "wallet src tree must be non-empty");
        let mut violations = Vec::new();
        for f in &files {
            let src = match std::fs::read_to_string(f) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let code = strip_comments(&src);
            let words: Vec<String> = code
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .map(|w| w.to_lowercase())
                .collect();
            for token in FORBIDDEN_CARD_TOKENS {
                let needle = token.to_lowercase();
                if words.iter().any(|w| w == &needle) {
                    violations.push(format!("{f}: card-data token '{token}'"));
                }
            }
        }
        assert!(violations.is_empty(), "P66 PCI red-line: {violations:?}");
    }

    #[test]
    fn no_break_glass_in_wallet() {
        let files = wallet_src_files();
        let mut violations = Vec::new();
        for f in &files {
            let src = match std::fs::read_to_string(f) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let code = strip_comments(&src);
            let words: Vec<String> = code
                .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                .map(|w| w.to_lowercase())
                .collect();
            for token in FORBIDDEN_BREAK_GLASS_TOKENS {
                let needle = token.to_lowercase();
                if words.iter().any(|w| w == &needle) {
                    violations.push(format!("{f}: break-glass token '{token}'"));
                }
            }
        }
        assert!(
            violations.is_empty(),
            "P66 self-custody red-line (no break-glass): {violations:?}"
        );
    }
}
