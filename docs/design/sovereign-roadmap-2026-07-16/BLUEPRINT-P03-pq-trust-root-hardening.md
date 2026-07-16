# BLUEPRINT — Phase 3: PQ TRUST-ROOT HARDENING

> Crypto-correctness phase. Harden the cryptographic foundation before anything rides on it.
> **Planning document only — NO crypto code is written or edited here.** Changes of this
> sensitivity get their own careful implementation pass, informed by this blueprint.
>
> - **Anchors:** M2, M4, M12, E10 (≡ E36 alias, per Phase 2 ruling), F19, F21, F24, F26
> - **Depends on:** — (Wave 0). **Gates:** Phase 6 (V1 identity), Phase 9 (wire confidentiality),
>   Phase 10 (kill-switch signing), Phase 13 (order signing). On the critical path P3→P9→P10→P13.
> - **Parallel-safe with:** Phase 1, 2, 4, 5.
> - **Repo (NOT dowiz):** `/root/bebop-repo/bebop2/` — crates `core`, `proto-cap`, `proto-crypto`,
>   `proto-wire`, `mesh-node`, `delivery-domain`. Cross-repo references: `/root/dowiz-pq/kernel/src/pq/`
>   (third PQ stack), `/root/bebop-repo/crates/bebop/` (legacy product node).
> - Source of truth for every claim below: `R1-A-mesh-crypto-gap-analysis.md`, cross-checked against
>   the live tree this session. Every file:line was re-read, not trusted from memory.

Canon (`ARCHITECTURE.md`) says of F21 "*Attacker has quantum computer … ML-DSA holds (FIPS204)*."
That is false as built: the thing ML-DSA protects (frames) is not the thing an attacker forges (the
authorization chain). This phase makes the claim true. It touches only the authorization and primitive
layers — it does **not** wire the mesh, encrypt the wire, or integrate dowiz (Phases 9/13, which would
be built on sand without this phase).

---

## 1. Current-state evidence (file:line)

### 1.1 The trust root is classically forgeable — F21 (HIGH)

Frames are hybrid-signed (Ed25519 + ML-DSA-65, both legs real and ACVP-verified —
`proto-cap/src/signed_frame.rs:186` classical, `:202` PQ storing the 3309-byte sig). **But the
authorization chain those frames ride on is Ed25519-only.**

- `proto-cap/src/roster.rs` — module doc `:11-14` states outright: "*a small, fixed set of **Ed25519**
  public keys enrolled at genesis*." The `Delegation` struct (`:97-118`) carries exactly one
  `signature: Vec<u8>` — "*Ed25519 signature (64 bytes) over `canonical_bytes()`, by `issued_by`*"
  (`:114`). `Delegation::sign` (`:167`) calls `bebop2_core::sign::sign(seed, &msg)` (Ed25519 only);
  `Delegation::verify_signature` (`:179`) calls `bebop2_core::sign::verify`. There is no PQ leg
  anywhere on the delegation path.
- `AnchorRoster` (`:191-226`) is a `HashSet<[u8; 32]>` of 32-byte **Ed25519** public keys.
  `verify_chain` (`:252-316`) roots trust at `root.issued_by` (a `[u8; 32]`) checked via
  `roster.contains(&root.issued_by)` (`:261`) — a purely classical anchor check.
- Genesis loader `proto-cap/src/node_id.rs:105,116` reads "*one hex-encoded 32-byte **Ed25519**
  public key per line*" and enrolls each into the `AnchorRoster`. The genesis file format itself is
  classical-only.

**The forgery (verbatim adversary from F21):** A quantum attacker recovers an enrolled anchor's
Ed25519 private key (Ed25519 is broken by Shor), signs a fresh `Delegation` chain rooted at that
anchor, and names *their own* subject key as the tail — the delegation's `subject` is attacker-chosen
(`roster.rs:104`). Because `verify_chain` binds the chain tail to `cap.subject_key` (`:300`) but never
to any PQ key, the attacker then supplies **their own** ML-DSA keypair as the capability's PQ leg. The
frame's ML-DSA signature verifies (it is the attacker's real ML-DSA key), and the chain verifies (real
Ed25519 forgeries). The entire post-quantum leg is **vacuous against exactly the F21 adversary it
exists to stop** — the PQ signature authenticates a key the attacker minted, chained off a classically
forged authorization. R1-A headline #4 names this; the canon does not.

