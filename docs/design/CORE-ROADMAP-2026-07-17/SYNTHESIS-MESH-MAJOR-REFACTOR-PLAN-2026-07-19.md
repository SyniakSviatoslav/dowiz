# SYNTHESIS — Mesh Authentication-Layer Major Refactor Plan: M1 → P93 → P92 → P94 (2026-07-19)

> **Planning document — writes ZERO product code, touches no branches, pushes nothing.**
> Per the operator's standing directive (research + blueprints + plan updates only). This doc is
> the authoritative, dependency-ordered execution plan tying together one coherent cluster of
> 2026-07-18/19 findings about the **bebop2 mesh authentication layer** (`proto-cap` /
> `proto-wire` / `mesh-node`, grounding tree `/root/bebop-repo/bebop2`). It sequences four units:
> one already-open correctness fix (**M1**), one already-fully-blueprinted optimization (**P92**),
> and two new blueprint proposals sketched here for a follow-up Opus expansion pass (**P93**,
> **P94**) — exactly as `SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` left P90/P91 as scoped stubs.
>
> **One sentence:** land the real RFC-5705 exporter binding first (M1 — an independent prerequisite
> bug), then harden the *existing* per-message store-and-forward path with transcript-binding +
> replay-windowing (P93), then layer the live-session fast-path on top of the now-real exporter
> (P92), then — only because P92 makes `effect ⊆ session_cap` the hot per-frame op — pack
> `Scope`/`Effect` into an in-memory bitmask (P94).

---

## 0. Provenance — the five source docs this synthesis consolidates (all read live this pass)

Row order = the causal order the findings surfaced this session; column 3 = what each contributed
to *this* plan.

| # | Source doc | What it established | Feeds |
|---|---|---|---|
| S1 | [`docs/research/OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md`](../../research/OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md) | The mesh relay is genuinely **semi-trusted** (not a closed 2-party channel) — hybrid_gate's per-frame crypto is *necessary*, not waste. MAC ≠ signature; a counter carries no transferable authority / non-repudiation. | The floor under all four units: none of this removes per-frame signing; it hardens or narrowly optimizes it. |
| S2 | [`docs/research/OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md`](../../research/OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md) | Per-message signing is **structurally required** for store-and-forward (`bpv7.rs` proves offline-authored bundles reach peers never in any handshake). Found ONE scoped opportunity: a live-only presence fast-path (§5b). Found the exporter binding is simulated **and** unenforced on recv. | M1 (the exporter gap), P92 (the fast-path). |
| S3 | [`CORE-ROADMAP-2026-07-17/BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md`](BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md) | Full blueprint of the fast-path. Verdict **GO-WITH-CONDITIONS + measure-first**. Elevated the exporter fix to hard prerequisite **C1** (== M1 here) and named it independently valuable (closes red-team F3/M1). | M1 (as its C1), P92 (it *is* P92). |
| S4 | [`docs/research/OPUS-PACKED-FLAGS-AUTH-LAYER-SCAN-2026-07-19.md`](../../research/OPUS-PACKED-FLAGS-AUTH-LAYER-SCAN-2026-07-19.md) | ~90% of the auth layer is already minimal. ONE candidate: `Scope`/`Effect = Vec<(Resource,Action)>` → an in-memory `[u32;18]` bitmask (`Copy`, branchless subset, zero per-frame alloc). Correctly scoped **as a P92 enabler**, in-memory only, **not** a wire-format change. | P94. |
| S5 | [`docs/research/OPUS-CORE-CONSOLIDATION-AUDIT-2026-07-19.md`](../../research/OPUS-CORE-CONSOLIDATION-AUDIT-2026-07-19.md) | Crypto primitives already centralized in `bebop2-core` (no dup to fix). Ruled: the new **transcript hash belongs in `proto-cap`** (`signed_frame.rs`, extends the existing `channel_binding` pattern); the new **`LastSeenNonce` replay-window must NOT go in core** (core is ambient-state-free — no clock) → it belongs in **`mesh-node`**. Governing law: "one authoritative impl per concern, at the lowest layer needing no ambient authority." | P93 (crate placement of both halves). |

