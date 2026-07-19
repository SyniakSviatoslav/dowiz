# OPUS — bebop2 core-consolidation audit: "can more calculation live directly in the core?"

> **Research-only.** Zero code written, no branches touched, no files modified beyond this doc.
> Every claim below was read live from `/root/bebop-repo/bebop2` (HEAD) and `/root/dowiz/kernel`
> this pass — cites are `file:line`, not memory. The operator's question is narrow and answerable
> with a clean recommendation: **the crypto that could be centralized already is; the two NEW
> pieces (transcript hash + replay window) split cleanly — one is a proto-cap extension of an
> existing pattern, the other must NOT go into core at all.**

---

## 0. TL;DR recommendation (stated up front)

1. **`bebop2-core` is already the centralized primitive authority.** It is not a thin types crate —
   it is a large, zero-dependency, `no_std`/wasm-empty-import "machine-code" layer holding every
   crypto primitive (Ed25519, ML-DSA-65, ML-KEM-768, SHA-3, KDF, AEAD, X25519, RNG) plus a
   verify-only ML-DSA verdict leg and the event-sourcing spine. There is **no crypto duplication
   across crates** to consolidate — `proto-cap`/`proto-wire`/`mesh-node` all call *into* core.

2. **The transcript-binding HASH construction should stay in `proto-cap` (`signed_frame.rs`), not
   move to core** — because the pattern *already exists there* (the F7 `channel_binding` slot,
   `signed_frame.rs:129-177`) and the hash primitive it needs (`sha3_256`) is *already* in core
   (`core/src/hash.rs:344`). Core supplies the primitive; proto-cap owns the transcript *layout*
   (which fields, what domain separation). That is exactly the correct seam.

3. **The `LastSeenNonce`-per-sender replay window must NOT go into `bebop2-core`.** Core is
   architected to have **no clock, no ambient state, no RNG reachable** (empty wasm import section,
   `lib.rs:2-4`, `ARCHITECTURE.md:42`). A replay window is stateful + time-relative + session-scoped
   by nature. It belongs at the `mesh-node`/`proto-cap` composition layer — which is exactly what
   the C3 red-team remediation already prescribes (`B2-protocol-authz.md:106`: "Move `seen` out of
   the per-connection gate into a bounded, expiry-pruned window keyed by `(subject_key, nonce)`").

4. **The current W/A/H crate split is well-reasoned and should NOT be collapsed.** It is a
   deliberate primitive-vs-composition layering (license boundary too: core=MIT, proto-cap=AGPL),
   and it mirrors dowiz's own discipline where the kernel centralizes primitives but is *not* the
   source of truth for everything (`dowiz/kernel/src/money.rs:348` — the server, not the kernel,
   stays authoritative for money). "Centralize the primitive, compose the policy" is the pattern
   already in force; the new work should follow it, not restructure around it.

---

## 1. Where `hybrid_gate.rs` lives and where its dependencies live

**`hybrid_gate.rs` lives in `proto-cap` (the "A" / authorization crate), NOT in `mesh-node`,
`core`, `proto-wire`, or `proto-crypto`.**

- File: `/root/bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs` (721 lines).
- Crate: `bebop-proto-cap` (`proto-cap/Cargo.toml:2`), which declares its single real dependency as
  `bebop2-core` (`proto-cap/Cargo.toml:9-11`, `default-features = false, features = ["std",
  "test_keygen"]`).

### 1.1 Dependency map of the verification path (verified by reading the call sites)

The gate is a thin **orchestrator**; every heavy operation delegates. Reading
`hybrid_gate.rs::check` (`:124-209`) top to bottom:

| Step in `check` | Cite | Where the real work lives |
|---|---|---|
| `capability.is_fresh(now)` (expiry) | `hybrid_gate.rs:134` | `proto-cap/src/capability.rs` (integer compare) |
| `verify_chain(roster, chain, cap, now)` | `hybrid_gate.rs:142` | `proto-cap/src/roster.rs` — UCAN-subset delegation walk; **≥1 Ed25519 verify per link**, each `bebop2_core::sign::verify` |
| red-line gate (armed) | `hybrid_gate.rs:150-154` | `proto-cap/src/redline.rs` (`RedLineGate::check`) |
| revocation lookups | `hybrid_gate.rs:159-168` | `proto-cap/src/revocation.rs` (`pq_key_id`, `revocation_hash`, hash-set) |
| `frame.verify_classical()` | `hybrid_gate.rs:171` | `signed_frame.rs:208-224` → `bebop2_core::sign::verify` (Ed25519) |
| `frame.verify_pq()` | `hybrid_gate.rs:181` | `signed_frame.rs:229-246` → `bebop2_core::pq_dsa::verify` (ML-DSA-65) |
| replay-nonce insert (`seen`) | `hybrid_gate.rs:193-206` | **in-file** `Mutex<HashSet<[u8;8]>>` (`:67`) — the ONE piece of live state, per-gate-instance |

So the composition is a clean three-tier stack:

```
bebop2-core        (MIT, zero-dep, no_std/wasm)  — PRIMITIVES: sign::verify, pq_dsa::verify, hash::sha3_256, pq_kem, aead, kdf, x25519, rng
   ▲
proto-cap          (AGPL, depends on core)       — COMPOSITION: SignedFrame(.verify_classical/.verify_pq), roster::verify_chain, revocation, redline, HybridGate
   ▲
proto-wire / mesh-node                           — RUNTIME: transports (wss/iroh/quic), node.rs owns Transport, breach signing
```

### 1.2 Answer to "is core verification already centralized, or scattered/duplicated?"

**Centralized, not scattered.** The *cryptographic* verification primitives are all in one place —
`bebop2-core` — and every downstream crate calls into them by path. Grepping for crypto
re-implementation outside core finds none; `proto-crypto` (the "H" ladder) explicitly states "No
primitives are (re)implemented here — they live in `bebop2-core`" (`proto-crypto/src/lib.rs`
header). What lives in `proto-cap` is not duplicated crypto — it is *protocol/authorization policy*
(delegation lattice, revocation set, red-line scope gate, hybrid policy), which is genuinely a
different concern and correctly separated from the primitives. There is **no awkward cross-crate
crypto dependency and no duplication to clean up.**

