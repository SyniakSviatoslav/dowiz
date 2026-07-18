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

/// Anchor-file path: two public keys, tagged role=K / role=V (BLUEPRINT-P06 §2).
pub const KV_GENESIS: &str = "config/kv-genesis.txt";

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
    pub role: char,     // 'K' or 'V'
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
/// The `sig` field carries the REAL hybrid (Ed25519⊕ML-DSA-65) signature over
/// `signing_bytes()` — see `signing_bytes` / `set_signature` and the gate's
/// real `bebop2-kv verify` check.
pub struct DiffAttestation {
    pub commit_sha3: [u8; 32],
    pub diff_sha3: [u8; 32],
    pub base_sha3: [u8; 32],
    pub key_k_anchor_id: [u8; 32],
    pub redline_touch: u8,
    pub timestamp: u64,
    /// REAL key_K hybrid signature over `signing_bytes()` (tag 0x07). Empty in
    /// the unsigned/Phase-1 state; non-empty once produced by `HybridSigner`.
    pub sig: Vec<u8>,
}

impl DiffAttestation {
    /// The exact byte slice the key_K signature commits to. Determinism: only
    /// the canonical binding fields, never the signature itself (avoids a
    /// self-referential loop). This is what `HybridSigner::sign` must be fed.
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
        tlv_put(&mut b, 0x07, &self.sig);
        b
    }
    pub fn decode(buf: &[u8]) -> Option<Self> {
        let m = tlv_get(buf);
        let get32 = |t: u8| field32(&m, t);
        Some(Self {
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
            sig: m.get(&0x07).cloned().unwrap_or_default(),
        })
    }
}

/// Verdict TLV (BLUEPRINT-P06 §4). Signed by key_V in production.
/// The `sig` field carries the REAL key_V hybrid signature over
/// `signing_bytes()` (tag 0x08) — see the gate's real `bebop2-kv verify` check.
pub struct Verdict {
    pub diff_attest_sha3: [u8; 32],
    pub recomputed_diff_sha3: [u8; 32],
    pub verdict: u8, // 0x00=RED, 0x01=GREEN
    pub key_v_anchor_id: [u8; 32],
    pub context_descriptor: String,
    pub rationale: String,
    /// REAL key_V hybrid signature over `signing_bytes()` (tag 0x08). Empty in
    /// the unsigned/Phase-1 state; non-empty once produced by `HybridSigner`.
    pub sig: Vec<u8>,
}