**Operator-surfaced refinements** (external analysis, cross-checked sound against S1–S5 this pass —
these are the *new* content that becomes P93): transcript binding `Hash(ReceiverID ‖ Nonce ‖
Timestamp ‖ Data)`; the **privacy fork** (plaintext ReceiverID leaks topology vs a blinded
`Hash(RecipientPubKey ‖ SharedSecret)`); the `LastSeenNonce`-per-sender replay window; and the
**open broadcast/multicast question** (one signed copy per recipient vs a shared group key /
wildcard ReceiverID).

---

## 1. The four units — one-glance

| Unit | Name | State | Path it protects | Blueprint |
|---|---|---|---|---|
| **M1** | Real RFC-5705/9266 exporter: capture + set-on-send + **enforce-on-recv** | **Independent prerequisite bug** (red-team F3/M1, STILL OPEN). Spec already written inside P92 §4.1/§6 as its own landable unit. | The **live-channel** full-signed path (defeats MITM/relay-splice). | P92 §4.1 (M1) — already specced; **no separate file needed**, but must land as its own reviewed commit. |
| **P93** | **Transcript-Binding + Replay-Window for the Store-and-Forward Path** (NEW) | **WRITTEN 2026-07-19** — full 20-point blueprint, D-93-A (privacy fork) and D-93-C (broadcast) both resolved, not deferred. | The **channel-less / detached** per-message store-and-forward + gossip path (defeats cross-node replay C3). | `BLUEPRINT-P93-transcript-binding-replay-window-2026-07-19.md` |
| **P92** | Mesh hot-stream fast-path (verify-once + channel-bound PQ session MAC) | **FULLY BLUEPRINTED** (S3). GO-WITH-CONDITIONS + measure-first. | The **live continuously-online same-scope** presence stream (narrow optimization). | [`BLUEPRINT-P92-…-2026-07-18.md`](BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md) — **do not re-derive; reference.** |
| **P94** | **Scope/Effect Bitmask Representation** (NEW) | **WRITTEN 2026-07-19** — full 20-point blueprint, sequenced after P92, TLV wire form confirmed unchanged. | The per-frame authz decision layer (in-memory only). | `BLUEPRINT-P94-scope-effect-bitmask-2026-07-19.md` |

---

## 2. The dependency-ordered execution sequence (reasoned, not assumed)

**Canonical order: `M1 → P93 → P92 → P94`.** The four edges are **not all the same kind** of
dependency — conflating them would be dishonest, so each is typed and justified from the actual
mechanics in S1–S5.

```
        (independent correctness floor; closes red-team F3/M1)
   ┌────────────────────  M1  ───────────────────────┐
   │  real RFC-5705 exporter: capture+send+ENFORCE-recv │
   │  edits signed_frame.rs binding source + recv compare│
   └──────────┬───────────────────────────┬────────────┘
     SOFT floor│                    HARD gate│  (P92 is a MITM
   (shared surface)│                          │   downgrade without it)
              ▼                            ▼
      ┌───────────────┐   PRIORITY    ┌──────────────┐   VALUE   ┌──────────────┐
      │      P93      │ ────────────▶ │      P92     │ ────────▶ │      P94     │
      │ transcript-   │ (P93 hardens  │ live-session │ (bitmask  │ Scope/Effect │
      │ bind + replay │  the DOMINANT │ fast-path    │  is worth │ [u32;18]     │
      │ window        │  default path;│ (optional,   │  it only  │ in-memory    │
      │ (existing S&F │  P92 is opt-in │  measure-1st │  once P92 │ mask         │
      │  path)        │  & gated)     │  gated)      │  exists)  │              │
      └───────────────┘               └──────────────┘           └──────────────┘
       cross-node replay ledger P92 §2.2 disclaims ──────────────┘ (P93 supplies it)
```

