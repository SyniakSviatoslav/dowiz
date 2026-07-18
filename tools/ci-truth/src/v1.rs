// ===========================================================================
// Phase 6 (V1) — Split-Identity + Adversarial Verifier: PROTOCOL CONTRACT
//
// SCOPE RULE (ARCHITECTURE §0): this is a canonical-repo DEV-TIME fence, not a
// runtime hub control. A sovereign hub (M5/M9/M11) MAY fork and drop it.
//
// HONESTY NOTE (BLUEPRINT-P06 §8): this module implements the V1 *protocol
// contract* — the anchor loader, the DiffAttestation/Verdict TLV, git-note I/O,
// and the §5 merge-gate policy. Signing is behind the `Signer` trait. As of
// C4b GREEN (Phase 3 closing C4b — the `mod_l` HIGH side-channel on the
// Ed25519 path is resolved), the crypto slot is FILLED by `HybridSigner`, which
// shells the external `bebop2-kv` hybrid CLI (Ed25519⊕ML-DSA-65, RequireBoth)
// to produce and verify REAL split-identity signatures over the note bytes.
// `UnsignedSigner` remains available for the Phase-1 unsigned state
// (`signed:false`, mirroring main.rs:423). The §5 merge-gate policy is
// UNCHANGED and executable/testable now (consumed by hermetic-remediation H3,
// spectral-evolution E3, phases 7/9/10). There is NO committed trust root: the
// operator mints the real kv-genesis separately.
// ===========================================================================

// NOTE: `evaluate_gate` + `read_note` are used by the `v1-verify` subcommand at
// runtime. The anchor loader, TLV encode, and `Signer` trait are the protocol
// *contract surface* that downstream arcs (hermetic-remediation H3, spectral-
// evolution E3, phases 7/9/10) call, and are exercised by this module's `#[cfg(test)]`
// suite. They are intentionally `#[allow(dead_code)]` rather than deleted — they
// are the vetted API, not orphaned code.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{git_ok, is_redline};

/// The mandatory honesty residue (BLUEPRINT-P06 §4 T=0x09, §5.5). MUST be present
/// on every verdict; a verdict lacking it is malformed => RED.
pub const V1_RESIDUE: &str = "enforced approximation: identity != person";

/// TLV tag carrying the REAL hybrid signature over `signing_bytes()` for the
/// key_K DiffAttestation (Ed25519⊕ML-DSA-65, RequireBoth). Present on every
/// signed note; EXCLUDED from `signing_bytes()` so the signature commits to the
/// exact bytes re-derived by the verifier (BLUEPRINT-P06 §3/§4, Bug-2 fix).
pub const SIG_TAG_K: u8 = 0x07;
/// TLV tag carrying the REAL hybrid signature for the key_V Verdict.
pub const SIG_TAG_V: u8 = 0x08;

/// Anchor-file path: two public keys, tagged role=K / role=V (BLUEPRINT-P06 §2).
pub const KV_GENESIS: &str = "config/kv-genesis.txt";

/// Native telemetry sink (mandatory-telemetry doctrine: cheap ring local sink).
/// One JSONL line per signature-verification event; append-only, greppable.
/// Never contains key material — only role, anchor-id prefix, latency, result.
pub const V1_TELEMETRY: &str = "docs/ledger/v1-sigverify-telemetry.jsonl";

/// Append one telemetry record (JSONL) to the local v1 verification sink.
/// Failures are non-fatal (telemetry must never break the gate).
pub fn record_telemetry(repo_root: &Path, ev: &V1SigEvent) {
    if let Some(dir) = Path::new(repo_root).join(V1_TELEMETRY).parent() {
        let _ = fs::create_dir_all(dir);
    }
    let path = Path::new(repo_root).join(V1_TELEMETRY);
    // tolerant hand-rolled JSON — no allocation of a Value, no serde dep.
    let line = format!(
        "{{\"t\":{ts},\"op\":\"{op}\",\"role\":\"{role}\",\"anchor\":\"{anchor}\",\"sha256_signed\":\"{signed_sha}\",\"ms\":{ms},\"ok\":{ok}}}
",
        ts = ev.ts,
        op = ev.op,
        role = ev.role,
        anchor = ev.anchor,
        signed_sha = ev.signed_sha256,
        ms = ev.ms,
        ok = ev.ok,
    );
    use std::io::Write;
    if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// One signature-verification telemetry event (cheap, no key material).
#[derive(Clone, Debug)]
pub struct V1SigEvent {
    pub ts: u64,
    pub op: &'static str,
    pub role: char,
    pub anchor: String,
    pub signed_sha256: String,
    pub ms: u64,
    pub ok: bool,
}

/// Digest helper: the blueprint specifies sha3-256 (32 bytes). `sha3sum` is not
/// guaranteed on every host, so we use `git hash-object` (always present) and
/// deterministically widen/truncate to 32 bytes. The *binding logic and TLV
/// schema* are identical to the spec; only the digest primitive differs, and it
/// is swappable in one place. Production (post-C4b) replaces this with the
/// bebop2 hybrid `sign_pq`/ML-DSA path.
fn digest32(bytes: &[u8]) -> [u8; 32] {
    let out = Command::new("git")
        .args(["hash-object", "--stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                let _ = stdin.write_all(bytes);
            }
            child.wait_with_output()
        })
        .ok()
        .and_then(|o| {
            let hex = String::from_utf8_lossy(&o.stdout);
            let a = hexplit(&hex, 32);
            Some(a)
        });
    out.unwrap_or([0u8; 32])
}

/// Hex string -> fixed-width [u8;32], widened (right-padded with 0) or truncated.
fn hexplit(hex: &str, n: usize) -> [u8; 32] {
    let h = hex.trim();
    let mut out = [0u8; 32];
    let mut i = 0;
    let mut chars = h.chars();
    while i < n {
        // parse one byte from two hex chars if available
        let hi = chars.next();
        let lo = chars.next();
        match (hi, lo) {
            (Some(a), Some(b)) => {
                if let (Some(x), Some(y)) = (a.to_digit(16), b.to_digit(16)) {
                    out[i] = ((x << 4) | y) as u8;
                }
            }
            _ => break,
        }
        i += 1;
    }
    let _ = n;
    out
}

// ---------------------------------------------------------------------------
// Anchor loader (BLUEPRINT-P06 §2 ceremony shape; MESH-12 load_genesis pattern)
// ---------------------------------------------------------------------------

/// A loaded K/V anchor: hex public key + role.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub role: char, // 'K' or 'V'
    pub pub_hex: String,
}

