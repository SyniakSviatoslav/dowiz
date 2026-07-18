# GAP AUDIT — Kernel / Delta-Data-Flow Architecture (2026-07-18)

> **What this is.** A read-only cross-reference pass over the *entire* kernel-architecture +
> delta-data-flow corpus produced this session, hunting for genuine gaps not yet captured or
> reconciled anywhere. No code, no commits. Output confined to this file.
>
> **Isolation.** Written from `/root/dowiz-verify-redteam` (worktree branch
> `research/dowiz-verify-redteam-2026-07-17`, fork point `4956faca3`) to avoid colliding with the
> LIVE `/root/dowiz` checkout, which advanced to `main @ 87da9ccd4` during this session — the
> movement is real and is itself a finding (§1, citation drift).
>
> **Method.** Every §1 status row and every "no gap" statement was re-verified against
> `git show main:<file>` on the live tree this pass, not trusted from a prior citation. The corpus
> read in full: the 185-item ledger (BEBOP2-MESH-MASTERWORK-V2), P-A/P-B kernel blueprints, round-1
> + round-2 fail-operational syntheses, the five round-2 blueprints (Fable-A..E), and both
> verification master syntheses (bebop2 + cross-repo).

---

## 1. Consolidated verdicted-but-unlanded fix list — FRESH live status

The corpus names these repeatedly without a single place tracking their *current* landed state.
Consolidated here, each re-verified against `main @ 87da9ccd4` this pass. **The headline: the two
that are STILL LIVE are the two the corpus most confidently treats as "known, just do it later" —
and one of them now silently defeats a fix that DID land.**

| # | Fix | Corpus verdict | FRESH live status (this pass) | Cite (main) |
|---|---|---|---|---|
| 1 | **NaN `is_finite` at `spectral_radius`** | ADOPT, "4th independent confirmation" (R1 §2.1); Pattern A headline (cross-repo §1) | **STILL LIVE — NOT landed** | `kernel/src/spectral.rs:218` still `eigenvalues(a).iter().map(\|e\| e.abs()).fold(0.0, f64::max)` — no `is_finite` guard |
| 2 | normalize-before-hash (W1-L10 / P-B Fix 2) | ADOPT (P-B §3) | **LANDED** | `spectral_cache.rs:213` `slem_cached(&mut …, tile: &NormalizedTile)`; `matrix_content_address` demoted private `:103`; `NormalizedTile`/`TileAddress`/`canonical_content_address` present; commits `7f2fc6880`/`fc330a622` |
| 3 | event_log exactly-once (`append_raw`, W1-L2 / P-B Fix 1) | corpus says "STILL LIVE / not merged here" (P-B §1 G1) | **LANDED — corpus is stale** | `event_log.rs:389` `commit_after_decide` persists via `append_raw`; regression test present `:679` `commit_after_decide_replay_on_nonempty_log_is_true_duplicate`. This is exactly cross-repo Pattern C4 (drift the *opposite* way — claimed-open, actually fixed) |
| 4 | householder `eig2x2` dedup (W1-L5 / A4) | ADOPT (P-A A4) | **LANDED** | `householder.rs:190` `fn eig2x2(...)`; bit-capture oracle `:506`; commit `989f70837` |
| 5 | `spectral_radius()` → proven const ρ=0 (W1-L4 / A7) | ADOPT (P-A A7) | **LANDED** | `order_machine.rs:334` `FSM_SPECTRAL_RADIUS = 0.0`; `:342` returns const; oracle retained `:985`; commit `eee1fe5a0` |
| 6 | hydra hysteresis band (W1-L3 / P-C) | ADOPT (P-C; item 167) | **LANDED** | `hydra.rs:85` `HysteresisBand`, `:94` `INTEGRITY_BAND`, `:220` `integrity_check` two-threshold + `!rho.is_finite()` guard `:223`; commit `a50d44ab0` |
| 7 | `ci-no-courier-scoring.sh` `trust_weight` gap | fix named (R1 §2.2) | **STILL LIVE — NOT fixed** | `/root/bebop-repo/scripts/ci-no-courier-scoring.sh:22` token list is `score\|rating\|reputation\|rank\|trust_score\|trust_level\|courier_score\|agent_rating`. Live regex test this pass: `pub trust_weight: f32` and `pub integrity_score: f32` both evade; only `pub score: u32` matches |
| 8 | money-law authority flip (A3 / R-4 / W4-L1) | shadow-only now, flip operator-gated | **CORRECTLY still shadow** (not a gap) | `money.rs`/`eqc_gen.rs` shadow organs + parity pin landed (`b2801d313`); callers untouched — matches design, R-4 gate intact |
| 9 | eqc-rs A1/A2/A6 (Asin/Atan2/DivHalfUp, ema_next organ, CORDIC) | ADOPT (P-A) | **LANDED** | `eqc_gen.rs` present; `householder`/CORDIC digest-pinned; commits `14d3ab6fa`/`d692c59fc` |