### 2.1 `M1` is first — and it is genuinely first, for three independent reasons

1. **It is an already-open bug, not new scope.** The RFC-5705 exporter binding is a *simulated
   literal* (`wss_transport.rs:1324`, `bpv7.rs:460`) and the receiver **never compares** the binding
   against the live channel (red-team F3/M1, `B3-wire-transport.md:57`, **STILL OPEN** — verified in
   S3 §0.2). This is a correctness defect on the **existing full-signed path** that exists regardless
   of P92/P93/P94. Fixing it is owed independent of any optimization.
2. **P92 hard-depends on it.** Without the *real* exporter, a malicious relay produces the *same* fake
   binding on both sides and a session-splice **succeeds silently** — P92 on a simulated binding is a
   "MITM downgrade, not an optimisation" (S3 VERDICT/C1, §7.4). This is a NO-GO gate, not a preference.
3. **It is the shared-surface floor both P92 and P93 build on.** Both extend the
   `signing_domain()` / `channel_binding` machinery in `signed_frame.rs` (S5 §3.1). Landing M1 first
   establishes the *enforce-binding-on-recv* pattern on a corrected surface, so P93's new
   transcript-field additions and P92's session-key rooting both compose onto real, enforced code
   rather than a simulated stub.

### 2.2 `P93` before `P92` — this is a PRIORITY + coherence ordering, **not** a hard functional block

Honest framing (do not overstate): P93 and P92 are **largely independent** — they protect *disjoint
frame populations*. P93 hardens the **channel-less** detached store-and-forward / gossip path
(where there is no live channel to bind to, so authentication must travel *with the frame* —
S2 §4); P92 optimizes the **live continuously-online** presence stream. A frame is essentially
either one or the other. Neither strictly blocks the other's core mechanism. P93 is sequenced first
because:

- **It hardens the dominant, always-on default path.** The mesh's real workload is delay-tolerant /
  store-and-forward / gossip (S2 §4/§5a); presence hot-streams are a narrow slice. Closing the
  cross-node replay class (C3) on the default path is higher-value correctness than a gated
  throughput win. P92 is explicitly opt-in, auto-fallback, and **measure-first NO-GO-gated** (S3
  §10.3); P93 is unconditional hardening.