The identity layer already knows better: `node_id.rs:46-50` derives `NodeId = SHA3-256(pq_pub ‖
classical_pub)` and `NodeKeys` (`:69-71`) holds both `pq_pub: Vec<u8>` and `classical_pub: [u8; 32]`.
Identity binds both keys; **authorization throws the PQ half away.** That asymmetry is the whole bug.

### 1.2 C4b — `mod_l` variable-time secret-dependent branch (HIGH, live classical side-channel)

`core/src/sign.rs` — the group-level scalar multiply `scalar_mul` (`:724-737`) was already made
constant-time under C4 (branch-free `point_select`, `:733`; guarded by the `scalar_mul_op_count_is_constant`
test). **But the scalar-reduction layer beneath it was not.** `mod_l` (`:625-654`) reduces secret
material with a secret-bit branch — `if (byte >> bit) & 1 == 1 { rem = add_be(&rem, &[1]); }`
(`:631-633`) — plus a data-dependent conditional subtract (`:634-636`) and variable-length `add_be`/
`sub_be` bignums. It is called on the secret nonce and key via `scalar_from_hash` (`:659`),
`scalar_mul_mod_l` (`:669`), and `scalar_add_mod_l` (`:675`), all reached from `sign` (`:822-823`).
The code's own comment tags this precisely: "*SAME bug class as the branch removed above … biased-nonce
lattice attacks make it HIGH priority. **Tracked as C4b.***" (`:712-723`). It prescribes the fix:
"*A fixed-width Barrett/Montgomery rewrite of BOTH the scalar-mod-L and field reduction closes the
gap.*" This was flagged as a follow-up in the `crypto-safe-first-pass-2026-07-14` pass and **never
closed**. `verify` uses only public scalars (`:723`), so it is unaffected — the fix is scoped to the
signing path.

### 1.3 C6b — test fixtures co-derive both hybrid legs (STILL OPEN, test-only)

`proto-wire/src/wss_transport.rs:707` and `iroh_transport.rs:421` feed the *same* `leaf_seed` to both
`keygen_derivable` and `sign_classical`, so the two hybrid legs are not independently keyed in fixtures.
Test-only, low risk, but it means a hybrid regression could hide behind correlated test keys. Tidy it
alongside C4b so the hybrid-anchor tests added in this phase key the two legs independently.

### 1.4 Three parallel "ML-KEM-768" stacks, two non-FIPS (CORRECTNESS + dual-authority)

| Stack | File | η₁ | G | Implicit rejection | Status |
|---|---|---|---|---|---|
| **core** (canonical) | `core/src/pq_kem.rs` | `2` (`:258`, FIPS) | SHA3-512 (`:222,603,679,713`) | `J(z‖c)=SHAKE256` (`:719`, test `:987`) | FIPS-203-exact, wired, ACVP-passing |
| proto-crypto | `proto-crypto/src/pq_kem.rs` | `3` (`:166`) | SHAKE256 (`:136`) | `H(sk‖c)` (`:583`) | non-FIPS, **unwired** |
| dowiz-pq | `/root/dowiz-pq/kernel/src/pq/kem.rs` | (matches proto-crypto) | — | `H(sk‖c)` | non-FIPS, separate repo |

