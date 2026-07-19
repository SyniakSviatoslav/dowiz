# BLUEPRINT P82 — bebop bench expansion (crypto lane + `HybridGate::check` + wire codec) (2026-07-19)

> **Standalone COVERAGE blueprint (bebop2 `core` / `proto-cap` / `proto-wire`).** One coherent,
> independently buildable unit against the 20-point contract in `CORE-ROADMAP-STANDARD-2026-07-17.md`
> §2. Research sources: `docs/research/OPUS-PERF-BENCH-COVERAGE-MAP-2026-07-18.md` §2C/§2D + §3-Wave-2,
> and `docs/research/OPUS-PERF-BEBOP-AUDIT-2026-07-18.md` P3 (the KEM-must-be-measured-before-NTT
> finding). Reconciled in `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.3-C3 / §5 (Tier C, Wave W2,
> unit **P82**). Format precedent: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`. Grounding
> tree: `/root/bebop-repo/bebop2` at HEAD, read live this pass.
>
> **RULE (memory `cross-branch-todo-map-2026-07-10.md`):** all edits in this blueprint land in
> **`/root/bebop-repo`** and push to the **`openbebop`** remote — never in `/root/dowiz`.
>
> **One sentence:** the bebop2 mesh crates bench **only** `verify` today (ML-DSA-65 + Ed25519, via the
> zero-dep `verify_lane.rs`); the **entire sign/KEM/AEAD/hash lane is untimed**, and the two
> highest-frequency hot paths — `HybridGate::check` (per-inbound-frame authorization) and the wire
> codec (per-frame encode/decode) — have **zero** coverage; P82 adds those benches so the D-9 NTT
> decision and P92's fast-path measure-first gate both get real numbers instead of estimates.

---

## VERDICT (stated up front, per session research discipline)

**GO — mechanical and high-leverage; but double-gated (P75 schema + the bebop commit-freeze gate-0).**
Like P81 this is additive bench code that changes no product logic and cannot regress the mesh. Its
leverage is higher than a normal coverage unit because two *downstream decisions currently block on
its numbers*:

- the **D-9 / OD-9 NTT wire-in** (re-introduce an NTT into `pq_kem::poly_mul` for ~100× on the KEM
  path) is explicitly **measure-first**: R4 P3 says "measure before touching … only if handshake
  latency is proven to matter." P82's KEM encaps/decaps bench **is** that measurement (S1 §4 D-3,
  MASTER-STATUS-LEDGER OD-9);
- **P92's D-BENCH** measure-first NO-GO gate wants `HybridGate::check` cost per `Presence/Send` frame;
  P82's `proto-cap gate` group is the natural home for that number (MASTER-STATUS-LEDGER §3.3), so
  P92 measures inside P82's lane instead of inventing a second convention.

Three conditions bound it honestly:

1. **Hard prerequisite A — P75 schema.** P82's baselines write into P75's `<group>/<n>` +
   `baseline.json` schema; P82 cites it, never redefines it (S1 §5 single-owner contract).
2. **Hard prerequisite B — the bebop gate-0 must clear first.** MASTER-STATUS-LEDGER §3 finding #2:
   the C3 ungated-keygen HARD-law red state + the unremediated NTT base (`986646a`) **freeze every
   hook-respecting commit on the bebop working branch** until **P85** closes and **C3** is ruled
   (OD-3/OD-6). So P82 — a bebop-repo commit — cannot land through the hooks until gate-0 clears.
   This is a *sequencing* gate, not a design dependency.
3. **Sequenced after P76 → P78.** Within Wave W2 the bebop lane order is **P78 → P82** (S1 §5,
   MASTER-STATUS-LEDGER §3 Wave-2), after **P76** (un-gate tests + bus fix) and after **M1** (Wave-1
   exporter fix) — so P82 benches a codebase whose P78 `MerkleDigest`/`hub_ring` fixes are already in
   (its numbers reflect the fixed code, not the pre-fix one).

**No NO-GO for P82 itself** — measuring an untimed crypto/auth path is unconditionally worth it; the
NO-GO gates it *feeds* (D-9, P92) are downstream and owned there.

---

## 0. Ground truth — every cite re-verified live this pass (standard §2 item 1)

> Read from `/root/bebop-repo/bebop2` this pass, not inherited from the research sketch.

### 0.1 What IS benched today (the whole coverage, R5 §1d, re-verified)

- **`bebop2/core/benches/verify_lane.rs`** exists (confirmed live): a **zero-dep `std::time`** binary
  timing ML-DSA-65 `verify_internal_bytes` vs `..._many` and Ed25519 `verify` vs `verify_many` at
  N∈{1,4,16,64}. It is a **measured number, not a criterion gate** — it writes no `baseline.json` row.
- **Keygen and sign are setup-only — NOT timed** (R5 §1d): `pq_dsa::keygen_bytes`,
  `pq_dsa::sign_internal_bytes`, `sign::keygen`, `sign::sign` build the corpus but their cost is
  discarded.
- **Zero benches** in `proto-cap`, `proto-wire`, `proto-crypto`, `delivery-domain`, `mesh-node` —
  confirmed live: `ls bebop2/proto-cap/benches` and `ls bebop2/proto-wire/benches` both **absent**.

### 0.2 The hot targets P82 covers (all verified live this pass)

