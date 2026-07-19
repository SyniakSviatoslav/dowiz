# OPUS — Kernel `pq::kem` ring-bug investigation (dedicated follow-up)

**Date:** 2026-07-18
**Author:** Opus (research-only; ZERO code written, no branches touched, nothing pushed)
**Scope:** the separate, deferred red-line review flagged by
`OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md` §1.2 / §5.4.
**Subject file:** `/root/dowiz/kernel/src/pq/kem.rs` — dowiz's OWN ML-KEM, distinct from bebop2's.

**Verdict (one line):** The finding is **CONFIRMED by reading the current source** — the kernel
`pq::kem` implements the **cyclic** ring `Z_q[x]/(x²⁵⁶−1)` with **η1=3**, so it is **NOT
ML-KEM-768 / FIPS-203**. Severity is **latent correctness/security bug in opt-in, un-wired
scaffolding — NOT an active production incident**. There is a real, separable severity amplifier:
the module's own doc-comment and its introducing commit message **assert FIPS-203/ACVP compliance
that does not exist**, which is a trap for whoever wires it next.

---

## 1. What the ring actually is (verified in current source, not trusted from the prior report)

Read of `kernel/src/pq/kem.rs` @ HEAD confirms every claim of the prior finding:

| Claim | Evidence in current file | Correct FIPS-203 ML-KEM-768 |
|---|---|---|
| **Cyclic ring, not negacyclic** | Test `ntt_mul_equals_schoolbook` (**line 429**) reduces the schoolbook product with `let idx = (i + j) % N;` — pure **cyclic** convolution `x²⁵⁶ ≡ +1`. There is no sign-flip on wraparound and no ψⁱ pre/post-weighting anywhere. | Negacyclic `x²⁵⁶ ≡ −1` (subtract on `i+j ≥ N`). |
| **Complete (not incomplete) NTT** | `ntt()` (**line 57**) runs a full **8-layer** Cooley-Tukey (`for s in 1..=8`, `m = 1<<s` up to 256) with `ROOT=17` (**line 24**), then multiplies **pointwise** `fq_mul(ah[j], bh[j])` (line 422) — no quadratic basemul. A complete length-256 transform + pointwise mul computes the **cyclic** product. | 7-layer **incomplete** NTT to 128 quadratic residues + quadratic `basemul` (because no 512th root of unity exists mod 3329: `512 = 2⁹ ∤ 3328 = 2⁸·13`). |
| **Wrong η1** | `pub const ETA1: usize = 3;` (**line 21**), with `K = 3` (line 18, ML-KEM-**768**). | η1 = **2** for ML-KEM-768 (η1=3 is the ML-KEM-**512** value). |
| η2 | `ETA2 = 2` (line 22) | 2 ✓ (only correct PQ param) |
| Ciphertext wire size | `CT_LEN = K*384 + 384 = 1536` (line 28); `encaps` re-packs compressed coeffs at **12 bits** (`poly_to_bytes`, 384 B/poly) regardless of du=10/dv=4. | **1088** bytes (`32·(du·k+dv)`); du=10, dv=4 bit-packing. |