- **P93 supplies the very ledger P92 disclaims.** P92 §2.2 explicitly puts the *mesh-scoped
  cross-node nonce ledger* out of its scope ("Fast-path frames never leave the session, so
  cross-node replay does not apply to them"). That mesh-scoped ledger **is** P93's replay window.
  Building P93 first means the store-and-forward frames P92 *doesn't* fast-path already have their
  cross-node replay defense in place. (Confirmed cross-consistent: S5 §3.2 note quotes P92
  disclaiming the nonce ledger, and routes it to P93's home in `mesh-node`.)
- **M1 → P93 is a soft floor, not a hard block.** P93's transcript binding is anchored on
  `ReceiverID`, *not* the TLS exporter — it is the store-and-forward *analog* of M1's channel
  binding for the path that has no channel. So P93 could in principle proceed without M1. It is
  sequenced after M1 only because they edit the same `signed_frame.rs` signing-domain surface and
  M1 is the isolated correctness floor (§2.1). If lane parallelism is ever needed, P93's
  transcript-field work and M1's exporter work are separable enough to run concurrently by careful
  authors — but the **default is M1-then-P93** to keep the shared surface single-writer.

### 2.3 `P94` last — a DEPENDENCY-OF-VALUE on P92, self-declared by the research

P94 (the `Scope`/`Effect` bitmask) does **not** functionally require P92 to compile or run — it is a
drop-in in-memory representation. It is sequenced last because its *value* is created by P92: S4 §5
is explicit — on the crypto-dominated full per-frame path the bitmask is only a "modest but real"
allocation-elimination win (2× Ed25519 + 1× ML-DSA-65 verify dominate, so shaving `Vec` clones is
secondary); but once P92's fast-path **amortizes the crypto to once-per-session**, `effect ⊆
C.scope` becomes "the dominant per-frame authorization operation," and the branchless word-AND
mask becomes a genuine compounding per-frame saving. The research's own recommendation: "flag the
bitmask as a fast-path enabler for P92, not a blanket rewrite … sequence it **after** P92" (S4 §5,
§7). Doing P94 before P92 would be optimizing a path the crypto already dominates — premature.

### 2.4 Why not fold P93 and P94 into P92?

Because they have different homes, different risk profiles, and different gates. P93 is a
**correctness** change to the always-on signed path (red-line-adjacent: it changes what bytes are
signed) touching `signed_frame.rs` + a new `mesh-node` replay-window; P92 is an **optional
performance** overlay in a new `proto-wire/fastpath.rs` gated on a benchmark; P94 is a pure
**in-memory representation** refactor in `scope.rs`/`roster.rs` with a signature-stability KAT. Three
concerns, three units, one authoritative impl each (the S5 governing law applied to the plan itself).

---

## 3. M1 — the prerequisite (already specced in P92; restated here as the plan's gate 0)

M1 is **not a new blueprint** — its full spec, RED tests, and exact edit targets already live in
**P92 §4.1 (M1), §6, and §14**. This synthesis only elevates it to the plan-level gate-0 and records
that it must land **as its own reviewed commit before any fast-path or transcript code**.

- **What it does:** at `connect`/`accept`, before `conn` is consumed into `(send, recv)`, call
  `conn.export_keying_material(&mut out32, EXPORTER_LABEL, EXPORTER_CONTEXT)` (quinn) /
  `tls.export_keying_material(...)` (rustls); store `ChannelBinding` on the carrier; set it on **every**
  outbound frame; and **reject on recv** any frame whose `channel_binding != Some(self.binding)` (and
  `None` when `require_tls_channel_binding` — flip that prod default to `true`). The recv comparison is
  the piece absent today.
- **What it closes independently of everything else:** red-team **F3/M1** for the full-signed path —
  a REGRESSION-LEDGER entry. This is why it is worth landing "regardless" (S3 §10.3, §6).
- **Falsifiers (from P92 §4.1):** `red_exporter_mismatch_rejected`,
  `red_none_binding_rejected_when_required`, `red_mitm_cert_swap_splits_exporter`.
- **Edit targets:** `proto-wire/src/{iroh_transport.rs, wss_transport.rs, transport_policy.rs}`;
  reuse `handshake::channel_binding_hash`. No new dep.

**Gate-0 rule:** do not write any P92 fast-path code or any P93 transcript code until
`red_mitm_cert_swap_splits_exporter` is GREEN and M1 has an independent adversarial review
(the exporter must do *real* MITM detection, or the whole binding is decorative — P92 §6/§8).

---

## 4. P93 — Transcript-Binding + Replay-Window for the Store-and-Forward Path (NEW — scope sketch)

> Full 20-point blueprint per `CORE-ROADMAP-STANDARD-2026-07-17.md` §2 is a **follow-up Opus pass**.
> This is the P90/P91-style scope stub: problem, design sketch, key decisions, dependencies,
> red-lines/invariants. **Filename when written:** `BLUEPRINT-P93-transcript-binding-replay-window.md`.

### 4.1 Problem

The **existing per-message-signed store-and-forward path** (the always-on default: `bpv7.rs` BPv7
overlay, `sync_pull.rs` anti-entropy gossip) is vulnerable to **cross-node replay (red-team attack
class C3)**: the same signed bytes captured off one node can be replayed to a *different* node,
because (a) the signature commits only to `(capability ‖ payload ‖ channel_binding)` and for a
detached bundle the channel binding is `None`/absent (there is no shared live channel — S2 §4), and
(b) the replay-nonce ledger is `HybridGate.seen: Mutex<HashSet<[u8;8]>>` rebuilt **fresh per
connection** (S5 §3.2, quoting `B2-protocol-authz.md:54` — "empty `seen` on each connection/node").
A per-session counter cannot see a replay that *leaves* the session (S2 §2, C3 row). This is the
one place S2's "cheap per-session counter" idea is provably insufficient.

M1 solves cross-node replay for **live-channel** frames (a spliced channel → different exporter →
rejected). P93 is the **store-and-forward analog**: for a frame that travels detached from any
channel, bind it instead to its **intended receiver + freshness context**, and track last-seen
per sender at the receiver.

### 4.2 Design sketch

**(a) Transcript binding — extend the signed domain (home: `proto-cap/src/signed_frame.rs`).**
Sign `Hash(ReceiverID ‖ Nonce ‖ Timestamp ‖ Data)` instead of raw `Data`, binding every signed
frame to a specific receiver + context. Per S5 §3.1 this is a **generalization of the existing F7
`channel_binding` pattern** — extend `signing_domain()` to add `ReceiverID` and `Timestamp` as new
canonical TLV fields (via `crate::tlv`), exactly where `channel_binding` already lives; call
`bebop2_core::hash::sha3_256` (primitive already centralized — **no core change**). Both
`sign_classical`/`verify_classical` and `sign_pq`/`verify_pq` commit to the extended domain.
A frame authored for receiver B and replayed at node C fails verification because C's `ReceiverID`
≠ B's. **This closes C3 for the store-and-forward path specifically, and it applies to the EXISTING
per-message-signed path — it is not a P92 fast-path concern.**

**(b) Replay windowing — a `LastSeenNonce`-per-sender window (home: `mesh-node`, NOT core).**
At the receiver, keep a bounded, expiry-pruned window keyed by `(subject_key, nonce/seq)`;
discard a frame whose sequence is **not greater** than the last seen for that sender (standard DTN
anti-replay). Per S5 §3.2/§6 this **must not** live in `bebop2-core` (core has no clock / no ambient
state by architectural contract — empty wasm import section) and must relocate out of the
per-connection `HybridGate.seen` up to a `mesh-node`-owned (or a proto-cap struct the node holds)
window "shared across connections" — this is verbatim the C3 remediation (`B2-protocol-authz.md:106`).
Keep the verify-then-record ordering already correct at `hybrid_gate.rs:188-206` (the H2 fix).

### 4.3 Key decisions

- **DECISION D-93-A — the privacy fork (operator-decidable; recommend blinded, flag it).**
  Plaintext `ReceiverID` in the transcript **leaks mesh topology** to an observing semi-trusted relay
  (which node is talking to which — a real metadata leak on a network whose whole point is
  operator-sovereignty and, later, `.onion`/anonymity tiers per fold-in L4/P53). The alternative is a
  **blinded recipient tag** `Hash(RecipientPubKey ‖ SharedSecret)` — the receiver can recognize its
  own tag (it knows the shared secret) but an observer cannot correlate frames to a recipient identity.
  Cost: added complexity + a shared-secret derivation per (sender, receiver) pair, and it interacts
  with the broadcast question (D-93-C).
  **Recommendation: adopt the blinded tag** — topology-privacy is load-bearing for a sovereign mesh and
  the cost is a single KDF call already available in-tree (`bebop2-core::kdf`). **But flag it as an
  explicit operator decision point** (never bypass human-gated design forks — MEMORY
  `never-bypass-human-gates`), because the plaintext option is simpler and materially easier to debug,
  and the blinded option's shared-secret needs a definition for the store-and-forward case where sender
  and receiver may never have handshaked (candidate: derive from the receiver's enrolled anchor pubkey +
  an ML-KEM encapsulation carried in the bundle — the blueprint pass must nail this down or the blinded
  option is under-specified). Record BOTH in the blueprint; the operator picks.
- **DECISION D-93-B — `LastSeenNonce` placement is settled: `mesh-node`.** Not core (S5 §3.2). Not a
  re-litigation — recorded so the blueprint pass does not drift it back into `proto-cap`/core.
- **OPEN QUESTION D-93-C — broadcast / multicast (NOT yet answered by any of S1–S5; the blueprint pass
  MUST resolve or explicitly defer-with-reason).** One courier message intended for *multiple* nearby
  mesh nodes breaks the single-`ReceiverID` transcript: does it require **(i)** a separate signed copy
  per recipient (N signatures, N transcripts — correct but O(N) signing cost and O(N) bandwidth), or
  **(ii)** a shared **group key / wildcard `ReceiverID`** (one signature, one transcript, but weaker
  binding — a group tag is replayable *within* the group, and a group key is forgeable by any member so
  it loses non-repudiation)?
  **Provisional recommendation (blueprint pass to confirm or overturn): DEFER broadcast transcript
  binding to a dedicated sub-unit, and in the interim keep broadcast/multicast frames on the *existing*
  full per-frame signature with a *wildcard/broadcast* `ReceiverID` sentinel that binds Nonce ‖
  Timestamp ‖ Data but not a specific recipient** — accepting that broadcast frames get freshness +
  authorship binding but NOT per-recipient cross-node-replay protection, because a broadcast is by
  definition intended for many nodes so "replay to another node" is partly its *purpose*. Rationale:
  the breach-alarm precedent (`iroh_transport.rs:366-389`) already treats broadcast as a self-signed,
  roster-bypassing, deliberately-widely-accepted frame — forcing per-recipient binding on it would
  fight its design. **This is a genuine open fork; the blueprint pass owns closing it** (option (i) for
  small known recipient sets, sentinel-defer for true broadcast is the leading shape). Do not let it
  silently default.

### 4.4 Dependencies

- **Soft prerequisite: M1** (shared `signed_frame.rs` signing-domain surface; §2.2). Functionally
  independent (ReceiverID-anchored, not exporter-anchored) but sequenced after M1 as the correctness
  floor.
- **Independent of P92** (disjoint frame path; §2.2). Supplies the cross-node nonce ledger P92 §2.2
  disclaims.
- **In-tree primitives only** (S5): `bebop2-core::{hash::sha3_256, kdf}`, `proto-cap::tlv`. No new dep,
  no core change.

### 4.5 Red-lines / invariants to preserve

- **This is a change to what bytes are signed** on the authoritative path → **signature-stability is a
  breaking concern.** The blueprint must define a **versioned signing-domain discriminant** (new TLV
  field tag, append-only, fail-closed on unknown — mirror `FrameKind`/`AlgSuite` discipline) so old and
  new frames are unambiguously distinguishable and there is a clean migration; never silently change the
  domain under the same version.
- **Verify-then-record ordering** (`hybrid_gate.rs:188-206`) must be preserved when relocating `seen`.
- **The store-and-forward / gossip / breach paths stay full per-frame hybrid-signed** — P93 *strengthens*
  what they sign; it never removes signing (S1/S2 floor).
- **No non-repudiation regression:** transcript binding must not weaken authorship binding; the blinded
  fork (D-93-A) must keep the frame signature over the *same asymmetric identity* — only the recipient
  *tag* is blinded, never the signature.
- **Independent adversarial-review gate** (same B4/SSR-2020 rigor as P92 §8): the review must attempt a
  cross-node replay that survives the new transcript, a topology-deanonymization against the blinded
  tag, and a broadcast-frame replay — before P93 ships.

---

## 5. P94 — Scope/Effect Bitmask Representation (NEW — scope sketch)

> Full blueprint is a **follow-up Opus pass**. Filename when written:
> `BLUEPRINT-P94-scope-effect-bitmask.md`. Scope stub only, below.

### 5.1 Problem

`Scope` and `Effect` are both `struct { grants: Vec<(Resource, Action)> }`
(`scope.rs:127-131`, `roster.rs:60-64`) — a **heap-allocated collection over a small, closed, fixed
domain** (`Resource` ≤32 variants, `Action` ≤32; 408 pairs today, ≤1024 with headroom). The hot
authz path (`verify_chain`, `RedLineGate::check`, `facade` read-gate) does **O(n·m) linear `Vec`
scans** for subset checks and **per-frame/per-link heap clones** (`roster.rs:129, :295, :305`;
`redline.rs:99` allocates a fresh `Vec` every call). On the crypto-dominated full path this is a
modest cost; **on P92's fast-path — where crypto is amortized to once-per-session —
`effect ⊆ session_cap` becomes the dominant per-frame operation** (S4 §5).

### 5.2 Design sketch

Introduce an **in-memory** `ScopeMask([u32; 18])` (`Copy`, 72 bytes; one `u32` action-set per
`Resource`) derived from `Scope`/`Effect` at parse/verify time:
- subset: `self.rows.iter().zip(sup.rows).all(|(a,b)| a & !b == 0)` — 18 word-ANDs, **branchless,
  zero allocation**.
- red-line: one precomputed `const RED_LINE: [u32; 18]` → `any(|(s,m)| s & m != 0)`, no per-call
  `Vec`.
- Making `Scope`/`Effect` `Copy` deletes **every** per-frame/per-link `.clone()` on the hot path.

### 5.3 Key decisions

- **DECISION D-94-A — in-memory ONLY; the wire/signing encoding does NOT change.** `Scope::to_tlv_bytes`
  is on the **signed path** (committed to by Ed25519 **and** ML-DSA-65 — `capability.rs:110-124`,
  `signed_frame.rs:144-162`). The TLV **ordered-pair list stays the canonical serialize/sign form**
  (canonicalized: sort pairs before encoding so equal sets have identical bytes); the bitmask is derived
  at parse time for checks and never serialized. This is the same two-representation discipline already
  in the crate (serde-transport vs hand-built-TLV-signing). **Non-negotiable — a wire/signature change is
  out of scope by construction.**
- **DECISION D-94-B — packing (A) `[u32; 18]` per-resource, not (B) flat `[u64; 16]`.** (A) is leaner and
  maps 1:1 to the resource/action structure (S4 §4a). Recommend (A); blueprint confirms.
- **Honesty constraint:** this is **not** a hash-avoidance win (the structure is a `Vec`, not a
  `HashSet`; typical scopes are single-pair so the scan is ~1 compare). State the gain precisely:
  **allocation elimination + `Copy` + branchless subset**, not "O(1) `&` instead of O(hash)" (S4 §4c-2).

### 5.4 Dependencies

- **Value-depends on P92** (§2.3): worth building because P92 makes `effect ⊆ session_cap` hot. On the
  full path alone it is a modest opportunistic clean-up. **Sequence after P92.**
- **In-tree only**; no new dep; no wire change; no core change.

### 5.5 Red-lines / invariants to preserve

- **Signature stability is the top invariant:** a signature-stability KAT must prove the TLV signed bytes
  are byte-for-byte unchanged before/after (S4 §7).
- **Behavioral equivalence:** a RED test must prove `ScopeMask`-subset is byte-for-byte equivalent to the
  current `Vec` subset across the **full 17×24 pair matrix** (S4 §7).
- **Red-line scope semantics unchanged:** the precomputed `RED_LINE` mask must be provably identical to
  the current `is_red_line` predicate over every pair — a divergence here is a money/auth authorization
  hazard.

---

## 6. What this plan deliberately does NOT do (anti-scope)

- **Does not remove or weaken per-frame signing anywhere** — S1/S2 proved it load-bearing; all four
  units either harden it (M1, P93) or optimize *around* it without replacing it on any store-and-forward
  / gossip / breach / control / red-line frame (P92, P94).
- **Does not touch `bebop2-core`** — S5's ruling: the transcript *hash primitive* is already in core;
  the transcript *layout* and the replay *window* are composition/mesh state and stay above core.
- **Does not collapse the W/A/H crate split** — S5 §5: it is a deliberate primitive/policy/assurance +
  MIT-vs-AGPL license boundary; keep it.
- **Does not change any wire format or signed-byte layout except P93's versioned, append-only
  signing-domain extension** — and even that keeps a fail-closed version discriminant; P94 changes
  nothing on the wire at all.
- **Does not re-derive P92** — P92's 862-line blueprint stands; this doc references and sequences it.

---

## 7. Status honesty — ALL of this is UNPUSHED / UNIMPLEMENTED planning

**Consistent with the operator's standing "research + blueprints only, no push, no branch work"
directive, NO code has been written for any of M1 / P92 / P93 / P94.**

| Unit | Code state | Branch state |
|---|---|---|
| M1 (exporter fix) | **0 code.** Spec exists inside P92 §4.1/§6. Red-team F3/M1 still OPEN. | none |
| P92 (fast-path) | **0 code.** Full blueprint written (planning doc). GO-WITH-CONDITIONS, measure-first NO-GO gate un-run. | none |
| P93 (transcript + replay) | **0 code.** Scope sketch only (this doc §4). Full blueprint not yet written. | none |
| P94 (bitmask) | **0 code.** Scope sketch only (this doc §5). Full blueprint not yet written. | none |

No branches were created or touched, nothing was pushed, and the only files this planning pass
writes are this synthesis doc and one new row in `CORE-ROADMAP-INDEX.md`. Every unit above remains
gated behind its own future implementation + independent-adversarial-review + (for P92) a
measure-first benchmark, none of which have run.

---

## 8. Follow-up passes this synthesis authorizes (planning only — each is itself a future decision)

1. **Opus blueprint-writing pass for P93** — expand §4 to the full 20-point contract; the pass MUST
   close open question **D-93-C** (broadcast/multicast) and fully specify the blinded-tag shared-secret
   derivation if **D-93-A** is ruled "blinded".
2. **Opus blueprint-writing pass for P94** — expand §5; write the signature-stability + full-matrix
   equivalence KATs as the DoD spine.
3. **M1** needs no new blueprint (P92 §4.1 suffices) — it needs an implementation pass + independent
   review, gated to land first.
4. **Operator decisions to surface:** D-93-A (privacy fork: plaintext vs blinded ReceiverID),
   D-93-C (broadcast strategy), and — inherited from P92 — the measure-first NO-GO ruling and the
   independent-adversarial-review gate for M1 and the fast-path.

---

*Cross-references: `BLUEPRINT-P92-MESH-HOTSTREAM-FASTPATH-2026-07-18.md` (the P92 blueprint, referenced
not re-derived) · `docs/research/OPUS-TRUST-BOUNDARY-CLOSED-CHANNEL-SCAN-2026-07-18.md` (S1) ·
`docs/research/OPUS-HANDSHAKE-ONCE-VS-PERMESSAGE-2026-07-18.md` (S2) ·
`docs/research/OPUS-PACKED-FLAGS-AUTH-LAYER-SCAN-2026-07-19.md` (S4) ·
`docs/research/OPUS-CORE-CONSOLIDATION-AUDIT-2026-07-19.md` (S5) ·
`CORE-ROADMAP-STANDARD-2026-07-17.md` (20-point contract for the P93/P94 writing passes) ·
`SYNTHESIS-WAVE3-CLOSEOUT-2026-07-18.md` (the P90/P91 scope-stub precedent this doc mirrors) ·
red-team `bebop2/docs/red-team/2026-07-13/{B2-protocol-authz.md, B3-wire-transport.md}` (C3, F3/M1) ·
memory: `crypto-safe-first-pass-2026-07-14.md` (B4/SSR-2020 review precedent),
`never-bypass-human-gates-2026-06-29.md` (D-93-A/D-93-C are human-gated forks).*