---

## 2. What `bebop2-core` actually contains (its real charter)

**`bebop2-core` is the foundational authority, not a thin shared-types crate.** Read
`core/Cargo.toml` and `core/src/lib.rs`:

- **Charter (self-described):** "From-scratch zero-dep post-quantum core… Pure core+alloc,
  empty-import wasm. No vendors." (`core/Cargo.toml:5`). "This IS the 'machine code' layer:
  deterministically verifiable, executed bit-exact." (`lib.rs:4`). `[dependencies] # none.`
  (`core/Cargo.toml` — intentionally zero deps, builds air-gapped).
- **Structural discipline that defines it:** `#![cfg_attr(not(feature = "std"), no_std)]`
  (`lib.rs:18`); the wasm artifact must have an **empty import section** — "no clock/RNG/socket
  reachable" (`lib.rs:2-3`, `ARCHITECTURE.md:42,95`). Feature `host` gates the f64 analytic kernel
  off the pure-crypto build (`core/Cargo.toml:features`).

### 2.1 Module inventory (from `lib.rs` `pub mod` declarations)

| Group | Modules | Cite |
|---|---|---|
| **Crypto primitives** | `aead` (XChaCha20-Poly1305), `hash` (SHA-512 + SHA-3), `kdf` (Argon2id), `pq_dsa` (ML-DSA-65 FIPS-204), `pq_kem` (ML-KEM-768 FIPS-203), `rng` (in-tree CSPRNG), `sign` (Ed25519), `x25519` | `lib.rs:359-372` |
| **Verify-only leg** | `key_v_verifier` (ML-DSA-65 split-identity verdict verify) | `lib.rs:368` |
| **AVX2 lane** | `keccak_x4_avx2` (x86_64+std only) | `lib.rs:365-366` |
| **Event-sourcing spine** | `event_log` (hash-chain), `anti_entropy`, `at_rest`, `deliberate` | `lib.rs:382-389` |
| **Analytic host kernel** (feature `host`) | `field`, `kalman`, `lyapunov`, `dmd`, `fft`, `chebyshev`, `active`, `vsa`, `algebra`, `micrograd`, `resonator`, `self_mod*` | `lib.rs:321-357` |
| **Single authoritative eigensolver** | `linalg` — explicitly labeled "the ONE authoritative eigensolver — the dual-authority hazard kill" | `lib.rs:345-355` |

**This is a kernel, not a types bag.** It is the direct bebop2 analog of `/root/dowiz/kernel` — a
deterministic, dependency-free authority layer. The design intent is explicitly "one authoritative
implementation per concern" (the `linalg` comment at `lib.rs:350-351` is the clearest statement of
it, killing a prior three-eigensolver dual-authority hazard).