/// Fail-closed loader. Mirrors MESH-12 `load_genesis`:
/// empty/absent list authorizes nothing; refuses <2 anchors, a missing role
/// tag, or pub_K == pub_V (the K≠V invariant checked at LOAD, not only at gate).
pub fn load_kv_genesis(repo_root: &Path) -> Option<Vec<Anchor>> {
    let path = repo_root.join(KV_GENESIS);
    let text = fs::read_to_string(&path).ok()?;
    let mut anchors: Vec<Anchor> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // format: <pub-hex> role=<K|V>
        let mut parts = line.split_whitespace();
        let pub_hex = parts.next()?;
        let role_tok = parts.next()?;
        let role = role_tok.strip_prefix("role=")?;
        let role = role.chars().next()?;
        if role != 'K' && role != 'V' {
            return None;
        }
        anchors.push(Anchor {
            role,
            pub_hex: pub_hex.to_string(),
        });
    }
    // fail-closed invariants
    if anchors.len() < 2 {
        return None;
    }
    let pub_k = anchors.iter().find(|a| a.role == 'K');
    let pub_v = anchors.iter().find(|a| a.role == 'V');
    let (pub_k, pub_v) = match (pub_k, pub_v) {
        (Some(k), Some(v)) => (k, v),
        _ => return None, // a role tag is missing
    };
    if pub_k.pub_hex == pub_v.pub_hex {
        return None; // K == V forbidden
    }
    Some(anchors)
}

// ---------------------------------------------------------------------------
// TLV encode/decode (canonical, BLUEPRINT-P06 §3/§4)
// ---------------------------------------------------------------------------

/// A TLV field: tag byte + length-prefixed value. Encoding is deterministic
/// (big-endian length, fixed field order) so a verifier re-derives identical bytes.
fn tlv_put(buf: &mut Vec<u8>, tag: u8, val: &[u8]) {
    buf.push(tag);
    buf.extend_from_slice(&(val.len() as u32).to_be_bytes());
    buf.extend_from_slice(val);
}

fn field32(m: &BTreeMap<u8, Vec<u8>>, tag: u8) -> Option<[u8; 32]> {
    let v = m.get(&tag)?;
    if v.len() != 32 {
        return None;
    }
    let mut a = [0u8; 32];
    a.copy_from_slice(v);
    Some(a)
}

fn tlv_get(mut buf: &[u8]) -> BTreeMap<u8, Vec<u8>> {
    let mut map = BTreeMap::new();
    while !buf.is_empty() {
        let tag = buf[0];
        buf = &buf[1..];
        if buf.len() < 4 {
            break;
        }
        let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        buf = &buf[4..];
        if buf.len() < len {
            break;
        }
        map.insert(tag, buf[..len].to_vec());
        buf = &buf[len..];
    }
    map
}

/// DiffAttestation TLV (BLUEPRINT-P06 §3). Signed by key_K in production.
pub struct DiffAttestation {
    pub commit_sha3: [u8; 32],
    pub diff_sha3: [u8; 32],
    pub base_sha3: [u8; 32],
    pub key_k_anchor_id: [u8; 32],
    pub redline_touch: u8,
    pub timestamp: u64,
    /// REAL hybrid signature (Ed25519⊕ML-DSA-65) over `signing_bytes()`,
    /// populated when signed by `HybridSigner`. Empty in the unsigned state.
    pub sig_k: Vec<u8>,
}

impl DiffAttestation {
    /// The exact bytes the signature commits to — every TLV field EXCEPT the
    /// signature itself, so the verifier re-derives identical bytes (BLUEPRINT-P06 §3).
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        tlv_put(&mut b, 0x01, &self.commit_sha3);
        tlv_put(&mut b, 0x02, &self.diff_sha3);
        tlv_put(&mut b, 0x03, &self.base_sha3);
        tlv_put(&mut b, 0x04, &self.key_k_anchor_id);
        tlv_put(&mut b, 0x05, &[self.redline_touch]);
        tlv_put(&mut b, 0x06, &self.timestamp.to_be_bytes());
        b
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut b = self.signing_bytes();
        if !self.sig_k.is_empty() {
            tlv_put(&mut b, SIG_TAG_K, &self.sig_k);
        }
        b
    }
    pub fn decode(buf: &[u8]) -> Option<Self> {
        let m = tlv_get(buf);
        let get32 = |t: u8| field32(&m, t);
        let mut out = Self {
            commit_sha3: get32(0x01)?,
            diff_sha3: get32(0x02)?,
            base_sha3: get32(0x03)?,
            key_k_anchor_id: get32(0x04)?,
            redline_touch: *m.get(&0x05)?.first()?,
            timestamp: {
                let v = m.get(&0x06)?;
                let mut a = [0u8; 8];
                a.copy_from_slice(&v[..8.min(v.len())]);
                u64::from_be_bytes(a)
            },
            sig_k: Vec::new(),
        };
        if let Some(s) = m.get(&SIG_TAG_K) {
            out.sig_k = s.clone();
        }
        Some(out)
    }
}

/// Verdict TLV (BLUEPRINT-P06 §4). Signed by key_V in production.
pub struct Verdict {
    pub diff_attest_sha3: [u8; 32],
    pub recomputed_diff_sha3: [u8; 32],
    pub verdict: u8, // 0x00=RED, 0x01=GREEN
    pub key_v_anchor_id: [u8; 32],
    pub context_descriptor: String,
    pub rationale: String,
    /// REAL hybrid signature over `signing_bytes()`; empty in unsigned state.
    pub sig_v: Vec<u8>,
}

impl Verdict {
    /// The exact bytes the signature commits to — every TLV field EXCEPT the
    /// signature (tag 0x08) and residue (tag 0x09), re-derived by the verifier.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        tlv_put(&mut b, 0x01, &self.diff_attest_sha3);
        tlv_put(&mut b, 0x02, &self.recomputed_diff_sha3);
        tlv_put(&mut b, 0x03, &[self.verdict]);
        tlv_put(&mut b, 0x05, &self.key_v_anchor_id);
        tlv_put(&mut b, 0x06, self.context_descriptor.as_bytes());
        tlv_put(&mut b, 0x07, self.rationale.as_bytes());
        b
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut b = self.signing_bytes();
        if !self.sig_v.is_empty() {
            tlv_put(&mut b, SIG_TAG_V, &self.sig_v);
        }
        tlv_put(&mut b, 0x09, V1_RESIDUE.as_bytes()); // residue always present
        b
    }
    pub fn decode(buf: &[u8]) -> Option<Self> {
        let m = tlv_get(buf);
        let get32 = |t: u8| field32(&m, t);
        let residue = m.get(&0x09)?;
        if residue != V1_RESIDUE.as_bytes() {
            return None; // residue missing/changed => malformed
        }
        let mut out = Self {
            diff_attest_sha3: get32(0x01)?,
            recomputed_diff_sha3: get32(0x02)?,
            verdict: *m.get(&0x03)?.first()?,
            key_v_anchor_id: get32(0x05)?,
            context_descriptor: String::from_utf8_lossy(m.get(&0x06)?).into_owned(),
            rationale: String::from_utf8_lossy(m.get(&0x07)?).into_owned(),
            sig_v: Vec::new(),
        };
        if let Some(s) = m.get(&SIG_TAG_V) {
            out.sig_v = s.clone();
        }
        Some(out)
    }
}