impl Verdict {
    /// The exact byte slice the key_V signature commits to. Determinism: only
    /// the canonical binding fields, never the signature itself.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut b = Vec::new();
        tlv_put(&mut b, 0x01, &self.diff_attest_sha3);
        tlv_put(&mut b, 0x02, &self.recomputed_diff_sha3);
        tlv_put(&mut b, 0x03, &[self.verdict]);
        tlv_put(&mut b, 0x05, &self.key_v_anchor_id);
        tlv_put(&mut b, 0x06, self.context_descriptor.as_bytes());
        tlv_put(&mut b, 0x07, self.rationale.as_bytes());
        tlv_put(&mut b, 0x09, V1_RESIDUE.as_bytes()); // residue always present
        b
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut b = self.signing_bytes();
        tlv_put(&mut b, 0x08, &self.sig);
        b
    }
    pub fn decode(buf: &[u8]) -> Option<Self> {
        let m = tlv_get(buf);
        let get32 = |t: u8| field32(&m, t);
        let residue = m.get(&0x09)?;
        if residue != V1_RESIDUE.as_bytes() {
            return None; // residue missing/changed => malformed
        }
        Some(Self {
            diff_attest_sha3: get32(0x01)?,
            recomputed_diff_sha3: get32(0x02)?,
            verdict: *m.get(&0x03)?.first()?,
            key_v_anchor_id: get32(0x05)?,
            context_descriptor: String::from_utf8_lossy(m.get(&0x06)?).into_owned(),
            rationale: String::from_utf8_lossy(m.get(&0x07)?).into_owned(),
            sig: m.get(&0x08).cloned().unwrap_or_default(),
        })
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

/// Tolerant parser for `bebop2-kv verify` output: `{"ok":bool}`, a bare
/// `"true"`/`"ok"`, or any line containing `"ok":true`.
fn parse_ok(s: &str) -> bool {
    let s = s.trim();
    if s.contains("\"ok\"") {
        return s.contains("true");
    }
    s.eq_ignore_ascii_case("true") || s.eq_ignore_ascii_case("ok")
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
        // bebop2-kv sign <role> <master-hex> <hex(bytes)> -> hex sig on stdout
        match Command::new(kv_bin())
            .args([
                "sign",
                &self.role.to_string(),
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
    /// `bebop2-kv verify <anchor-line> <hex(bytes)> <sig_hex>`.
    pub fn verify_signature(&self, pub_anchor_line: &str, bytes: &[u8], sig_hex: &str) -> bool {
        match Command::new(kv_bin())
            .args([
                "verify",
                pub_anchor_line,
                &hex_encode(bytes),
                sig_hex,
            ])
            .output()
        {
            Ok(o) if o.status.success() => parse_ok(&String::from_utf8_lossy(&o.stdout)),
            _ => false,
        }
    }

    /// Derive this role's public anchor line (hex) for `verify_signature`, by
    /// shelling `bebop2-kv genkeys <master-hex>` and returning the line whose
    /// `role=<R>` matches this signer's role. (The real CLI has NO `pubkey`
    /// subcommand — only `genkeys|sign|verify` — so we re-derive deterministically
    /// from the master seed, which is the single kv trust root.) Empty on failure.
    pub fn pub_anchor_line(&self) -> String {
        match Command::new(kv_bin()).args(["genkeys", &self.master_hex]).output() {
            Ok(o) if o.status.success() => {
                let out = String::from_utf8_lossy(&o.stdout);
                for line in out.lines() {
                    if line.trim_end().ends_with(&format!("role={}", self.role)) {
                        return line.trim().to_string();
                    }
                }
                String::new()
            }
            _ => String::new(),
        }
    }
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

/// Signedness mode for the gate. In `Production` mode the gate binds REAL
/// hybrid (`bebop2-kv`) signatures — the gated bytes MUST verify against the
/// kv-genesis anchor for each role. In `Unsigned` mode the gate is
/// policy-only (the Phase-1 contract, `signed:false`) and never shells `verify`.
#[derive(Clone, Debug)]
pub struct SignedGate {
    /// The kv-genesis root to load anchors from (BLUEPRINT-P06 §2).
    pub repo_root: PathBuf,
    /// master seed used by the gate's verifier to re-derive the role anchors.
    /// (Production mints anchors from the same master the signer used; the
    /// operator's kv-genesis.txt is the committed trust surface.)
    pub master_hex: String,
}

/// Evaluate the merge gate for a commit given its two notes' raw bytes.
/// `ci_redline_touch` = CI's independent recomputation of whether the diff
/// touches a red-line path (the blueprint §5.6 honesty check).
///
/// When `signed` is `Some(SignedGate{..})` the gate additionally performs REAL
/// split-identity signature verification via `bebop2-kv verify`: the key_K
/// attestation sig must verify over `DiffAttestation::signing_bytes()`, and the
/// key_V verdict sig must verify over `Verdict::signing_bytes()`. A failed or
/// missing signature is fail-closed => RED. This is the actual `HybridSigner`
/// crypto slot (C4b GREEN) — evaluation no longer merely checks TLV
/// self-consistency; it proves the notes are authentically signed.
pub fn evaluate_gate(
    diff_attest_raw: &[u8],
    verdict_raw: &[u8],
    ci_redline_touch: bool,
    signed: Option<&SignedGate>,
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

    // §5.7 (C4b GREEN) — REAL hybrid signature verification (the crypto slot).
    // Fail-closed: any error/missing-sig/verify-false => RED.
    if let Some(sg) = signed {
        let anchors = match load_kv_genesis(&sg.repo_root) {
            Some(a) if a.len() >= 2 => a,
            _ => return GateVerdict::Red("kv-genesis missing/unloadable (signed mode)".into()),
        };
        let k_anchor = match anchors.iter().find(|a| a.role == 'K') {
            Some(a) => a.pub_hex.clone(),
            None => return GateVerdict::Red("kv-genesis missing role=K anchor".into()),
        };
        let v_anchor = match anchors.iter().find(|a| a.role == 'V') {
            Some(a) => a.pub_hex.clone(),
            None => return GateVerdict::Red("kv-genesis missing role=V anchor".into()),
        };

        // attestation signed by key_K over its signing bytes
        let k_signer = HybridSigner { role: 'K', master_hex: sg.master_hex.clone() };
        if attest.sig.is_empty() {
            return GateVerdict::Red("key_K DiffAttestation carries no signature".into());
        }
        if !k_signer.verify_signature(
            &format!("{} role=K", k_anchor),
            &attest.signing_bytes(),
            &hex_encode(&attest.sig),
        ) {
            return GateVerdict::Red("key_K DiffAttestation signature FAILED verification".into());
        }

        // verdict signed by key_V over its signing bytes
        let v_signer = HybridSigner { role: 'V', master_hex: sg.master_hex.clone() };
        if verdict.sig.is_empty() {
            return GateVerdict::Red("key_V Verdict carries no signature".into());
        }
        if !v_signer.verify_signature(
            &format!("{} role=V", v_anchor),
            &verdict.signing_bytes(),
            &hex_encode(&verdict.sig),
        ) {
            return GateVerdict::Red("key_V Verdict signature FAILED verification".into());
        }
    }

    GateVerdict::Green
}

// ---------------------------------------------------------------------------
// Subcommand: v1-verify <sha>  — fetches notes, runs the gate, emits RED/GREEN
// ---------------------------------------------------------------------------

pub fn v1_verify(pos: &[String]) -> i32 {
    let sha = pos.first().cloned().or_else(|| git_ok(&["rev-parse", "HEAD"])).unwrap_or_default();
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

    // §5.7 (C4b GREEN): when the real `bebop2-kv` binary is present AND a kv
    // master seed is supplied (env `V1_KV_MASTER`), evaluate in SIGNED mode —
    // REAL hybrid signature verification through `bebop2-kv verify`. Without a
    // master seed we fall back to the unsigned (policy-only) gate, mirroring
    // main.rs:423 `signed:false`. This keeps `v1-verify` runnable on machines
    // that only have the policy contract, while proving real signatures wherever
    // the crypto CLI + master are wired.
    let signed = if kv_bin_available() {
        std::env::var("V1_KV_MASTER").ok().filter(|m| !m.is_empty()).map(|m| SignedGate {
            repo_root: repo_root.clone(),
            master_hex: m,
        })
    } else {
        None
    };

    let gate = evaluate_gate(&attest, &verdict, ci_redline_touch, signed.as_ref());
    match &gate {
        GateVerdict::Green => {
            println!("V1-GATE: GREEN");
            println!("{{\"v1_gate\":\"GREEN\",\"sha\":\"{sha}\",\"red_line_touch\":{ci_redline_touch}}}");
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
// Note-emitter helpers (BLUEPRINT-H3 §consume): the *note production* side, as
// a downstream breach-probe arc (hermetic H3 / spectral E3) would call them
// BEFORE writing the two git notes. The base shape mirrors exactly the encode()
// the gate consumes. When `master_hex: Some(seed)` is supplied (C4b GREEN),
// the emitters produce REAL hybrid (Ed25519⊕ML-DSA-65) signatures via
// `HybridSigner` and embed them in the TLV `sig` field, so the gate's §5.7
// verification bites on authentic notes end-to-end.
// ---------------------------------------------------------------------------

/// Build the key_K DiffAttestation note (raw TLV bytes), as an honest diff-owner
/// would before `git notes --ref v1-diff-attest add`. When `master_hex` is
/// `Some(seed)`, the note is signed with the REAL key_K hybrid key derived from
/// `seed`; otherwise it is unsigned (Phase-1 state, `signed:false`).
pub fn build_attestation(
    commit: [u8; 32],
    diff_sha3: [u8; 32],
    base_sha3: [u8; 32],
    key_k: [u8; 32],
    redline_touch: u8,
    timestamp: u64,
    master_hex: Option<&str>,
) -> Vec<u8> {
    let mut a = DiffAttestation {
        commit_sha3: commit,
        diff_sha3,
        base_sha3,
        key_k_anchor_id: key_k,
        redline_touch,
        timestamp,
        sig: Vec::new(),
    };
    if let Some(seed) = master_hex {
        let signer = HybridSigner { role: 'K', master_hex: seed.to_string() };
        a.sig = signer.sign(&a.signing_bytes());
    }
    a.encode()
}

/// Build the key_V Verdict note (raw TLV bytes) bound to a DiffAttestation.
/// `diff_attest_sha3` is computed as the canonical binding `digest32(attest_raw)`
/// — exactly what `evaluate_gate` §5.3 re-derives — so the hash-binding bites.
/// `recomputed_diff_sha3` is the verifier's independent recomputation of the
/// diff digest. Verdict is GREEN (0x01). When `master_hex` is `Some(seed)` the
/// verdict carries a REAL key_V hybrid signature.
pub fn build_verdict(
    attest_raw: &[u8],
    recomputed_diff_sha3: [u8; 32],
    key_v: [u8; 32],
    ctx: &str,
    rationale: &str,
    master_hex: Option<&str>,
) -> Vec<u8> {
    let diff_attest_sha3 = digest32(attest_raw);
    let mut v = Verdict {
        diff_attest_sha3,
        recomputed_diff_sha3,
        verdict: 0x01,
        key_v_anchor_id: key_v,
        context_descriptor: ctx.to_string(),
        rationale: rationale.to_string(),
        sig: Vec::new(),
    };
    if let Some(seed) = master_hex {
        let signer = HybridSigner { role: 'V', master_hex: seed.to_string() };
        v.sig = signer.sign(&v.signing_bytes());
    }
    v.encode()
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
            redline_touch,
            timestamp: 1_700_000_000,
            sig: Vec::new(),
        }
        .encode();
        let att_digest = digest32(&attest);
        let verdict = Verdict {
            diff_attest_sha3: att_digest,
            recomputed_diff_sha3: diff_sha,
            verdict: 0x01,
            key_v_anchor_id: key_v,
            context_descriptor: "test".into(),
            rationale: "ok".into(),
            sig: Vec::new(),
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
        assert_eq!(evaluate_gate(&a, &v, false, None), GateVerdict::Green);
    }

    #[test]
    fn gate_red_missing_attestation_note() {
        let (_, v) = valid_pair([0x99; 32], 0);
        assert!(matches!(
            evaluate_gate(&[], &v, false, None),
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
            context_descriptor: "forge".into(),
            rationale: "self-signed".into(),
            sig: Vec::new(),
        }
        .encode();
        assert!(matches!(
            evaluate_gate(&a, &forged, false, None),
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
            .position(|w| w[0] == 0x09 && u32::from_be_bytes([w[1], w[2], w[3], w[4]]) as usize == V1_RESIDUE.len())
            .expect("residue present in valid pair");
        let rec_len = 5 + V1_RESIDUE.len();
        v.drain(idx..idx + rec_len);
        assert!(Verdict::decode(&v).is_none());
        // if someone bypasses decode, the gate must still catch the missing residue
        assert!(matches!(
            evaluate_gate(&a, &v, false, None),
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
            evaluate_gate(&a, &v, true, None),
            GateVerdict::Red(s) if s.contains("GREEN")
        ));
    }

    #[test]
    fn gate_red_redline_touch_mismatch() {
        // CI recomputes red-line touch, author lied about bit
        let (a, v) = valid_pair([0x99; 32], 0);
        // ci says it DOES touch red-line, but author signed redline_touch=0
        assert!(matches!(
            evaluate_gate(&a, &v, true, None),
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

        let attest = build_attestation(commit, diff_sha3, base, key_k, 0, 1_700_000_000, None);
        // verifier's independent recomputation of the diff digest == author's
        let verdict = build_verdict(&attest, diff_sha3, key_v, "h3-breach-probe", "no breach found", None);

        assert_eq!(evaluate_gate(&attest, &verdict, false, None), GateVerdict::Green);
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
        let attest_a = build_attestation(commit, diff_a, base, key_k, 0, 1_700_000_000, None);
        // a verdict honestly built against a DIFFERENT attestation (diff_b)
        let attest_b = build_attestation(commit, diff_b, base, key_k, 0, 1_700_000_000, None);
        let mismatched = build_verdict(&attest_b, diff_a, key_v, "h3", "mismatched binding", None);

        // gate must REJECT: verdict.diff_attest_sha3 != digest32(attest_a)
        assert!(matches!(
            evaluate_gate(&attest_a, &mismatched, false, None),
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
        let k = HybridSigner { role: 'K', master_hex: "deadbeef".into() };
        assert!(k.signed());
        // No binary present (or present but unknown master) => fail-closed.
        let bytes = b"v1-diff-attest-bytes";
        let sig = k.sign(bytes);
        // Fail-closed: either an empty sig, or — if a binary IS wired — a
        // valid sig that must verify roundtrip. We only assert non-panic +
        // that verify of an empty/garbage sig is false when no real sig exists.
        if sig.is_empty() {
            assert!(!k.verify_signature("role=K deadbeef", bytes, ""));
        }
        // UnsignedSigner must still report unsigned (Phase-1 retained).
        let u = UnsignedSigner;
        assert!(!u.signed());
    }

    /// DETERMINISTIC TEST ANCHOR (NOT a committed trust root): the REAL end-to-end
    /// falsifier for the P06 key_V `HybridSigner` feature (bugs #1/#2/#3).
    ///
    /// It proves, with the real `bebop2-kv` CLI:
    ///   1. `build_attestation`/`build_verdict` produce notes carrying REAL
    ///      hybrid (Ed25519⊕ML-DSA-65) signatures via `HybridSigner` (sign path,
    ///      bug #2 — TLV `sig` field populated).
    ///   2. `evaluate_gate(.., signed=Some(_))` actually shells `bebop2-kv verify`
    ///      and GATEs on its result: authentic notes => GREEN (bug #1 fixed;
    ///      `pub_anchor_line` uses the real `genkeys` subcommand, bug #3 fixed).
    ///   3. A 1-bit-flipped signature is fail-closed => RED.
    ///
    /// Requires the `bebop2-kv` CLI (set `BEBOp_REPO_ROOT` or `V1_KV_BIN`).
    /// If absent the test is skipped (prints a notice) rather than failing — so
    /// the unsigned policy suite stays green on machines without the crypto CLI.
    ///
    /// NOTE: this is a REAL passing test (no `#[ignore]`), gated only on binary
    /// availability; when the CLI is wired (as in CI) it exercises the actual
    /// crypto slot and must be GREEN.
    #[test]
    fn real_hybrid_sig_roundtrip_and_corruption_rejected() {
        // Locate the crypto CLI up-front; skip (don't fail) if not wired.
        let bin = if let Ok(p) = std::env::var("V1_KV_BIN") {
            let pb = std::path::PathBuf::from(&p);
            if pb.exists() { Some(pb) } else { None }
        } else if let Ok(root) = std::env::var("BEBOp_REPO_ROOT") {
            let cands = [
                "target/debug/bebop2-kv",
                "target/release/bebop2-kv",
                "bebop2/target/debug/bebop2-kv",
            ];
            cands.iter().find_map(|c| {
                let pb = std::path::Path::new(&root).join(c);
                pb.exists().then_some(pb)
            })
        } else {
            None
        };
        let bin = match bin {
            Some(b) => b,
            None => {
                eprintln!(
                    "SKIP real_hybrid_sig_roundtrip_and_corruption_rejected: \
                     bebop2-kv not found (set BEBOp_REPO_ROOT or V1_KV_BIN)"
                );
                return;
            }
        };
        // Make the whole signing/verify path use this binary.
        std::env::set_var("V1_KV_BIN", &bin);

        let master = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

        // Derive the two role anchors from the master (the real `genkeys` path),
        // and lay down a kv-genesis.txt the gate's verifier will load.
        let k_signer = HybridSigner { role: 'K', master_hex: master.to_string() };
        let v_signer = HybridSigner { role: 'V', master_hex: master.to_string() };
        let k_line = k_signer.pub_anchor_line();
        let v_line = v_signer.pub_anchor_line();
        assert!(!k_line.is_empty() && !v_line.is_empty(), "anchors must derive");

        let tmp = std::env::temp_dir().join(format!("ci-truth-e2e-{}", std::process::id()));
        let _ = std::fs::create_dir_all(tmp.join("config"));
        // kv-genesis.txt format (BLUEPRINT-P06 §2): "<pub-hex> role=K" / "<pub-hex> role=V".
        // `pub_anchor_line()` ALREADY emits exactly that shape ("<hex> role=K"),
        // so write the full lines verbatim — do NOT re-parse (a naive
        // `rsplit(' ').next()` would grab the trailing "role=K" token and emit
        // "role=K role=K", which `load_kv_genesis` rejects as a bad anchor).
        let genesis = tmp.join("config").join("kv-genesis.txt");
        std::fs::write(&genesis, format!("{k_line}\n{v_line}\n"))
            .expect("write kv-genesis.txt");

        // Build a valid, SIGNED note pair (real hybrid sigs), as an honest
        // diff-owner + downstream verifier would before writing the git notes.
        let commit: [u8; 32] = [0xaa; 32];
        let diff_sha3: [u8; 32] = [0x99; 32];
        let base: [u8; 32] = [0u8; 32];
        let key_k: [u8; 32] = [0x11; 32]; // anchor id; != key_v by construction
        let key_v: [u8; 32] = [0x22; 32];
        let attest_raw = build_attestation(commit, diff_sha3, base, key_k, 0, 1_700_000_000, Some(master));
        // verifier's independent recomputation of the diff digest == author's
        let verdict_raw = build_verdict(
            &attest_raw,
            diff_sha3,
            key_v,
            "e2e-real-sig",
            "signed end-to-end",
            Some(master),
        );

        // Decode to confirm the `sig` field is actually populated (bug #2).
        let attest = DiffAttestation::decode(&attest_raw).expect("decode attest");
        let verdict = Verdict::decode(&verdict_raw).expect("decode verdict");
        assert!(!attest.sig.is_empty(), "key_K DiffAttestation MUST carry a real sig");
        assert!(!verdict.sig.is_empty(), "key_V Verdict MUST carry a real sig");

        let signed = SignedGate {
            repo_root: tmp.clone(),
            master_hex: master.to_string(),
        };

        // (1)+(2): authentic signed notes => GREEN through REAL verification.
        assert_eq!(
            evaluate_gate(&attest_raw, &verdict_raw, false, Some(&signed)),
            GateVerdict::Green,
            "real signed notes must verify GREEN"
        );

        // (3): flipping 1 bit of the key_V verdict signature MUST fail-closed.
        let mut bad_verdict_raw = verdict_raw.clone();
        let last = bad_verdict_raw.len() - 1;
        bad_verdict_raw[last] ^= 0x80;
        assert!(matches!(
            evaluate_gate(&attest_raw, &bad_verdict_raw, false, Some(&signed)),
            GateVerdict::Red(s) if s.contains("verify") || s.contains("signature")
        ), "1-bit-flipped verdict sig must be RED");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
