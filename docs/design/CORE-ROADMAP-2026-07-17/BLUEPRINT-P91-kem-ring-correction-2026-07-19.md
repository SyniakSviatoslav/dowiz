# BLUEPRINT P91 — Kernel `pq/kem.rs` ring correction (cyclic→negacyclic, η1 3→2, ct 1536→1088) + false-compliance defusal (2026-07-19)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion). **Crypto today: ML-DSA-65 and ML-KEM-768 ACVP-exact in `crates/dowiz-core/src/pq/`, hybrid seal wired but OFF until `BACKUP_SEAL_PK` (OA-3); `capability_cert` is one of the 13 kept, uncalled modules (N4.2).**


> **Standalone CRYPTO-CORRECTNESS blueprint (dowiz-kernel `kernel/src/pq/kem.rs`).** One coherent unit
> against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md` §2. Research source:
> `docs/research/OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md` (recovered — see MASTER-STATUS-LEDGER
> §0); scoped in `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §3. Port source:
> `/root/bebop-crypt/bebop2/core/src/pq_kem.rs` @ commit `f38f2c5` (correct, un-quarantined; provenance
> pinned §0.5) and/or the P85-quarantined NTT in `/root/bebop-repo`. Format precedent:
> `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`,
> `BLUEPRINT-P59-capability-cert-chain.md`.
>
> **One sentence:** dowiz's own `kernel/src/pq/kem.rs` is **not ML-KEM-768 / FIPS-203** — it implements
> the **cyclic** ring `Z_q[x]/(x²⁵⁶−1)` with **η1=3** and a **1536-byte ciphertext**, while its header
> falsely advertises FIPS-203 compliance; because the whole `pq` module is feature-gated off and the
> consumer chain terminates with zero live callers, this is a **fix-before-wiring correctness/security
> bug, NOT an active incident**, addressed in three ordered parts: an immediate near-zero-risk header
> defusal (P91.0), a ring-layer rewrite that ports bebop2's proven negacyclic arithmetic (P91.1), and a
> ship-RED ACVP-KAT + 3-model-review gate that must pass before the module may claim FIPS-203 or before
> `volume.rs` is ever wired (P91.2).

---

## VERDICT (stated up front, per session research discipline)

**FIX-BEFORE-WIRING — real bug, wrong-ring + wrong-noise + wrong-wire-format, but latent, not live.**
The kernel KEM is doubly non-compliant (wrong ring *and* wrong η1) plus a wrong ciphertext size, and it
is *internally self-consistent* — its own round-trip/tamper tests pass trivially in the wrong ring,
which is exactly why its suite cannot see the bug. **Blast radius is decisive and verified: NOT live.**
The whole `pq` module is `#[cfg(feature = "pq")]`, off by default, no dependent crate enables it, and
the consumer chain `kem.rs → hybrid.rs → volume.rs` terminates with **zero callers of `volume.rs`**. It
touches none of cert issuance, signing, auth, RLS, money, or orders. Therefore: **not an emergency, no
data to rotate, no incident to declare — a correctness bug to fix before it is ever wired in.**

Three sub-items, in strict order:

1. **P91.0 — immediate, separable, near-zero-risk (operator can approve independently, OD-5/W3-5).**
   Correct the **false compliance claims** in the `kem.rs` header only. **This changes zero behaviour —
   it removes a false claim — so it is safe even under a code-freeze.** Flagged as immediately
   actionable ahead of the real fix if the operator wants the trap defused now. Default if unruled: the
   header keeps falsely claiming FIPS-203 and the trap stays armed.

2. **P91.1 — the real fix (red-line lane).** Replace the ring layer with correct negacyclic arithmetic,
   set η1=2, fix ct packing to du=10/dv=4 (1088 B), reconcile the one-seed vs two-seed (`d`,`z`) FO
   structure. **Port bebop2's proven code, do NOT re-derive.** *Port-source decision (§5):* the
   un-quarantined **schoolbook negacyclic `poly_mul`** (`/root/bebop-crypt`) is correctness-first and
   **not** P85-gated; the faster **incomplete NTT** (`/root/bebop-repo` `986646a`) is P85-gated because
   it is the process-quarantined artifact. Recommend schoolbook-first, NTT as a P85-gated speed
   follow-up.

3. **P91.2 — the gate (ship RED first).** Real NIST ACVP ML-KEM-768 encaps/decaps vectors into
   `kat/acvp/` + byte-exact tests + a negacyclic-wrap KAT (`x²⁵⁵·x == −1`), and the **same 3-model
   review rigor as the bebop2 NTT work** (which P85 exists to enforce). "Its own tests pass" is
   **inadmissible** as evidence — that is the self-consistency trap that hid the bug.

**Anti-scope:** P91 does not wire `volume.rs`, does not touch the Ed25519⊕ML-DSA *signature* seam
(a different primitive, on the live path), and does not adopt the bug's "Upgrade path: none needed"
framing.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> "Ground truth is non-discussible." Every row below was **read from `kernel/src/pq/kem.rs` @ HEAD this
> pass**, not inherited from the research doc. The header's own claims were read verbatim.

### 0.1 What the ring/params actually are (read this pass)

| Claim | Evidence in current source (this pass) | Correct FIPS-203 ML-KEM-768 |
|---|---|---|
| **Cyclic ring, not negacyclic** | the schoolbook reference reduces the product with `let idx = (i + j) % N; sb[idx] = fq_add(sb[idx], …)` — pure **cyclic** convolution `x²⁵⁶ ≡ +1`, **no sign flip on wraparound**, no ψⁱ pre/post-weighting (research cite: `kem.rs:429`) | negacyclic `x²⁵⁶ ≡ −1` (subtract on `i+j ≥ N`) |
| **Complete (not incomplete) NTT** | `ntt()` runs a full **8-layer** Cooley-Tukey with `ROOT = 17` (`kem.rs:24`), then multiplies **pointwise** `fq_mul(ah[j], bh[j])` — a complete length-256 transform + pointwise mul computes the **cyclic** product (research cite `kem.rs:57`) | 7-layer **incomplete** NTT to 128 quadratic residues + quadratic `basemul` (no 512th root of unity mod 3329) |
| **Wrong η1** | `pub const ETA1: usize = 3; // ML-KEM-768` (`kem.rs:21`), with `K = 3` (`kem.rs:18`) | η1 = **2** (η1=3 is the ML-KEM-**512** value) |
| η2 | `pub const ETA2: usize = 2` (`kem.rs:22`) | 2 ✓ (only correct PQ param) |
| Ciphertext wire size | `pub const CT_LEN: usize = K * 384 + 384 = 1536` (`kem.rs:28`); DU=10/DV=4 are *declared* (`kem.rs:19-20`) but the packing re-encodes at 12 bits (384 B/poly) regardless | **1088** = `32·(du·k+dv)`; du=10, dv=4 bit-packing |