`core::pq_kem` is FIPS-203-exact and passes the official ACVP vectors:
`kem_implicit_rejection_equals_fips203_j` is GREEN (`:987`), and a `dual_impl_bit_exact` byte-agreement
test already exists (`:930`). `proto-crypto/src/pq_kem.rs` sets `ETA1 = 3` "*matched to the external
reference impl*" (`:166`), uses `H(sk‖c)` for implicit rejection (`:583`), and `G = SHAKE256` (`:136`).
**Its own header contradicts itself:** lines 4-5 claim "*the FIPS 203 §8 Table-2 parameter set … η₁=2*"
and "*FIPS 203 interoperable*", while line 166 hard-codes `η₁=3`. It is KAT-locked to `dowiz-pq`'s
kernel KEM, which shares the same non-FIPS choices. So two implementations both named "ML-KEM-768" are
bit-exact to **different, mutually inconsistent oracles** — the same failure family as the
3-eigensolver dual-authority bug. A downstream consumer that picks the wrong one is silently
non-interoperable with every real FIPS-203 peer, and the divergence is currently invisible because no
CI test pins the two against each other.

### 1.5 Red-line gate maps only Money — M12 / F26 (Auth/Secrets/Migrations unmapped)

`proto-cap/src/redline.rs` — `RedLineCategory` (`:26-38`) declares all four categories
`Money | Secrets | Migrations | Auth`, and `RedLinePolicy::default()` is `DenyByDefault` (`:54-58`,
fail-closed). **But `is_red_line` (`:68-79`) maps only Money verbs** — `(Ledger, SettlementRecorded)`,
`(Ledger, Append)`, `(Order, CreateOrder)`, `(Claim, _)`. The comment admits it: "*this build maps
only the money verbs that actually exist*" (`:66-67`). The root cause is upstream in
`proto-cap/src/scope.rs`: the `Resource` enum (`:12-43`) has no `Auth`/`Secret`/`Migration` variant,
and `Action` (`:47-90`) has no auth/secret/migration verb. **A capability whose scope names an
auth-, secrets-, or migration-class effect cannot even be *expressed*, so it slips past the red-line
deny path entirely** — the gate cannot deny a category it has no vocabulary for. This directly
contradicts M12 ("red-line deny (auth/money/secrets/migrations)") and the module's own doc
(`redline.rs:10-11`).

### 1.6 Classical KEM fallback (X25519) absent from bebop2 — M2 "classical-fallback"

`grep x25519|ecdh` over `bebop2/` returns **zero**. Classical *signature* fallback exists
(Ed25519, `core/src/sign.rs`), but the classical *KEM* leg of M2's "ML-KEM-768 … + classical-fallback"
does not. The only ML-KEM⊕X25519 hybrids in the tree are external-crate-based and out of the wire
boundary: `/root/dowiz-pq/kernel/src/pq/hybrid.rs` ("*X25519 (classical) + ML-KEM-768 … BOTH
mandatory*", via `x25519-dalek`/`curve25519-dalek`) and legacy `crates/bebop/src/vault.rs` (RustCrypto
`x25519-dalek`/`ml-kem`). Both violate M6's zero-dep wire boundary, so neither can be lifted into
`core` as-is.

### 1.7 The ladder/wycheproof/fips-regen harness is placeholder stubs

`proto-crypto/src/{ladder,wycheproof,fips_regen,constant_time}.rs` are each a bare
`pub struct Placeholder;` with a `TODO(P0-6/H)` header — no tier registry, no vector loader, no
re-derivation oracle, no active CT assertion (`ladder.rs:10`, `wycheproof.rs:11`, `fips_regen.rs:11`,
`constant_time.rs:14`). The only real assertions in that crate are compile-time proofs that
`Placeholder` is zero-sized and score-free (`lib.rs:64-81`). There is **no negative-test / malformed-
input harness** proving the primitives reject bad ciphertexts, tampered signatures, or out-of-spec
encodings.

### 1.8 Stale "PQ is TODO" docstring — M12 doc-drift

`proto-cap/src/lib.rs:11-13` still reads: "*the post-quantum (ML-DSA-65) leg is a marked TODO pending
the `bebop2-core::pq_dsa` pack/unpack byte API*." R1-A confirmed this is false — the leg is real,
wired (`signed_frame.rs:202`), and ACVP-verified. The header actively lies about the security posture
of the crate it documents.