| target | cite (verified this pass) | why it matters (R4/R5) |
|---|---|---|
| `HybridGate::check(...)` | `proto-cap/src/hybrid_gate.rs:124` | **THE per-inbound-frame authorization gate** (`iroh_transport.rs`, `wss_transport.rs`, `stdio_transport.rs`, `facade.rs`); highest-value hot path in the crate; verified clean-but-untimed (R4 §2) |
| `SignedFrame::verify_pq(&self)` | `proto-cap/src/signed_frame.rs:229` | 1× ML-DSA verify + domain rebuild — dominant crypto cost per frame under `RequireBoth` |
| `SignedFrame::verify_classical(&self)` | `proto-cap/src/signed_frame.rs:208` | always-verified Ed25519 leg; re-derives TLV domain per call |
| `roster::verify_chain(...)` | `proto-cap/src/roster.rs:252` | O(chain) delegation validation; isolates the cost `check` hides |
| `matcher::assign(order, candidates, max)` | `proto-cap/src/matcher.rs:63` | per-order HRW courier scan; the blessed Schwartzian `hub_ring` should mirror (P78) |
| `wire_codec::encode_frame(frame)` | `proto-wire/src/wire_codec.rs:198` | serializes every outbound frame to canonical bytes |
| `wire_codec::decode_frame(buf)` | `proto-wire/src/wire_codec.rs:240` | deserializes every inbound frame **before** verify — hostile-input hot path |
| `pq_kem::encaps(ek, rng)` / `encaps_internal(ek, m)` | `core/src/pq_kem.rs:693` / `:674` | ML-KEM-768 session-key establishment; **feeds the D-9 NTT decision** |
| `pq_kem::decaps(dk, ct)` | `core/src/pq_kem.rs:703` | heaviest KEM op (decrypt + full re-encrypt); per-handshake receive |
| `pq_dsa::sign(...)` / `sign::sign(...)` | (R5 §2C `pq_dsa.rs:1155` / `sign.rs:892`) | ML-DSA-65 + Ed25519 **sign** — the hybrid-gate sign half, completely untimed today |
| `aead_xchacha20_poly1305_encrypt/decrypt` | (R5 §2C `aead.rs:278`/`:300`) | size-dependent SEAL/OPEN for the at-rest store — length-scaling unmeasured |
| `hash::sha3_256(msg)` | (R5 §2C `hash.rs:344`) | highest-frequency crypto primitive — once per event AND per signed message; unbenched at any size |
| `x25519::x25519(k, u)` | (R5 §2C `x25519.rs:356`) | sovereign DH baseline (swap-in-vs-dalek), classical leg of the hybrid handshake |

*(The `pq_kem::encaps/decaps` symbols also appear inside a `#[cfg(test)]` shim at `pq_kem.rs:921/924`
— the bench targets the **public** `:693/:703` entry points, not the test shim. Noted so the worker
picks the right symbol.)*

### 0.3 Why `poly_mul` is schoolbook and why the KEM bench is the gate (R4 P3, load-bearing)

`pq_kem::poly_mul` is deliberately schoolbook **O(N²)** (N=256 → up to 65,536 inner mults/mul; K=3
scheme), a documented correctness-first choice: `pq_kem.rs:329-335` records that a shipped NTT was
found **incorrect** (forward/inverse not a valid pair; basemul ≠ schoolbook) and was ripped out
rather than ship a subtly-wrong fast path. `pq_dsa` (ML-DSA-65) by contrast **does** use a verified
NTT (`pq_dsa.rs:198`). The KEM runs **per handshake, not per frame**, so the blast radius is
session-setup latency, not steady-state throughput. **This is exactly why P82 measures before anyone
touches it** — and why any future NTT (D-9) must ship with the verified pair `intt(ntt(a))==a` AND
`intt(basemul(ntt(a),ntt(b)))==schoolbook(a,b)` per the codebase's own bar (`pq_kem.rs:335`), a
crypto red-line requiring operator sign-off (OD-9).

### 0.4 Primitives + harness are all in-tree — zero new runtime deps

No new primitive is invented and no new *runtime* dependency is added; the only new **dev**-dependency
is the `criterion` sibling (§1.2 decision), matching the dowiz kernel/engine posture. Every benched
function already exists and is KAT-gated.

---

## 1. Prior-art map — adopt, don't invent (standard §2 item 19)

### 1.1 In-repo harness precedents

| Prior art | What it is | How P82 uses it |
|---|---|---|
| **`bebop2/core/benches/verify_lane.rs`** | zero-dep `std::time` binary; N-sweep verify timings | **Extend it** for the `core` crypto lane (sign/KEM/AEAD/sha3/x25519) — keeps the sovereign zero-dep ethos where it already lives (§1.2). |
| **dowiz `kernel/benches/criterion.rs`** | criterion `harness = false`, `<group>/<n>` sweeps, `baseline.json`-tracked | **The template for the NEW `proto-cap`/`proto-wire` criterion benches** — those crates have no bench at all, and criterion is what P75's gate parses. |
| **`proto-cap/matcher.rs:63` `assign`** (the blessed Schwartzian) | correct decorate-sort-undecorate HRW | benched as-is; also the pattern P78's `hub_ring` fix mirrors — P82 benching `assign` gives the reference curve P78's fix should match. |