### 2.2 The relevant in-core precedent: `key_v_verifier.rs`

There **is** already a verifier living in core (`core/src/key_v_verifier.rs`), and its shape is the
template for what *does* belong in core:

- It is **verify-only, pure, fail-closed, `no_std`** (`alloc::vec::Vec` only, `key_v_verifier.rs:14`).
- It does a bounded TLV parse (`parse_verdict`, `:73-148`) then calls
  `pq_dsa::verify_internal_bytes` (`:165`) — i.e. it composes a core primitive with a canonical
  byte-layout check, and holds **no state, no clock, no session context**.

That is the litmus test: a pure, stateless, time-free verification *function* is a legitimate core
citizen. A stateful, time-relative *session control* is not.

---

## 3. Where the NEW transcript-binding + replay-window logic should live

The operator's new work is two distinct pieces. They have **opposite** correct homes. Treating them
as one "move it into core" decision would be the mistake.

### 3.1 Piece A — transcript hash `Hash(ReceiverID‖Nonce‖Timestamp‖Data)`: **stays in `proto-cap` (`signed_frame.rs`)**

This is not a new pattern — it is a **generalization of the F7 channel-binding that already exists
in `signed_frame.rs`.** The existing code already appends a `SHA3-256` transcript hash into the
signing domain to defeat cross-*channel* replay:

- `SignedFrame.channel_binding: Option<[u8;32]>` (`signed_frame.rs:83-87`), documented as "SHA3-256
  over the handshake transcript" (`signed_frame.rs:14-16`).
- `signing_domain()` builds the canonical TLV over `(capability ‖ payload ‖ channel_binding)`
  (`signed_frame.rs:144-162`); `binding_signing_domain()` appends the 32-byte slot
  (`signed_frame.rs:172-177`); both `sign_classical`/`verify_classical` and `sign_pq`/`verify_pq`
  commit to it (`signed_frame.rs:184-246`).
- The cross-context replay defense is already proven by the RED→GREEN test
  `bound_frame_fails_on_different_channel` (`signed_frame.rs:423-454`).

The proposed C3 transcript (`ReceiverID‖Nonce‖Timestamp‖Data`) is the **same construction with more
fields bound in.** It should extend `signing_domain()` (add `ReceiverID` and `Timestamp` as new
canonical TLV fields via `crate::tlv`), exactly where `channel_binding` already lives.

**Why not core:** the *hash primitive* (`sha3_256`) is already centralized in core
(`core/src/hash.rs:344`) — the transcript construction calls it. What is left over is the
**transcript layout** (which fields, in what order, with what domain tag) — that is protocol/wire
policy, and `ARCHITECTURE.md:75` explicitly places "hand-written fixed-layout (de)serializer" and
canonical byte layout as a *core-adjacent-but-composition* concern kept out of the pure primitive
core. `proto-cap` already owns the canonical TLV codec (`proto-cap/src/tlv.rs`,
`DOMAIN_SIGNED_FRAME`/`FIELD_CHANNEL_BINDING`). Moving the layout into core would split the TLV
authority across two crates for no gain and would drag `ReceiverID`/capability shape (a proto-cap
type) down into the primitive layer.

**Verdict A: extend `signed_frame.rs`; call `bebop2_core::hash::sha3_256`. No core change.**

### 3.2 Piece B — `LastSeenNonce`-per-sender replay window: **must NOT go into core; belongs at `mesh-node`/`proto-cap`**

The replay ledger today is `HybridGate.seen: Mutex<HashSet<[u8;8]>>` (`hybrid_gate.rs:67`) —
per-gate-instance, in-process, and the gate is rebuilt per connection, which is *exactly* the C3
bug: "every `connect`/`accept` builds a fresh gate → empty `seen` on each connection/node"
(`B2-protocol-authz.md:54`). The **red-team's own prescribed fix already names the home**, and it
is *not* core:

> "Move `seen` out of the per-connection gate into a bounded, expiry-pruned window keyed by
> `(subject_key, nonce)` shared across connections; insert only *after* `verify_classical`
> succeeds." — `B2-protocol-authz.md:106`

Three hard reasons this cannot be a core primitive:

1. **Core has no clock and no ambient state — by architectural contract.** The wasm artifact must
   have an empty import section, "no clock/RNG/socket reachable" (`lib.rs:2-3`). A window that
   expiry-prunes needs a notion of *now*; the gate deliberately threads a caller-supplied
   `now: u64` tick precisely to avoid a wall-clock dependency (`hybrid_gate.rs:104,124`). Putting
   time-relative pruning state in core would violate the very property (`ARCHITECTURE.md:42,95`)
   that makes core deterministically verifiable.
2. **It is session/mesh state, not a primitive.** A `(subject_key, nonce)` window "shared across
   connections" is inherently node-runtime state. `mesh-node` is the crate that owns the transport
   and node lifecycle (`mesh-node/src/node.rs`, depends on all three lower crates —
   `mesh-node/Cargo.toml:10-17`). That is where a mesh-scoped ledger lives; the gate should take a
   `&mut` window reference the node owns, not manufacture one per instance.
3. **No such abstraction exists in core today** (grep for `last_seen|nonce_window|replay_window|
   seen_nonce` across `core/src/` returns nothing) — so this would be *new* stateful surface in the
   one crate whose whole value proposition is being stateless and side-effect-free. That is the
   opposite of consolidation; it is contamination.

**Verdict B: relocate `seen` up to a `mesh-node`-owned (or proto-cap struct the node owns),
bounded, expiry-pruned, `(subject_key, nonce)`-keyed window; keep verify-then-record ordering
(already correct at `hybrid_gate.rs:188-206`, the H2 fix). Core stays untouched.**

> Note: P92 (`BLUEPRINT-P92-…-2026-07-18.md:173-175`) explicitly disclaims the mesh-scoped nonce
> ledger as out of its scope — so this replay-window work is a *separate* unit from the fast-path
> blueprint and does not collide with it.

---

## 4. The dowiz-kernel "sole source of truth" pattern — and its honest limits

The operator asked whether bebop2-core should adopt dowiz kernel's "sole source of truth"
centralization. **The answer is nuanced: dowiz kernel centralizes *primitives and invariants*, but
is deliberately NOT the source of truth for everything — and that nuance is the correct model for
bebop2-core too.**

Evidence from `/root/dowiz/kernel`:

- **Where the kernel IS authoritative:** `DT_STABLE` is "the single source of truth: if you change
  it here, you MUST change `engine/src/loop_.rs::DT_STABLE` to match" (`kernel/src/lib.rs:314-319`,
  pinned by a fail-closed test `dt_stable_is_authoritative` at `:337`). The FSM lifecycle graph has
  a golden-signature boot gate that refuses to start on drift (`kernel/src/lib.rs:300-308`). These
  are *invariants/primitives*.
- **Where the kernel deliberately is NOT authoritative:** "The SERVER (apps/api orders.ts fee
  ladder) stays the single source of truth for what is CHARGED; this mirror only drives what the
  client SEES" (`kernel/src/money.rs:348-352`). The kernel holds a *mirror*, not the truth, for
  money charged — because that authority correctly lives at a different layer.

The bebop2-core analog is already in place and follows the same rule:

- **bebop2-core IS authoritative for primitives:** one eigensolver ("the dual-authority hazard
  kill", `core/lib.rs:350-351`), one Ed25519, one ML-DSA-65, one SHA-3. This is the pattern to
  preserve.
- **bebop2-core is deliberately NOT authoritative for protocol policy:** delegation lattice,
  revocation, red-line scope, replay windows, transcript layout — these live in proto-cap/proto-wire
  by design, the same way the fee ladder lives in the server, not the kernel.

**So the principle to adopt is not "move more into core" — it is "one authoritative implementation
per concern, at the lowest layer that needs no ambient authority."** Crypto primitives clear that
bar and are already in core. A clock-dependent replay window does not clear it and must stay above.

---

## 5. Why the W/A/H crate split exists (check before proposing to collapse)

The `proto-wire` (W) / `proto-cap` (A) / `proto-crypto` (H) separation is **deliberate and
documented**, not accidental:

- Root workspace comment: "Tier-0 P0-6 protocol W/A/H library lines… Separate crate names from the
  agent core (`crates/bebop`) and the PQ core (`bebop2/core`)" (`/root/bebop-repo/Cargo.toml`
  members block).
- **A (`proto-cap`)** = "REPLACES bearer JWT with a per-frame signed capability"
  (`proto-cap/src/lib.rs:1-19`) — authorization policy.