So it is doubly non-compliant (**wrong ring** *and* **wrong η1**) with a **wrong wire format** (1536 vs
1088). Confirmed n=256/q=3329/k=3/η2=2/du/dv match the standard — the two that define the *scheme's
security* (the ring) and *noise distribution* (η1) do not.

### 0.2 The false-compliance header — the trap (read verbatim this pass)

`kem.rs:1-12` header states, verbatim:
- *"ML-KEM-768 (FIPS 203, Module-Lattice-Based Key-Encapsulation Mechanism)."*
- *"a complete Cooley-Tukey NTT over `Z_q[x]/(x^256+1)`"* — **doubly false**: it claims FIPS-203 *and*
  claims the negacyclic ring `x²⁵⁶+1`, while the code's own schoolbook reference (§0.1) computes the
  **cyclic** `x²⁵⁶−1` product. The header lies even about its *own* ring.
- *"the NTT is provably a ring isomorphism … verified by the test suite. Upgrade path: none needed."* —
  the isomorphism is real but for the **wrong (cyclic)** ring; "verified by the test suite" is the
  self-consistency trap (§6); "none needed" actively discourages the fix.

Introducing commit `0a85184b0` message: *"107 KAT tests byte-exact vs NIST ACVP (… ML-KEM-768)"* — the
KEM ACVP claim is **unsubstantiated** (§0.3).

### 0.3 KAT truth — there is NO KEM ACVP vector anywhere (read this pass)

`kernel/src/pq/kat/acvp/` contains **only** `key-gen.json`, `sig-gen.json`, `sig-ver.json` (all
**ML-DSA**). The 107 KAT tests all come from the ML-DSA suite (`dsa/dsa_acvp_tests.rs`). **No KEM ACVP
vector and no KEM KAT exists in the repo.** The KEM's only tests are self-referential round-trip/tamper
checks that pass trivially in the wrong ring. Directory listing confirmed this pass — three JSON files,
no `kem`/`encap`/`decap` vector among them.

### 0.4 Blast radius — decisive, verified this pass (the whole reason this is not an incident)

```
kem.rs  (wrong ring)
  └─ hybrid.rs   pq::hybrid  — X25519 ⊕ ML-KEM-768 hybrid KEM
       └─ volume.rs  pq::volume  — P2 at-rest AES-256-GCM volume crypto (KEM-DEM: dk = KDF(ss))
            └─ (NO consumer anywhere in kernel/src — the chain terminates here)
```

- **Feature-gated off by default.** `kernel/src/lib.rs:13-14`: `#[cfg(feature = "pq")] pub mod pq;`
  (confirmed this pass — `lib.rs:13` is the `#[cfg(feature = "pq")]`, `:14` the `pub mod pq;`). No
  dependent crate enables `pq`; only a manual `cargo build/test --features pq` compiles it.
- **`volume.rs` has zero callers** — exercised only by its own `#[cfg(test)]`. The KEM is reachable at
  runtime by **no live capability**.
- **NOT on the signature/cert/money path.** The `hybrid` in `capability_cert.rs`/`payment.rs` is the
  **Ed25519 ⊕ ML-DSA-65 *signature*** seam (`dsa.rs`) — a *different primitive*; `lib.rs:179`:
  "production injects real bebop2 crypto at the seam" (confirmed this pass). `kem.rs` touches none of
  cert issuance, signing, auth, RLS, money, or orders.

**Conclusion:** dead/opt-in scaffolding for a future at-rest-volume feature (P2/D4) that is itself
un-wired. It encrypts/exchanges **no real data today**. A correctness bug to fix before it is wired —
not an active breach.

### 0.5 The correct reference (port source), read this pass

`/root/bebop-crypt/bebop2/core/src/pq_kem.rs` — **correct and un-quarantined**:
- header `:1-11`: cites FIPS-203 §8 Table 2 verbatim — *"ML-KEM-768 | 256 | 3329 | 3 | 2 | 2 | 10 | 4"*,
  i.e. **η1=2**, ring `Z_q[x]/(x²⁵⁶+1)`; explicitly warns the wrong modulus "would produce a
  non-interoperable, broken scheme."
- `poly_mul` (schoolbook **negacyclic**) at `:296`; `poly_mul_matches_schoolbook` test at `:831`.
- typed keys/ct: `MlKem768Ek/Dk/Ct` (`:252-254`); **two-seed** `keygen_internal(d, z)` (`:589`) — the
  FIPS-203 K-PKE `d` + FO `z`.

This tree is the stable, KAT-gated reference. The *incomplete-NTT* variant (`poly_mul_ntt`, exhaustively
proven 0/65 536 basis-pair mismatches) is the **separate** `/root/bebop-repo` `986646a` work — which is
**P85-process-quarantined** (`--no-verify` bypass). The distinction is load-bearing for §5.

**Port-source provenance pin (closes audit G4's second half — matching P85's `986646a` discipline, §3 of
that blueprint; verified live this pass):**
- **Repo:** `/root/bebop-crypt` — a git repo, accessible in-environment, working tree clean (`git status`
  → nothing to commit). Remotes: `origin` → `git@github.com:SyniakSviatoslav/bebop.git`, `openbebop` →
  `git@github.com:SyniakSviatoslav/OpenBebop.git`.
- **Commit:** **`f38f2c5`** (full `f38f2c57db3e2aa8e04849b4b4df99ec5446de2e`) —
  `chore: clean-slate publish — remediated keep-set (fresh history)`, 2026-07-14. `git log --follow --
  bebop2/core/src/pq_kem.rs` shows this is the **only** commit that has ever touched the file in this
  repo's (deliberately squashed) history, and `git rev-list --count f38f2c5` = 1 — it is the **root**
  commit of the whole repo, so there is no parent to diff against; inspect the exact introducing content
  via `git show f38f2c5:bebop2/core/src/pq_kem.rs` (the P85-style `NTT_DIFF` convention does not apply to
  a root commit). Reachable from `openbebop/main` and three other remote branches (`git branch -r
  --contains f38f2c5`) — pushed, not a fragile local-only tip.