**Takeaway:** of the recurring "verdicted-but-unlanded" set, **only #1 (NaN) and #7 (CI gate) remain
live**; everything else the corpus frets over has landed, and the P-B blueprint's own ground-truth
(§1 G1, "exactly-once STILL LIVE") is now stale — a concrete instance of the "citation drift under
concurrent development" the task flagged.

---

## 2. The single most concerning genuinely-NEW architectural gap

**The just-landed `RetainedBase::admit` drift-gate is NaN-fail-open, because the NaN fix (#1) it
structurally depends on did not land with it.** No document in the corpus connects these two.

Fresh trace (all on `main`):
- `RetainedBase::admit` gates by `classify_drift(&raw.to_dense())` and rejects **only**
  `DriftClass::Unstable` (`spectral_cache.rs:267-270`).
- `classify_drift` computes `rho = spectral_radius(a)` and returns `Damped` when `rho < 1.0 - BAND`
  (`spectral.rs:344-346`).
- `spectral_radius` still masks NaN via `fold(0.0, f64::max)` (`spectral.rs:218`) → a NaN spectrum
  reports ρ = 0.0 → `Damped` → **admitted as a healthy retained base.**

So P-B built the type-safe snapshot-admission gate (item #2, landed) exactly as designed, and it is
*silently defeated* by the one upstream primitive left unfixed. P-B §4.2's own anti-vacuity finding
("the gate must run on RAW dynamics, not the normalized always-ρ=1 form") is correct and is honored
— but it did not anticipate the *other* vacuity source: a raw operator whose spectrum is non-finite.
The round-2 corpus lists the NaN fix only as a round-1 carry-over "still owed" (Master §4.6) and
never notes that a round-2/P-B mechanism now depends on it. **This is the highest-leverage
genuinely-new finding: fixing #1 is now a correctness precondition for a gate that already shipped.**

---

## 3. Architectural gaps in the delta/patch design (Fable-E)

Verified against `BLUEPRINT-DELTA-KERNEL-DIFF-TOPOLOGY.md` §8/§10 and the round-2 master.

**(a) Delta SIZE bounds — PARTIAL.** Op *count* is capped (`MAX_PATCH_OPS = 256`, §8:479) and the
read-view is capped (`MAX_STATE_VIEW_BYTES = 64 KiB` placeholder, §8:466). But `PatchOp::Put { value:
Box<[u8]> }` (§8:476) has **no named per-op value-byte bound**, and `LaneFrameHeader.payload_len: u32`
is described as "payload_len cap" (Master §3.2 [B1]) with **no `MAX_LANE_PAYLOAD_BYTES` const defined
anywhere**. 256 ops × arbitrary-size values is bounded only by the outer wire `max_frame_bytes` check
([A6]), which is a network-lane guard, not a lane-payload guard — the adapter-emitted patch surface
inherits no explicit byte budget. Ties directly to the V3 resource-exhaustion class. **Gap: name a
per-op and per-patch byte cap alongside the existing op-count cap.**

**(b) Delta ORDERING at the same `base_epoch` — REAL GAP.** The `base_epoch` gate rejects a patch
"outside the lane's **declared window**" (T9, §10.6:707), but **"declared window" is never defined**
(single epoch? range? monotonic floor? — two references, zero definition, §10.1:555 + §10.6:707).
For two *distinct* patches carrying the *same* valid `base_epoch` from the *same* adapter, the design
gives result-determinism (ABSOLUTE `Put`/`Remove` compose per-key last-writer-wins, §10.2:595) and
replay-idempotency (identical patch → `commit_after_decide` `Duplicate`), but is **silent on
admission**: is the second distinct patch accepted (last-write-wins), rejected (stale after the first
advances the epoch), or merged? Undecided. **Gap: specify the window and the same-base second-patch
rule.**

**(c) Cross-adapter delta CONFLICTS — NOT structurally prevented (verified precisely, not assumed).**
There is **no single-writer-per-lane invariant anywhere** in the delta or CWR blueprints (grep for
"single writer / lane owner / one adapter per lane" → zero hits). Lane-scope is a *per-adapter grant*
enforced per-op (`LaneScopeReject`, §8:484) — it stops an adapter writing *outside* its grant, but
does **not** preclude two different adapters both being granted the same `LaneId` and both `Put`-ing
to the same `(lane, key)`. That resolves to commit-order last-writer-wins — deterministic on one
node, but neither asserted-as-intended nor tested. The CWR "single-stream key `S`" clause
(`AdmittedFrame<S>`/`Predicted<S>`, CWR §:171) prevents cross-stream *fusion*, which is a **different
concern** (estimator input provenance), not lane write-ownership. **Answer to the task's precise
question: the LaneFrameHeader/lane-scope design does NOT structurally prevent cross-adapter overlap;
it bounds each adapter to its granted lanes but says nothing about lane exclusivity. Gap.**

**(d) Delta REPLAY across a Snapshot-Checkpoint boundary — MOSTLY COMPOSES, one documentation gap.**
For the STATELESS-ABSOLUTE default (§10.1) there is no correctness hole: a stale in-flight `Put`
computed against a pre-reset base writes a *well-formed absolute value* → "omission-staleness, never
corruption of named keys" (§10.1:552), bounded and caught by the state-hash heartbeat (§7). §10.4
delivers the golden snapshot into adapter memory / reinstantiates. **But** §10.4 never walks through
the *atomicity* of window-advance vs. a pre-reset in-flight patch — whether the checkpoint advances
the `base_epoch` window past the old epoch so the stale patch is rejected by (b)'s gate, or whether it
could apply against the new snapshot first. For absolute ops the worst case is bounded staleness (not
a correctness bug), so this is a **documentation/spec gap, not a hole** — and it is *contingent on (b)*
being specified first (the window semantics are the mechanism that would discard the stale patch).

---

## 4. Verification CRITICALs vs. the round-2 design

**CSC-LAW / delta admission does NOT prevent the forged-order-total pattern — and honestly cannot;
the connection is a NON-prevention, correctly.** The dowiz worst finding E1 (`apply_event_js` /
`order_from_in` trusts attacker `subtotal`/`total`; `price_trusted:false` set but read by no money
path) lives on the **product client-WASM path** (`wasm.rs:142-161`, `domain.rs:59/184`), verified
still live this pass: `domain.rs` sets `price_trusted` at `:184`/`:237` but `post_earn` (`:86`),
`recompute_total` (`:110`), `apply_event` (`:256`), `apply_event_with_trust` (`:337`, new — only folds
a Kalman `TrustEstimate`, does not gate on `price_trusted`) **none read the flag**. This is a
*different code path* from the adapter→kernel `BridgeResult`/`DeltaPatch` gate the round-2 work
governs. Two honest halves:
- The round-2 **adapter** path *does* structurally block a forged *money* leg: red-line resources are
  un-nameable in bridge scope + un-typeable at commit (sealed `RedLineAdmissible`, Fable-B §2), and
  money is a named LAW-LANE-UNSQUASHABLE lane (Fable-E §10.2). So an adapter cannot emit a money
  `Put`. Good — but E1 is not on that path, so this neither fixes nor worsens it.
- The forged-*total* shape IS precisely the CSC **RC-2-broad residual** the corpus pins open and
  never claims to close (`wrong_content_patch_accepted_in_lane`, B-T4/E-T4). E1's fix is the money
  RED-LINE recompute-from-truth (server `compute_order_total` + hard reject; T2/G11, Layer G) — a
  different control the corpus already routes there. **No forced connection; stated as the honest
  non-prevention it is.**

**The NaN `is_finite` fix is NOT a numbered DoD in any round-2 blueprint's own inventory — CONFIRMED.**
Checked every round-2 DoD table: FEC (§4.1 T1–T8), Containment (§4.2 B-T*/D-T*), Header/lane (§4.3
D-T*), CWR (§4.4), Delta (§4.5 E-T1–T12) — **none** includes a `spectral_radius` NaN test. It appears
only in Master §4.6 "Round-1 action items' tests (unchanged by round 2, still owed)" — i.e. parked as
a carry-over, never adopted as a round-2 blueprint's own falsifier. Given §2 (a landed round-2/P-B
gate now depends on it), **it should be promoted to an owned DoD item of the state/consistency lane
(Layer B), not left in the round-1 "still owed" bucket.**

---

## 5. dowiz kernel vs. bebop2 mesh — delta-format question RESOLVED CLEANLY

They are **different architectural layers that do not need to share a delta format** — verified, not
assumed:
- **bebop2 mesh delta** = `anti_entropy::diff(local, remote) -> SyncPlan` over `(u64 seq, [u8;32]
  hash)` frame-index tuples (`/root/bebop-repo/bebop2/core/src/anti_entropy.rs:75`). It answers
  *"which whole signed frames does the peer still need"* — cross-node frame-set reconciliation,
  shipping opaque ≤1 MiB signed frames. It is **not** a field-level op-list.
- **dowiz kernel delta** = `event_log.rs` content-addressed `MeshEvent` intents + the *proposed*
  `DeltaPatch`/`PatchOp` op-list (Fable-E, **NEW, unbuilt** — `grep DeltaPatch\|PatchOp` across
  bebop-repo and dowiz kernel = zero hits this pass). It is an *intra-node adapter→kernel* field-level
  patch against named state targets.
- The dowiz kernel has **no `anti_entropy` module** (confirmed absent) and bebop2 has **no
  DeltaPatch** — a `DeltaPatch` would be the *content* of one signed frame's opaque payload, which
  bebop2's `diff` reconciles as an opaque unit. **They compose by nesting, not by sharing a schema.**

**No operator input needed on the delta-format question.** One minor reconciliation gap worth a line:
Fable-E introduces `DeltaPatch`/`PatchOp` as "NEW-small" without reconciling against ledger **#58
("COO for gossip/patches, CSR for compute") — ALREADY-EQUIVALENT, "no new struct"** (V2 ledger:394).
#58 asserts the existing edge-tuple/COO contract *is* the patch layer; Fable-E adds a new struct
without noting why #58's "no new struct" verdict does not cover it. Not a contradiction (COO edges vs.
`(lane,key,value)` state-ops are genuinely different shapes), but an un-joined citation.