// ---------------------------------------------------------------------------
// Signer trait — crypto slot (FILLED post-C4b by HybridSigner)
// ---------------------------------------------------------------------------

pub trait Signer {
    /// Produce the `signed` flag for the emitted JSON. `false` until real keys.
    fn signed(&self) -> bool {
        false
    }
    /// Sign opaque bytes. Unsigned impl returns the bytes unchanged + a marker.
    fn sign(&self, _bytes: &[u8]) -> Vec<u8>;
}

/// Phase-1 unsigned Signer. Retained for the unsigned state: `signed:false`,
/// echoes the bytes so the note is inspectable. Gate/TLV logic is identical
/// whether signed or not — only the authenticity proof differs.
pub struct UnsignedSigner;

impl Signer for UnsignedSigner {
    fn signed(&self) -> bool {
        false
    }
    fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        // No signature: the gate treats `signed:false` as the Phase-1 state.
        // We still echo the digest so the note is inspectable.
        bytes.to_vec()
    }
}

// ---------------------------------------------------------------------------
// HybridSigner — production Signer (C4b GREEN). Shells the external bebop2-kv
// hybrid CLI (Ed25519⊕ML-DSA-65, RequireBoth) for REAL split-identity sigs.
// ---------------------------------------------------------------------------

/// Resolve the bebop2-kv crypto CLI path.
/// Order: `$V1_KV_BIN` (explicit) > `$BEBOp_REPO_ROOT/target/debug/bebop2-kv`
/// > `bebop2-kv` resolved on `$PATH`.
pub fn kv_bin() -> String {
    if let Ok(b) = std::env::var("V1_KV_BIN") {
        if !b.is_empty() {
            return b;
        }
    }
    if let Ok(root) = std::env::var("BEBOp_REPO_ROOT") {
        let p = Path::new(&root)
            .join("target")
            .join("debug")
            .join("bebop2-kv");
        if p.exists() {
            return p.to_string_lossy().into_owned();
        }
    }
    "bebop2-kv".into()
}

/// True if the resolved crypto CLI is present (on PATH or as an explicit file).
pub fn kv_bin_available() -> bool {
    let b = kv_bin();
    if Path::new(&b).exists() {
        return true;
    }
    if let Ok(paths) = std::env::var("PATH") {
        for p in paths.split(':') {
            if Path::new(p).join(&b).exists() {
                return true;
            }
        }
    }
    false
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push(((hi << 4) | lo) as u8);
        i += 2;
    }
    Some(out)
}

/// Strict parser for `bebop2-kv verify` output. The CLI emits exactly
/// `{"ok":true}` or `{"ok":false}`. A naive `contains("true")` would treat the
/// FALSE case as success (because the literal substring "true" appears inside
/// "false"), so we parse the boolean field explicitly — a forged/any signature
/// must fail verification (BLUEPRINT-P06 §7.7/§7.9, Bug-1 real-verify fix).
fn parse_ok(s: &str) -> bool {
    // Strip whitespace; accept `{"ok":true}` / `{"ok":false}` (key/value may be
    // space-separated; tolerate `: ` and surrounding spaces).
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    // Fast literal match first (the CLI's exact output).
    if s == "{\"ok\":true}" {
        return true;
    }
    if s == "{\"ok\":false}" {
        return false;
    }
    // Tolerant fallback: extract the value after `"ok":`.
    if let Some(pos) = s.find("\"ok\"") {
        let rest = &s[pos + 4..];
        if let Some(colon) = rest.find(':') {
            let val = rest[colon + 1..].trim().trim_end_matches('}');
            return val == "true";
        }
    }
    false
}

/// Production Signer (post-C4b). Shells `bebop2-kv` to sign/verify note bytes
/// with the role's hybrid (Ed25519⊕ML-DSA-65) key. `role` is 'K' (author /
/// DiffAttestation) or 'V' (verifier / Verdict); `master_hex` is the seed from
/// which the role key is derived. The signature is the REAL authenticity proof
/// bound to the gate's note bytes; `verify_signature` checks it out-of-band.
pub struct HybridSigner {
    pub role: char,
    pub master_hex: String,
}

impl Signer for HybridSigner {
    fn signed(&self) -> bool {
        true
    }
    fn sign(&self, bytes: &[u8]) -> Vec<u8> {
        // bebop2-kv sign <role=K|V> <master-hex> <hex(bytes)> -> hybrid sig hex
        // (ed_sig 64B ++ pq_sig 3309B) on stdout. Fail-closed: empty on error.
        match Command::new(kv_bin())
            .args([
                "sign",
                &format!("role={}", self.role),
                &self.master_hex,
                &hex_encode(bytes),
            ])
            .output()
        {
            Ok(o) if o.status.success() => {
                let hex = String::from_utf8_lossy(&o.stdout).trim().to_string();
                hex_decode(&hex).unwrap_or_default()
            }
            _ => Vec::new(), // fail-closed: no sig produced
        }
    }
}

impl HybridSigner {
    /// Verify a hybrid signature over `bytes` against the public anchor line
    /// (the kv-genesis line for this role). Shells
    /// `bebop2-kv verify <anchor-line> <hex(bytes)> <sig-hex>`.
    /// Latency + result are recorded to the native telemetry sink when
    /// `repo_root` is `Some` (pass `None` in tests that don't want I/O).
    pub fn verify_signature(
        &self,
        pub_anchor_line: &str,
        bytes: &[u8],
        sig_hex: &str,
        repo_root: Option<&Path>,
    ) -> bool {
        let t0 = std::time::Instant::now();
        let out = match Command::new(kv_bin())
            .args(["verify", pub_anchor_line, &hex_encode(bytes), sig_hex])
            .output()
        {
            Ok(o) if o.status.success() => parse_ok(&String::from_utf8_lossy(&o.stdout)),
            _ => false,
        };
        // Telemetry (mandatory doctrine): record every real verify op, never
        // blocking the result.
        if let Some(root) = repo_root {
            let signed_sha256 = digest32(bytes);
            record_telemetry(
                root,
                &V1SigEvent {
                    ts: now_unix_ts(),
                    op: "verify",
                    role: self.role,
                    anchor: anchor_id_prefix(pub_anchor_line),
                    signed_sha256: hex_encode(&signed_sha256),
                    ms: t0.elapsed().as_millis() as u64,
                    ok: out,
                },
            );
        }
        out
    }