### 1.2 Harness-shape decision (engineering, resolved here — operator need not rule)

**Split by crate, matching each crate's existing ethos:**
- **`core` crypto lane → EXTEND `verify_lane.rs`** (zero-dep `std::time`). The sovereign `core` crate
  is deliberately dependency-minimal; adding sign/KEM/AEAD/sha3/x25519 timings alongside the existing
  verify timings keeps one coherent zero-dep timing binary. *Trade-off, stated honestly:* these
  numbers are **measured, not gated** (no `baseline.json` row) — acceptable for the `core` lane
  because they primarily feed **one-shot decisions** (D-9 NTT: is decaps hot? P92: is verify hot?),
  not a continuous per-commit regression gate.
- **`proto-cap` + `proto-wire` → NEW `criterion` sibling benches** (`benches/criterion.rs` each). These
  are the **per-frame** hot paths (`HybridGate::check`, `encode/decode_frame`) where a *continuous
  gate* matters, so they must produce `baseline.json` rows P75's gate consumes. `criterion` is added
  as a **dev-dependency** to those two crates only (their default lib build is unaffected).

Honest note: this yields two harness styles in one blueprint. That is deliberate — the `core` lane's
one-shot-decision benches and the proto crates' continuous-gate benches have genuinely different
consumers. Recorded so nobody "unifies" them without re-reading this rationale.

---

## 2. Scope — what P82 owns vs deliberately does NOT (standard §2 items 11, 18, 19)

### 2.1 P82 OWNS

1. **EXTEND `bebop2/core/benches/verify_lane.rs`**: add timings for `pq_dsa::sign`, `sign::sign`,
   `pq_kem::encaps`/`decaps`, `x25519::x25519`, and **size-swept** `aead_*_encrypt`/`decrypt` +
   `hash::sha3_256` over plaintext ∈ {64 B, 1 KiB, 64 KiB}.
2. **NEW `bebop2/proto-cap/benches/criterion.rs`** — group `gate`: `HybridGate::check` swept over chain
   length {0,1,4,16}; `verify_pq`/`verify_classical` single; `tlv_signing_input` over field-count;
   `roster::verify_chain` swept over chain length; `matcher::assign` over candidates {8,64,256}. Plus
   the `criterion` dev-dep + `[[bench]]` in `proto-cap/Cargo.toml`.
3. **NEW `bebop2/proto-wire/benches/criterion.rs`** — group `codec`: `encode_frame`/`decode_frame`
   swept over chain length; `framing::encode`/`decode`; `envelope::to_bytes`/`from_bytes` (the
   only serde_json on the carrier path — quantify vs the hand-rolled TLV it wraps). Plus the dev-dep +
   `[[bench]]` in `proto-wire/Cargo.toml`.
4. **The `<group>/<n>` bench-ids** for the proto-crate surface, in **P75's** schema (cited, never
   redefined).