- **H (`proto-crypto`)** = "the crypto ladder library line — the verification/strength ladder that
  sits over the from-scratch primitives in `bebop2-core`… No primitives are (re)implemented here —
  they live in `bebop2-core`" (`proto-crypto/src/lib.rs` header). It is the Wycheproof→FIPS
  evidence ladder, a *test/assurance* surface, not a runtime one.
- **Core** = MIT-licensed, zero-dep, wasm-empty-import primitive kernel (`core/Cargo.toml:2-6`);
  **proto-cap** = AGPL (`proto-cap/Cargo.toml:2`). The split is also a **license boundary** — a real
  reason not to collapse policy code into the MIT primitive crate.

**Recommendation: do not collapse the split.** It cleanly separates (primitive | authorization
policy | assurance ladder | runtime), matches the license boundary, and matches the dowiz
kernel-vs-server discipline. The one legitimate consolidation observation is internal to core and
already done: the single-eigensolver `linalg` de-duplication.

---

## 6. Concrete scoped recommendation — what moves, what stays

| Item | Recommendation | Home | Reason (cited) |
|---|---|---|---|
| Ed25519 / ML-DSA-65 / ML-KEM / SHA-3 / KDF / AEAD / X25519 primitives | **Stay** (already centralized) | `bebop2-core` | Zero-dep primitive kernel; no duplication exists (`core/lib.rs:359-372`) |
| `verify_chain` (delegation lattice), revocation set, red-line gate, hybrid policy | **Stay** | `proto-cap` | Authorization *policy*, not primitive; license + concern boundary (`proto-cap/lib.rs:1-19`) |
| **Transcript hash `Hash(ReceiverID‖Nonce‖Timestamp‖Data)` construction** | **Extend existing code; do NOT move to core** | `proto-cap/src/signed_frame.rs` (+ `tlv.rs` fields) | Same pattern as F7 `channel_binding` (`signed_frame.rs:129-177`); calls `bebop2_core::hash::sha3_256` (`core/hash.rs:344`) — primitive already centralized |
| **`LastSeenNonce`-per-sender replay window** | **Relocate out of the per-instance gate; do NOT move to core** | `mesh-node` (node-owned) / a proto-cap struct the node holds | Stateful + clock-relative; core has no clock/ambient state by contract (`lib.rs:2-3`); C3 remediation says so (`B2-protocol-authz.md:106`) |
| verify-then-record ordering (H2) | **Keep as-is** | `hybrid_gate.rs:188-206` | Already correct; preserve when relocating `seen` |
| W/A/H crate split | **Keep** | — | Deliberate primitive/policy/assurance + license boundary (§5) |

### 6.1 The one-line answer to "can more calculation live in core?"

**The calculation that belongs in core (the crypto math) is already there.** The two new pieces are
(a) a *transcript layout* — belongs in proto-cap, calling the core hash primitive — and (b) a
*stateful, time-relative replay window* — which by core's own empty-import/no-clock contract
**cannot** live in core and must live at the node/composition layer. So the honest recommendation is
**not** "centralize more into core," but "keep composing: primitive in core, policy in proto-cap,
mesh state in mesh-node." The current split is already the right shape; the new work should slot into
it, not restructure it.

---

## 7. Sources (all read live this pass)

- `/root/bebop-repo/bebop2/proto-cap/src/hybrid_gate.rs` (gate orchestration + `seen` ledger)
- `/root/bebop-repo/bebop2/proto-cap/src/signed_frame.rs` (F7 channel-binding, signing domain, verify_*)
- `/root/bebop-repo/bebop2/proto-cap/src/lib.rs`, `proto-cap/Cargo.toml`
- `/root/bebop-repo/bebop2/core/src/lib.rs`, `core/Cargo.toml`, `core/src/key_v_verifier.rs`, `core/src/hash.rs`
- `/root/bebop-repo/bebop2/proto-crypto/src/lib.rs`
- `/root/bebop-repo/bebop2/mesh-node/Cargo.toml` (+ src listing), `/root/bebop-repo/bebop2/ARCHITECTURE.md`
- `/root/bebop-repo/bebop2/docs/red-team/2026-07-13/B2-protocol-authz.md` (C3 replay + remediation #2)
- `/root/bebop-repo/Cargo.toml` (workspace members + W/A/H rationale)
- `/root/dowiz/kernel/src/lib.rs` (DT_STABLE / FSM source-of-truth), `/root/dowiz/kernel/src/money.rs:348` (server-is-authoritative counter-example)
- `/root/dowiz/docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` (§0.4 primitives-in-core table, §2.2 excludes the mesh nonce ledger)