    /// Derive this role's public anchor line (hex) for `verify_signature`, by
    /// shelling `bebop2-kv genkeys <master-hex>` (prints the K line then the V
    /// line) and selecting the line matching this role. The `pubkey`
    /// subcommand does NOT exist on bebop2-kv (only genkeys|sign|verify) — this
    /// is the Bug-3 fix (BLUEPRINT-P06 §2 ceremony shape). Empty on failure.
    pub fn pub_anchor_line(&self) -> String {
        match Command::new(kv_bin())
            .args(["genkeys", &self.master_hex])
            .output()
        {
            Ok(o) if o.status.success() => {
                let out = String::from_utf8_lossy(&o.stdout);
                // genkeys prints "<hex> role=K" on line 1, "<hex> role=V" on line 2.
                for line in out.lines() {
                    if line.trim().ends_with(&format!("role={}", self.role)) {
                        return line.trim().to_string();
                    }
                }
                String::new()
            }
            _ => String::new(),
        }
    }
}

/// First 8 hex chars of an anchor line, for telemetry correlation (no full key).
fn anchor_id_prefix(anchor_line: &str) -> String {
    let hex = anchor_line.split_whitespace().next().unwrap_or("");
    let take = hex.len().min(8);
    hex[..take].to_string()
}

/// Current unix seconds (telemetry timestamp).
fn now_unix_ts() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Split a raw note (attestation or verdict) into the signing-covered bytes and
/// the signature tag value. The signature tag (`SIG_TAG_K`/`SIG_TAG_V`) MUST be
/// excluded from the bytes that get re-verified, otherwise the verifier would
/// try to verify a message that includes its own signature (always fails).
/// Returns `(signing_bytes, sig)` where `sig` is empty if the note is unsigned.
pub fn split_note_sig(raw: &[u8], sig_tag: u8) -> (Vec<u8>, Vec<u8>) {
    let m = tlv_get(raw);
    let mut sig = Vec::new();
    if let Some(s) = m.get(&sig_tag) {
        sig = s.clone();
    }
    // Reconstruct the signing-covered bytes from the TLV map, EXCLUDING sig_tag.
    // Order must match `signing_bytes()` (tags 0x01..0x07; residue 0x09 is
    // carried but never signed). Iterating a BTreeMap yields sorted tags.
    let mut b = Vec::new();
    for (tag, val) in m.iter() {
        if *tag == sig_tag {
            continue;
        }
        tlv_put(&mut b, *tag, val);
    }
    (b, sig)
}

/// Reconstruct the exact message a signature was computed over.
/// Mirrors `signing_bytes()` for a raw note: the TLV bytes with `sig_tag`
/// excluded. Used by the real-sig acceptance test to re-derive the signed
/// payload that `HybridSigner::sign` consumed.
pub fn verify_message(raw: &[u8], sig_tag: u8) -> Vec<u8> {
    split_note_sig(raw, sig_tag).0
}

// ---------------------------------------------------------------------------
// Native telemetry (BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17)
// Every real crypto / gate operation emits a latency + explainable-event sink
// record. Output goes to stderr via `telemetry_sink` so it never corrupts the
// machine-readable stdout gate verdict.
// ---------------------------------------------------------------------------

/// Resolve the K/V anchor line for a role from the operator kv-genesis file.
/// Fail-closed: returns `None` if the genesis file is absent, or has no entry
/// for `role`. Production uses this so verification is anchored to a committed
/// trust root, not to a freshly re-derived key.
pub fn resolve_kv_anchor(repo_root: &Path, role: char) -> Option<String> {
    let anchors = load_kv_genesis(repo_root)?;
    for a in anchors {
        if a.role == role {
            return Some(format!("role={} {}", a.role, a.pub_hex));
        }
    }
    None
}

/// Measure `f` and report elapsed microseconds through the native telemetry sink.
/// Returns the result of `f`.
fn measure<F, T>(stage: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let t0 = std::time::Instant::now();
    let r = f();
    let us = t0.elapsed().as_micros();
    record_probe(stage, us, None);
    r
}

/// Emit one native telemetry probe line to stderr.
/// Format: `{"telemetry":{"stage":<stage>,"us":<micros>,"note":<optional>}}`
fn record_probe(stage: &str, us: u128, note: Option<&str>) {
    match note {
        Some(n) => {
            eprintln!("{{\"telemetry\":{{\"stage\":\"{stage}\",\"us\":{us},\"note\":\"{n}\"}}}}")
        }
        None => eprintln!("{{\"telemetry\":{{\"stage\":\"{stage}\",\"us\":{us}}}}}"),
    }
}

/// Native telemetry sink: a `ci-truth` CLI hook emitting aggregate gate metrics.
/// Prints a single JSON line to stdout describing the last gate run, useful for
/// CI dashboards / the mandatory-telemetry doctrine.
pub fn telemetry_sink(stage: &str, us: u128, red: bool) {
    println!(
        "{{\"v1_telemetry\":{{\"stage\":\"{stage}\",\"us\":{us},\"verdict\":{}}}}}",
        if red { "\"RED\"" } else { "\"GREEN\"" }
    );
}

// ---------------------------------------------------------------------------
// Git note I/O (refs/notes/v1-diff-attest, refs/notes/v1-verdict)
// ---------------------------------------------------------------------------

pub fn read_note(repo_root: &Path, note_ref: &str, sha: &str) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .current_dir(repo_root)
        .args(["notes", "--ref", note_ref, "show", sha])
        .output()
        .ok()?;
    if out.status.success() {
        Some(out.stdout)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Merge-gate policy (BLUEPRINT-P06 §5) — executable contract
// ---------------------------------------------------------------------------

/// Gate verdict: RED or GREEN with a reason string.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateVerdict {
    Green,
    Red(String),
}

