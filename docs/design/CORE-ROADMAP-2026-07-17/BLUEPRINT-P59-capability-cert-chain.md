# BLUEPRINT P59 — Capability-cert chain & crypto-agility (2026-07-18)

> **Wave W1 foundation blueprint.** One coherent, independently-buildable unit against the 20-point
> contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Scope source: `SYNTHESIS-LAUNCH-BLOCKERS-
> 2026-07-18.md` §5 (W1 table, row **P59**), cross-cut reasoning in **X4** (crypto-agility reframed
> from invention to adoption) and **X8** (identity chain is upstream of claim, couriers, owners,
> wallets). Technical grounding: `OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md` §2–§3. Format
> precedent: `BLUEPRINT-P51-open-map-routing.md`.
>
> **One sentence:** extend the identity/capability substrate that already exists in the kernel
> (`cap.rs`'s UCAN-subset delegation chain + `pq/dsa.rs` ML-DSA-65 + `pq/root_delegation.rs`) into a
> **biscuit-style, hybrid-signed, algorithm-agile capability-cert chain** — self-signed hub roots by
> default, owner multi-hub delegation, standards-mapped suite versioning, and a mandatory independent
> adversarial-review gate before it ships.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every claim below was read from source **this pass**, not
> inherited. The single most important correction this section makes: **there is no kernel struct
> literally named `HybridSigner`.** The roadmap/synthesis shorthand "the existing HybridSigner"
> denotes a real, three-part surface that P59 builds on; naming it precisely is a prerequisite for
> writing a correct blueprint.

### 0.1 What "the existing HybridSigner" actually is (three real pieces)

| Roadmap shorthand | Real code (verified this pass) | What it provides |
|---|---|---|
| the hybrid **signature seam** | `kernel/src/ports/agent/cap.rs:80-95` — `trait SignatureVerifier { sign_classical / verify_classical / sign_pq / verify_pq / classical_public / pq_public }` | The injection seam. Production injects real bebop2 **Ed25519 (classical) + ML-DSA-65 (PQ)**; `RefSigner` (`cap.rs:102`) is the in-tree SHA3 reference (NOT production crypto). |
| the **PQ half** | `kernel/src/pq/dsa.rs` — `MlDsa65Pk` (`:979`), `MlDsa65Sk` (`:982`), `MlDsa65Sig` (`:985`), `keygen` (`:990`), `sign` (`:996`), `verify` (`:1003`) | Real **FIPS-204 ML-DSA-65**, KAT-gated byte-exact vs NIST ACVP vectors (`pq/kat/acvp/*.json`). No invented crypto. |
| the **AND-verify floor** | `kernel/src/ports/agent/cap.rs:42-49` — `enum HybridPolicy { RequireBoth }`, "the ONLY policy; there is no weaker code point" | Both legs must verify. This IS the "AND-verification semantics" the P59 scope mandates, already a type-level floor. |
| the literal name **`HybridSigner`** | `tools/ci-truth/src/v1.rs:11-12` — "the crypto slot is FILLED by `HybridSigner`, which shells the external `bebop2-kv` hybrid CLI (Ed25519⊕ML-DSA-65, RequireBoth)"; TLV sig field tags `0x07` (key_K) / `0x08` (key_V) (`v1.rs:200,255`) | A **dev-tooling CI adapter**, out of the kernel graph. This is the P06 `key_V` closure (memory: commit `58987d79d`, 2026-07-18). It is NOT the runtime identity primitive; it is the CI truth-attestation signer. |

**Consequence for P59:** the blueprint builds on the `SignatureVerifier` seam + `pq/dsa.rs`, NOT on
`tools/ci-truth`. Where this document says "hybrid-sign," it means "produce **both** a `sign_classical`
(Ed25519) and a `sign_pq` (ML-DSA-65) signature, verified under `HybridPolicy::RequireBoth`."

### 0.2 The capability chain is ~70% already built — P59 extends, does not green-field (standard §2 item 19)

`kernel/src/ports/agent/cap.rs` (682 lines) already contains a **UCAN-subset, anchor-rooted,
attenuating delegation chain** with revocation and gossip. Verified this pass:

| Existing element | `cap.rs` cite | What it already does | P59 relationship |
|---|---|---|---|
| `NodeId(pub [u8;32])` + `from_keys(pq_pub, classical_pub)` | `:54-65` | Identity = hash of **both** keys; "changing EITHER public key changes the id — no CA, no assignable owner." | **Reuse verbatim.** This is the §17.7 "self-signed root, no CA" property already in code. |
| `Capability` | `:169-219` | subject_key (Ed25519 32B), subject_key_pq (ML-DSA-65 1952B = `ML_DSA_65_PK_LEN`, `:40`), scope, nonce, expiry; `canonical_bytes_tlv` (`:204`, domain-separated length-prefixed TLV); `is_fresh(now)` (`:217`). | **Reuse; add `alg_suite` field** (§3, §6). Already dual-key. |
| `Delegation` (one chain link) | `:288-371` | "a single delegation link in a **UCAN-subset chain** (mirrors bebop2)." issued_by, subject, scope, **effect (⊆ scope = attenuation)**, expiry, nonce, `signature: Vec<u8>`; `canonical_bytes` with `DOMAIN_DELEGATION` (`:319`), length-prefixed TLV. | **EXTEND — the real gap:** `Delegation::sign` calls `verifier.sign_classical` **only** (`:347`); `verify_signature` calls `verify_classical` **only** (`:369`). **Delegation links are Ed25519-only today** — they do NOT carry the PQ half. Closing this to hybrid `RequireBoth` is P59's central build item (M2). |
| `AnchorRoster` | `:373-408` | Frozen trust-anchor set; `&mut`-gated `enroll`/`remove` (out-of-band operator/genesis only); `contains`; `snapshot_sorted`. | **Reuse; generalize the anchor from "operator ML-DSA root" to "any self-signed hybrid root (hub or owner)"** (§3, M4). |
| `RevocationSet` | `:410-457` | Append-only `revoke_key` + `revoke_capability(cap_hash)`; `is_revoked_key`; **`merge` (anti-entropy union — i.e. gossip)** (`:438`); `drop_anchor`. | **Reuse verbatim** as the owner-fleet revocation-blob substrate (§5). The gossip primitive the revocation design needs already exists. |
| `revocation_hash` / `pq_key_id` | `:460`, `:465` | SHA3-256 over canonical TLV / over PQ key. | Reuse. |
| `verify_chain` | `:486-531` | Anchor-rooted; every link signed by `issued_by`; child == next issuer; **scope attenuates only** (`is_subset_of`, `:515`); tail binds `cap.subject_key`; requested effect ⊆ tail effect. `ChainError { UnknownIssuer, BadSignature, Expired, ScopeViolation }` (`:469-479`). | **EXTEND** to hybrid link verification + `alg_suite` gate + revocation check (M2/M3). The attenuation/expiry/binding logic is reused unchanged. |
| `Scope` | `kernel/src/ports/agent/scope.rs:191-334` | `to_tlv_bytes` (`:211`), **strict fail-closed decode** ("an unknown resource/action" rejected, `:221`), `is_subset_of`. Resource/Action enums = the Datalog-adjacent scope-fact vocabulary. | **Reuse** as the "Datalog-style scope facts" carrier. dowiz's scope model is a bounded lattice, not a general Datalog engine (§2 anti-scope). |
| domain separators | `cap.rs:31-33` | `DOMAIN_CAPABILITY = b"dowiz.agent.cap\x01"`, `DOMAIN_FRAME`, `DOMAIN_DELEGATION` — cross-type-confusion resistance ("a capability-domain signature can never verify" as a frame, `:249-251`). | **Reuse; add a per-block suite tag** into the canonical bytes so cross-**suite** confusion is also blocked (M2, §7). |

### 0.3 The two other existing roots (the delegation model is not blank either)

- `kernel/src/pq/root_delegation.rs` (192 lines): `RootDelegationPolicy { OperatorSigned, Overlay{depth<=1}, Deferred }` (`:22-35`), `verify_root` (`:63`), `sign_root` (`:107`), `operator_root_keygen` (`:113`). **ML-DSA-65-only today** (`verify_root` calls `pq::dsa::verify`, `:71`); the **`depth <= 1` invariant is enforced at construction** (`overlay()`, `:44-49`) so a double-hop is unrepresentable. This is operator ruling A / `DECISIONS.md` D10 — the **operator** mesh-root overlay policy. P59 does not overturn it; it adds a **second, parallel anchor kind** (self-signed hub/owner roots) alongside it (§2.4, honest reconciliation).
- `kernel/src/pq/codesign.rs` (239 lines): `PinnedRoot` (single trusted root, `:30`), `UpdateBlob { Signed | Unsigned }` RED gate (`:40`), `ApplyLedger` append-only replay guard (`:78`), `apply` refuses unsigned/tampered/wrong-root (`:113-127`). Reuses `pq::envelope` (ML-DSA-65 `seal`/`open`). This is the **pinned-single-root + append-only-ledger pattern** P59's self-signed-root design mirrors for the hub genesis root.
- `kernel/src/pq/hybrid.rs`: the hybrid **KEM** (X25519 + ML-KEM-768), with a RED "no classical fallback" gate and a key-confirmation tag (`:87-100`). Not a signer — but the **`RequireBoth`/no-downgrade precedent** for confidentiality, mirrored here for authenticity.

### 0.4 The B4 / SSR-2020 precedent — why the adversarial-review gate is mandatory (§8)

Memory (`crypto-safe-first-pass-2026-07-14.md`, mesh arc): the bebop `verify_batch` shortcut was
**forgeable** — an independent reviewer built and ran an Ed25519 batch-verify mixed-order **SSR-2020**
forgery (a small-order filter blind to `R = R0 + T`) that the pre-fix code *wrongly accepted*; fixed
(`6541ae8`, walked-back `84a1e272d`) by confirming **every** batch-accept through full single-verify —
"batching now has NO throughput benefit (honest, re-benched) — correctness over speed." R3 §7 risk #3
names this exact lesson for the cert chain: re-implementing biscuit's signed-block attenuation over a
hybrid signer is *new crypto-adjacent code* and "needs genuine adversarial review — canonicalization
of blocks, block-reordering/truncation attacks, cross-suite confusion — before it holds owner
delegation authority. Do not treat 'it's just biscuit-shaped' as safety." **This is a hard DoD gate,
not a suggestion (§8, §9).**

---

## 1. Standards & prior-art map — adoption, not invention (standard §2 item 19; SYNTHESIS X4)

The single most consequential research finding (R3 §3.1): **dowiz's exact hybrid is already a
standardized algorithm suite.** P59 therefore *adopts a registry*, it does not *invent a versioning
scheme*. Each row is a real, cited external artifact and the exact way P59 uses it.

| Prior art | What it really is (cited from R3) | How P59 uses it — and what it does NOT take |
|---|---|---|
| **`draft-ietf-lamps-pq-composite-sigs` v19** | `id-MLDSA65-Ed25519-SHA512` = **OID `1.3.6.1.5.5.7.6.48`**, signature label `COMPSIG-MLDSA65-Ed25519-SHA512`, SHA-512 pre-hash. **Verification is AND**: "valid iff **all** component signatures validated." Versioning mechanism *is* the OID/label. (R3 §3.1) | **Adopt the registry.** `alg_suite v1 = MLDSA65-Ed25519` maps to `1.3.6.1.5.5.7.6.48`. AND-verify matches the existing `HybridPolicy::RequireBoth` + the B4 lesson. **NOT taken:** we do NOT emit X.509/CMS composite blobs on the wire — we carry a compact internal `alg_suite: u16` that *maps* to the OID (draft is pre-RFC; OID may shift — R3 risk #6 → one-line remap, §6.2). |
| **biscuit-auth (Eclipse/Rust)** | Public-key capability token: **offline delegation by attenuation** — "a new valid token created from another by attenuating its rights, by its holder, without communicating with anyone"; **public-key, not shared-secret** — "any application holding the root **public** key can verify" (strictly better than macaroons, which need the root **secret**); **signed-block chain + Datalog checks** — each attenuation appends a signed block carrying facts/rules/checks. (R3 §2.3) | **Adopt the construction** — append-only chain of independently-signed blocks, each narrowing authority, verified against a root **public** key. This is already `cap.rs`'s `Delegation`+`verify_chain` shape. **NOT taken:** stock biscuit signs blocks with **Ed25519 only** (some secp256r1) — it "cannot be taken as-is under §17.2." We re-implement the construction over the hybrid seam. We also do NOT embed a general Datalog interpreter — dowiz's `Scope` lattice is a bounded, fail-closed subset (§2 anti-scope). |
| **SPIFFE (spec, not SPIRE)** | trust domain = root of trust; SVID-style stable IDs; **federation = exchange trust bundles** (root public-key sets); optional **UpstreamAuthority** co-sign. (R3 §2.1) | **Adopt the vocabulary/topology:** hub self-signed root = trust domain; `NodeId` = stable SVID-style id; optional dowiz detached co-sign = UpstreamAuthority. **NOT taken (disqualified):** the **SPIRE runtime** — Go/K8s-heavy daemon, **no ML-DSA/PQC** (open unshipped issue #6975), X.509-SVID has "no clean slot for a capability/attenuation graph." R3's two constraints (Rust-native + PQC-from-Wave-0) are *individually* disqualifying. |
| **bare X.509 / `rcgen`** | Rust X.509/CSR crate; can sign child certs but "performs **no validation that the issuer is a CA**," AKI/chain bugs fixed only in 0.13.1, and **no ML-DSA**. (R3 §2.2) | **NOT taken as the identity design.** `rcgen` is plumbing usable *only if* a separate X.509 wire format is later needed for TLS interop; it "does not solve the §16.48 delegation requirement and carries no PQ half." P59's identity object is the hybrid capability chain, not X.509. |
| **OpenSSH 6.8+ `UpdateHostKeys` / `hostkeys@openssh.com`** (→ `draft-ietf-sshm-hostkey-update`) | Server **publishes all its host keys including new algorithms**; clients learn new keys *while still trusting old*; after an **overlap window** the deprecated key is removed. Deliberate overlap so no client is stranded. (R3 §3.2) | **Adopt overlap-rotation** for fleet-wide algorithm migration: a hub dual-publishes new-suite + old-suite credentials for a hub-local window `W`, then retires old. No flag-day, no mesh fork (§6.3). |
| **TLS 1.3 suite negotiation** | Advertise supported algorithm lists, agree the strongest mutually supported; downgrade protection binds the highest advertised suite into the transcript. (R3 §3.2-3.3) | **Adopt suite negotiation + downgrade binding** at every hub↔peer handshake: advertise a `SuiteList`, choose the strongest common, bind the advertised list into the signed transcript so an attacker cannot force both sides to a weaker suite (§6.4). |

**The blueprint's job therefore shrinks (SYNTHESIS X4) to: wire format + negotiation + rotation +
hybrid-signing the existing chain — the algorithm-identification problem is already solved upstream.**

---

## 2. Scope — what P59 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P59 OWNS

1. **Hybrid-signing the delegation chain.** Extend `cap.rs::Delegation` (and `verify_chain`) so every
   link carries BOTH an Ed25519 and an ML-DSA-65 signature, verified under `RequireBoth`. Close the
   Ed25519-only gap (0.2, M2).
2. **The `alg_suite` field + suite→OID registry** on every signed block (`Capability`, `Delegation`,
   root attestation) — the crypto-agility adoption (M1, §6).
3. **Self-signed hub roots by default** (§17.7): a hub is its own hybrid root (genesis anchor);
   optional **detached** dowiz co-sign for claim-flow convenience only, never load-bearing (M4).
4. **Owner multi-hub delegation** (§16.48): a root credential the owner holds that mints/attenuates
   child hub-certs — self-service add/modify/revoke, **no dowiz account needed** (M5).
5. **Suite negotiation with downgrade binding** + **SSH-style overlap rotation** for fleet migration
   (M6, §6).
6. **Revocation: short-TTL re-mint (primary) + owner-signed revocation blob gossiped via the existing
   `RevocationSet::merge` (immediate) + optional dowiz transparency feed** — under the closed §4-B
   ruling that **self-custody is absolute (no break-glass anywhere)** (M7, §5).
7. **The mandatory independent adversarial-review gate** as a DoD-blocking checkpoint (§8, §9).

### 2.2 P59 does NOT own (anti-scope — prevents collision & scope-creep)

- **Cloudflare tunnel automation, Hetzner warm pool, the golden image, the claim service** → **P67**
  (provisioning). P59 provides the *pre-minted self-signed root* + *owner-root handoff / child-block
  append*; P67 bakes and hands them out (SYNTHESIS X8/X9).
- **The owner UI surface** that fans out N hub connections under one root → **P70**. P59 provides the
  root/child primitive; P70 renders it.
- **The courier working surface** → **P71** (extends P52). P59 provides the courier-cert mint/verify;
  P71 consumes it.
- **Wallet device-to-device transfer** → **P66**. Explicitly a *separate, simpler* mechanism sharing
  only the primitive *family* (X25519/HKDF/AEAD). **Do NOT merge wallet-transfer crypto into the cert
  chain** (SYNTHESIS X8, verbatim). P59 reuses only the *self-custody framing*, not the code.
- **A general Datalog engine.** dowiz scope facts are the bounded `Scope` lattice (`scope.rs`), not a
  Turing-complete policy language. "Datalog-style" = attenuating fact subsets, nothing more.
- **X.509/TLS-cert emission.** Only if a later blueprint needs TLS interop (then `rcgen` as plumbing);
  not P59.
- **The operator mesh-root `RootDelegationPolicy` (`root_delegation.rs`).** P59 leaves ruling A /
  D10's `depth<=1` operator overlay untouched (§2.4).

### 2.3 Dependencies (standard §2 item 7 — named by artifact)

**Existing input (hard dependency, already in tree):**
- The hybrid signature seam `SignatureVerifier` + `RefSigner` (`cap.rs:80-165`) — *"the existing
  HybridSigner"* per the P59 charter.
- ML-DSA-65 primitive `pq/dsa.rs` (FIPS-204, KAT-gated).
- The UCAN-subset chain: `Delegation`/`AnchorRoster`/`RevocationSet`/`verify_chain` (`cap.rs`), `Scope`
  (`scope.rs`).
- Production Ed25519 (bebop2-injected via the seam; the kernel does not link bebop2 directly — `cap.rs`
  header note).

**Consumers (this blueprint is upstream of — SYNTHESIS X8, "one crypto surface, four consumers"):**
- **P67** hub provisioning & claim — pre-minted self-signed roots at snapshot time; claim hands an
  owner root or appends a child block.
- **P70** owner surface — the owner multi-hub root; add/modify/revoke child hub-certs.
- **P71** courier surface — courier-cert mint/verify (§16.3).
- **M1** (first real order) hub certs — the first hand-onboarded hub runs on directly-issued certs
  from the *same chain code*, so P59 is in W1 while P67's automation is W3 (SYNTHESIS §3.2.3).

### 2.4 Honest reconciliation: two anchor kinds, one `depth<=1` shape (standard §2 item 6)

There is a genuine tension a careless design would paper over. `root_delegation.rs` already caps the
**operator** mesh-root overlay at `depth<=1` (ruling A / D10). §16.48 asks for an **owner** root that
delegates to N child hubs. These are **not the same anchor**:

- The **operator root** is the mesh sovereignty floor (one operator, ML-DSA-only overlay, `depth<=1`).
  P59 **does not touch it**.
- The **owner root** (§17.7/§16.48) is a *self-signed hybrid* root the vendor holds. It delegates
  **owner → hub** as a **single attenuating hop** with `may_delegate = false` on every child (R3 §2.4
  verbatim: children carry `may_delegate: false`). Fan-out is *breadth* (N hubs), not *depth* — each
  hub is a leaf; no hub re-delegates.

So the owner model is **also `depth<=1`** — it reuses the exact invariant, rooted at a different
anchor. P59 generalizes `AnchorRoster` to hold *self-signed hybrid roots* (hub or owner) in addition
to the operator root, and adds a construction-time `may_delegate` gate mirroring `overlay()`'s
`MaxDepthExceeded`. **No operator ruling is overturned; the depth ceiling is preserved and made
anchor-parametric.** (This is the one design choice §4-B/§16.48 could otherwise collide on — resolved
here so no consumer discovers it late.)

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

All new types live in a **new module `kernel/src/pq/cert_chain.rs`** (keeps `cap.rs` from growing an
eighth responsibility; imports the `cap.rs` chain types and `pq::dsa`). Constants are named, never
magic.

```rust
// kernel/src/pq/cert_chain.rs  (NEW)

/// Algorithm-suite identifier carried in every signed block. Adoption of the
/// composite-sigs registry (draft-ietf-lamps-pq-composite-sigs v19), NOT a bespoke scheme.
/// The `u16` is an INTERNAL enum that MAPS to the OID — the mapping is the only place the
/// pre-RFC OID lives, so an OID shift is a one-line remap (R3 risk #6), never a cert-format
/// migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AlgSuite {
    /// v1 — the CURRENT dowiz hybrid. Maps to id-MLDSA65-Ed25519-SHA512.
    MlDsa65Ed25519 = 0x0001,
    // v2+ reserved (e.g. MlDsa87Ed448, MlDsa65SlhDsa) — added by registering a new code point,
    // NEVER by forking the wire format. Unknown code points are REJECTED (fail-closed), never
    // "best-effort" verified.
}

/// The standardized OID string for a suite (composite-sigs registry). ONE mapping table —
/// the single remap point if the pre-RFC OID changes.
pub const OID_MLDSA65_ED25519_SHA512: &str = "1.3.6.1.5.5.7.6.48";
/// The composite-sigs signature label (transcript/domain use).
pub const LABEL_MLDSA65_ED25519_SHA512: &[u8] = b"COMPSIG-MLDSA65-Ed25519-SHA512";

impl AlgSuite {
    pub fn oid(self) -> &'static str { match self { Self::MlDsa65Ed25519 => OID_MLDSA65_ED25519_SHA512 } }
    /// Fail-closed decode: an unknown u16 is None, mirroring scope.rs's strict decode.
    pub fn from_u16(v: u16) -> Option<Self> { match v { 0x0001 => Some(Self::MlDsa65Ed25519), _ => None } }
}

/// Per-block domain tag that binds the SUITE into the signed bytes, so a signature made under
/// one suite can never be replayed as another (cross-suite confusion — R3 risk #3). Prepended
/// to canonical bytes alongside the existing DOMAIN_DELEGATION separator.
pub const DOMAIN_SUITE_PREFIX: &[u8; 16] = b"dowiz.pq.suite\x01\x01";

/// A hybrid signature pair over one block. RequireBoth: verification is AND (composite-sigs +
/// B4/SSR-2020 lesson). A missing OR non-verifying half is total failure — never a soft pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSig {
    pub alg_suite: AlgSuite,
    pub classical: [u8; 64],   // Ed25519 signature
    pub pq: Vec<u8>,           // ML-DSA-65 signature (envelope::SIG_LEN)
}

/// A self-signed hybrid root (§17.7). Block 0 of a chain: signed by its OWN keypair.
/// This is the trust-domain root — NO dowiz needed. Mirrors codesign::PinnedRoot but hybrid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfSignedRoot {
    pub classical_pub: [u8; 32],
    pub pq_pub: Vec<u8>,               // ML-DSA-65 pub (1952 B = ML_DSA_65_PK_LEN)
    pub node_id: NodeId,               // == NodeId::from_keys(pq_pub, classical_pub) — MUST match
    pub alg_suite: AlgSuite,
    pub self_sig: HybridSig,           // signature over canonical root bytes by its own keys
    pub not_after: u64,                // root TTL (monotonic tick); short by policy (§5)
}

/// Optional detached dowiz co-signature over a root's public keys (§17.7). Its ABSENCE never
/// invalidates the root — it is a second voucher for relying parties that trust dowiz, nothing
/// more. Convenience only (claim-flow), never load-bearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowizCoSign {
    pub over_node_id: NodeId,
    pub sig: HybridSig,                // dowiz-key hybrid sig over the root's node_id + suite
}

/// The negotiated-suite handshake message (TLS-style). Advertised suites, strongest-common
/// selection, and the transcript binding that defeats downgrade (§6.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteAdvertisement {
    pub offered: Vec<AlgSuite>,        // in preference order, strongest first
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedSuite {
    pub chosen: AlgSuite,
    /// SHA3-256 over BOTH parties' full offered lists — bound into the session's first signed
    /// frame so a MITM cannot strip strong suites without breaking the signature (downgrade bind).
    pub transcript_hash: [u8; 32],
}

/// SSH-style overlap rotation state for fleet-wide suite migration (§6.3). A hub in `Overlapping`
/// publishes BOTH credentials for a hub-local window; verifiers learn the new; then `retire`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotationState {
    Stable  { suite: AlgSuite },
    Overlapping { old: AlgSuite, new: AlgSuite, overlap_until: u64 }, // window W is hub-local policy
}

/// Signed, gossip-able revocation blob (§5). Reuses cap.rs::RevocationSet as the merge substrate;
/// this is the SIGNED envelope an owner root publishes to its own fleet. No dowiz relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationBlob {
    pub issuer_root: NodeId,           // the owner/hub root that authored it
    pub revoked_keys: Vec<[u8; 32]>,
    pub revoked_cap_hashes: Vec<[u8; 32]>,
    pub seq: u64,                      // monotonic; a later seq supersedes (LWW within one issuer)
    pub not_after: u64,
    pub sig: HybridSig,                // hybrid sig over canonical blob bytes by issuer_root
}
```

**Named constants (policy values — §2 item 4; exact values are engineering-decision E, §5.3):**

```rust
pub const ROOT_TTL_TICKS: u64        = /* short — see §5.3 */;  // owner/hub root validity
pub const CHILD_CERT_TTL_TICKS: u64  = /* shorter than root */; // per-hub child cert validity
pub const OVERLAP_WINDOW_TICKS: u64  = /* hub-local W */;       // suite-rotation overlap
pub const REVOCATION_BLOB_GOSSIP_TICKS: u64 = /* cadence */;    // owner→fleet push cadence
pub const MAX_CHAIN_LEN: usize       = 4;   // hard cap: root + ≤3 hops; truncation/DoS guard
pub const MAX_DELEGATION_DEPTH: u8   = 1;   // owner→hub single hop, may_delegate=false (§2.4)
```

---

## 4. Build items — spec → RED test → code, each with adversarial cases (standard §2 items 2, 3, 5)

Each item: **spec first (types/invariant), a test that goes RED before the change, code, then GREEN.**
State transitions modeled as events; tests assert on the sequence, not just end-state (item 3).

### 4.1 M1 — `AlgSuite` field + suite→OID mapping (the crypto-agility adoption)

- **Spec:** every signed block carries `alg_suite: AlgSuite`; canonical bytes prepend
  `DOMAIN_SUITE_PREFIX || (alg_suite as u16 le)` so the suite is *inside* the signature. `from_u16`
  is fail-closed.
- **RED test `red_unknown_suite_rejected`:** a block with `alg_suite` raw `0x0002` (unregistered) →
  `AlgSuite::from_u16` returns `None` → verification returns `CertError::UnknownSuite`. RED today
  (no field exists), GREEN after.
- **RED test `red_suite_swap_breaks_sig`:** sign a block as v1, then flip the stored `alg_suite` code
  point without re-signing → signature MUST fail (suite is bound into the bytes). Guards cross-suite
  confusion at the field level.
- **Adversarial:** `red_oid_remap_is_one_line` — a compile-time test asserting `OID_MLDSA65_ED25519_
  SHA512` is referenced in exactly one `match` arm (grep-asserted in the test), proving the pre-RFC
  remap surface is a single line (R3 risk #6).

### 4.2 M2 — hybrid-sign the delegation chain (close the Ed25519-only gap — the central item)

- **Spec:** `Delegation` gains a `HybridSig` (replacing the classical-only `signature: Vec<u8>`);
  `Delegation::sign` produces BOTH `sign_classical` AND `sign_pq`; `verify_signature` requires BOTH
  under `HybridPolicy::RequireBoth`. `verify_chain` verifies both legs per link. Backward shape kept
  (attenuation/expiry/tail-binding logic unchanged, `cap.rs:502-529`).
- **RED test `red_classical_only_link_rejected`:** construct a link with a valid Ed25519 leg but a
  **missing/zeroed PQ leg** → `verify_chain` → `ChainError::BadSignature`. This test is RED against
  today's code (which accepts classical-only, `cap.rs:369`) and GREEN after — it is the literal
  regression that proves the gap is closed. **Add to `REGRESSION-LEDGER.md`.**
- **RED test `red_pq_forged_classical_valid`:** valid Ed25519, PQ signature copied from a *different*
  message → PQ `verify` fails → whole link fails. Proves AND, not OR (the B4 lesson in miniature).
- **Adversarial `red_block_reordering`:** take a valid 3-link chain, swap links 1 and 2 → `child ==
  next issuer` check (`cap.rs:511`) breaks → `UnknownIssuer`. Proves reorder-resistance.
- **Adversarial `red_chain_truncation`:** drop the tail link so a broader intermediate becomes the
  tail → tail no longer binds `cap.subject_key` → `ScopeViolation`; plus `MAX_CHAIN_LEN` rejects an
  over-long spliced chain. Proves truncation-resistance (R3 risk #3).

### 4.3 M3 — chain verification with revocation + `alg_suite` gate

- **Spec:** `verify_chain_hybrid(verifier, roster, revset, chain, cap, now)` = existing `verify_chain`
  + (a) every link/cap `alg_suite` must be registered and consistent across the chain, (b) no link
  key nor `revocation_hash(cap)` appears in `RevocationSet`. Fail-closed.
- **RED test `red_revoked_key_rejected`:** valid chain, then `revset.revoke_key(tail.subject)` →
  verification → `ChainError::Revoked`. RED today (no revocation check in `verify_chain`), GREEN after.
- **RED test `red_revoked_cap_hash_rejected`:** revoke by `revocation_hash(cap)` → rejected even
  though keys are live (single-cap revocation).
- **Adversarial `red_expired_root_rejected`:** `SelfSignedRoot.not_after < now` → `Expired` even with a
  perfect chain (TTL is load-bearing, §5).

### 4.4 M4 — self-signed hybrid roots by default + optional detached dowiz co-sign (§17.7)

- **Spec:** `SelfSignedRoot::mint(seed)` derives a hybrid keypair (Ed25519 via seam + ML-DSA-65 via
  `pq::dsa::keygen`), sets `node_id = NodeId::from_keys(...)`, self-signs. `verify_self()` requires the
  self-sig valid under RequireBoth AND `node_id` matches the keys. `DowizCoSign` is *optional*:
  `verify_self()` MUST return `Ok` with NO co-sign present.
- **RED test `red_root_without_dowiz_is_valid`:** mint a root, NO co-sign, `verify_self()` → `Ok`.
  This is the §17.7 property as an executable assertion: *the hub is trust-self-sufficient without
  dowiz.* RED if any code path requires a co-sign; GREEN proves independence.
- **RED test `red_forged_node_id_rejected`:** mint a root, then overwrite `node_id` with a different
  hash → `verify_self()` → `CertError::NodeIdMismatch` (the id-binds-both-keys invariant, `cap.rs:571`
  precedent).
- **Adversarial `red_dowiz_cosign_absence_never_blocks` + `red_bad_dowiz_cosign_ignored`:** a present
  but *invalid* dowiz co-sign is ignored for validity (it is a voucher, not a gate) but MUST NOT be
  reported as a valid voucher — it is dropped, and `verify_self()` still returns `Ok` on the root's
  own merit. Proves co-sign is strictly additive, never load-bearing and never spoof-trusted.

### 4.5 M5 — owner multi-hub delegation (§16.48): mint / attenuate / revoke child hub-certs

- **Spec:** `OwnerRoot` (a `SelfSignedRoot`) appends a child `Delegation` block per hub, offline, with
  `may_delegate = false` and `MAX_DELEGATION_DEPTH = 1` (§2.4). `attenuate(child, narrower_scope)`
  produces a superseding block (never widens — `is_subset_of`). `revoke(child)` = add to the owner's
  `RevocationBlob` (§5). **No dowiz account touched anywhere in this path.**
- **RED test `red_owner_mints_child_offline`:** owner root → child hub cert → a hub verifies it knowing
  ONLY the owner root's *public* key, no network. Asserts the biscuit offline-attenuation property
  (R3 §2.3) end-to-end.
- **RED test `red_child_cannot_redelegate`:** a child cert with `may_delegate=false` tries to append a
  grandchild → construction/verify rejects (`MaxDepthExceeded`-style). Proves the depth ceiling holds
  under the *owner* anchor (§2.4).
- **RED test `red_child_cannot_widen_scope`:** child attempts a scope NOT ⊆ parent → `ScopeViolation`
  (reuses `cap.rs:515`). Attenuation-only under the owner root.
- **Adversarial `red_cross_owner_forgery`:** owner A's chain presented under owner B's anchor → root
  issuer not in B's roster → `UnknownIssuer`. Proves no cross-tenant delegation.

### 4.6 M6 — suite negotiation + downgrade binding + overlap rotation

- **Spec:** `negotiate(local: SuiteAdvertisement, peer: SuiteAdvertisement) -> Option<NegotiatedSuite>`
  picks the strongest common suite and sets `transcript_hash = SHA3-256(local.offered || peer.offered)`
  bound into the session's first signed frame. `RotationState::Overlapping` accepts BOTH suites until
  `overlap_until`, then `Stable` accepts only the new (SSH model).
- **RED test `red_downgrade_stripped_suite_detected`:** MITM removes the strong suite from one side's
  advertised list before it reaches the other → `transcript_hash` no longer matches what the signer
  committed → first-frame signature fails. RED today (no negotiation), GREEN after.
- **RED test `red_overlap_accepts_both_then_retires_old`:** during `Overlapping`, a credential in the
  OLD suite verifies; after `overlap_until`, the SAME old-suite credential is rejected. Event sequence
  asserted: `Stable(v1) → Overlapping(v1,v2) → Stable(v2)` (item 3).
- **Adversarial `red_no_common_suite_fails_closed`:** disjoint suite lists → `negotiate` returns
  `None` → handshake refused (never a silent fallback to a default).

### 4.7 M7 — revocation: short-TTL re-mint + owner-signed gossip blob (see §5 for the tradeoff)

- **Spec:** `RevocationBlob::sign(owner_root, keys, hashes, seq, not_after)`; `apply_blob(revset,
  blob, verifier)` verifies the hybrid sig, checks `not_after > now`, then `revset.merge(...)` (reuses
  the existing anti-entropy union, `cap.rs:438`). Higher `seq` from the same issuer supersedes (LWW).
- **RED test `red_unsigned_revocation_blob_ignored`:** a blob with a bad/absent hybrid sig → `apply_
  blob` → `Err(BadSignature)`, `revset` UNCHANGED. A rogue peer cannot inject revocations. (Mirrors
  `codesign::apply` refusing unsigned, `codesign.rs:119`.)
- **RED test `red_expired_revocation_blob_ignored`:** `blob.not_after < now` → ignored. Bounds the
  blob's authority in time.
- **Adversarial `red_stale_seq_cannot_unrevoke`:** issuer publishes seq=5 revoking key K, then a
  replayed seq=3 blob NOT containing K → K stays revoked (revocation is append-only; a lower seq never
  removes). Proves revocation is monotone (`RevocationSet` is append-only by construction, `cap.rs:410`).

---

## 5. Revocation, TTL & the self-custody tradeoff — stated, not softened (standard §2 items 2, 13; §4-B CLOSED)

**Binding ruling applied (do not re-ask):** operator has ruled §4-B **closed** — *self-custody is
absolute; there is NO break-glass recovery mechanism anywhere (no backup keys, no owner-root recovery,
no dowiz-held recipient in any recovery set).* P59 designs around honest, bounded TTLs, not a backdoor.

### 5.1 The mechanism (R3 §2.5, preference order — all three, layered)

1. **Short expiry + re-mint (PRIMARY).** Certs/roots carry a short `not_after`; "revocation" is
   non-renewal. Offline-friendly, no CRL, no central authority. This is SPIFFE's own answer and sets a
   hard upper bound on the compromise window = the TTL.
2. **Owner-signed `RevocationBlob`, gossiped within the owner's fleet (IMMEDIATE).** For "revoke this
   child *now*," the owner root signs a blob and pushes it hub-to-hub via the existing
   `RevocationSet::merge` anti-entropy union. Works within one owner's fleet **without dowiz** (M7).
3. **Optional dowiz revocation-transparency feed (CONVENIENCE, never load-bearing).** Like the §17.7
   CVE-advisory stance — a second channel, absent-safe.

### 5.2 The tradeoff, stated explicitly (no softening — operator directive)

Short-TTL re-mint needs *something alive to re-mint against.* Under absolute self-custody:

> **If the owner loses their root key, the fleet is eventually stranded.** Every child hub-cert
> expires at `CHILD_CERT_TTL_TICKS`; with no root to re-mint and no break-glass by ruling, the hubs
> go dark as their certs lapse. This is the *designed* consequence of §4-B, not a bug. The only knobs
> are TTL length (shorter = tighter compromise window but faster stranding on loss; longer = wider
> compromise window but more grace on loss) and how loudly the owner is warned to safeguard/duplicate
> their own root **before** loss. There is **no recovery path**, by ruling — the mitigation is honest
> pre-loss custody guidance and a bounded, chosen TTL, never a backdoor.

This is the same philosophical floor as §16.47 ("loss is the user's responsibility") and is consistent
with the §4-B(i) backup ruling (no dowiz recipient) — P59 and P68 stay aligned by construction.

### 5.3 Engineering-decision E (named default; blueprint decides — SYNTHESIS §4-E)

`ROOT_TTL_TICKS`, `CHILD_CERT_TTL_TICKS`, `OVERLAP_WINDOW_TICKS`, `REVOCATION_BLOB_GOSSIP_TICKS` are
policy values, not operator forks. **Proposed defaults** (tunable, documented): child certs on the
order of **24h** re-mint under a hot/attended owner root; root TTL on the order of **90d**; overlap
window **≥ 2× the gossip cadence** so no verifier is stranded mid-rotation (SSH lesson). These are
`kernel/src/pq/cert_chain.rs` constants with a one-line change surface; the blueprint sets them, the
operator need not.

---

## 6. Crypto-agility mechanics — the concrete scheme (R3 §3.3; SYNTHESIS X4)

### 6.1 Suite ID field
Every block carries `alg_suite` (§3, M1). `v1 = MlDsa65Ed25519 → 1.3.6.1.5.5.7.6.48`. Adding a suite =
registering a new `AlgSuite` code point + one `oid()`/`from_u16` arm — **never** a wire-format fork.
Old and new certs coexist by ID; a verifier that lacks a code point fails **closed** (never
best-effort).

### 6.2 The pre-RFC remap risk, isolated (R3 risk #6)
The draft is pre-RFC; `1.3.6.1.5.5.7.6.48` could shift before RFC. **Named explicitly.** Mitigation:
the OID string lives in exactly ONE constant (`OID_MLDSA65_ED25519_SHA512`) reached by one `match`
arm — a shift is a one-line edit, asserted by `red_oid_remap_is_one_line` (M1). The *internal* `u16`
enum (`0x0001`) never changes, so on-tree certs never migrate even if the external OID does.

### 6.3 Overlap rotation (SSH model — R3 §3.2)
A hub migrating suites enters `RotationState::Overlapping { old, new, overlap_until }`: it
**dual-publishes** both credentials; verifiers learn the new root/suite while still trusting the old;
after `overlap_until` (a hub-local `OVERLAP_WINDOW_TICKS` policy value that **survives dowiz
disappearing**) the hub goes `Stable { new }` and the old suite is retired. No mesh-wide flag-day, no
fork (M6). This is how a future ML-DSA-87/Ed448 or pure-PQ suite lands fleet-wide with zero downtime.

### 6.4 Downgrade protection (TLS model — R3 §3.3)
At every hub↔peer handshake both sides advertise a `SuiteAdvertisement`; `negotiate` picks the
strongest common suite and binds `transcript_hash = SHA3-256(both offered lists)` into the session's
first hybrid-signed frame. An attacker stripping strong suites changes the hash the signer committed,
so the first-frame signature fails — the classic negotiation pitfall is closed (M6). No-common-suite →
fail-closed, never a silent default.

---

## 7. Standards-compliance map for the crypto (audit-ready)

| Requirement (P59 charter / R3) | Standard adopted | Where realized |
|---|---|---|
| hybrid = current dowiz suite | `id-MLDSA65-Ed25519-SHA512` OID `1.3.6.1.5.5.7.6.48` | `AlgSuite::MlDsa65Ed25519` (M1) |
| both halves must verify | composite-sigs AND-semantics + `HybridPolicy::RequireBoth` + B4 lesson | `HybridSig` verify (M2), M2 RED tests |
| offline delegation-by-attenuation | biscuit construction (public-key, signed-block chain) | `Delegation`/`verify_chain` hybrid (M2, M5) |
| self-signed root, optional co-sign | SPIFFE trust-domain + UpstreamAuthority *model* | `SelfSignedRoot`/`DowizCoSign` (M4) |
| suite versioning, no hard fork | composite-sigs OID-as-version | `AlgSuite` + `from_u16` fail-closed (M1) |
| fleet migration no flag-day | OpenSSH `UpdateHostKeys` overlap | `RotationState::Overlapping` (M6) |
| downgrade resistance | TLS 1.3 negotiation + transcript bind | `NegotiatedSuite.transcript_hash` (M6) |
| revocation without central authority | SPIFFE short-TTL + gossiped blob | `ROOT/CHILD_CERT_TTL` + `RevocationBlob` (M7) |

---

## 8. Mandatory independent adversarial-review gate — DoD-BLOCKING (standard §2 items 5, 6, 14; R3 risk #3)

**This is a hard gate, grounded in a real prior incident (0.4). The cert chain does NOT ship — is not
marked done — until an independent adversarial review passes.** Unit tests are necessary and not
sufficient; the B4/SSR-2020 forgery *passed the pre-fix unit tests* and was caught only by a reviewer
who **built and ran an actual forgery**.

### 8.1 Reviewer independence (decorrelation, per the B4 precedent)
The review is performed by an actor **not** the implementer — a different model/agent (e.g. the
`system-breaker` / `security-sentinel` role, or a decorrelated model, mirroring how B4's forgery came
from an independent reviewer, not the author). The reviewer's mandate is to **produce a working
forgery or a proof of its impossibility**, not to read and approve.

### 8.2 The attack surface the review MUST attempt (each = a concrete forgery attempt, not a checkbox)
1. **Block canonicalization ambiguity** — two distinct logical certs that serialize to the same signed
   bytes (length-prefix confusion, TLV field-splitting). Attempt: craft a scope whose TLV re-parses as
   a different, wider scope. Defense under test: `scope.rs` strict fail-closed decode + length-prefixed
   `canonical_bytes`.
2. **Block reordering / truncation** — splice, drop, or reorder chain links to widen authority.
   Attempt: the M2 `red_block_reordering` / `red_chain_truncation` attacks, run *by the adversary*, not
   just as fixtures. Defense: `child==next issuer`, tail-binding, `MAX_CHAIN_LEN`.
3. **Cross-suite confusion** — replay a signature made under suite v1 as if under a (future) v2, or
   strip the suite tag. Attempt: forge a block whose `alg_suite` disagrees with the bytes actually
   signed. Defense: `DOMAIN_SUITE_PREFIX` binding (M1).
4. **AND→OR degradation** — find any path where one valid leg + one absent/forged leg is accepted
   (the literal B4 bug class). Attempt: the M2 `red_classical_only_link_rejected` /
   `red_pq_forged_classical_valid`, executed adversarially. Defense: `RequireBoth`, no shortcut.
5. **Downgrade** — strip strong suites from a handshake. Attempt: M6 `red_downgrade_stripped_suite`.
   Defense: transcript binding.
6. **Revocation bypass / un-revoke** — inject an unsigned revocation, or replay a stale `seq` to
   un-revoke. Attempt: M7 `red_unsigned_revocation_blob_ignored` / `red_stale_seq_cannot_unrevoke`.
   Defense: hybrid-signed blobs + monotone append-only `RevocationSet`.
7. **Co-sign spoof** — make an invalid dowiz co-sign be trusted as a valid voucher. Attempt: M4
   `red_bad_dowiz_cosign_ignored`. Defense: co-sign is additive-only, dropped if invalid.

### 8.3 Gate outcome (falsifiable)
- **PASS** = the reviewer produces a written attestation that (a) each attack in 8.2 was *attempted*
  with a concrete input, (b) each was rejected by the code with the expected typed error, and (c) any
  forgery found was fixed and re-attempted-and-rejected. Attestation filed under
  `docs/reflections/` and referenced from this blueprint's DoD.
- **FAIL** = any forgery accepted, OR any attack not genuinely attempted. On FAIL the chain is RED and
  does not ship — exactly as B4 walked back a shipped optimization when a forgery was found.

---

## 9. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

A prose checkbox is not a DoD item; each below is a test that is **RED before the change, GREEN after**,
or an artifact that exists/doesn't.

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D1 | every signed block carries `alg_suite`, bound into the signature | `red_suite_swap_breaks_sig`, `red_unknown_suite_rejected` (M1) |
| D2 | delegation links are hybrid `RequireBoth` — no classical-only accept | `red_classical_only_link_rejected`, `red_pq_forged_classical_valid` (M2) — **REGRESSION-LEDGER entry** |
| D3 | chain rejects reorder / truncation / over-length | `red_block_reordering`, `red_chain_truncation` (M2) |
| D4 | revoked keys & cap-hashes rejected in `verify_chain_hybrid` | `red_revoked_key_rejected`, `red_revoked_cap_hash_rejected` (M3) |
| D5 | a self-signed root verifies with NO dowiz co-sign (§17.7 property) | `red_root_without_dowiz_is_valid`, `red_forged_node_id_rejected` (M4) |
| D6 | dowiz co-sign is additive-only, absence never blocks, invalid never trusted | `red_dowiz_cosign_absence_never_blocks`, `red_bad_dowiz_cosign_ignored` (M4) |
| D7 | owner mints/attenuates child hub-certs offline, no dowiz, no re-delegation, no widening | `red_owner_mints_child_offline`, `red_child_cannot_redelegate`, `red_child_cannot_widen_scope`, `red_cross_owner_forgery` (M5) |
| D8 | suite negotiation defeats downgrade; overlap rotation retires old cleanly; no common suite fails closed | `red_downgrade_stripped_suite_detected`, `red_overlap_accepts_both_then_retires_old`, `red_no_common_suite_fails_closed` (M6) |
| D9 | revocation blobs: unsigned/expired ignored; stale seq cannot un-revoke | `red_unsigned_revocation_blob_ignored`, `red_expired_revocation_blob_ignored`, `red_stale_seq_cannot_unrevoke` (M7) |
| D10 | TTL/stranding tradeoff is documented with no recovery backdoor anywhere in the code | grep: zero `break_glass` / recovery-key path in `cert_chain.rs`; §5.2 present |
| D11 | **independent adversarial-review attestation exists and PASSES** | §8.3 artifact under `docs/reflections/`; FAIL ⇒ blueprint RED |
| D12 | kernel builds & full `cargo test --lib` green incl. all new REDs now GREEN | `cargo test -p kernel --lib` |
| D13 | no operator ruling A / D10 regression: `root_delegation.rs` `depth<=1` and ML-DSA-only path untouched | `double_hop_overlay_depth_2_rejected` (existing, `root_delegation.rs:147`) still green |

---

## 10. Benchmark plan (standard §2 item 10) — existing harness, zero new infra

Hybrid signing adds an ML-DSA-65 sign/verify per link on top of Ed25519. ML-DSA-65 is the dominant
cost (sig ≈ 3.3 KB, verify ~100µs-class). Measured, not asserted:

| Bench | Measures | Harness |
|---|---|---|
| `bench_delegation_sign_hybrid` | Ed25519+ML-DSA-65 sign of one link vs classical-only baseline | `cargo bench` (existing `pq` bench pattern) |
| `bench_verify_chain_hybrid` | full-chain verify at `MAX_CHAIN_LEN=4`, both legs | same |
| `bench_revocation_merge` | `RevocationSet::merge` at 10²/10³/10⁴ entries (scaling axis, §11) | same |

Telemetry hook: chain-verify latency emitted through the existing kernel metrics seam (`metrics.rs`)
so a rotation or chain-length regression surfaces automatically, not at review time (item 14).

---

## 11. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as math (item 6):** the unsafe state = "a cert verifies with < 2 valid legs" or "a
  child holds authority ⊄ its parent." Both are made **unrepresentable at the boundary**: `HybridSig`
  verify is AND with no OR code point (mirrors `HybridPolicy::RequireBoth` having "no weaker code
  point"); attenuation is enforced by `is_subset_of` in `verify_chain`; `may_delegate=false` +
  `MAX_DELEGATION_DEPTH=1` enforced at construction (mirrors `overlay()`'s `MaxDepthExceeded`, so a
  bad state is never constructed, not merely rejected at use). Reachability is argued from type
  structure, per the Monocoque/finite-anchored-authority doctrine.
- **Schemas & scaling axis (item 8):** scaling axis = **certs per owner fleet** (N hubs) and
  **revoked entries per `RevocationSet`**. `RevocationSet` is `HashSet`-backed; it changes shape when
  a single owner exceeds ~10⁴ live revocations (then a Bloom/summary digest for gossip, not full
  sets). Chain length is hard-capped at `MAX_CHAIN_LEN=4` — it never scales. Stated, not timeless.
- **Isolation / bulkhead (item 11):** the cert chain is a **pure predicate** (no I/O, no clock passed
  in as `now`, no network — mirrors `root_delegation.rs`'s "no network, no clock, no I/O" note). A
  compromise of the provisioning service (P67) cannot forge a cert because signing keys never touch
  CF/Hetzner (R3 §1.5) — the failure of the infra plane does not propagate to the identity plane.
- **Mesh awareness (item 12):** roots and child certs are **node-local**; the **`RevocationBlob` is
  gossip-propagated** via `RevocationSet::merge` anti-entropy (owner fleet only, no dowiz relay).
  Payload budget: a blob is `O(revoked_count × 32B)` + one hybrid sig (~3.4 KB) — well within a mesh
  frame; gossip cadence `REVOCATION_BLOB_GOSSIP_TICKS` bounds frequency.
- **Living memory (item 15):** certs are **time-scoped** (`not_after`, `is_fresh`) and
  **topology-scoped** (owner→hub tree). Revocation is temporal (TTL) + append-only (never deleted, only
  superseded by seq) — the demote-never-delete pattern of the living-memory arc.
- **Rollback / self-healing vocabulary, as math not metaphor (item 13):** **Self-termination** = the
  hard invariant boundary (a < 2-leg or over-attenuated cert is unrepresentable — not a supervisor's
  decision). **Snapshot re-entry** = short-TTL re-mint IS regeneration from the last valid epoch (the
  owner root). **Self-healing** is deliberately NOT claimed — there is no error-correcting recovery of
  a lost root (§5.2); claiming it would be false.
- **Error-propagation / smart index (item 14):** the bug class this introduces (a leg silently
  dropped, a suite silently downgraded) is turned into a **compile-time/test-time** failure by: the
  typed `HybridSig` (both fields non-optional), `AlgSuite::from_u16` fail-closed decode, and D2/D8
  regression tests wired into `cargo test --lib`. Not a runtime surprise.
- **Tensor/spectral (item 16):** **N/A, honestly** — capability verification is a signature predicate
  over a small DAG, not a linear-algebra kernel; forcing `spectral.rs` here would be over-engineering
  (ponytail). Stated rather than shoehorned.
- **Linux discipline (item 9):** verdict framework — **EXTENDS** the existing `cap.rs` UCAN chain
  (hybrid legs + suite field + revocation-in-verify); **REINFORCES** `root_delegation.rs`'s
  fail-closed construction pattern; **ALREADY-EQUIVALENT** on domain separation and anchor-rooting;
  **DOES-NOT-TRANSFER**: no new daemon, no SPIRE-style attestation service.

---

## 12. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Mentalism / Correspondence:** the identity *is* the hash of both public keys (`NodeId::from_keys`)
  — "as above (keys), so below (id)"; there is no external CA authority to correspond to, the id is
  self-describing.
- **Polarity / no-middle:** `HybridPolicy::RequireBoth` — a cert is valid or not, there is no
  degraded/partial-trust code point. The AND-verify floor is polarity made type-level.
- **Cause & Effect:** every authority has a signed cause (a parent block); nothing is authorized by
  correlation or reputation (mesh red-line: capability, never score).

---

## 13. Standard-compliance map (all 20 points, checkable — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth with live `file:line` | §0 (every existing element cited; the "no `HybridSigner` struct" correction) |
| 2 | Falsifiable DoD | §9 (D1–D13, each a RED→GREEN test or artifact check) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per M; event-sequence asserts in M6/M7) |
| 4 | Predefined types & constants | §3 (`AlgSuite`, `HybridSig`, `SelfSignedRoot`, … named before impl) |
| 5 | Adversarial/breaking tests | §4 (every M has RED adversarial cases), §8 (forgery gate) |
| 6 | Hazard-safety from type structure | §11 (unrepresentable < 2-leg / over-attenuated states), §2.4 |
| 7 | Links to docs & memory | §14 below |
| 8 | Schemas with scaling axis | §11 (certs/fleet, revocations/set, chain-len capped) |
| 9 | Linux engineering discipline | §11 (EXTENDS/REINFORCES/… verdict) |
| 10 | Benchmarks + telemetry | §10 |
| 11 | Isolation / bulkhead | §11 (pure predicate; infra-plane compromise ≠ identity forge) |
| 12 | Mesh awareness | §11 (node-local certs, gossiped revocation blob, payload budget) |
| 13 | Rollback/self-heal as math | §11 (self-termination = invariant; re-mint = snapshot re-entry; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §11 (typed HybridSig, fail-closed decode, regression wiring) |
| 15 | Living-memory awareness | §11 (time-scoped + append-only revocation) |
| 16 | Tensor/spectral where applicable | §11 (N/A, stated honestly) |
| 17 | Regression tracking | §9 D2 (REGRESSION-LEDGER entry for the classical-only-rejected test) |
| 18 | Clear worker instructions | §14 |
| 19 | Reuse-first, upgrade-if-needed | §0.2 (extends existing chain), §1 (adopt not invent), §2.2 (anti-scope) |
| 20 | Hermetic principles | §12 |

---

## 14. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / supersedes / cites:**
- `SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` §5 (P59 row), X4 (crypto-agility adoption), X8 (identity
  upstream of claim/couriers/owners/wallets), §4-B (self-custody CLOSED).
- `OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md` §2 (cert hierarchy), §3 (crypto-agility), §6.2/6.3,
  §7 risks #2/#3/#6.
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract this document is measured against).
- `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` §16.48, §17.2, §17.7.
- Memory: `crypto-safe-first-pass-2026-07-14.md` (B4/SSR-2020 forgery precedent — §0.4, §8).
- Format precedent: `BLUEPRINT-P51-open-map-routing.md`.

**Existing code this blueprint edits/extends (exact targets):**
- **NEW** `kernel/src/pq/cert_chain.rs` — all §3 types; `verify_chain_hybrid`; negotiation; rotation;
  revocation blob.
- **EDIT** `kernel/src/ports/agent/cap.rs` — `Delegation` gains `HybridSig` (replace classical-only
  `signature`); `Delegation::sign`/`verify_signature` → hybrid `RequireBoth`; `Capability` gains
  `alg_suite`; `verify_chain` → suite + revocation gate (or wrap in `cert_chain.rs`).
- **REUSE unchanged** `kernel/src/pq/dsa.rs`, `kernel/src/ports/agent/scope.rs`,
  `RevocationSet`/`AnchorRoster` (`cap.rs`), `pq/envelope.rs`.
- **DO NOT TOUCH** `kernel/src/pq/root_delegation.rs` (operator ruling A / D10 — §2.4, §9 D13).

**For the worker with zero session context — exact acceptance path:**
1. Write §3 types in `cert_chain.rs` first (types before tests before code — item 3).
2. Implement M1→M7 in order; each M's RED tests must fail before its code and pass after.
3. Add the D2 regression test to `docs/regressions/REGRESSION-LEDGER.md`.
4. `cargo test -p kernel --lib` fully green (D12); the existing `double_hop_overlay_depth_2_rejected`
   must stay green (D13 — proves no operator-ruling regression).
5. **Do NOT mark P59 done until §8's independent adversarial-review attestation PASSES (D11).** A green
   unit suite is necessary and NOT sufficient — that is the entire B4 lesson. Route the review to an
   independent reviewer role (`system-breaker`/`security-sentinel` or a decorrelated model), whose job
   is to *build a forgery*, not to approve.
6. Anti-scope: do NOT build tunnel/pool/claim/UI (P67/P70/P71); do NOT merge wallet-transfer crypto;
   do NOT add a break-glass/recovery path anywhere (§5.2, §4-B closed).