### 1.9 Already BUILT — regression-guard targets, not build targets

- **F19** (fail-closed unknown capability scope): BUILT. `scope.rs` `from_discriminant → Option`
  (`:166,215`) returns `None` on unknown bytes; `verify_chain` rule (e) rejects out-of-subtree effects
  as `ScopeViolation` (`roster.rs:304-308`); typed `CapError`, never a silent pass.
- **F24** (expired capability rejected): BUILT. `Capability::is_fresh` + per-link expiry check
  (`roster.rs:271`) and the gate pre-check. Tests `green_expired_link_rejected` (`roster.rs:522`).
- The hybrid sign/verify **pattern already exists** in `signed_frame.rs:186-241` (both legs). Phase 3
  does not invent it — it applies the same pattern to `roster.rs`, where it was never applied.

**This phase's job for F19/F24 is regression-guarding through the refactor, not building from zero.**

---

## 2. The three-stack reconciliation decision

**Decisive fact discovered this session:** `grep` for any importer of `bebop-proto-crypto` across the
workspace returns only its own `Cargo.toml` — **no crate consumes `proto-crypto`.** Its divergent KEM
and its placeholder harness are unwired standalone code. `core::pq_kem` is the FIPS-exact, ACVP-passing,
*wired* implementation. This makes the decision cheap and unambiguous:

**DECISION — `core::pq_kem` (ML-KEM-768) and `core::pq_dsa` (ML-DSA-65) are the ONE canonical PQ
primitive line.** Both are FIPS-exact, both pass vendored NIST ACVP vectors, both are the versions the
signed path already calls. Everything else is reconciled to them or struck.

**proto-crypto's KEM — two acceptable outcomes, pick one, do not leave silent disagreement:**

- **Option A (preferred): strike it.** Delete `proto-crypto/src/pq_kem.rs`. Nothing imports it; its
  non-FIPS constants are a liability with no consumer. Cleanest and removes the dual-authority hazard
  at the root. If any future need for the "H"-line crypto surface arises, it re-exports `core::pq_kem`.