### Other 185-ledger delta-adjacent items never revisited (task point 1)
Checked #47–#77. **#59** (Sparse Delta Updates) → cited by Fable-E K2 (connected). **#57** (Sorted-COO
for cross-node hashing) → landed as P-B normalize-before-hash (connected). **#72** (delta/relative
*index* encoding) → a `col_idx` compression, a *different* "delta", correctly not connected — **no
gap**. **#106/#154** (dirty-bit / CoW page snapshot) → Fable-E explicitly leaves "#44/#107/#110/#154
defers untouched" (accounted). **Only #58 is genuinely un-joined** (above).

---

## 6. Summary of findings

1. **Two verdicted fixes still live** (§1): NaN `is_finite` at `spectral.rs:218`, and the
   `ci-no-courier-scoring.sh` `trust_weight`/`integrity_score` evasion. All other recurring
   "unlanded" items have landed; the P-B blueprint's "exactly-once STILL LIVE" is now stale.
2. **Most concerning new gap** (§2): the landed P-B `RetainedBase::admit` drift-gate is NaN-fail-open
   because fix #1 did not land with it — `admit → classify_drift → spectral_radius` masks a NaN
   spectrum to ρ=0.0=Damped=admitted. No document connects them.
3. **Delta-design gaps** (§3): (b) same-`base_epoch` ordering + undefined "declared window" and
   (c) no single-writer-per-lane invariant (cross-adapter lane overlap not structurally prevented)
   are real, un-specified. (a) per-op/per-patch byte cap is unnamed (only op-count capped).
   (d) snapshot-boundary replay is bounded-safe for absolute ops but under-documented, contingent on (b).
4. **CSC/forged-total** (§4): honest non-prevention — E1 is a separate product-WASM path (still live),
   is the RC-2-broad residual the corpus already pins open; needs the money-recompute control, not
   delta work. The NaN test is confirmed absent from every round-2 blueprint's own DoD (only in the
   round-1 "still owed" bucket) — should be promoted to an owned Layer-B DoD.
5. **dowiz/bebop2 delta-format** (§5): **resolved cleanly, no operator input needed** — different
   layers (frame-set reconciliation vs. intra-node op-list), compose by nesting; only #58 is an
   un-joined citation.