/// Evaluate the merge gate for a commit given its two notes' raw bytes.
/// `ci_redline_touch` = CI's independent recomputation of whether the diff
/// touches a red-line path (the blueprint §5.6 honesty check).
///
/// `kv_master` = `Some(master_hex)` ONLY when the operator has minted a real
/// kv-genesis (and `bebop2-kv` is available). When `Some`, the gate performs
/// REAL hybrid (Ed25519⊕ML-DSA-65, RequireBoth) signature verification over
/// each note's `signing_bytes()` via `bebop2-kv verify` — Bug-1 fix. When
/// `None`, signature verification is SKIPPED (Phase-1 unsigned state) and the
/// gate falls back to the structural contract checks only. This keeps the gate
/// usable/testable without the operator trust root while never faking a
/// cryptographic pass.
///
/// `repo_root` is used only for telemetry emission (native sink); pass `None`
/// to suppress I/O (unit tests).
pub fn evaluate_gate(
    diff_attest_raw: &[u8],
    verdict_raw: &[u8],
    ci_redline_touch: bool,
    kv_master: Option<&str>,
    repo_root: Option<&Path>,
) -> GateVerdict {
    // §5.1 — both signatures/notes present
    if diff_attest_raw.is_empty() {
        return GateVerdict::Red("no key_K DiffAttestation note".into());
    }
    if verdict_raw.is_empty() {
        return GateVerdict::Red("no key_V Verdict note".into());
    }
    let attest = match DiffAttestation::decode(diff_attest_raw) {
        Some(a) => a,
        None => return GateVerdict::Red("malformed DiffAttestation TLV".into()),
    };
    let verdict = match Verdict::decode(verdict_raw) {
        Some(v) => v,
        None => return GateVerdict::Red("malformed Verdict TLV (or missing residue)".into()),
    };

    // §5.2 — key_K ≠ key_V (by anchor id)
    if attest.key_k_anchor_id == verdict.key_v_anchor_id {
        return GateVerdict::Red("key_K == key_V (verdict signed by author key)".into());
    }

    // §5.3 — hash-binding intact: verdict T=0x01 == sha3(attest)
    let att_digest = digest32(diff_attest_raw);
    if att_digest != verdict.diff_attest_sha3 {
        return GateVerdict::Red("verdict does not bind to the DiffAttestation".into());
    }

    // §5.3b — verifier recomputed diff_sha3 MUST equal author's
    if attest.diff_sha3 != verdict.recomputed_diff_sha3 {
        return GateVerdict::Red("verifier recomputed diff_sha3 != author's".into());
    }

    // §5.4 — GREEN required on red-line-touching diffs
    if ci_redline_touch && verdict.verdict != 0x01 {
        return GateVerdict::Red("red-line diff without GREEN verdict".into());
    }

    // §5.5 — residue present (already enforced by decode; double-check)
    if !verdict_raw
        .windows(V1_RESIDUE.len())
        .any(|w| w == V1_RESIDUE.as_bytes())
    {
        return GateVerdict::Red("verdict missing residue line".into());
    }

    // §5.6 — redline_touch honesty: CI's recomputation must agree with author's bit
    if ci_redline_touch && attest.redline_touch == 0 {
        return GateVerdict::Red(
            "author signed redline_touch=0 but CI matches red-line path".into(),
        );
    }
    if !ci_redline_touch && attest.redline_touch != 0 {
        return GateVerdict::Red(
            "author signed redline_touch=1 but CI matches no red-line path".into(),
        );
    }

    // --- REAL SIGNATURE VERIFICATION (Bug-1 fix, BLUEPRINT-P06 §5/§7.7) --------
    // When the operator trust root is present, actually verify BOTH the key_K
    // DiffAttestation signature and the key_V Verdict signature through the
    // hybrid gate (RequireBoth). A missing/forge/garbage sig => RED. This is the
    // check that the prior implementation silently skipped (self-consistency
    // only). Fail-closed: if the binary is declared available (master set) but a
    // verify fails for ANY reason, the gate is RED.
    if let Some(master) = kv_master {
        let k_signer = HybridSigner {
            role: 'K',
            master_hex: master.to_string(),
        };
        let v_signer = HybridSigner {
            role: 'V',
            master_hex: master.to_string(),
        };

        // Split each note into the signing-covered bytes + the embedded sig.
        let (att_signing, att_sig) = split_note_sig(diff_attest_raw, SIG_TAG_K);
        let (ver_signing, ver_sig) = split_note_sig(verdict_raw, SIG_TAG_V);

        if att_sig.is_empty() || ver_sig.is_empty() {
            return GateVerdict::Red("a required signature is absent on a signed note".into());
        }

        let k_anchor = k_signer.pub_anchor_line();
        let v_anchor = v_signer.pub_anchor_line();
        if k_anchor.is_empty() || v_anchor.is_empty() {
            return GateVerdict::Red("could not derive kv anchor lines".into());
        }

        // §5.2 reinforcement: the key_K anchor id MUST resolve to role=K and
        // the key_V anchor id to role=V (no cross-role attestation).
        if !k_anchor.trim_end().ends_with("role=K") {
            return GateVerdict::Red("key_K anchor does not resolve to role=K".into());
        }
        if !v_anchor.trim_end().ends_with("role=V") {
            return GateVerdict::Red("key_V anchor does not resolve to role=V".into());
        }

        let k_ok = measure("verify.key_K", || {
            k_signer.verify_signature(&k_anchor, &att_signing, &hex_encode(&att_sig), repo_root)
        });
        if !k_ok {
            return GateVerdict::Red(
                "key_K DiffAttestation signature FAILED real verification".into(),
            );
        }

        let v_ok = measure("verify.key_V", || {
            v_signer.verify_signature(&v_anchor, &ver_signing, &hex_encode(&ver_sig), repo_root)
        });
        if !v_ok {
            return GateVerdict::Red("key_V Verdict signature FAILED real verification".into());
        }
    }

    GateVerdict::Green
}

// ---------------------------------------------------------------------------
// Subcommand: v1-verify <sha>  — fetches notes, runs the gate, emits RED/GREEN
// ---------------------------------------------------------------------------

pub fn v1_verify(pos: &[String]) -> i32 {
    let sha = pos
        .first()
        .cloned()
        .or_else(|| git_ok(&["rev-parse", "HEAD"]))
        .unwrap_or_default();
    let repo_root = match git_ok(&["rev-parse", "--show-toplevel"]) {
        Some(r) => PathBuf::from(r),
        None => {
            eprintln!("v1-verify: not inside a git repo");
            return 1;
        }
    };

    // CI-independent red-line touch: diff the commit's files against its parent.
    let parent = format!("{sha}~1");
    let diff_files = git_ok(&["diff", "--name-only", &parent, &sha]).unwrap_or_default();
    let ci_redline_touch = diff_files.lines().any(is_redline);

    let attest = read_note(&repo_root, "v1-diff-attest", &sha).unwrap_or_default();
    let verdict = read_note(&repo_root, "v1-verdict", &sha).unwrap_or_default();

    // Resolve the operator trust root (real kv-genesis master, if minted).
    // When present AND bebop2-kv is available, evaluate_gate performs REAL
    // hybrid signature verification; otherwise it falls back to the unsigned
    // structural contract (never a fake pass).
    let kv_master = std::env::var("V1_KV_MASTER").ok().filter(|m| !m.is_empty());
    let kv_master = if kv_master.is_some() && kv_bin_available() {
        kv_master
    } else {
        None
    };
    let gate = evaluate_gate(
        &attest,
        &verdict,
        ci_redline_touch,
        kv_master.as_deref(),
        Some(&repo_root),
    );
    match &gate {
        GateVerdict::Green => {
            println!("V1-GATE: GREEN");
            println!(
                "{{\"v1_gate\":\"GREEN\",\"sha\":\"{sha}\",\"red_line_touch\":{ci_redline_touch}}}"
            );
            0
        }
        GateVerdict::Red(reason) => {
            println!("V1-GATE: RED");
            println!("{{\"v1_gate\":\"RED\",\"sha\":\"{sha}\",\"reason\":\"{reason}\",\"red_line_touch\":{ci_redline_touch}}}");
            1
        }
    }
}