- **Pin format (matching P85 §3's named-constant convention):**
  `PORT_SOURCE_COMMIT_BEBOP_CRYPT = f38f2c5` (`bebop2/core/src/pq_kem.rs`, `/root/bebop-crypt`).
- This pin is exactly the evidence Q1's "Conformance / provenance" checklist row requires before D3 can
  reach `DONE-VERIFIED` (`BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md` §Q1-b: *"the
  vectors must exist and be pinned before the DoD can be checked"*).

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

| Prior art | What it is | How P91 uses it — and what it does NOT take |
|---|---|---|
| **FIPS-203 / ML-KEM-768 (CRYSTALS-Kyber)** | the standard: negacyclic ring `x²⁵⁶+1`, q=3329, k=3, η1=2, η2=2, du=10, dv=4, 1088-byte ct, two-seed FO | **Adopt as the correctness target** — every DoD check is "byte-exact vs the standard's ACVP vectors" (§9). **NOT taken:** the header's claim that the *current* code already is this. |
| **bebop2 schoolbook negacyclic `poly_mul`** (`/root/bebop-crypt`, `:296`) | O(n²) schoolbook multiply in the correct negacyclic ring; simple, KAT-gated, un-quarantined | **Primary port source (§5).** Correctness-first (ponytail: simplest thing that is *correct*), and — crucially — **not P85-gated**. **NOT taken:** re-deriving the ring math from scratch (the research doc's explicit anti-recommendation). |
| **bebop2 incomplete NTT** (`poly_mul_ntt`, `/root/bebop-repo` `986646a`) | O(n log n) 7-layer incomplete NTT + quadratic basemul, proven bit-identical to schoolbook across all 65 536 basis pairs | **Optional speed follow-up, P85-GATED** — building on the process-quarantined artifact would compound A4, so it may only be ported once P85 closes. **NOT taken by default** — schoolbook is enough for correctness. |
| **NIST ACVP ML-KEM-768 test vectors** | the authoritative external byte-exact conformance vectors (encaps/decaps) | **Adopt as the ship-RED gate (P91.2).** The module may not claim FIPS-203 until these pass. **NOT taken:** the current "self-consistent round-trip test" as evidence — that is the trap. |
| **The bebop2 crypto-change discipline** (independent review + KAT, per `crypto-safe-first-pass-2026-07-14`) | RED-first + a decorrelated reviewer who *builds a break*, not read-and-approve | **Adopt verbatim (§8).** Same rigor the bebop2 NTT work used and that P85 exists to enforce. |

---

## 2. Scope — what P91 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P91 OWNS
1. **P91.0** — correcting the false compliance claims in the `kem.rs` header (comment-only, §4.1).
2. **P91.1** — the ring-layer rewrite: negacyclic arithmetic, η1=2, du=10/dv=4 ct packing (1088 B), and
   the one-seed→two-seed (`d`,`z`) FO reconciliation (§4.2), ported from bebop2.
3. **P91.2** — the ship-RED gate: real ACVP ML-KEM-768 vectors + byte-exact tests + the negacyclic-wrap
   KAT + the 3-model review attestation (§4.3, §8).
4. The **do-not-wire invariant**: `volume.rs` stays un-wired until P91.2 is green (§7.4).

### 2.2 P91 does NOT own (anti-scope — prevents collision & compounding risk)
- **Wiring `pq::volume` / at-rest volume crypto** — that is the P2/D4 lane; P91 is its *precondition*,
  and P91 explicitly *blocks* it until the gate is green.
- **The Ed25519 ⊕ ML-DSA-65 signature seam** (`dsa.rs`, `capability_cert.rs`, `payment.rs`) — a
  *different primitive* on the *live* path; P91 must not touch it (§0.4).
- **P85 itself** — the NTT process-remediation is its own unit; P91.1's *NTT-port option* depends on it,
  but P91's recommended schoolbook path does not (§5).
- **The bebop2 KEM** (`/root/bebop-crypt`/`/root/bebop-repo`) — it is the *source*, already correct; P91
  changes the dowiz kernel copy, not bebop2. (Adopting/sharing bebop2's module wholesale is an in-scope
  *alternative* to patching, §5.2 — but that is a source-selection choice, not a bebop2 edit.)

### 2.3 Dependencies (named by artifact — standard §2 item 7)
**Hard inputs:** `kernel/src/pq/kem.rs` (the fix target); `kernel/src/pq/{hybrid,volume}.rs` (downstream
consumers — must keep compiling); `kernel/src/pq/kat/acvp/` (the KAT dir — needs a *new* KEM vector
file, sourced + pinned per §4.3 P91.2.0(a) — a tracked prerequisite, not assumed-available); the port
source `/root/bebop-crypt/bebop2/core/src/pq_kem.rs` @ commit `f38f2c5` (schoolbook, provenance §0.5)
and/or `/root/bebop-repo` `986646a` (NTT, P85-gated). **External input:** NIST ACVP ML-KEM-768 vectors
(§9).
**Depends on:** operator ruling on P91.0 early execution (OD-5); **P85** *only if the NTT-port option is
chosen* (§5). **Blocks:** any future wiring of `pq::volume` (P2/D4).

### 2.4 Honest reconciliation (standard §2 item 6)
The research verdict is binding and P91 does not overturn it: this is **latent, not live**; there is no
incident, no data to rotate. P91 is the *fix-before-wiring* action, sequenced so the trap (the false
header) can be defused *immediately and independently* (P91.0) while the real fix (P91.1/P91.2) proceeds
under full crypto discipline. The default posture until the gate is green is: **do not wire, and do not
trust the header.**

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

The corrected constants (the single source of the fix's parameter surface — no magic numbers):

```rust
// kernel/src/pq/kem.rs  (corrected)

pub const Q: i32   = 3329;   // unchanged (already correct)
pub const N: usize = 256;    // unchanged
pub const K: usize = 3;      // ML-KEM-768, unchanged
pub const DU: usize = 10;    // unchanged (declared) — but the PACKING must actually honor it
pub const DV: usize = 4;     // unchanged (declared) — likewise
pub const ETA2: usize = 2;   // unchanged (already correct)

pub const ETA1: usize = 2;   // WAS 3 — the ML-KEM-512 value; MUST be 2 for ML-KEM-768   ← FIX
pub const CT_LEN: usize = 32 * (DU * K + DV);   // = 1088 — WAS K*384 + 384 = 1536        ← FIX
// ring: x^256 + 1 (negacyclic) — schoolbook reduces with SUBTRACT on i+j >= N            ← FIX

// keygen: WAS keygen_internal(d: &[u8;32])  — one seed
//         MUST be keygen_internal(d: &[u8;32], z: &[u8;32]) — FIPS-203 K-PKE `d` + FO `z` ← FIX

// KAT fixtures (NEW — do not exist today, §0.3):
//   kernel/src/pq/kat/acvp/kem-encap-decap.json   (real NIST ACVP ML-KEM-768 vectors)
```

**Negacyclic reference (the correctness anchor for the port), stated as the ground-truth test shape:**
```rust
// schoolbook negacyclic multiply — the ONLY correct reduction for x^256 + 1:
//   for i in 0..N { for j in 0..N {
//       let k = i + j;
//       if k < N { sb[k]      = fq_add(sb[k],      fq_mul(a[i], b[j])); }
//       else      { sb[k - N] = fq_sub(sb[k - N],  fq_mul(a[i], b[j])); }   // SIGN FLIP on wrap
//   } }
// The current code's `(i+j) % N` with fq_ADD on wrap is the CYCLIC bug (§0.1).
```

**Falsifier tests (named for the DoD, §9):** `kem_negacyclic_wrap` (`x²⁵⁵·x == −1` in the ring),
`kem_acvp_encaps_decaps_byte_exact` (vs the new ACVP file), `kem_eta1_is_two`, `kem_ct_len_is_1088`,
`kem_decaps_constant_time_dudect` (Welch-t timing gate on decaps — audit G5, §8.2 item 6; **not**
satisfied by any of the functional KATs above, see D-CT §9).

---

## 4. Build items — spec → RED test → code, in strict order (standard §2 items 2, 3, 5)

### 4.1 P91.0 — header defusal (immediate, near-zero-risk, SEPARABLE; OD-5)

- **Spec:** strike the false claims from `kem.rs:1-12` — "ML-KEM-768 (FIPS 203)", "`Z_q[x]/(x^256+1)`",
  "provably a ring isomorphism … verified by the test suite", "Upgrade path: none needed" — and add a
  prominent warning:
  `// NOT FIPS-203: cyclic ring (x^256-1) + η1=3, ct=1536 — do NOT wire; see OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md`.
- **Why it is safe under a code-freeze:** it changes **zero behaviour** — it only removes a *false claim*
  and adds a *true warning*. No function, constant, or test changes. This is why it is separable from
  P91.1 and **immediately actionable** if the operator approves (OD-5).
- **RED test (a smart-index gate, item 14):** `kem_header_no_false_fips_claim` — a source/comment
  assertion (grep-gate) that **fails** if `kem.rs` contains "FIPS 203"/"FIPS-203" or "Upgrade path: none
  needed" **without** the `NOT FIPS-203` marker present. RED today (the false claim is there, the marker
  is not), GREEN after the edit. This turns "someone re-adds a false compliance claim" into a CI failure,
  not a future trap.
- **Governance note:** even a comment on a red-line file goes through the file owner — P91.0 is *flagged
  for operator approval* (OD-5), not auto-applied. The distinction is stated so the operator can approve
  this comment-only defusal ahead of, and independently of, the real fix.

### 4.2 P91.1 — the real ring fix (red-line lane)

- **Spec:** (a) replace the ring layer with **negacyclic** arithmetic (sign-flip on `i+j ≥ N`); (b) set
  `ETA1 = 2`; (c) fix ciphertext packing to real du=10/dv=4 compression → `CT_LEN = 1088`; (d) reconcile
  the one-seed `keygen_internal(d)` to the two-seed `keygen_internal(d, z)` FIPS-203 FO structure. **Port
  bebop2's proven code, do not re-derive** (§5 chooses the source).
- **RED tests (fail before, pass after):**
  - `kem_negacyclic_wrap` — asserts `x²⁵⁵ · x ≡ −1` in the ring (the defining negacyclic property).
    **RED today** (current ring gives `+1`), GREEN after the ring fix.
  - `kem_eta1_is_two`, `kem_ct_len_is_1088` — parameter guards. RED today, GREEN after.
  - `kem_two_seed_keygen_matches_fips` — keygen consumes `(d, z)` and produces a spec-shaped
    key/ct. RED today (one-seed), GREEN after.
- **Adversarial case (`kem_wrong_ring_rejects_acvp`):** the *current* cyclic implementation, run against a
  real ACVP vector (P91.2), **fails byte-exactness** — proving the old ring is genuinely non-compliant
  and that the fix is what makes ACVP pass (not a test that was always green).
- **Behaviour note:** `hybrid.rs`/`volume.rs` must keep compiling; their *test* KATs will change (the
  wire format changes 1536→1088), which is *correct* — a changed KAT here signals the fixed format, and
  each must be re-derived from the corrected implementation, never hand-patched to pass.

### 4.3 P91.2 — the ship-RED gate (ACVP KAT + review)

- **P91.2.0 — vector-sourcing + KEM-loader build (REAL PREREQUISITE, NOT assumed-available
  infrastructure).** The D3 conformance spine below presumes "real NIST ACVP ML-KEM-768 vectors" and a
  test that loads them — **neither exists in-repo today** (§0.3: `kernel/src/pq/kat/acvp/` holds only the
  three **ML-DSA** files `key-gen.json`/`sig-gen.json`/`sig-ver.json`; the loader `dsa/dsa_acvp_tests.rs`
  is ML-DSA-shaped only). This sub-step **must be completed first** — it is a tracked prerequisite of
  P91.2, not free infrastructure, and P91.2 cannot start until it lands:
  - **(a) Source the real vectors (pin provenance — closes audit G4).** Acquire the official NIST vectors
    from **`usnistgov/ACVP-Server`** (the authoritative ACVP generator repo), path
    `gen-val/json-files/`, two directories confirmed to exist this pass:
    **`ML-KEM-keyGen-FIPS203/`** and **`ML-KEM-encapDecap-FIPS203/`**. From each, take
    `internalProjection.json` (the `isSample: true` export that carries the expected answers inline;
    `prompt.json` = inputs only, `expectedResults.json` = answers only — the projection is the combined
    file the existing ML-DSA loader already mirrors). Vendor them into
    `kernel/src/pq/kat/acvp/kem-keygen.json` + `kem-encap-decap.json`, filtering to `parameterSet ==
    "ML-KEM-768"`. **Pin the exact `usnistgov/ACVP-Server` commit SHA** the JSON was exported from in the
    file header / commit message, matching P85's `986646a` discipline (the audit flagged this vector set
    as having "no NIST URL, version tag, or generation script"). RustCrypto `ml-kem/tests` mirrors this
    same NIST export and is an acceptable cross-check, but `usnistgov/ACVP-Server` is the source of truth.
    This vector-provenance pin, together with the §0.5 port-source commit pin, is exactly the evidence
    Q1's "Conformance / provenance" checklist row requires
    (`BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md` §Q1-b) before D3 can reach
    `DONE-VERIFIED`.
  - **(b) Build a KEM ACVP loader/harness — model it on the existing ML-DSA one.** The pattern already
    exists and MUST be reused, not reinvented: `kernel/src/pq/dsa/dsa_acvp_tests.rs` (`OnceLock`-cached
    parse, `KAT_DIR = concat!(env!("CARGO_MANIFEST_DIR"), "/src/pq/kat/acvp/")`, `serde`/`serde_json`
    dev-deps, `hex()` decode, per-`tcId` `#[test]` via `paste::item!`, plus aggregate count guards). Add a
    sibling `kernel/src/pq/kem/kem_acvp_tests.rs` with KEM-shaped structs — the ML-KEM ACVP schema differs
    from ML-DSA: **keyGen** groups carry the two seeds `d`,`z` per test with expected `ek`,`dk`;
    **encapDecap** groups carry a `function` field (`"encapsulation"`/`"decapsulation"`), where encaps
    tests give `ek`,`m` → expected `c`,`k`, and decaps groups give a group-level `dk` with tests `c` →
    expected `k`. Wire it into the `pq` module tree the same way `dsa_acvp_tests.rs` is.
  - **(c) This is a prerequisite sub-task of P91.2, explicitly.** It is added to the §2.3 dependencies and
    the §14 worker acceptance path (step 3 cannot "ship RED first" against vectors that do not yet exist).
    Until P91.2.0 is done, D3's `kem_acvp_encaps_decaps_byte_exact` has nothing to load and the FIPS-203
    conformance claim rests on vectors + a harness that are not in the tree.

- **Spec:** add real NIST ACVP ML-KEM-768 encaps/decaps vectors to `kat/acvp/kem-encap-decap.json` and a
  byte-exact test `kem_acvp_encaps_decaps_byte_exact` (loaded via the P91.2.0 harness); add the
  `kem_negacyclic_wrap` KAT; require the §8 independent 3-model review attestation.
- **RED-first discipline:** the ACVP test is written and committed **RED** (against the un-fixed code, it
  fails; that failing state is the proof the vectors are real and discriminating) *before* P91.1 lands —
  ship RED first, exactly as the bebop2 NTT work and `verified-by-math-2026-07-07` require.
- **Inadmissibility rule (the whole point):** "its own round-trip tests pass" is **not** evidence — that
  is the self-consistency trap (§6). Only external ACVP byte-exactness + independent review count.
- **Blocking:** the module may not claim FIPS-203 in any comment/commit, and `volume.rs` may not be
  wired, until `kem_acvp_encaps_decaps_byte_exact` and the review attestation are both green.

---

## 5. The port-source decision (honest refinement of the sketch — standard §2 items 6, 19)

The sketch (S3 §3) blanket-gates P91.1 on **P85** because "the bebop2 NTT is process-quarantined." Read
precisely, that gate binds **only the NTT-port option**, because the quarantined artifact is the NTT
(`986646a`, `/root/bebop-repo`) — not the schoolbook multiply. Two port sources therefore exist, with
different gating:

| Port source | Location (verified this pass) | Speed | P85-gated? | Recommendation |
|---|---|---|---|---|
| **Schoolbook negacyclic `poly_mul`** | `/root/bebop-crypt/bebop2/core/src/pq_kem.rs:296` @ commit `f38f2c5` (§0.5) — stable, KAT-gated, **un-quarantined** | O(n²) | **No** | **PRIMARY** — correctness-first (ponytail: simplest correct thing), decouples the fix from the P85 freeze |
| **Incomplete NTT `poly_mul_ntt`** | `/root/bebop-repo` `986646a` — exhaustively proven 0/65 536 but committed `--no-verify` | O(n log n) | **Yes** | **Optional speed follow-up** — only after P85 closes; building on a quarantined crypto artifact would compound A4 |

### 5.1 Recommendation (correctness-first, then speed)
Port the **schoolbook negacyclic `poly_mul`** first: it makes the KEM *correct* and *ACVP-passing* with
no dependency on the P85 remediation, and schoolbook is the clearest form to review (a reviewer verifies
256×256 well-defined term reductions, not a transform's twiddle schedule). Once P85 closes and the NTT
is de-quarantined, an **optional** P91.1-follow-up may swap in `poly_mul_ntt` for speed — gated on P85,
and only if a KEM bench (P82) shows the KEM is hot enough to justify it (the KEM is un-wired today, so
this is unlikely to matter soon). This honours *performance-priority-is-scoped-not-blanket* (perf is a
follow-up, correctness is the fix) and *crypto correctness over speed*.

**Flag for the operator (refines OD-5/S3 §3):** S3's "P91.1 gated on P85" is true for the *NTT-port*
variant; the schoolbook-port variant is **not** P85-gated. Choosing schoolbook-first lets the KEM
correctness fix proceed *before* the bebop lane's C3/P85 freeze clears — a real unblocking, stated
explicitly so it is a deliberate decision, not a silent divergence.

### 5.2 The "adopt bebop2 wholesale" alternative
Because the divergence is **structural** (ring, seeds, domain, params, wire format), replacing the whole
`kem.rs` with a shared/adopted copy of bebop2's correct `pq_kem.rs` is a legitimate alternative to
patching in place — the research doc explicitly notes this. If chosen, it is a *source-selection* change
(share the crate / vendor the module), not a bebop2 edit, and it still owes the P91.2 ACVP gate + review
(never trust the source's own tests). Recorded as an option; the default is an in-place schoolbook port
of `kem.rs` to keep the change surface reviewable.

---

## 6. The self-consistency trap — why its own suite can't see the bug (standard §2 item 6)

The KEM is *internally consistent*: keygen/encaps/decaps all use the same cyclic ring end-to-end, so its
round-trip test (encaps ss == decaps ss) and its tamper gate pass **trivially** — a wrong-but-consistent
scheme round-trips perfectly. **Consistency is not correctness.** This is the exact reason the DoD (§9)
forbids "its own tests pass" as evidence and requires *external* ACVP byte-exactness + independent
review. It is the same trap the B4/SSR-2020 forgery exposed (a shortcut that *passed the unit tests* and
was caught only because a reviewer built an actual break) — recorded so the fix is not "verified" by the
very mechanism that hid the bug.

---

## 7. Adversarial self-check — real effort to break the plan (standard §2 items 3, 5)

### 7.1 Could someone wire `volume.rs` trusting the header, shipping non-ML-KEM as FIPS-203?
**Today: yes — that is the trap, and it is the highest-severity latent risk.** The header says FIPS-203
and "Upgrade path: none needed", the commit claims ACVP conformance, and the module's own tests are
green. A developer who trusts any of these and wires `volume.rs` ships an unvetted, non-standard lattice
scheme believing it is ML-KEM-768. **P91.0 defuses exactly this** (removes the false claim, adds a
do-not-wire marker) and its grep-gate keeps the claim from being re-added. This is why P91.0 is worth
approving *immediately and independently* of the full fix.

### 7.2 Is "cyclic" merely incompatible, or a security downgrade? (honest, no over-claim)
`x²⁵⁶−1` **fully splits** over Z₃₃₂₉ (ROOT=17 has order 256) into 256 linear factors, CRT-decomposing
the ring into 256 independent Z_q copies — including the evaluation-at-1 homomorphism that maps a
Ring-LWE sample to a scalar-LWE relation on coefficient sums. Kyber/ML-KEM chose the negacyclic ring
`x²⁵⁶+1` (the 512th cyclotomic) *precisely to avoid* this full-splitting structure (it stays 128
quadratic factors). **No specific break is claimed** — but Kyber's MLWE security reduction and its
decryption-failure/noise analysis are stated over the negacyclic ring and **do not transfer** to the
cyclic variant, and η1=3 is the wrong noise. The honest characterisation: an **unvetted, non-standard
lattice scheme**, not ML-KEM-768. This is why the fix is a *security* fix, not merely an interop fix.

### 7.3 Could the port introduce a *different* bug?
Possible — which is why P91.2 gates on **external** ACVP vectors, not the ported code's own tests, and
on the §8 independent review whose mandate is to *build a break*. A port that passed the source's tests
but failed ACVP would be caught by `kem_acvp_encaps_decaps_byte_exact`. The schoolbook source (§5) is
chosen partly because it is the *most reviewable* form (no transform to get subtly wrong).

### 7.4 What stops `volume.rs` from being wired before the gate is green?
The **do-not-wire invariant** (§2.1/§4.3): `volume.rs` stays un-wired until P91.2 is green, enforced by
(a) the P91.0 marker + grep-gate, (b) the un-wired chain being kept un-wired (no default-feature
enablement), and (c) the DoD (§9 D5) making "`volume.rs` remains un-wired" a checkable state. A future
wiring attempt must first turn the ACVP KAT and review green — the gate is the guard.

### 7.5 Could P91.0 (comment-only) accidentally change behaviour?
No — it edits only `//!`/`//` comment lines. The grep-gate `kem_header_no_false_fips_claim` and an
unchanged `cargo test --features pq` (same pass/fail as before, since no code changed) are the proof.
This is why it is safe even under a code-freeze.

---

## 8. Mandatory independent adversarial-review gate — DoD-BLOCKING (standard §2 items 5, 6, 14)

**Grounded in a real prior incident** (`crypto-safe-first-pass-2026-07-14.md`): the bebop `verify_batch`
shortcut was *forgeable* — an independent reviewer **built and ran** an Ed25519 mixed-order SSR-2020
forgery that the pre-fix code wrongly accepted; it *passed the unit tests* and was caught only because a
reviewer built an actual break. **P91.1 is a ring-layer crypto rewrite and does NOT ship until an
independent adversarial review passes.** Unit-green — and especially "its own round-trip test passes" —
is necessary, not sufficient (§6).

### 8.1 Reviewer independence
Performed by an actor **not** the implementer — a decorrelated model / `security-sentinel` /
`system-breaker` — with the **same 3-model rigor as the bebop2 NTT work** (the discipline P85 exists to
enforce). Mandate: **produce a concrete discrepancy or a proof of correctness**, not read-and-approve.

### 8.2 What the review MUST attempt (each a concrete artifact)
1. **ACVP mismatch** — run the ported implementation against the real NIST ACVP ML-KEM-768 vectors; any
   byte mismatch in encaps/decaps is a FAIL.
2. **Ring correctness** — verify `x²⁵⁵·x == −1` and that the schoolbook/NTT agree on random polynomials
   (negacyclic, not cyclic).
3. **Parameter conformance** — η1=2, du=10/dv=4, ct=1088, two-seed keygen — each checked against the
   standard, not the code's comments.
4. **Decryption-failure sanity** — confirm the corrected noise (η1=2) yields the spec's decryption-
   failure regime, not the η1=3 distribution.
5. **No live-path leakage** — confirm `volume.rs` is still un-wired and the signature seam is untouched.
6. **Constant-time / no secret-dependent branch (audit G5 — distinct from items 1-5, which are all
   functional).** The concrete decaps timing-leak sites named by the audit: the **FO re-encryption**
   inside decaps (re-running K-PKE.Encrypt on the decrypted message and comparing to the received
   ciphertext), the **implicit-rejection** branch (the constant-time select between the real shared
   secret and the pseudorandom rejection value — never an `if`/early-return keyed on the comparison
   result), and the **mod-q reductions** in `poly_mul`/byte-decode (any operation whose timing depends on
   a secret coefficient's *value* rather than only on public shape). The reviewer must inspect each for
   secret-dependent branches or variable-time arithmetic and file a dudect-style statistical timing
   argument (fixed-vs-random decaps inputs, Welch `|t|` threshold) — **reuse the repo's own existing
   dudect pattern** (`bebop2/core/src/sign.rs` C4b gate, `|t| < 4.5` for `mod_l`, `/root/bebop-crypt`) as
   prior art rather than inventing a new statistical-timing harness. **This is the audit's own explicit
   instruction: do not close P91 on functional KATs alone** — items 1-5 and D3's ACVP byte-exactness
   prove the KEM is *correct*; they say nothing about whether decaps is *constant-time*, and a
   functionally-correct, timing-leaky decaps is still a FAIL of this item (tracked separately as D-CT,
   §9 — not folded into D3/D4).

### 8.3 Gate outcome (falsifiable)
- **PASS** = written attestation that ACVP byte-exactness holds, each §8.2 check was *attempted with a
  concrete input*, and any discrepancy found was fixed and re-verified. Filed under `docs/reflections/`,
  referenced from the DoD (D-REVIEW).
- **FAIL** = any ACVP mismatch, or any check not genuinely attempted → the fix is RED and does not ship,
  and the module keeps its `NOT FIPS-203` marker.

---

## 9. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (RED test / check) |
|---|---|---|
| D0 | the false compliance claims are gone and cannot be re-added silently (P91.0) | `kem_header_no_false_fips_claim` grep-gate GREEN; `cargo test --features pq` unchanged (no behaviour change) |
| D1 | the ring is negacyclic (`x²⁵⁶+1`) | `kem_negacyclic_wrap` (`x²⁵⁵·x == −1`) — RED today, GREEN after |
| D2 | parameters are ML-KEM-768-correct | `kem_eta1_is_two`, `kem_ct_len_is_1088`, `kem_two_seed_keygen_matches_fips` — RED today, GREEN after |
| D3 | the KEM is byte-exact vs real NIST ACVP ML-KEM-768 vectors | `kem_acvp_encaps_decaps_byte_exact` against the NEW `kat/acvp/kem-encap-decap.json` — RED against the old ring, GREEN after the fix (the discriminating gate) |
| D4 | the fix is proven by *external* evidence + independent review, not self-consistency | §8.3 PASS attestation under `docs/reflections/` (D-REVIEW); "its own tests pass" is explicitly inadmissible |
| D-CT | decaps has no secret-dependent branch/timing on FO re-encryption, implicit rejection, or mod-q reduction (audit G5) — **D1-D3's functional KATs do NOT discharge this row** | §8.2 item 6 attestation (`kem_decaps_constant_time_dudect` / equivalent Welch-t argument) filed alongside D-REVIEW; RED if D1-D4 are all green but D-CT is unaddressed |
| D5 | `volume.rs` remains un-wired until all the above are green | the `pq` feature stays default-off; no dependent crate enables it; the do-not-wire marker present (§7.4) |
| D-PORT | the port source is chosen and its gating recorded | schoolbook (`/root/bebop-crypt` @ `f38f2c5`, not P85-gated) OR NTT (`/root/bebop-repo` `986646a`, P85-gated) — the choice + gate + commit pin written into the commit/PR (§5, §0.5) |
| D-BUILD | `cargo test --features pq` fully green incl. all new REDs now GREEN; default build (452 tests) unaffected; no dep added | `cargo test` (default) + `cargo test --features pq` |
| D-NOREG | the signature seam / cert / money paths are provably untouched | grep-confirm no edit to `dsa.rs`/`capability_cert.rs`/`payment.rs`; their tests stay green |

**DoD honesty:** D3 + D4 are the functional spine — external ACVP conformance and independent review.
**D-CT is a separate, non-optional spine item (audit G5), not subsumed by D3/D4** — byte-exactness proves
correctness, not constant-time; a KEM can pass D3 and still leak secrets through decaps timing, so D-CT
must be independently attested before P91 is closed. D0 is the separable near-zero-risk item shippable
ahead of everything else (OD-5). D-PORT records the §5 refinement so the P85-gating is a deliberate,
visible choice, and now also carries the `f38f2c5` provenance pin (§0.5, closes audit G4's port-source
half).

---

## 10. Benchmarks + telemetry (standard §2 item 10 — honest applicability)

**Correctness is the deliverable here, not speed** — but a KEM bench belongs in P82's bebop/KEM bench
lane for the *record*, so that if the NTT-port speed follow-up (§5.1) is ever pursued, the schoolbook
baseline and the NTT candidate are measured head-to-head in one convention:

| Bench | Measures | Note |
|---|---|---|
| `bench_kem_keygen/encaps/decaps` (schoolbook) | the corrected schoolbook KEM cost | the baseline; establishes whether the KEM is anywhere near a hot path (it is un-wired today, so almost certainly not) |
| `bench_kem_poly_mul {schoolbook, ntt}` | ring-multiply cost | only relevant *if* the P85-gated NTT follow-up is pursued |

**Telemetry:** N/A while un-wired — there is no live KEM path to instrument. Stated honestly rather than
inventing a hook for dead code. If `volume.rs` is ever wired (post-gate), it inherits the kernel span-
metrics of P83, not a P91-specific hook.

---

## 11. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16)

- **Hazard-safety as math (item 6):** the unsafe state — *a non-ML-KEM scheme wired to real at-rest data
  while believed FIPS-203* — is made unreachable by (i) the do-not-wire invariant + the P91.0 marker/
  grep-gate (a false FIPS claim is a CI failure), (ii) the ACVP gate (the module cannot *become*
  wired-and-trusted without external byte-exact conformance), and (iii) the D-CT constant-time gate
  (audit G5) — a functionally-correct-but-timing-leaky decaps must not become "trusted" either;
  functional byte-exactness is explicitly inadmissible evidence for *that* hazard (§8.2 item 6). Argued
  from the gate/flow structure, not a prose assurance.
- **Schemas & scaling axis (item 8):** the KEM wire schema is **fixed-size** (ek/dk/ct are constant-
  length per ML-KEM-768: ct 1088 after the fix). It does not scale with nodes/events — it is a per-
  operation fixed record. The only "scaling" is calls/sec once wired, bounded by whatever P2/D4 volume
  crypto drives it (not P91's concern while un-wired).
- **Isolation / bulkhead (item 11):** the `pq` feature gate **is** the bulkhead — the bug is quarantined
  behind a default-off compile flag with zero live callers, so its failure cannot propagate to any live
  path. P91 *keeps* that bulkhead closed until the fix is proven. The signature seam is a separate
  primitive (separate bulkhead), untouched.
- **Mesh awareness (item 12):** **node-local, honestly** — the KEM is at-rest volume crypto (KEM-DEM),
  not a transport/gossip primitive; nothing here rides `iroh_transport.rs`. The mesh's PQ is the
  *signature* seam (a different unit). No payload/frequency budget applies. Stated.
- **Rollback / self-healing as math (item 13):** **Self-termination** = the feature gate + do-not-wire
  invariant (a wrong-ring KEM cannot reach live data — unrepresentable, not a supervisor's choice).
  **Snapshot re-entry / self-healing = NOT claimed** — a KEM is a stateless primitive; there is no
  epoch to re-enter and no error-correction. Claiming either would be false.
- **Error-propagation / smart index (item 14):** the bug class (a false compliance claim; a wrong ring
  passing self-consistent tests) is turned into CI/compile failures by `kem_header_no_false_fips_claim`
  (grep-gate on the header), `kem_negacyclic_wrap` (ring property), and `kem_acvp_encaps_decaps_byte_exact`
  (external conformance) — not a runtime surprise, and specifically not "the self-consistent suite is
  green so it's fine."
- **Living-memory awareness (item 15):** **N/A while un-wired.** Once wired, at-rest volume ciphertext is
  durable-by-design (the opposite of ephemeral) — but that is the P2/D4 lane's concern; P91 only fixes
  the primitive. Stated.
- **Tensor/spectral (item 16):** **partially applicable, honestly** — the NTT *is* a spectral transform
  (a length-256 DFT over Z_q), and the correct fix is precisely a *negacyclic* transform. But the
  recommended path (§5) uses **schoolbook** for reviewability/correctness-first, and the spectral (NTT)
  form is an optional P85-gated speed follow-up. So spectral machinery is *acknowledged as the natural
  fast form* but deliberately deferred — not forced in for its own sake (ponytail).
- **Linux discipline (item 9):** **REINFORCES** the "fix the primitive, keep the bulkhead closed" idiom
  (default-off feature gate = a compile-time firewall around unproven code); **EXTENDS** the KAT-gated
  crypto pattern to the KEM (which today has *no* KAT — the gap this closes); **DOES-NOT-TRANSFER** — no
  new subsystem, and the port is reuse-not-rewrite (§5).

---

## 12. Hermetic principles honored (standard §2 item 20 — load-bearing only)

- **Correspondence ("as above, so below"):** the code must correspond to what it *claims* — the header
  said FIPS-203 while the ring said otherwise. P91.0 restores correspondence between claim and code
  (defuse the lie); P91.1 restores correspondence between code and the *standard* (make it actually
  ML-KEM-768). The whole unit is a correspondence-repair.
- **Polarity / no-middle:** a scheme is either *conformant ML-KEM-768* (ACVP byte-exact) or *not* — there
  is no "mostly compliant" middle. The self-consistent-but-wrong state is exactly the false middle the
  ACVP gate abolishes: pass the external vectors or you are not FIPS-203, full stop.
- **Cause & Effect:** trust in a crypto primitive must have a *cause* — an external conformance proof +
  independent review — never the effect (a green self-referential suite) standing in for its own cause.
  §6/§8 encode this: the effect ("tests pass") is inadmissible as its own cause.

---

## 13. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (ring/η1/ct/header/KAT-dir/consumer-chain all read this pass; `kem.rs:21/24/28`, `lib.rs:13-14/179`, `kat/acvp/` listing) |
| 2 | Falsifiable DoD | §9 (D0–D-NOREG, each a RED→GREEN test or attestation) |
| 3 | Spec→test→code, event-driven | §4 (spec-first per sub-item; ACVP RED-first) |
| 4 | Predefined types & constants | §3 (corrected `ETA1`/`CT_LEN`/two-seed keygen; negacyclic reference; falsifier names) |
| 5 | Adversarial/breaking tests | §4.2 `kem_wrong_ring_rejects_acvp`, §7 (self-attack), §8 (independent review builds a discrepancy) |
| 6 | Hazard-safety from structure | §11 (feature-gate bulkhead + do-not-wire invariant + ACVP gate make wired-and-trusted-but-wrong unreachable), §6 (self-consistency trap) |
| 7 | Links to docs & memory | §14 |
| 8 | Schemas with scaling axis | §11 (fixed-size KEM records; calls/sec only once wired) |
| 9 | Linux engineering discipline | §11 (REINFORCES/EXTENDS/DOES-NOT-TRANSFER verdict) |
| 10 | Benchmarks + telemetry | §10 (KEM bench for the record in P82's lane; telemetry N/A while un-wired, stated) |
| 11 | Isolation / bulkhead | §11 (`pq` feature gate = the bulkhead; kept closed until proven) |
| 12 | Mesh awareness | §11 (node-local at-rest crypto, not transport; stated) |
| 13 | Rollback/self-heal as math | §11 (self-termination = gate+invariant; self-healing NOT claimed) |
| 14 | Error-propagation / smart index | §11 + §4.1 (grep-gate on the header, ring KAT, ACVP conformance catch the bug class at CI time) |
| 15 | Living-memory awareness | §11 (N/A while un-wired; durable-by-design once wired = P2/D4's concern) |
| 16 | Tensor/spectral where applicable | §11 (NTT is spectral; acknowledged, deferred to schoolbook-first + P85-gated NTT follow-up — not forced) |
| 17 | Regression tracking | §9 D1/D3 (`kem_negacyclic_wrap` + ACVP byte-exact test are permanent guards); add to REGRESSION-LEDGER |
| 18 | Clear worker instructions | §14 |
| 19 | Reuse-first, upgrade-if-needed | §1/§5 (port bebop2's proven code, do not re-derive; schoolbook-first, NTT as gated upgrade) |
| 20 | Hermetic principles | §12 |

---

## 14. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-KEM-RING-BUG-INVESTIGATION-2026-07-18.md` (the dedicated follow-up; recovered per
  MASTER-STATUS-LEDGER §0 — restore from the scratchpad `recovered/` dir if the path does not resolve).
- `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` §3 (P91.0/P91.1/P91.2 structure), §6 W3-5 (OD-5).
- `SYNTHESIS-PHYSICS-PERFORMANCE-VISION-2026-07-18.md` §3/§4.1 (P85 — the NTT remediation the NTT-port
  option gates on).
- `MASTER-STATUS-LEDGER-2026-07-19.md` (P91 row; OD-5/OD-6; §3 wave-3 sequence).
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Port source: `/root/bebop-crypt/bebop2/core/src/pq_kem.rs` @ commit `f38f2c5` (correct, un-quarantined
  — schoolbook; provenance pinned §0.5, closes audit G4's port-source-anchoring half); `/root/bebop-repo`
  `986646a` (NTT, P85-quarantined).
- `BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md` §Q1-b (the "Conformance / provenance"
  claim-shape this blueprint's §0.5/§4.3(a) pins now discharge).
- Memory: `crypto-safe-first-pass-2026-07-14.md` (B4/SSR-2020 — the review discipline §8),
  `verified-by-math-2026-07-07.md` (ship-RED), `rust-native-bare-metal-decision-2026-07-14.md`
  (reuse-first / DECART for any dep change), `never-bypass-human-gates-2026-06-29.md` (OD-5 is the
  operator's ruling).

**Existing code this blueprint edits (exact targets — dowiz kernel):**
- **EDIT (comment-only, P91.0)** `kernel/src/pq/kem.rs:1-12` — strike the false FIPS-203/ring/"none
  needed" claims; add the `NOT FIPS-203` marker. Shippable independently (OD-5).
- **EDIT (ring layer, P91.1)** `kernel/src/pq/kem.rs` — negacyclic multiply (sign-flip on `i+j≥N`),
  `ETA1 = 2`, du=10/dv=4 packing → `CT_LEN = 1088`, two-seed `keygen_internal(d, z)`. **Port** from the
  §5-chosen source; do not re-derive.
- **RE-DERIVE KATs** `kernel/src/pq/hybrid.rs` + `volume.rs` test vectors — regenerate from the corrected
  implementation (the wire format changes; never hand-patch a KAT to pass).
- **NEW** `kernel/src/pq/kat/acvp/kem-encap-decap.json` — real NIST ACVP ML-KEM-768 vectors (P91.2).
- **DO NOT TOUCH** `kernel/src/pq/dsa.rs`, `capability_cert.rs`, `payment.rs` — the Ed25519⊕ML-DSA
  *signature* seam (different primitive, live path).
- **DO NOT WIRE** `kernel/src/pq/volume.rs` into any live path until D3+D4 are green.

**For the worker/operator with zero session context — exact acceptance path:**
1. **Decide OD-5 (P91.0).** If approved, apply the comment-only header defusal *now* — it is safe under a
   freeze, defuses the trap, and its grep-gate keeps the false claim from returning. This can land ahead
   of everything else.
2. **Choose the port source (§5, D-PORT).** Default: schoolbook negacyclic `poly_mul` from
   `/root/bebop-crypt` @ commit `f38f2c5` (§0.5) — correctness-first, **not** P85-gated. Only choose the
   NTT (`/root/bebop-repo` `986646a`) if P85 has closed, and record the gate in the PR.
2a. **Vector + harness prerequisite (P91.2.0, §4.3) must land before step 3.** Source + pin the real
   NIST ACVP ML-KEM-768 vectors (provenance: `usnistgov/ACVP-Server` commit SHA, §4.3(a)) and build the
   KEM ACVP loader harness modeled on `dsa_acvp_tests.rs` (§4.3(b)) — this is a named prerequisite of
   P91.2, not something D3 can assume exists.
3. **Ship RED first (P91.2).** Add the real NIST ACVP ML-KEM-768 vectors + `kem_acvp_encaps_decaps_byte_exact`
   and `kem_negacyclic_wrap` as RED tests against the *current* code — confirm they FAIL (proving they
   are real and discriminating) before touching the ring.
4. **Apply P91.1** — port the negacyclic arithmetic, set η1=2, fix ct packing to 1088, reconcile the
   two-seed keygen. Re-derive the `hybrid`/`volume` KATs from the corrected code. Turn the RED tests
   GREEN; `cargo test --features pq` fully green; default build (452) unaffected.
5. **Route the §8 independent review** — a decorrelated reviewer must run the ACVP vectors + attempt each
   §8.2 discrepancy (including item 6, constant-time on FO re-encryption/implicit-rejection/mod-q
   reduction — audit G5) and file the PASS attestation under `docs/reflections/` (D-REVIEW). "Its own
   tests pass" is inadmissible, and **passing D3's functional ACVP KATs does not by itself satisfy D-CT**
   — the constant-time attestation is a separate, required artifact.
6. **Register the regression** — add `kem_negacyclic_wrap` + the ACVP byte-exact test to
   `docs/regressions/REGRESSION-LEDGER.md` (item 17).
7. **Anti-scope:** never wire `volume.rs` before D1-D5 (incl. D-CT) green; never touch the signature
   seam; never re-derive the ring from scratch; never let the module claim FIPS-203 until ACVP
   byte-exactness holds; never treat functional-KAT-green as a substitute for the D-CT constant-time
   attestation.