- **Option B (only if proto-crypto must keep a KEM as an independent cross-check oracle): mark it
  oracle-only and pin it.** Rename the module to make its role explicit (it is *not* a second
  production KEM), fix the self-contradicting header (η₁ is 3, say so, and drop the "FIPS 203
  interoperable" claim — it is interoperable with the *dowiz-pq reference*, not with FIPS), and add a
  **CI byte-agreement test** that is allowed to assert **disagreement on the FIPS-divergent points and
  agreement on the FIPS-invariant ones**, so the divergence is a documented, tested fact rather than a
  latent trap. Given that no consumer exists, Option A is strictly better; Option B is only for the
  case where the operator wants an intentionally-different second oracle retained.

**The byte-agreement CI test (mechanism).** Precedent already exists: `core/src/pq_kem.rs:930`
`dual_impl_bit_exact` and `:987` `kem_implicit_rejection_equals_fips203_j` are exactly this shape —
keypair from a fixed seed, run encaps/decaps, `assert_eq!` the pk/sk/ct/shared-secret bytes against a
pinned oracle. `dowiz-pq` lives in a separate repo (out of scope to delete here), so it is *fenced*: if
it remains a consumer of `bebop2` crypto it consumes `core::pq_kem` and its own `kem.rs` is struck; if
it stays independent, a `#[test]` records the exact bytes at which it diverges (η₁, G, implicit-
rejection input) so the difference is **named and asserted**, never a surprise. **Done-test: exactly ONE
implementation on the live path (`core`); every other copy deleted or pinned to it by a CI test.**

**ML-DSA-65:** already single-source (`core::pq_dsa`, `PUBLICKEYBYTES=1952`/`SIGNATUREBYTES=3309`,
`:58,64`; ACVP-verified). No reconciliation needed — only the new hybrid delegation path (§3.1) must
call it, and the stale docstring (§1.8) must stop calling it a TODO.

---

## 3. Target-state design per gap item

### 3.1 Hybrid (ML-DSA + Ed25519) delegation links + genesis anchors — F21, M4

Mirror the proven `signed_frame.rs` hybrid pattern into the delegation path. Specifically:

- **`roster.rs` `Delegation` struct (`:97-118`):** add the PQ leg alongside the classical one —
  a PQ issuer public key (or a reference resolvable to it) and `signature_pq: Vec<u8>` (3309 bytes,
  ML-DSA-65). `canonical_bytes()` (`:126`) stays the signing input for **both** legs (single message,
  two signatures — same as a frame). `Delegation::sign` (`:148`) gains a `pq_sk` parameter and calls
  `bebop2_core::pq_dsa::sign(sk, &msg, rnd)` after the existing `bebop2_core::sign::sign`.
  `verify_signature` (`:172`) verifies **both** legs and fails closed if either is absent or invalid.
- **Anchors bind both keys.** An enrolled anchor becomes a `NodeId = SHA3-256(pq_pub ‖ classical_pub)`
  (the identity already computed at `node_id.rs:46-50`), or equivalently the pair of full public keys.
  `AnchorRoster` (`:191-226`) stores anchors such that `verify_chain` can check that the delegation's
  issuer keys hash to an enrolled anchor **and** that the ML-DSA leg verifies. This closes the forgery:
  a quantum attacker who forges the Ed25519 leg still cannot produce the anchor's ML-DSA signature.
- **Genesis file format (`node_id.rs:105,116`):** change from "one hex Ed25519 key per line" to a
  format carrying **both** the ML-DSA and Ed25519 public keys per anchor (e.g. `pq_pub_hex :
  classical_pub_hex`, or the `NodeId` hex if the full keys are distributed out-of-band). The loader
  stays fail-closed (missing/malformed/empty → error, per the existing `require_explicit_policy`
  contract, `node_id.rs:179`).
- **`verify_chain` (`:252-316`):** the root check (`:261`) becomes "issuer keys hash to an enrolled
  anchor NodeId"; each link's `verify_signature` now checks both legs; the tail must bind to
  **both** of `cap`'s keys, not just `subject_key`. The gate ordering in `hybrid_gate.rs:124-201`
  (expiry → chain → red-line → revocation → classical → PQ → commit-nonce) is unchanged — the chain
  step simply becomes hybrid.
- **Quantum-attacker test fixture:** construct a chain where the Ed25519 leg is a valid forgery
  (attacker holds the anchor's Ed25519 key) but the ML-DSA leg is absent/attacker-keyed; assert
  `verify_chain` rejects it. This is the falsifiable done-test #1.

### 3.2 Close C4b — constant-time `mod_l` + dudect gate — HIGH

Per the code's own prescription (`sign.rs:712-723`): replace the bit-serial `mod_l` (`:625-654`) with a
**fixed-width, branch-free reduction** (Barrett or Montgomery over the group order L), and give the
field layer (`reduce_p`, `limbs_ge_p`, `limbs_sub_p`) the same treatment so no secret-dependent control
flow or data-dependent conditional subtract remains on the signing path. The RFC 8032 §7.1 KAT must
still pass bit-for-bit (the math is unchanged; only the *trace* becomes secret-independent). Add a
**dudect-style statistical CT test** (the pattern already used for the KEM at
`proto-crypto/pq_kem.rs:16` "*dudect-style statistical timing gate*") over `mod_l`/`sign` with
fixed-vs-random secret inputs, and wire it as a CI gate. C4b is "provably closed" only when this gate
is GREEN — a patch without the gate does not satisfy the done-test.

### 3.3 Close C6b — independent test fixtures

In `wss_transport.rs:707` / `iroh_transport.rs:421`, derive the two hybrid legs from **independent**
seeds (or from `derive_pq_seed(master)` for the PQ leg so the two are provably decorrelated). Purely
test-hygiene, but it is a precondition for the §3.1 hybrid-anchor tests to be meaningful.

### 3.4 One canonical KEM/DSA + named oracles — M2

Execute §2's decision: `core::pq_kem` + `core::pq_dsa` are canonical. Strike
`proto-crypto/src/pq_kem.rs` (Option A) or oracle-mark + pin it (Option B). Either way, add/keep the
byte-agreement CI test so the FIPS-exact impl is the only thing on the live path and any surviving copy
is pinned to it. Fix the `proto-crypto` header self-contradiction as part of whichever option is taken.

### 3.5 Full red-line category mapping (Auth/Secrets/Migrations) — M12, F26

Two coordinated edits:

- **`scope.rs`:** add the missing vocabulary. New `Resource` variants (e.g. `Auth`, `Secret`,
  `Migration`) and any needed `Action` verbs, each with a **new pinned discriminant** continuing the
  stable sequence (next free `Resource` byte is `0x0E`; `Action` next is `0x15`) — additive only, never
  renumber existing variants (`scope.rs:144-240` is a wire contract; the `scope_discriminants_are_stable`
  test at `:305` enforces this). Update `from_discriminant`/`discriminant` and the round-trip test.
- **`redline.rs` `is_red_line` (`:68-79`):** map the new `(Resource, Action)` pairs to
  `RedLineCategory::Auth`, `::Secrets`, `::Migrations` — the categories already exist in the enum
  (`:33-37`), they were just unreachable. Add tests mirroring `money_mutations_are_red_line`
  (`:137`) and `deny_by_default_rejects_red_line_scope` (`:153`) for each new category, proving
  `DenyByDefault` rejects an Auth-, Secrets-, and Migrations-scoped capability. This is done-test #4.

### 3.6 X25519 classical KEM leg — M2

Add a **from-scratch RFC 7748 X25519** to `core` (matching the zero-dep posture of `core/src/sign.rs`
and `core/src/pq_kem.rs` — M6 forbids `x25519-dalek` at the wire boundary, so the `dowiz-pq`/`vault.rs`
dalek impls are references for the *math*, not code to lift). Expose a hybrid KEM
(`X25519 ⊕ ML-KEM-768`, both mandatory, shared secret = KDF over both) modeled on the *shape* of
`dowiz-pq/hybrid.rs` but built on `core`'s own X25519 + `core::pq_kem`. This phase lands the primitive
and its KAT; **actual transport use of the hybrid KEM is Phase 9** (F16). Landing it here means Phase 9
encrypts with a `core`-canonical, ACVP-pinned hybrid KEM rather than inventing one under wire pressure.

### 3.7 Real negative-test harness (replace placeholders) — M2

Replace the `Placeholder` stubs in `proto-crypto/src/{ladder,wycheproof,fips_regen,constant_time}.rs`
with real content: a vendored-vector loader (Wycheproof JSON under `kat/`, no network — same pattern as
`core/src/kat/`), negative vectors (malformed ciphertexts, tampered signatures, out-of-range encodings
that MUST be rejected), and the CT assertion shim tying `mod_l`/decaps to the dudect gate from §3.2.
The `fips_regen` oracle is the second independent derivation used for the byte-agreement test in §3.4.

### 3.8 Fix the stale docstring — M12

Reword `proto-cap/src/lib.rs:11-13` to state the ML-DSA-65 leg is **real, wired, and ACVP-verified**
(and, after §3.1, that delegation links and genesis anchors are hybrid too). Done-test #5 is a `grep`
proving the "TODO" claim is gone.

---

## 4. Migration steps (dependency order)

Sub-fixes are largely independent, but there is a real ordering where it matters:

1. **KEM/DSA canonicalization (§3.4) FIRST.** Everything else references "the canonical primitive."
   Strike-or-oracle-mark `proto-crypto` KEM and land the byte-agreement CI test. Cheap, unblocks clean
   references. (No live consumer, so zero blast radius.)
2. **C4b constant-time `mod_l` + dudect gate (§3.2) BEFORE hybrid anchors (§3.1).** The hybrid
   delegation path signs with the *same* Ed25519 primitive whose signing side-channel C4b fixes. Land
   the CT fix + gate first so the new hybrid-anchor code is built on a non-leaking Ed25519, and Phase 6
   (which derives K/V hybrid keys) inherits a closed C4b. Bundle C6b (§3.3) here — the fixtures the
   hybrid tests need are the ones C6b tidies.
3. **`scope.rs` vocabulary + `redline.rs` mapping (§3.5).** Independent of the crypto changes; can run
   in parallel with steps 1–2. Additive discriminants only; keep `scope_discriminants_are_stable`
   GREEN.
4. **X25519 + hybrid KEM primitive (§3.6).** Depends on step 1's canonical `core::pq_kem`. Parallel
   with steps 2–3.
5. **Hybrid delegation links + genesis anchors (§3.1).** The keystone. Depends on step 2 (clean
   Ed25519) and needs the ML-DSA API (already present). Changes `roster.rs`, `node_id.rs`,
   `verify_chain`, and the genesis file format together (one atomic wire-format change), plus the
   quantum-attacker fixture.
6. **Negative-test harness (§3.7).** Depends on steps 1 (canonical primitive to test) and 2 (CT shim
   target). Fills the `proto-crypto` placeholders.
7. **Stale docstring (§3.8).** Last — it must describe the *finished* state (hybrid anchors + real PQ
   leg). Trivial, but do it after §3.1 lands so it is true.

Throughout: **run the F19 (`scope.rs`) and F24 (`roster.rs` expiry) test suites after every step** —
they are the regression tripwire, not build targets (done-test #6). The `hybrid_gate.rs:124-201`
ordering must remain byte-stable; the chain step changing from classical to hybrid must not reorder the
gate.

---

## 5. Acceptance criteria (numbered checklist)

Falsifiable done-tests. Every item must be a passing/failing test, not a claim.

1. **Hybrid trust root (F21).** A classically-forged delegation chain — attacker holds an enrolled
   anchor's Ed25519 key but not its ML-DSA key — is **rejected** by `verify_chain` under a
   quantum-attacker fixture. A fully hybrid-signed, anchor-rooted chain is accepted. Genesis anchors and
   every delegation link carry an ML-DSA leg.
2. **C4b closed (HIGH).** A dudect/constant-time CI gate is GREEN on `mod_l` (and the field-reduction
   layer) with fixed-vs-random secret inputs; the RFC 8032 §7.1 KAT still passes bit-for-bit. Closed by
   *proof of no timing signal*, not by an unmeasured patch.
3. **One canonical primitive (M2).** Exactly one ML-KEM-768 (`core::pq_kem`) and one ML-DSA-65
   (`core::pq_dsa`) are on the live path, both passing ACVP KAT. Every other copy is deleted OR carries
   a CI byte-agreement test pinning it to the canonical one; `proto-crypto`'s η₁/G/implicit-rejection
   divergence is reconciled to FIPS or struck; its self-contradicting header is fixed.
4. **Full red-line mapping (M12/F26).** `RedLineGate::check` under `DenyByDefault` provably **denies**
   an Auth-scoped, a Secrets-scoped, and a Migrations-scoped capability — not just Money-scoped — with a
   test per category. New `scope.rs` discriminants are additive and `scope_discriminants_are_stable`
   stays GREEN.
5. **X25519 KEM leg (M2).** A from-scratch, zero-dep X25519 (RFC 7748) + `X25519 ⊕ ML-KEM-768` hybrid
   KEM lives in `core` with passing KAT vectors. (Transport use is Phase 9; the primitive lands here.)
6. **Negative-test harness (M2).** The `proto-crypto` `Placeholder` stubs are replaced by a real
   vendored-vector / Wycheproof-style negative-test harness that rejects malformed ciphertexts and
   tampered signatures.
7. **Docstring truthful (M12).** `grep "TODO"` / "marked TODO" over `proto-cap/src/lib.rs` around the
   PQ-leg description returns nothing; the header states the ML-DSA leg (and hybrid anchors) are real
   and ACVP-verified.
8. **Regression (F19/F24).** The pre-existing F19 (`scope.rs` unknown-scope fail-closed) and F24
   (`roster.rs` expiry) test suites stay GREEN through every step of the refactor. C6b fixtures key the
   two hybrid legs independently.
9. **Whole-workspace green.** `cargo test -p bebop2-core -p bebop-proto-cap -p bebop-proto-crypto
   -p bebop-proto-wire -p bebop-mesh-node --offline` is exit-0 (baseline: 232 core + all-crate GREEN,
   R1-A §3), now including the new hybrid-anchor, red-line-category, X25519-KAT, and CT-gate tests.

---

## 6. What this phase unblocks

Phase 3 sits at the head of the critical path **P3 → P9 → P10 → P13**. Each downstream phase signs or
authenticates *on this trust root*. Building any of them first bakes a forgeable foundation into the
product.

- **Phase 6 — V1 split-identity + adversarial verifier** (`P6 ← P1, P3`). V1's `key_K`/`key_V`
  ceremony derives **hybrid** keys via `derive_pq_seed` + the `load_genesis` pattern; its merge gate
  rejects a PR unless a `key_V`-signed GREEN verdict verifies. If C4b is open (§1.2), the keys that gate
  every merge are signed with a leaking Ed25519 primitive — a side-channel on the epistemic bedrock. If
  the delegation root is classically forgeable (§1.1), an attacker forges a `key_V` authorization and
  self-certifies GREEN. V1 is meaningless on an unhardened root; specifically it needs C4b closed before
  hybrid keys are derived.

- **Phase 9 — confidential, self-healing wire** (`P9 ← P3, P4`). F16 encrypts just-met-peer traffic
  with ML-KEM. With three disagreeing "ML-KEM-768"s (§1.4), Phase 9 could encrypt with a non-FIPS
  variant — silently non-interoperable with real peers, or ship the divergent one to some hubs and the
  FIPS one to others, splitting the mesh cryptographically. The X25519 hybrid leg (§3.6) is the classical
  half of F16's session key. Handing Phase 9 an unreconciled KEM means redoing wire encryption when the
  canon is later fixed.

- **Phase 10 — hub runtime: kill-switch signing** (`P10 ← P3, P6, P9`). M9's operator kill is an
  **anchor-signed frame** → COLD-backup-then-halt; the kill authority *is* a genesis anchor. If anchors
  are Ed25519-only (§1.1), a quantum attacker forges the operator's kill frame and halts arbitrary hubs —
  or forges a *refusal* of a legitimate kill. The one mandated global bound in the whole architecture
  (M9) rides directly on the anchor being unforgeable.

- **Phase 13 — delivery on protocol: order signing** (`P13 ← P4, P7, P9, P10`). Orders, Proof-of-
  Delivery, and payout sagas are edge-ML-DSA-signed and authorized by capability chains rooted in the
  roster. A forgeable root (§1.1) lets a quantum attacker mint a capability authorizing
  `Order::CreateOrder` or a `Claim` payout — the money red-line (F26) is the last brake. If
  Auth/Secrets/Migrations are unmapped (§1.5), a capability could authorize an auth or migration effect
  with **no** red-line brake at all. Money moves and PoD disputes settle on these signatures; an
  unhardened root makes every order forgeable.

**In one line:** you do not integrate a product, encrypt a wire, or wire a kill-switch onto a trust
root that a quantum attacker forges classically and whose signing primitive leaks the key over a
side-channel. Phase 3 is the precondition for all of it.

---

*Blueprint P03. Planning only — no crypto code written. Evidence re-verified against
`/root/bebop-repo/bebop2/` this session (roster.rs, redline.rs, scope.rs, sign.rs, lib.rs, node_id.rs,
signed_frame.rs, core/pq_kem.rs, proto-crypto/pq_kem.rs + stubs; proto-crypto confirmed unwired via
importer grep). Implementation pass is a separate, careful task gated on operator rulings O2 (E10≡E36)
and any canon rewording of F21 per R2 §4.*