5. **A NEW bebop-repo CI bench job** for the per-frame `proto-cap`/`proto-wire` criterion baselines —
   authored in **bebop2's OWN `.github/workflows/`** — modeled on P75's kernel `bench-regression`
   same-runner A/B job (P75 §5) but running
   `cd bebop2 && cargo bench -p bebop-proto-cap -p bebop-proto-wire --bench criterion --
   --save-baseline {base,pr}` and gating with `native-trackers compare` against each crate's
   `baseline.json`. Plus the committed `bebop2/proto-cap/benches/baseline.json` +
   `bebop2/proto-wire/benches/baseline.json`. **This is new work P82 now owns explicitly — it is NOT
   something P75 already provides.** Ground truth: P75's *running* gate lives in **dowiz's**
   `.github/workflows/ci.yml` and benches the dowiz **kernel only** (P75 §2.2/§5); bebop's own `ci.yml`
   has **zero** bench jobs today (§0.1). Without this job the proto-crate baselines are **recorded but
   never gated**, D4 (below) is unsatisfiable, and P82's per-frame "continuous-gate" value proposition
   is structurally broken.
   - **⚠ CROSS-REPO COMPLEXITY FLAG (the part that makes this materially harder than P81's engine job).**
     bebop2 is a **separate git repository** (`/root/bebop-repo`, pushed to the `openbebop` remote —
     memory `cross-branch-todo-map-2026-07-10.md`), so this job **cannot** be a second job inside
     dowiz's `ci.yml`; it must be authored in **bebop's own `.github/workflows/`**. But P75's gate
     machinery — the `native-trackers compare` binary, the `GateExit` contract, the `<group>/<n>` +
     `baseline.json` v2 schema — currently lives **only in `/root/dowiz`** (`tools/telemetry/
     native-trackers`, §2.2). So P82 must additionally specify **how the bebop CI runner obtains that
     machinery**: either (a) vendor/publish the released `native-trackers` binary into the bebop repo,
     or (b) a **cross-repo multi-checkout** of the sibling dowiz `tools/telemetry/native-trackers` at a
     known path — the *same* sibling-checkout topology P76 needs for its `kernel-rlib` leg (flagged as
     META-GAP-AUDIT finding **G10**). P82 must pin one of these as an explicit environmental
     prerequisite of its bebop bench job; it does **not** re-implement the gate.
   - The `core` `verify_lane.rs` numbers stay **measured-not-gated** by design (§1.2, one-shot D-9/P92
     inputs) and therefore need **no** CI job — only the per-frame `proto-cap`/`proto-wire` criterion
     baselines require this new bebop gate.

   This item corrects `META-GAP-AUDIT-2026-07-19.md` finding **G3** ("no blueprint owns creating the
   bebop-repo CI bench job; P82's is cross-repo"), rather than the prior silent assumption that "the P75
   gate" — which lives in dowiz and benches only the kernel — would somehow run the bebop crates.

### 2.2 P82 does NOT own (anti-scope)

- **Any product-source change to bebop2 crates.** Zero. In particular P82 does **not** wire an NTT
  into `pq_kem` (that is **D-9**, triple-gated on P82's own bench + P85 + operator sign-off, OD-9) and
  does **not** touch `MerkleDigest`/`hub_ring` (those are **P78**). P82 benches the code P78 leaves.
- **The dowiz kernel's own PQ lane** (`dowiz/kernel/src/pq/*`) — that is **P80**'s `kernel_crypto_pq`
  group, a *different implementation* in a *different repo* (R5 §1f note). Collision-free by repo lane.
- **`crates/bebop`** (the legacy TUI crate, R4 §5 / S1 E13) — confirmed dev-tooling off the mesh
  product path; its `loop_cycle`/`wire` benches stay as-is; P82 adds nothing there.
- **`bench_track.py`/`native-trackers`/the gate schema** — **P75**'s single-owner contract.
- **`delivery-domain`/`mesh-node`/`wasm-host`** — out of scope for this pass (R5 lists them zero-cover,
  but their hot paths — `hub_ring` post-P78, sync anti-entropy — are D-3/P78-adjacent, benched there
  or deferred with a written trigger; P82 stays on the `core`/`proto-cap`/`proto-wire` lane).

### 2.3 Dependencies (named by artifact — standard §2 item 7)

**Hard inputs:** **P75** — for its **`<group>/<n>` + `baseline.json` v2 schema and the reusable
`native-trackers compare` gate binary + `GateExit` contract**, *not* for a ready-made bebop CI job
(P75's running gate is dowiz-kernel-only and lives in dowiz's repo, §2.2/§5). The **cross-repo bebop
`bench-regression` job is P82's own deliverable** (§2.1.5), including specifying how bebop CI reaches
P75's machinery (the G10 sibling-checkout/vendored-binary topology). **bebop gate-0** = **P85** closed +
**C3** ruled (OD-3/OD-6) — unfreezes bebop commits; **P76** landed (Wave-1) and **P78** landed (Wave-2,
sequenced before P82).
**Feeds:** **D-9/OD-9** (KEM bench → NTT wire-in decision) and **P92 D-BENCH** (`gate` group →
fast-path measure-first NO-GO). Neither is a hard block *on* P82 — P82 produces the data; the gates
are decided downstream.

### 2.4 Honest reconciliation with R4's "clean" verdict (standard §2 item 6)

R4 §2 verified `HybridGate::check`, `matcher::assign`, `verify_chain`, and the wire codec **clean** —
O(1)/sig, O(chain-depth)/frame, no accidental super-linear scaling. **P82 does not contradict that.**
Benching a *clean* path is not an accusation that it is slow; it is a **regression fence** so it
*stays* clean and so the D-9/P92 decisions rest on measured cost, not on "we believe it's cheap."
The sweeps confirm the O(chain) shape empirically (a chain-16 gate cost meaningfully above chain-0
*is* the linear-in-chain-depth claim made falsifiable), which is exactly item 6's "argue from
structure, then measure."

---

## 3. Predefined types & constants — named BEFORE implementation (standard §2 item 4)

No product types added. Sweep sets are the growth-curve schema; fixtures are seeded-deterministic.

```rust
// shared bench conventions (both new criterion files + the verify_lane extension)

/// Delegation-chain depths for the per-frame auth + codec sweeps. {0,1,4,16} straddles
/// "no delegation" → "typical UCAN depth" → "deep" so the O(chain) term is a visible slope.
const CHAIN_SWEEP: &[usize] = &[0, 1, 4, 16];

/// Candidate-set sizes for the HRW matcher sweep (matcher::assign).
const CAND_SWEEP: &[usize] = &[8, 64, 256];

/// Field counts for tlv_signing_input (the canonical leaf allocation under every sign/verify).
const FIELD_SWEEP: &[usize] = &[4, 16, 64];

/// Plaintext sizes for the size-dependent AEAD + sha3 lane (verify_lane extension).
const PT_SWEEP: &[usize] = &[64, 1024, 64 * 1024]; // 64 B, 1 KiB, 64 KiB — Throughput::Bytes

/// Deterministic fixture seed — all keypairs/frames/rosters derived from this via the crates'
/// own keygen_from_entropy with a FIXED entropy buffer. NEVER RNG-from-OS-entropy in a bench.
const BENCH_SEED: [u8; 32] = [0x5E; 32];
```

**Bench-id map (into P75's `<group>/<n>` schema):**

| crate | group | ids | target |
|---|---|---|---|
| `proto-cap` | `gate` | `/chain_0`, `/chain_1`, `/chain_4`, `/chain_16` | `hybrid_gate.rs:124` `HybridGate::check` |
| `proto-cap` | `gate` | `/verify_pq`, `/verify_classical` | `signed_frame.rs:229` / `:208` |
| `proto-cap` | `gate` | `/tlv_fields_4`, `/_16`, `/_64` | `tlv.rs` `tlv_signing_input` |
| `proto-cap` | `gate` | `/verify_chain_{0,1,4,16}` | `roster.rs:252` |
| `proto-cap` | `gate` | `/assign_{8,64,256}` | `matcher.rs:63` |
| `proto-wire` | `codec` | `/encode_chain_{0,1,4,16}`, `/decode_chain_{0,1,4,16}` | `wire_codec.rs:198` / `:240` |
| `proto-wire` | `codec` | `/framing_encode`, `/framing_decode` | `framing.rs` |
| `proto-wire` | `codec` | `/envelope_to_bytes`, `/envelope_from_bytes` | `envelope.rs` (serde_json cost) |
| `core` (verify_lane, measured-not-gated) | — | `dsa_sign`, `ed25519_sign`, `kem_encaps`, `kem_decaps`, `x25519`, `aead_enc/{64,1k,64k}`, `aead_dec/{…}`, `sha3/{…}` | §2.1 item 1 |

---

## 4. Build items — spec → RED check → code, each anti-cheat-guarded (standard §2 items 2, 3, 5)

"RED" = the id is absent from `baseline.json` (proto-cap/proto-wire) or unprinted (verify_lane), and
an injected slowdown does not surface; "GREEN" = measured + (for the criterion groups) gating.

### 4.1 M1 — `core` crypto-lane extension of `verify_lane.rs`

- **Spec:** add timed sections for `pq_dsa::sign`, `sign::sign`, `pq_kem::encaps`/`decaps`,
  `x25519::x25519`, and size-swept `aead_*_encrypt`/`decrypt` + `sha3_256` over `PT_SWEEP`. All keys
  and messages derived from `BENCH_SEED` via the crates' `*_from_entropy` with a fixed buffer.
- **RED `red_kem_untimed`:** before this, KEM encaps/decaps have **no printed number** → the D-9 NTT
  decision has no evidence and would be taken (or rejected) blind. This is the exact gap R4 P3 names.
- **GREEN:** `kem_encaps`/`kem_decaps` print a stable per-op cost; `dsa_sign`/`ed25519_sign` complete
  the hybrid-gate *sign* half that was setup-only.
- **Anti-cheat `kem_roundtrips`:** assert (sibling test) `decaps(dk, encaps(ek).ct) == encaps.ss` on
  the fixture — proving the benched encaps/decaps are the real KEM, not a stubbed constant. Guards
  against benching a degenerate path.
- **D-9 hand-off note (doc-comment):** *the `kem_decaps` number is the OD-9 input. An NTT is worth it
  only if decaps is proven hot on the handshake budget; and any NTT MUST ship with the
  `intt(ntt(a))==a` + `intt(basemul(…))==schoolbook` verified pair (`pq_kem.rs:335`) + operator
  sign-off. P82 measures; it does not decide.*

### 4.2 M2 — NEW `proto-cap/benches/criterion.rs` group `gate` (the per-frame auth headline)

- **Spec:** `bench_with_input` over `CHAIN_SWEEP` for `HybridGate::check` — build a seeded roster + a
  delegation chain of depth k + a valid `RequireBoth` frame; time one `check`. Single benches for
  `verify_pq`/`verify_classical`; `FIELD_SWEEP` for `tlv_signing_input`; `CHAIN_SWEEP` for
  `roster::verify_chain`; `CAND_SWEEP` for `matcher::assign`. `black_box` frame, roster, chain.
- **RED `red_gate_absent`:** no `gate/*` id in `baseline.json` → the per-frame authorization cost is
  ungated; a regression in `verify_chain` or the ML-DSA verify ships silently.
- **GREEN:** `gate/chain_{0,1,4,16}` present; the curve confirms the O(chain) structure R4 §2 argued.
- **Anti-cheat `gate_rejects_forged_frame`:** the fixture used by the bench must be a frame `check`
  **accepts**; a sibling asserts a *tampered* frame is **rejected** — proving the benched `check` runs
  the real verify, not an early-return on a malformed input (a bench that timed a fast rejection would
  understate the true cost). Ties to the B4/SSR-2020 lesson: never let a bench measure a non-verify.
- **P92 hand-off note (doc-comment):** *`gate/chain_1` (typical presence-frame depth) is P92's D-BENCH
  `bench_presence_full_signed` input; P92 measures its fast-path saving against this number in this
  lane rather than minting a second convention.*

### 4.3 M3 — NEW `proto-wire/benches/criterion.rs` group `codec`

- **Spec:** `CHAIN_SWEEP` for `encode_frame`/`decode_frame` (a frame with a k-deep chain); single
  benches for `framing::encode`/`decode`; `envelope::to_bytes`/`from_bytes` (serde_json). `black_box`
  the byte buffers.
- **RED `red_codec_absent`:** no `codec/*` id → the per-frame serialize/deserialize cost — including
  the **hostile-input `decode_frame` path that runs before verify** — is ungated.
- **GREEN:** `codec/{encode,decode}_chain_*` present; the serde_json envelope cost is quantified vs
  the hand-rolled TLV (answering R5 §2D's open question).
- **Anti-cheat `codec_roundtrips`:** assert `decode_frame(encode_frame(f)) == f` on the fixture —
  proving the benched codec is lossless real work, not a truncated encode. Guards fake-green.

### 4.4 M4 — manifest wiring (both proto crates)

- **Spec:** add `criterion = "0.5"` under `[dev-dependencies]` and a
  `[[bench]] name = "criterion" harness = false` stanza to `proto-cap/Cargo.toml` and
  `proto-wire/Cargo.toml`. Confirmed live: both currently have a `[dev-dependencies]` stanza and **no**
  criterion / `[[bench]]`.
- **RED `red_no_bench_target`:** `cargo bench -p bebop-proto-cap` finds no bench target today.
- **Anti-cheat / cleanliness `D-CLEAN`:** `cargo tree -e no-dev -p bebop-proto-cap` (and `-wire`) must
  be **unchanged** — criterion is dev-only, the sovereign default build stays as-is.

---

## 5. DoD — falsifiable, RED→GREEN, machine-checkable (standard §2 item 2)

| # | Done when… | Falsifier (check) |
|---|---|---|
| D1 | `verify_lane.rs` prints stable sign/KEM/AEAD/sha3/x25519 numbers | run the binary → every §3 `core` id prints; `kem_decaps` is non-trivial and stable |
| D2 | `proto-cap/benches/criterion.rs` exists and all `gate/*` ids land in P75's `baseline.json` | `bench_track.py`/P75 lists them; a missing id fails the coverage assertion |
| D3 | `proto-wire/benches/criterion.rs` exists and all `codec/*` ids land in the schema | same |
| D4 | an injected 2× slowdown in `HybridGate::check`'s verify trips the **bebop-repo** `bench-regression` CI job (the cross-repo gate P82 authors in §2.1.5, in bebop2's OWN `.github/workflows/`, modeled on P75's job) RED, and clean HEAD → GREEN | inject a redundant `verify_pq`, run the **bebop** bench job → RED; revert → GREEN (proves real signal). **NB (per META-GAP-AUDIT G3):** P75's running gate is dowiz-kernel-only and lives in the dowiz repo, so this DoD is satisfiable **only because P82 now owns authoring the equivalent cross-repo bebop CI job** (§2.1.5) — reusing P75's `native-trackers compare` binary + exit contract (made reachable to bebop CI per the §2.1.5 cross-repo topology), **not** assuming a P75 gate already benches bebop. |
| D5 | the KEM bench answers "is decaps hot?" with a number the D-9 decision can cite | `kem_decaps` cost is recorded in the pass's output + referenced from OD-9 |
| D6 | anti-cheat siblings pass (each benched call is real work) | `kem_roundtrips`, `gate_rejects_forged_frame`, `codec_roundtrips` GREEN |
| D7 | every bench uses seeded deterministic inputs; no OS entropy, no wall-clock, no network | grep the benches for `from_os_rng`/`SystemTime`/socket → empty; `BENCH_SEED` drives all keys |
| D-CLEAN | the sovereign default lib builds of `core`/`proto-cap`/`proto-wire` gain **no** runtime dep | `cargo tree -e no-dev -p bebop-proto-cap -p bebop-proto-wire` unchanged; `core` gains nothing |
| D-NOREG | no bebop2 product source changed; full `cargo test` green | `git diff --stat bebop2/*/src` empty; `cargo test -p bebop-proto-cap -p bebop-proto-wire -p bebop2-core` green |

---

## 6. Benchmarks + telemetry + the growth-curve gate (standard §2 item 10)

- **P82 IS the baseline** for the mesh crypto/auth lane — the "before" is *no coverage*; the "after"
  is (a) a gating growth curve for the per-frame `gate`/`codec` paths via P75's `baseline.json`, and
  (b) measured one-shot numbers for the `core` crypto lane feeding D-9/P92.
- **Telemetry hook:** the proto-cap/proto-wire criterion baselines flow through
  `bench_track.py` → `native-trackers` → `baseline.json` (P75's fixed path), so a future regression on
  the auth/codec hot paths surfaces in CI automatically (item 10 + item 14). The `verify_lane`
  extension prints to stdout (the established zero-dep contract), captured in the pass record.
- **Scaling axis (item 8):** **delegation-chain depth** (auth + codec), **candidate-set size**
  (matcher), **field count** (TLV), **plaintext size** (AEAD/sha3). Each sweep's revisit threshold is
  recorded in its doc-comment (e.g. *chain depth >16 is beyond any real UCAN chain — if a deployment
  drives deeper, revisit `verify_chain`'s per-link cost*).

---

## 7. Cross-cutting obligations (standard §2 items 6, 8, 9, 11–16, 20)

- **Hazard-safety (item 6):** the hazard P82 could introduce is a **fake-green crypto bench** — e.g.
  timing a `check` that early-returns on a malformed frame (understating verify cost) or a KEM stub.
  Made unrepresentable by the §4 anti-cheat siblings (`gate_rejects_forged_frame`, `kem_roundtrips`,
  `codec_roundtrips`), each asserting the benched call did the *real* cryptographic work. This is the
  direct application of the B4/SSR-2020 memory (`crypto-safe-first-pass-2026-07-14.md`): a green
  measurement is necessary, not sufficient — the bench must provably exercise the true path.
- **Isolation / bulkhead (item 11):** all benches are dev-targets; D-CLEAN proves the sovereign
  default builds are untouched. A bench failure means "bench doesn't build/run," never "mesh regresses."
- **Mesh awareness (item 12):** the benched paths **are** the mesh's per-frame authorization and wire
  surfaces — `HybridGate::check` runs on every inbound frame, `encode/decode_frame` on every carrier
  read/write. The benches respect the real payload budgets (chain-depth sweep = real UCAN shapes; the
  frame fixtures are canonical wire bytes). P82 gossips nothing — the benches are local, deterministic.
- **Rollback/self-heal (item 13):** N/A as math — test scaffolding; rollback = delete the two bench
  files + the `verify_lane` additions + the four manifest lines.
- **Error-propagation / smart index (item 14):** *a per-frame auth/codec function silently regressing*
  → a **CI-time** failure via P75's gate on P82's baselines; *a bench that stops exercising the real
  verify* → a **test-time** failure via the anti-cheat siblings. Both gated, not runtime surprises.
- **Living-memory awareness (item 15):** N/A — seeded ephemeral fixtures.
- **Tensor/spectral (item 16):** N/A, honestly — crypto verify/KEM/codec are not linear-algebra
  kernels (the one heavy-math surface in `core`, `kalman.rs`/`dmd.rs`, is off the mesh per-frame path,
  R4 §3, and out of P82's scope). Stated, not shoehorned.
- **Linux discipline (item 9):** **EXTENDS** `verify_lane.rs` for the `core` lane and the kernel
  criterion pattern to two new crates; **REINFORCES** deterministic-seeded inputs + `harness = false`
  + the sovereign zero-runtime-dep default build; **DOES-NOT-TRANSFER** — no new gate schema (P75
  owns it), no NTT (D-9 owns it).
- **Hermetic principles (item 20):** **Correspondence** — each `<group>/<n>` id faithfully mirrors the
  function it times (the anti-cheat tests forbid a bench that does not correspond to real crypto
  work). **Cause & Effect** — the D-9/P92 decisions become *caused by a measured number*, not asserted
  belief; "measure before you touch" is Cause-before-Effect made procedural.

---

## 8. Standard-compliance map (all 20 points — standard §2)

| # | Standard item | Where satisfied |
|---|---|---|
| 1 | Ground truth, live `file:line` | §0 (every bebop cite re-verified; the public-vs-test KEM symbol correction; the two-absent-benches confirmation) |
| 2 | Falsifiable DoD | §5 (D1–D-NOREG, incl. the D4 injected-slowdown gate proof) |
| 3 | Spec→check→code, event-ordered | §4 (spec-first per M; RED-before / GREEN-after per bench-id) |
| 4 | Predefined types & constants | §3 (sweep sets, seed, bench-id map named before code) |
| 5 | Adversarial / anti-cheat cases | §4 (per-M anti-cheat siblings), §7 (fake-green crypto-bench hazard) |
| 6 | Hazard-safety from structure | §7 (fake-green bench unrepresentable via anti-cheat + the B4 lesson) |
| 7 | Links to docs & memory | §9 |
| 8 | Schemas with scaling axis | §3/§6 (chain depth / candidates / fields / plaintext size; per-sweep revisit thresholds) |
| 9 | Linux engineering discipline | §7 (EXTENDS/REINFORCES/DOES-NOT-TRANSFER) |
| 10 | Benchmarks + telemetry | §6 (P82 *is* the baseline; feeds `bench_track.py` + the one-shot decisions) |
| 11 | Isolation / bulkhead | §7 (dev-target isolation; D-CLEAN proves sovereign default build untouched) |
| 12 | Mesh awareness | §7 (benches the real per-frame auth/codec surfaces; real payload shapes) |
| 13 | Rollback/self-heal as math | §7 (N/A; rollback = delete bench files + manifest lines) |
| 14 | Error-propagation / smart index | §7 (regression → CI gate; fake-green → test-time fail) |
| 15 | Living-memory awareness | §7 (ephemeral seeded fixtures) |
| 16 | Tensor/spectral where applicable | §7 (N/A honestly; heavy math is off the mesh per-frame path) |
| 17 | Regression tracking | §6 (baselines are the permanent gate; §9 REGRESSION-LEDGER note) |
| 18 | Clear worker instructions | §9 |
| 19 | Reuse-first, upgrade-if-needed | §1 (extend verify_lane; criterion sibling for per-frame gate paths, decided with reason) |
| 20 | Hermetic principles | §7 (Correspondence, Cause & Effect) |

---

## 9. Links to docs & memory + instructions for other agentic workers (standard §2 items 7, 18)

**Depends on / cites:**
- `docs/research/OPUS-PERF-BENCH-COVERAGE-MAP-2026-07-18.md` §1d (verify_lane coverage), §2C
  (`core` sign/KEM/AEAD/sha3/x25519 gaps), §2D (`proto-cap`/`proto-wire` zero-cover table), §3-Wave-2.
- `docs/research/OPUS-PERF-BEBOP-AUDIT-2026-07-18.md` §2 (the clean-but-untimed verdict), P3 (KEM
  schoolbook + measure-before-NTT), §6 P3 action.
- `SYNTHESIS-PERFORMANCE-AUDIT-2026-07-18.md` §3.3-C3, §4 D-3 (NTT gate), §5 (Wave W2, unit P82,
  "P75 hard; P76/P78 landed").
- `MASTER-STATUS-LEDGER-2026-07-19.md` §1 (P82 row; "KEM bench gates D-9; substrate for P92 D-BENCH"),
  §3 (bebop gate-0 = P85+C3; Wave-2 P78→P82), §5 OD-3/OD-6/OD-9 (the gating operator decisions).
- **P75** (`BLUEPRINT-P75-*`, to be written) — owns the `<group>/<n>` + `baseline.json` schema + the
  working gate. **P82 cites; never redefines.**
- **P78** (`BLUEPRINT-P78-*`) — lands the `MerkleDigest`/`hub_ring` fixes P82 benches on top of.
- **P92** (`BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`) — consumes P82's `gate` lane for its
  D-BENCH; format precedent.
- `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 (the 20-point contract).
- Memory: `crypto-safe-first-pass-2026-07-14.md` (B4/SSR-2020 — the anti-cheat rationale),
  `cross-branch-todo-map-2026-07-10.md` (bebop files → `/root/bebop-repo`, push `openbebop`),
  `performance-priority-over-minimal-change-2026-07-17.md`.

**Existing code this blueprint edits/creates (exact targets — `/root/bebop-repo`, NOT dowiz):**
- **EDIT** `bebop2/core/benches/verify_lane.rs` — add the sign/KEM/AEAD/sha3/x25519 timings (§4.1);
  stays zero-dep `std::time`.
- **NEW** `bebop2/proto-cap/benches/criterion.rs` — group `gate` (§4.2).
- **NEW** `bebop2/proto-wire/benches/criterion.rs` — group `codec` (§4.3).
- **EDIT** `bebop2/proto-cap/Cargo.toml` + `bebop2/proto-wire/Cargo.toml` — `criterion` dev-dep +
  `[[bench]] harness = false` (§4.4). No `[dependencies]` change.
- **NEW** `bebop2/proto-cap/benches/baseline.json` + `bebop2/proto-wire/benches/baseline.json` — the
  per-crate v2 manifests (P75 §3.2) holding the §3 `gate`/`codec` bench-ids; committed so the bebop
  gate has something to compare against.
- **NEW** a bebop-repo CI bench job in **bebop2's OWN `.github/workflows/`** (§2.1.5) — the cross-repo
  `bench-regression` gate for the proto-cap/proto-wire criterion baselines, modeled on P75's kernel
  job. **Cross-repo prerequisite:** pin how the bebop runner obtains P75's `native-trackers` binary +
  schema (vendor the released binary, or a sibling-checkout of dowiz `tools/telemetry/native-trackers`
  — the G10 topology). Closes META-GAP-AUDIT G3 for bebop. The `core` `verify_lane.rs` numbers stay
  measured-not-gated (§1.2) and need no CI job.
- **DO NOT TOUCH** any bebop2 product `src/*` (bench-only; NTT wire-in is D-9, `MerkleDigest`/`hub_ring`
  are P78).

**For the worker with zero session context — exact acceptance path:**
1. **Confirm gate-0 is clear** (P85 closed + C3 ruled, OD-3/OD-6) and **P75 has landed** and **P78 has
   landed**. If any is open, STOP and report the block — P82 cannot commit through the bebop hooks
   until gate-0 clears, and its proto-crate baselines are ungated without P75.
2. Extend `verify_lane.rs` (§4.1); confirm `kem_encaps`/`kem_decaps` print stable numbers; record
   `kem_decaps` as the OD-9 input.
3. Write the two criterion siblings copying `kernel/benches/criterion.rs`'s structure; seed all keys
   from `BENCH_SEED`; `black_box` inputs/outputs; add the §4 anti-cheat sibling tests.
4. Wire the four manifest lines; `cargo bench -p bebop-proto-cap -p bebop-proto-wire` emits every
   `<group>/<n>` id; commit the two proto-crate `baseline.json` files.
5. **Author the cross-repo bebop `bench-regression` CI job** in bebop2's OWN `.github/workflows/`
   (§2.1.5), modeled on P75's kernel job but pointed at the proto crates + their `baseline.json` — this
   is P82's own deliverable, not something P75 provides (P75's gate lives in dowiz and benches only the
   kernel). First pin how the bebop runner reaches P75's `native-trackers` binary/schema (vendored
   binary or the G10 sibling-checkout topology). Then prove D4 (injected slowdown → the **bebop** gate
   RED; revert → GREEN) and D-CLEAN (`cargo tree -e no-dev` unchanged).
6. Add a `docs/regressions/REGRESSION-LEDGER.md` row: "bebop mesh crypto/auth/codec bench coverage
   established (P82); `gate`/`codec` gated, KEM/sign/AEAD measured."
7. Hand `gate/chain_1` to P92 (D-BENCH input) and `kem_decaps` to OD-9. Anti-scope: do **not** wire an
   NTT, do **not** touch `MerkleDigest`/`hub_ring`, do **not** invent a bench-id convention, do **not**
   add a runtime dependency to any sovereign crate.