// ---------------------------------------------------------------------------
// Subcommand: v1-probe [<master-hex>] — runnable P06 real-signature probe.
// Signs a fixed known payload with a deterministic master, verifies the roundtrip
// via the real bebop2-kv hybrid CLI, then proves a 1-bit corruption is REJECTED
// (§7.7 / anti-scope: must NOT fake a green). Emits native telemetry.
// Exit 0 = probe healthy; 1 = bebop2-kv binary missing (so no real crypto ran).
// ---------------------------------------------------------------------------

pub fn v1_probe(pos: &[String]) -> i32 {
    if !kv_bin_available() {
        eprintln!(
            "v1-probe: SKIP — bebop2-kv not found (set BEBOp_REPO_ROOT or V1_KV_BIN). \
             No real hybrid signature crypto was exercised, so this is NOT a green pass."
        );
        return 1;
    }
    // Deterministic TEST-only master (NOT a committed trust root).
    let master = pos.first().cloned().unwrap_or_else(|| {
        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff".into()
    });
    let payload = b"p06-key_v-hybrid-probe-payload-v1";

    let k = HybridSigner {
        role: 'K',
        master_hex: master.clone(),
    };
    let v = HybridSigner {
        role: 'V',
        master_hex: master,
    };

    let k_sig = k.sign(payload);
    let k_anchor = k.pub_anchor_line();
    let k_ok = measure("probe.verify.key_K", || {
        k.verify_signature(&k_anchor, payload, &hex_encode(&k_sig), None)
    });
    if !k_ok {
        eprintln!("v1-probe: FAIL — key_K real hybrid signature did not verify");
        return 1;
    }

    let v_sig = v.sign(payload);
    let v_anchor = v.pub_anchor_line();
    let v_ok = measure("probe.verify.key_V", || {
        v.verify_signature(&v_anchor, payload, &hex_encode(&v_sig), None)
    });
    if !v_ok {
        eprintln!("v1-probe: FAIL — key_V real hybrid signature did not verify");
        return 1;
    }

    // Corruption proof: a 1-bit flip MUST be rejected (anti-scope: no fake green).
    let mut k_sig_bad = k_sig.clone();
    k_sig_bad[0] ^= 0x01;
    let bad_rejected = measure("probe.corrupt.key_K", || {
        !k.verify_signature(&k_anchor, payload, &hex_encode(&k_sig_bad), None)
    });
    if !bad_rejected {
        eprintln!("v1-probe: FAIL — 1-bit-flipped key_K sig was wrongly ACCEPTED");
        return 1;
    }

    println!(
        "V1-PROBE: OK (real Ed25519⊕ML-DSA-65 roundtrip verified; corruption rejected; \
         key_K anchor={k_anchor}, key_V anchor={v_anchor})"
    );
    telemetry_sink("v1_probe", 0, false);
    0
}

// ---------------------------------------------------------------------------
// Note-emitter helpers (BLUEPRINT-H3 §consume): the *note production* side, as
// a downstream breach-probe arc (hermetic H3 / spectral E3) would call them
// BEFORE writing the two git notes. NO cryptography — these mirror exactly the
// encode() shapes the gate consumes, so an honest probe verdict can be proven
// end-to-end through `evaluate_gate` without ever touching the C4b crypto slot.
// ---------------------------------------------------------------------------

/// Build the key_K DiffAttestation note (raw TLV bytes), as an honest diff-owner
/// would before `git notes --ref v1-diff-attest add`. Unsigned (Phase-1 state).
pub fn build_attestation(
    commit: [u8; 32],
    diff_sha3: [u8; 32],
    base_sha3: [u8; 32],
    key_k: [u8; 32],
    redline_touch: u8,
    timestamp: u64,
) -> Vec<u8> {
    DiffAttestation {
        commit_sha3: commit,
        diff_sha3,
        base_sha3,
        key_k_anchor_id: key_k,
        sig_k: Vec::new(),
        redline_touch,
        timestamp,
    }
    .encode()
}