So it is doubly non-compliant: **wrong ring** *and* **wrong η1**, and as a bonus a **wrong wire
format** (1536-byte ct vs the spec's 1088). It is *internally self-consistent* — keygen/encaps/decaps
all use the same cyclic ring end-to-end, so its own round-trip and tamper tests pass — but that
consistency is exactly why the bug is invisible to its own suite.

### 1.1 FIPS-203 ML-KEM-768 parameters — authoritative confirmation
WebSearch budget for this session was exhausted, but the canonical values are quoted verbatim from
FIPS 203 §8 Table 2 inside the *correct* bebop2 implementation
(`/root/bebop-crypt/bebop2/core/src/pq_kem.rs:5-11`), which cites the standard directly:

> "FIPS 203 (ML-KEM) … `q = 3329` … Table 2: **ML-KEM-768 | 256 | 3329 | 3 | 2 | 2 | 10 | 4**"
> i.e. **n=256, q=3329, k=3, η1=2, η2=2, du=10, dv=4**, ring **Z_q[x]/(x²⁵⁶+1)**.

The kernel matches only n, q, k, η2, du, dv — and misses the two that define the scheme's security
(the ring) and its noise distribution (η1).

### 1.2 Why "cyclic" is a security downgrade, not merely "incompatible" (honest, measured)
`x²⁵⁶−1` **fully splits** over Z₃₃₂₉ (ROOT=17 has order 256) into 256 linear factors, so the ring
CRT-decomposes into 256 independent copies of Z_q — including the evaluation-at-1 homomorphism
`R → Z_q` that maps a Ring-LWE sample to a scalar LWE relation on the coefficient sums. The negacyclic
ring `x²⁵⁶+1` (the 512th cyclotomic) is chosen by Kyber/ML-KEM **precisely to avoid** this
full-splitting structure — it stays as 128 quadratic factors. I make **no claim of a specific
break**, but state plainly: Kyber's MLWE security reduction and its decryption-failure/noise analysis
are stated over the negacyclic ring and **do not transfer** to this cyclic variant. Combined with
η1=3 (wrong noise), the construct is an **unvetted, non-standard lattice scheme**, not ML-KEM-768.

---

## 2. Blast radius — THE decisive question (answer: opt-in + un-wired scaffolding)

Full consumer trace inside `/root/dowiz`:

```
kem.rs  (wrong ring)
  └─ hybrid.rs        pq::hybrid  — X25519 ⊕ ML-KEM-768 hybrid KEM
       │              (hybrid_keygen/encaps/decaps call kem::keygen_internal/encaps/decaps)
       └─ volume.rs   pq::volume  — P2 at-rest AES-256-GCM volume crypto (KEM-DEM: dk=KDF(ss))
            └─ (NO consumer anywhere in kernel/src — chain terminates here)
```

- **`grep` for consumers of `pq::hybrid`** → only `volume.rs`.
- **`grep` for consumers of `pq::volume`** → **nothing**. `volume.rs` is exercised only by its own
  `#[cfg(test)]`. The KEM is reachable at runtime by **no live capability**.
- **Feature-gated opt-in.** The entire `pq` module is `#[cfg(feature = "pq")] pub mod pq;`
  (`lib.rs:13-14`). The `pq` feature (`kernel/Cargo.toml:56`) is **not enabled by default and no
  dependent crate turns it on** — `agent-adapters`, `agent-facade`, `engine`, `llm-adapters`,
  `wasm`, `agent-governance-wasm` all depend on `dowiz-kernel` with **default features only** (no
  `features = ["pq"]`). The introducing commit confirms "Default build + 452 tests unaffected."
  Only a manual `cargo test/build --features pq` ever compiles this code.
- **NOT on the capability-cert / money path.** The `hybrid` in `capability_cert.rs` and
  `payment.rs:462` (`Capability::new_hybrid`) is the **Ed25519 ⊕ ML-DSA-65 *signature*** seam
  (`dsa.rs`), a completely different primitive. `capability_cert` is default-built and rides the
  `RefSigner` seam; `lib.rs:154-156` states "**production injects real bebop2 crypto at the seam**."
  The KEM (`kem.rs`) touches **none** of cert issuance, signing, auth, RLS, money, or orders.

**Conclusion:** this is **dead/opt-in scaffolding** for a future at-rest-volume feature (P2/D4) that
is itself not wired into any live path. It is **not** encrypting or key-exchanging any real data
today. Therefore: **a correctness bug to fix before it is ever wired in — not an active breach.**

### 2.1 The real severity amplifier (do not under-state this half)
The bug is dangerous *latently* because the code **advertises compliance it does not have**:
- `kem.rs:1-12` header: *"ML-KEM-768 (FIPS 203) … the NTT is provably a ring isomorphism … Upgrade
  path: none needed."* — false; it is a **cyclic** isomorphism, the wrong ring, and the "none needed"
  line actively discourages the fix.
- Introducing commit `0a85184b0` message: *"107 KAT tests byte-exact vs NIST ACVP (ML-DSA-65
  keyGen/sigGen/sigVer, **ML-KEM-768**)."* — the ACVP claim for **ML-KEM is unsubstantiated**: the
  `kat/acvp/` directory holds **only ML-DSA vectors** (`key-gen.json`, `sig-ver.json`,
  `sig-gen.json`); the "107 KAT" all come from `dsa/dsa_acvp_tests.rs`. **There is no KEM ACVP
  vector and no KEM KAT anywhere in the tree.** The KEM's only tests are self-referential
  round-trip/tamper checks that pass trivially in the wrong ring.

So anyone who trusts the header/commit and wires `volume.rs` into production would ship a
non-ML-KEM at-rest scheme believing it was FIPS-203. That is the risk to flag loudly — a **trap**,
not a live incident.

---

## 3. Provenance vs bebop2 (scopes the fix: rewrite, not one-line patch)

The kernel `kem.rs` is a **separate, independent, earlier from-scratch implementation** — **not**
derived or copied from bebop2's now-correct `pq_kem.rs`. Distinguishing evidence:

| Aspect | dowiz `kernel/src/pq/kem.rs` | bebop2 `core/src/pq_kem.rs` (now-correct) |
|---|---|---|
| `keygen_internal` signature | `(d: &[u8;32])` — **one** seed | `(d, z: &[u8;32])` — **two** seeds (FIPS-203 K-PKE `d` + FO `z`) |
| Ring multiply | complete **cyclic** NTT + pointwise `fq_mul` | schoolbook **negacyclic** `poly_mul` (line 296), NTT deliberately removed |
| Domain stored | applies inverse NTT in `serialize_vec` | **coefficient domain** (correctness-by-construction) |
| η1 | **3** (wrong) | **2** (correct) |
| Ciphertext length | 1536 (12-bit repack) | 1088 (`KEM768_CT_LEN`, correct compression) |
| Key/ct types | raw `Vec<u8>` | typed `MlKem768Ek/Dk/Ct` |

Two genuinely different codebases. The kernel one is the buggier independent attempt; the bebop2 one
is the version that just received the exhaustively-proven negacyclic NTT
(`OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md`). Because the divergence is structural (ring, seeds,
domain, params, wire format), the fix is **not** a targeted parameter tweak — it is effectively a
**ring-layer rewrite + η1 correction + wire-format correction**.

---

## 4. Remediation recommendation (sketch only — NOT implemented here; red-line, needs its own review)

**Urgency: fix-before-any-future-wiring, NOT emergency.** Because the module is opt-in and un-wired,
there is no live data to rotate and no incident to declare. The correct sequencing:

1. **Immediately (docs-only, low-risk, could be a fast follow-up):** correct the **false compliance
   claims** so nobody wires it trusting them — strike "FIPS 203 / ML-KEM-768 / ring isomorphism /
   Upgrade path: none needed" from the `kem.rs` header and add a prominent `// NOT FIPS-203: cyclic
   ring + η1=3, do NOT wire — see OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md`. This defuses the
   trap without touching the crypto. (Flagged, not done — even a comment on a red-line file should go
   through the owner.)
2. **The real fix (dedicated red-line task):** replace the ring layer with the **correct negacyclic
   arithmetic** and set **η1=2**. Strongly prefer **porting bebop2's now-verified logic** over
   re-deriving: either its schoolbook negacyclic `poly_mul`, or its just-landed incomplete NTT
   (`ntt_fwd_kem`/`ntt_inv_kem`/`basemul_kem`/`poly_mul_ntt`) which is **exhaustively proven
   bit-identical** to schoolbook across all 65 536 basis pairs. Also fix the ciphertext compression
   to the real du=10/dv=4 packing (1088-byte ct) and reconcile the one-seed vs two-seed (`d`,`z`) FO
   structure. This is large enough that adopting bebop2's module wholesale (or sharing it) is worth
   considering over patching the kernel copy.
3. **Gate the fix exactly like the bebop2 NTT work:** ship RED first — add real **NIST ACVP
   ML-KEM-768 KEM vectors** (encaps/decaps) to `kat/acvp/` and a byte-exact test, plus a
   negacyclic-wrap KAT (`x²⁵⁵·x == −1`). The bit-identity + ACVP gate is mandatory before the module
   may claim FIPS-203, and mandatory before `volume.rs` is ever wired to a live path. Same
   crypto-change discipline (independent review + KAT-verification) the bebop2 pass used.
4. **Do NOT** treat "its own tests pass" as evidence of correctness — that is precisely the
   self-consistency trap documented in §1.

---

## 5. Files / evidence index
- **Subject:** `/root/dowiz/kernel/src/pq/kem.rs` (wrong ring @ L429/L57/L24, η1=3 @ L21, ct=1536 @ L28).
- **Consumers:** `kernel/src/pq/hybrid.rs` (L16/39/66/93) → `kernel/src/pq/volume.rs` (L34/128/171) → **no further consumer**.
- **Gating:** `kernel/src/lib.rs:13-14` (`#[cfg(feature="pq")]`); `kernel/Cargo.toml:56` (`pq` feature, default-off, no dependent enables it).
- **KAT truth:** `kernel/src/pq/kat/acvp/{key-gen,sig-ver,sig-gen}.json` = **ML-DSA only**; `kernel/src/pq/dsa/dsa_acvp_tests.rs` = the 107 tests. **No KEM ACVP vector exists.**
- **Introducing commit:** `0a85184b0` (2026-07-17, SyniakSviatoslav) "extract KAT-gated PQ core (ML-DSA-65/ML-KEM-768) as opt-in `pq` feature" — its "ACVP … ML-KEM-768" claim is unsubstantiated.
- **Correct reference (provenance/fix source):** `/root/bebop-crypt/bebop2/core/src/pq_kem.rs` (negacyclic, η1=2, ct=1088) + `OPUS-PERF-NTT-IMPLEMENTATION-2026-07-18.md`.
- **No fix applied. No code written. No git action taken.** Research-only, per operator directive.