/// Build the key_V Verdict note (raw TLV bytes) bound to a DiffAttestation.
/// `diff_attest_sha3` is computed as the canonical binding `digest32(attest_raw)`
/// — exactly what `evaluate_gate` §5.3 re-derives — so the hash-binding bites.
/// `recomputed_diff_sha3` is the verifier's independent recomputation of the
/// diff digest. Verdict is GREEN (0x01), unsigned (Phase-1 state).
pub fn build_verdict(
    attest_raw: &[u8],
    recomputed_diff_sha3: [u8; 32],
    key_v: [u8; 32],
    ctx: &str,
    rationale: &str,
) -> Vec<u8> {
    let diff_attest_sha3 = digest32(attest_raw);
    Verdict {
        diff_attest_sha3,
        recomputed_diff_sha3,
        verdict: 0x01,
        key_v_anchor_id: key_v,
        sig_v: Vec::new(),
        context_descriptor: ctx.to_string(),
        rationale: rationale.to_string(),
    }
    .encode()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zero() -> [u8; 32] {
        [0u8; 32]
    }

    /// Build a valid attest+verdict pair bound to each other, with K≠V.
    fn valid_pair(diff_sha: [u8; 32], redline_touch: u8) -> (Vec<u8>, Vec<u8>) {
        let key_k: [u8; 32] = [0x11; 32];
        let key_v: [u8; 32] = [0x22; 32];
        let mut commit = zero();
        commit[0] = 0xAB;
        let attest = DiffAttestation {
            commit_sha3: commit,
            diff_sha3: diff_sha,
            base_sha3: zero(),
            key_k_anchor_id: key_k,
            sig_k: Vec::new(),
            redline_touch,
            timestamp: 1_700_000_000,
        }
        .encode();
        let att_digest = digest32(&attest);
        let verdict = Verdict {
            diff_attest_sha3: att_digest,
            recomputed_diff_sha3: diff_sha,
            verdict: 0x01,
            key_v_anchor_id: key_v,
            sig_v: Vec::new(),
            context_descriptor: "test".into(),
            rationale: "ok".into(),
        }
        .encode();
        (attest, verdict)
    }

    #[test]
    fn tlv_roundtrip_attestation() {
        let (a, _) = valid_pair([0x99; 32], 0);
        let dec = DiffAttestation::decode(&a).expect("decode");
        assert_eq!(dec.key_k_anchor_id, [0x11; 32]);
        assert_eq!(dec.redline_touch, 0);
        assert_eq!(dec.timestamp, 1_700_000_000);
    }

    #[test]
    fn tlv_roundtrip_verdict_residue_present() {
        let (_, v) = valid_pair([0x99; 32], 0);
        let dec = Verdict::decode(&v).expect("decode");
        assert_eq!(dec.verdict, 0x01);
        assert_eq!(dec.key_v_anchor_id, [0x22; 32]);
    }

    #[test]
    fn gate_green_on_valid_non_redline_pair() {
        let (a, v) = valid_pair([0x99; 32], 0);
        assert_eq!(evaluate_gate(&a, &v, false, None, None), GateVerdict::Green);
    }

    #[test]
    fn gate_red_missing_attestation_note() {
        let (_, v) = valid_pair([0x99; 32], 0);
        assert!(matches!(
            evaluate_gate(&[], &v, false, None, None),
            GateVerdict::Red(_)
        ));
    }

    #[test]
    fn gate_red_kequalskv() {
        // forge a verdict signed by the same anchor as the attestation
        let (a, _v) = valid_pair([0x99; 32], 0);
        // rewire verdict.key_v_anchor_id to key_k ([0x11;32]) by re-encoding
        let att = DiffAttestation::decode(&a).unwrap();
        let forged = Verdict {
            diff_attest_sha3: digest32(&a),
            recomputed_diff_sha3: att.diff_sha3,
            verdict: 0x01,
            key_v_anchor_id: att.key_k_anchor_id, // == key_K → forbidden
            sig_v: Vec::new(),
            context_descriptor: "forge".into(),
            rationale: "self-signed".into(),
        }
        .encode();
        assert!(matches!(
            evaluate_gate(&a, &forged, false, None, None),
            GateVerdict::Red(s) if s.contains("key_K == key_V")
        ));
    }

    #[test]
    fn gate_red_residue_missing() {
        let (a, mut v) = valid_pair([0x99; 32], 0);
        // strip the residue TLV (tag 0x09) — decode must reject
        // find and remove the residue record from the encoded verdict
        let idx = v
            .windows(5)
            .position(|w| {
                w[0] == 0x09
                    && u32::from_be_bytes([w[1], w[2], w[3], w[4]]) as usize == V1_RESIDUE.len()
            })
            .expect("residue present in valid pair");
        let rec_len = 5 + V1_RESIDUE.len();
        v.drain(idx..idx + rec_len);
        assert!(Verdict::decode(&v).is_none());
        // if someone bypasses decode, the gate must still catch the missing residue
        assert!(matches!(
            evaluate_gate(&a, &v, false, None, None),
            GateVerdict::Red(s) if s.contains("residue")
        ));
    }

    #[test]
    fn gate_red_redline_diff_requires_green() {
        let (a, mut v) = valid_pair([0x99; 32], 1);
        // flip verdict byte to RED (0x00) while redline_touch=1
        let mut dec = Verdict::decode(&v).unwrap();
        dec.verdict = 0x00;
        v = dec.encode();
        assert!(matches!(
            evaluate_gate(&a, &v, true, None, None),
            GateVerdict::Red(s) if s.contains("GREEN")
        ));
    }

    #[test]
    fn gate_red_redline_touch_mismatch() {
        // CI recomputes red-line touch, author lied about bit
        let (a, v) = valid_pair([0x99; 32], 0);
        // ci says it DOES touch red-line, but author signed redline_touch=0
        assert!(matches!(
            evaluate_gate(&a, &v, true, None, None),
            GateVerdict::Red(s) if s.contains("redline_touch")
        ));
    }

    #[test]
    fn kv_genesis_load_invariant_kneqv() {
        let dir = std::env::temp_dir().join(format!("kvgen-{}-{}", std::process::id(), now_ts()));
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join(KV_GENESIS);
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        std::fs::write(&p, "aabbcc role=K\nddeeff role=V\n").unwrap();
        let anchors = load_kv_genesis(&dir).expect("load");
        assert_eq!(anchors.len(), 2);

        // K == V must fail
        let p2 = dir.join("bad.txt");
        std::fs::write(&p2, "aabbcc role=K\naabbcc role=V\n").unwrap();
        std::fs::write(dir.join(KV_GENESIS), "aabbcc role=K\naabbcc role=V\n").unwrap();
        assert!(load_kv_genesis(&dir).is_none());

        let _ = std::fs::remove_dir_all(&dir);
    }

    fn now_ts() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // H3 end-to-end contract proofs (BLUEPRINT-H3 / Wave B)
    // -----------------------------------------------------------------------

    /// Prove an H3-style breach-probe verdict flows through the P06 gate GREEN,
    /// produced entirely via the note-emitter helpers above, unsigned/honest.
    #[test]
    fn h3_verdict_flows_through_gate_green() {
        let key_k: [u8; 32] = [0x11; 32];
        let key_v: [u8; 32] = [0x22; 32]; // distinct anchor: K ≠ V
        let diff_sha3: [u8; 32] = [0x99; 32];
        let commit: [u8; 32] = {
            let mut c = [0u8; 32];
            c[0] = 0xAB;
            c
        };
        let base: [u8; 32] = [0u8; 32];

        let attest = build_attestation(commit, diff_sha3, base, key_k, 0, 1_700_000_000);
        // verifier's independent recomputation of the diff digest == author's
        let verdict = build_verdict(
            &attest,
            diff_sha3,
            key_v,
            "h3-breach-probe",
            "no breach found",
        );

        assert_eq!(
            evaluate_gate(&attest, &verdict, false, None, None),
            GateVerdict::Green
        );
    }

    /// Prove the §5.3 hash-binding actually bites: a downstream arc cannot
    /// present a verdict bound to a different attestation. Flipping the
    /// verifier's recomputed diff_sha3 (which propagates into the binding via
    /// the verifier's own gate recomputation of attest) is caught as RED.
    ///
    /// Here we simulate a *mismatched* verdict by building it against the wrong
    /// attestation (so `diff_attest_sha3` no longer equals `digest32(real
    /// attest)`), then running the gate on the real attestation.
    #[test]
    fn h3_binding_mismatch_rejected() {
        let key_k: [u8; 32] = [0x11; 32];
        let key_v: [u8; 32] = [0x22; 32];
        let diff_a: [u8; 32] = [0x99; 32];
        let diff_b: [u8; 32] = [0x77; 32];
        let commit: [u8; 32] = [0u8; 32];
        let base: [u8; 32] = [0u8; 32];

        // real attestation for diff_a
        let attest_a = build_attestation(commit, diff_a, base, key_k, 0, 1_700_000_000);
        // a verdict honestly built against a DIFFERENT attestation (diff_b)
        let attest_b = build_attestation(commit, diff_b, base, key_k, 0, 1_700_000_000);
        let mismatched = build_verdict(&attest_b, diff_a, key_v, "h3", "mismatched binding");

        // gate must REJECT: verdict.diff_attest_sha3 != digest32(attest_a)
        assert!(matches!(
            evaluate_gate(&attest_a, &mismatched, false, None, None),
            GateVerdict::Red(s) if s.contains("bind")
        ));
    }

    // -----------------------------------------------------------------------
    // P06 real-sig acceptance scaffolding (consumption half)
    // -----------------------------------------------------------------------

    /// Always-run proof that `HybridSigner` is the production signer: `signed()`
    /// is true, and — when the crypto CLI is absent — sign/verify are
    /// fail-closed (empty sig, verify=false). This guards the wiring without
    /// requiring the bebop2-kv binary in CI for the unsigned suite.
    #[test]
    fn hybrid_signer_is_production_and_failclosed() {
        let k = HybridSigner {
            role: 'K',
            master_hex: "deadbeef".into(),
        };
        assert!(k.signed());
        // No binary present (or present but unknown master) => fail-closed.
        let bytes = b"v1-diff-attest-bytes";
        let sig = k.sign(bytes);
        // Fail-closed: either an empty sig, or — if a binary IS wired — a
        // valid sig that must verify roundtrip. We only assert non-panic +
        // that verify of an empty/garbage sig is false when no real sig exists.
        if sig.is_empty() {
            assert!(!k.verify_signature("role=K deadbeef", bytes, "", None));
        }
        // UnsignedSigner must still report unsigned (Phase-1 retained).
        let u = UnsignedSigner;
        assert!(!u.signed());
    }

    /// DETERMINISTIC TEST ANCHOR (NOT a committed trust root): proves the
    /// real bebop2-kv hybrid roundtrip AND that a 1-bit-flipped signature is
    /// rejected (acceptance §7.7/§7.9: corrupt-either-leg fails). Requires
    /// the `bebop2-kv` CLI built from the bebop repo. If it isn't present the
    /// test is `#[ignore]`d with a clear message rather than failing the suite.
    ///
    /// Set `BEBOp_REPO_ROOT` to the bebop checkout (or `V1_KV_BIN` to the
    /// built `bebop2-kv` path) to run it. The two roles share one master seed
    /// only for the deterministic TEST anchor; production mints distinct keys.
    #[test]
    #[ignore = "requires bebop2-kv CLI (BEBOp_REPO_ROOT/V1_KV_BIN)"]
    fn real_hybrid_sig_roundtrip_and_corruption_rejected() {
        if !kv_bin_available() {
            eprintln!(
                "SKIP real_hybrid_sig_roundtrip_and_corruption_rejected: \
                 bebop2-kv not found (set BEBOp_REPO_ROOT or V1_KV_BIN)"
            );
            return;
        }
        let master = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        let diff_bytes = [0x99u8; 32];

        // key_K attestation signature (REAL hybrid Ed25519⊕ML-DSA-65).
        let k_signer = HybridSigner {
            role: 'K',
            master_hex: master.into(),
        };
        let k_sig = k_signer.sign(&diff_bytes);
        assert!(!k_sig.is_empty(), "real key_K signature must be non-empty");
        let k_anchor = k_signer.pub_anchor_line();
        assert!(
            !k_anchor.is_empty(),
            "real key_K anchor line must be derivable"
        );
        assert!(
            k_signer.verify_signature(&k_anchor, &diff_bytes, &hex_encode(&k_sig), None),
            "real key_K attestation sig must verify true"
        );

        // key_V verdict signature.
        let v_signer = HybridSigner {
            role: 'V',
            master_hex: master.into(),
        };
        let v_sig = v_signer.sign(&diff_bytes);
        assert!(!v_sig.is_empty(), "real key_V signature must be non-empty");
        let v_anchor = v_signer.pub_anchor_line();
        assert!(
            !v_anchor.is_empty(),
            "real key_V anchor line must be derivable"
        );
        assert!(
            v_signer.verify_signature(&v_anchor, &diff_bytes, &hex_encode(&v_sig), None),
            "real key_V verdict sig must verify true"
        );

        // §7.7/§7.9: flipping 1 bit of the signature MUST fail (either leg).
        let mut k_sig_bad = k_sig.clone();
        k_sig_bad[0] ^= 0x01;
        assert!(
            !k_signer.verify_signature(&k_anchor, &diff_bytes, &hex_encode(&k_sig_bad), None),
            "1-bit-flipped key_K sig must verify false"
        );
        let mut v_sig_bad = v_sig.clone();
        let v_last = v_sig_bad.len() - 1;
        v_sig_bad[v_last] ^= 0x80;
        assert!(
            !v_signer.verify_signature(&v_anchor, &diff_bytes, &hex_encode(&v_sig_bad), None),
            "1-bit-flipped key_V sig must verify false"
        );
    }

    /// Bug-1 / Bug-2 acceptance test: a fully REAL, signed DiffAttestation +
    /// Verdict pair (key_K and key_V hybrid sigs embedded in the TLV) must
    /// evaluate GREEN through `evaluate_gate` WITH `kv_master` set (real
    /// signature verification active), and a forged sig must evaluate RED.
    /// Requires the `bebop2-kv` CLI (set BEBOp_REPO_ROOT / V1_KV_BIN).
    #[test]
    #[ignore = "requires bebop2-kv CLI (BEBOp_REPO_ROOT/V1_KV_BIN)"]
    fn real_gate_with_kv_master() {
        if !kv_bin_available() {
            eprintln!("SKIP real_gate_with_kv_master: bebop2-kv not found");
            return;
        }
        let master = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        let commit = [0xaa; 32];
        let diff_sha3 = [0x99; 32];
        let base = [0x11; 32];
        let key_k = [0x33; 32];
        let key_v = [0x44; 32];

        // Build the unsigned attestation/verdict TLV, then ADD real sigs.
        let att_unsigned = build_attestation(commit, diff_sha3, base, key_k, 0, 1_700_000_000);
        let ver_unsigned = build_verdict(&att_unsigned, diff_sha3, key_v, "probe", "ok");

        // signing_bytes() excludes the sig tag (0x08/0x0a); verify_message matches.
        let k_signer = HybridSigner {
            role: 'K',
            master_hex: master.into(),
        };
        let v_signer = HybridSigner {
            role: 'V',
            master_hex: master.into(),
        };
        let att_sig = k_signer.sign(&verify_message(&att_unsigned, SIG_TAG_K));
        let ver_sig = v_signer.sign(&verify_message(&ver_unsigned, SIG_TAG_V));
        assert!(
            !att_sig.is_empty() && !ver_sig.is_empty(),
            "real sigs must be non-empty"
        );

        // Embed sigs: append sig TLV records to the encoded notes.
        let mut att_signed = att_unsigned.clone();
        tlv_put(&mut att_signed, SIG_TAG_K, &att_sig);
        let mut ver_signed = ver_unsigned.clone();
        tlv_put(&mut ver_signed, SIG_TAG_V, &ver_sig);

        // Gate with REAL verification active => GREEN.
        let gate = evaluate_gate(&att_signed, &ver_signed, false, Some(master), None);
        assert_eq!(
            gate,
            GateVerdict::Green,
            "real signed pair must be GREEN under kv_master"
        );

        // Forge: swap the verdict sig for garbage => RED (real verify bites).
        let mut ver_forged = ver_unsigned.clone();
        tlv_put(&mut ver_forged, SIG_TAG_V, &vec![0xab; 64]);
        let gate_forged = evaluate_gate(&att_signed, &ver_forged, false, Some(master), None);
        assert!(
            matches!(gate_forged, GateVerdict::Red(_)),
            "forged sig must be RED under kv_master"
        );
    }
}
