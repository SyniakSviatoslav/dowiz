# ROADMAP UPDATE — Session Synthesis (2026-07-18)

> **What this is.** A single reviewable patch document consolidating this session's research/plans into
> the standing roadmap, produced from the isolated worktree `/root/dowiz-verify-redteam` (branch
> `research/dowiz-verify-redteam-2026-07-17`, HEAD `4956faca3`). **No code, no commits** ("поки жодних
> комітів" standing). It does **not** edit the live target docs — the live `/root/dowiz` checkout has
> its own uncommitted local modifications to `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` from
> concurrent session activity, and editing it here would collide. §6 gives concrete *proposed* diffs
> the operator applies by hand.
>
> **Ground-truth basis.** Every status claim below traces to a doc read fresh this pass:
> `round-2/GAP-AUDIT-KERNEL-DELTA-ARCHITECTURE.md` (the freshest live-verified pass, re-verified against
> `main @ 87da9ccd4`), the four `verification-2026-07-17/` red-team docs (dowiz, agentic-mesh, hermes,
> spectral), the two `repo-maintenance-2026-07-17/` audits, and the two fail-operational syntheses
> (round-1 + round-2 master). Two load-bearing still-live findings were independently re-confirmed via
> `git show main:<file>` this pass (§0).

---

## 0. What changed since GROUND-TRUTH-2026-07-17 — the ground-truth doc is now 41 commits stale

`GROUND-TRUTH-2026-07-17.md` anchors **`origin/main = 9f78b91d5`** with **452 lib + 107 pq tests**.
That anchor is stale:

| Measure | GROUND-TRUTH claim | Fresh this pass | Delta |
|---|---|---|---|
| `origin/main` tip | `9f78b91d5` | **`87da9ccd4`** (gap-audit's live-verified commit) | **+41 commits** |
| This worktree HEAD | — | `4956faca3` | 10 past anchor, **31 behind live main** |
| Lib test count | 452 | **NOT re-counted this pass** | stale — do not trust the 452 figure |

**Why the commit distance matters (operator flagged live core dev is ongoing).** The 41 commits are not
noise — they are the **P-A/P-B/P-C/F/G/H wave landings** (`git log 9f78b91d5..87da9ccd4`): T1–T8 eqc-rs
CORDIC/ema_next organs, the P-B `normalize-before-hash` type invariant **and** the `RetainedBase`
drift-gate (`7f2fc6880`/`fc330a622`), P-C hysteresis (`a50d44ab0`), the DecisionUnit-family/DomainTag
routing (`f96bc7721`), the web-G kernel-export binding, and the P-H chaos harness + A1–A6 adversarial
suite + bench-regression CI gate (`f4802927e`/`a952af354`). Because so many test-adding waves landed,
**the 452 count must be re-run, not trusted** — treat GROUND-TRUTH as a superseded snapshot, and note
that live `/root/dowiz` may already be past `87da9ccd4` (this worktree's `main` ref sees `87da9ccd4`;
concurrent dev could be further).

**Consolidated verdicted-fix status (from GAP-AUDIT §1, re-verified) — the update to GROUND-TRUTH's
implicit "these are still owed" backlog:**

| Fix | Corpus said | FRESH live status @ `87da9ccd4` |
|---|---|---|
| **NaN `is_finite` at `spectral_radius`** | ADOPT, 4× confirmed | **STILL LIVE** — `spectral.rs:218` still `fold(0.0, f64::max)` (re-confirmed via `git show main` this pass) |
| **`ci-no-courier-scoring.sh` `trust_weight` gap** | fix named | **STILL LIVE** — bebop-repo token list still `…trust_score\|trust_level…`; `trust_weight`/`integrity_score` both evade (re-confirmed this pass) |
| event_log exactly-once (`append_raw`) | "STILL LIVE / not merged" | **LANDED** — `event_log.rs:389`; regression test `:679`. Corpus stale. |
| normalize-before-hash (W1-L10 / P-B Fix 2) | ADOPT | **LANDED** — `spectral_cache.rs` `NormalizedTile`/`TileAddress` |
| householder `eig2x2` dedup (A4) | ADOPT | **LANDED** — `householder.rs:190` |
| `spectral_radius()`→const ρ=0 for FSM (A7) | ADOPT | **LANDED** — `order_machine.rs:334` |
| hydra hysteresis band (P-C) | ADOPT | **LANDED** — `hydra.rs:85` `HysteresisBand` (+ `!rho.is_finite()` guard on the *hydra* path only) |
| money-law authority flip (A3/R-4) | shadow-only, gated | **CORRECTLY still shadow** (`b2801d313`); not a gap |
| eqc-rs A1/A2/A6 | ADOPT | **LANDED** — `eqc_gen.rs` present |

**Headline:** of the recurring "verdicted-but-unlanded" set, **only the two NaN/CI-gate items remain
live** — and one of them (NaN) now silently defeats a gate that DID land (§2, URGENT).

---

## 1. Per-arc status + where each now sits in the Layer A–I structure

Each of this session's five arcs, with **current live-status** (not just names) and its Layer slot.
Where an arc already mapped itself into Layer A–I internally, that is stated plainly rather than
re-derived (the corpus is partly self-consistent — see §1.5).

### 1.1 Mesh-masterwork ledger (185 items) — Layer A/B/C/D/E/F/G via **P30**

**Status: already slotted, no re-slotting needed.** The 185-item ledger
(`bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS-V2.md`) is
**P30** in SOVEREIGN §8.12 and is rolled up across Layers A/B/C/D/E/F/G in `CORE-ROADMAP-INDEX.md` §1.
Verdict tally (V2 §B): 14 ADOPT (+4 gated), 10 EXTEND, 17 ALREADY-EQUIVALENT, 16 DEFER-with-trigger, 19
REJECT-on-physics; zero concepts dropped. **Live-status delta since it was written:** its Wave-1
correctness items have largely LANDED (event_log exactly-once, normalize-before-hash, eig2x2, hydra
hysteresis, order_machine ρ=0, eqc-rs organs — all confirmed §0). The **still-open** masterwork items are
the P06-`key_V` signed DecisionUnit import (Layer D), the R-1..R-4 operator rulings (§4), and the
W4 product write-path gated on NOBYPASSRLS. No Layer gap.

### 1.2 Verification / red-team CRITICALs — all repos, with live-status

**Update (2026-07-18, post-regeneration).** The prior pass of this document found the bebop2 cross-repo
master synthesis genuinely missing — confirmed via git archaeology (empty reflog, `fsck --unreachable`,
`rev-list --objects --all` all came up empty) as a real, permanent loss: the original
`/root/bebop2-verify-redteam` worktree vanished and its branch ref never advanced past its base commit,
because the work was never pushed to a remote. It has since been **regenerated from scratch** (fresh
re-audit, not reconstructed from memory) and is now real, reviewed, and pushed:
`/root/bebop2-verify-redteam/docs/verification-2026-07-17/CROSS-REPO-VERIFICATION-MASTER-SYNTHESIS.md`
(branch `research/bebop2-verify-redteam-2026-07-17`, pushed to `openbebop`, remote HEAD `310cf33`,
verified via `git ls-remote`). The bebop2-side corpus is now V1-V9 + a per-repo `VERIFICATION-MASTER-SYNTHESIS.md`,
adversarially self-reviewed (two independent internal review passes caught and the author corrected: a
wrong file path on V2, an inflated "byte-for-byte" wording, a mis-attributed money-law citation on the
cross-repo Pattern-1 counter-example, and — the most consequential correction — **V7 (mobile compile-target
failure) was downgraded from HIGH to LOW-MEDIUM**: no plan actually claims native Android/iOS reach, and
the `compile_error!` on non-wasm32 targets is the *prescribed* fail-closed remediation behavior from
REMEDIATION-BLUEPRINT §3B, not a defect). New: V8 (replay-ledger eviction reopens a replay window) and V9
(a genuine positive finding — the PQ A-from-CBD remediation is real, backed by committed NIST ACVP
vectors). **The "11 HIGH" figure from the pre-regeneration pass is now stale** — re-tally against the live
V1-V9 docs before citing a count.

The four *other* per-repo docs (dowiz, agentic-mesh, spectral-evolution, hermes) remain the authority for
their own repos as before; this update only concerns the bebop2/openbebop leg and the cross-repo
synthesis that aggregates all five.

**dowiz kernel (V1 + V3, pinned at `4956faca3`, still-live on main unless noted):**

| ID | Finding | Priority | Layer |
|---|---|---|---|
| V3 4.1 | **spectral NaN mask → integrity/drift fail-OPEN** (`spectral.rs:218` `fold(0.0,f64::max)`) — the URGENT one, §2 | HIGH | **B** (+C law) |
| V1 #5 | `budget.rs` NaN/negative `estimate` permanently flips degrade-**closed** → degrade-open (no `is_finite`/`>=0` guard) | HIGH | **C/A** |
| V1 #6 | `budget.rs:147,156` `.lock().unwrap()` poison-cascade — hardened in `token_bucket.rs`, **relocated not eliminated** | MED | **C** |
| V3 1.2 / 5.6 | `apply_event_js`/`price_trusted` — **E1 forged-order-total, confirmed STILL LIVE** (flag set, read by no money path) | HIGH | **G** (money recompute) |
| V3 1.3 | `place_order_js` negative qty/unit_price → negative money | HIGH | **G/A** |
| V3 1.4 | `apply_tax` div-by-zero panic at `tax_rate≈-1.0` (all build profiles) | HIGH | **A/G** |
| V3 1.5/1.6 | `estimate_order_total` i64 + `apply_tax` i128 unchecked overflow → fabricated total/tax | HIGH | **A** |
| V3 1.1/1.7/3.3/3.4 | `channel_ledger_js` `Box::leak` OOM; `harmonic_centrality` unbounded n; uncapped `payload`; unbounded log growth | HIGH | **A** (MAX_* caps) / **H** (CI) |
| V3 2.1/2.2 | `append` zero-prev replay double-commit; multi-writer TOCTOU re-runs `decide` (pgrust target) | HIGH | **B** |
| V3 4.2 | jagged matrix → index-OOB panic via `pub` drift-gate | HIGH | **A/B** |
| V3 4.10 | drift-gate `intervention=true` = unauthenticated bool disables the spectrum gate; **no real `fn kill`** exists | HIGH | **C** |
| V3 5.2/5.3 | `CompensatedRefund` reachable without ledger reversal; `compensate` reverses on any legal edge | HIGH/MED | **B/G** |

**agentic-mesh (V1+V3, branch `84a1e272d`, Wave-0 landed `f30189262`) — Layer D/E (cross-cutting arc):**

- **B-3 (HIGH, arc-original, most dangerous):** `RefSigner` is `pub` (not `#[cfg(test)]`-gated),
  trivially forgeable AND **leaks the signer's secret** — observing one anchor-rooted delegation recovers
  the anchor key → unlimited anchored Sybils, no anchor compromise needed.
- **B-1 (HIGH):** nonce eviction half-drop → replay (`admission.rs:243`, verbatim bebop2 clone).
- **B-2 (HIGH):** caller-controlled `now=0` → total expiry bypass.
- **A5 (HIGH, inherited):** unbounded per-anchor Sybil issuance — no `IssuanceBudget`/`RootDelegationPolicy`.
- **A7/B-6 (MED):** red-line gate arming is caller-optional AND inspects the wrong scope field — manifest
  `action_scopes` (money/auth/secret/migration) never pass `RedLinePolicy::check` at admit.
- **B-4 (MED):** `TokenBucket .lock().unwrap()` poison cascade on the admission hot path.
- **Memory-claim corrections (do not carry the old claims forward):** the "0x12 collision found+fixed" is
  **NOT fixed — DEFERRED/UNRATIFIED** (B1 took 0x12 unilaterally, B2 unbuilt); the "B4 SSR-2020 fix
  protects this arc" is **closed by non-existence** (no batch path in-tree); **B2/B3
  WorkReceipt/Settlement/ExposureLedger are NOT built** (blueprint-only — all their claimed properties
  untestable). Survivors: A6 Poly-Network invariant (3-layer test incl. compile-time borrow guard,
  stronger than bebop2), P07 dedup, `MAX_VERIFY_CHAIN_LINKS=16`.

**hermes-agent-kernel-rewrite (V1+V3, `45520a7`) — NO clean Layer A–I home (§1.6):**

- **T1 (MED-HIGH, single most severe):** observed-group-chatter prompt injection — a non-operator's group
  message is stored as a `role:"user"` turn with prompt-only isolation; the exact
  fabricated-urgency-exfil class this session encountered has a live analog. Config-gated.
- **T2 (MED):** no inbound rate limit on the Telegram ingest path (authorized-party cost-DoS).
- **Claim corrections:** "governance.sh doesn't call the kernel" is **STALE/FALSE** — no such script
  exists (HK-10 was ported *from* it); the kernel **is** on the live turn-loop (routing + degrade-closed
  verification gate). Kernel prompt-injection is **closed by construction** (only scalars cross the
  boundary). Real residual gap = HK-07/08/10/11 are Rust-only, unwired.

**spectral-evolution (V1+V3, `6bd181a02`) — Layer B/C + P06 (cross-cutting arc):**

- **#4 eigensolve NaN-swallow:** the **same** `spectral.rs:218` `fold(0.0,f64::max)` — a poisoned operator
  reads as *stable*; `eval_loss` coerces non-finite→0.0. This is the third independent arrival at the one
  NaN fix (with dowiz V3 4.1 and the gap-audit drift-gate finding — §2).
- **#2 Lyapunov gate unsound as a primitive:** fail-**OPEN** on NaN (`NaN>tol==false`, no `is_finite`
  guard — authors work around it in the *test*, not the *gate*), per-step-only tol admits unbounded
  cumulative growth, PSD assumed but `w≥0` unenforced. The 12×12 unit-weight *application* survives; the
  exported `noether.rs` primitive does not.
- **#5 E3 Phase-A/Phase-B `key_V` boundary is documentation-only:** zero structural gate; the built
  `SelfAdaptator` auto-applies to a real kernel knob (`KalmanFilter::set_q_scaler`) with no key_V
  precondition — reinforces that P06 is a *structural* gap, not just a docket item.
- Survivors: #1 E1 sign-split (the arc's best artifact, red-provable) and #3 Wilson `0.7575` (bit-exact;
  reframe as "~nominal-coverage Wilson limit," ~2.2 pts above the exact CP floor `0.7354`, **not** a
  guaranteed worst-case floor).

**Cross-repo pattern (load-bearing):** the single `spectral_radius` NaN fold at `spectral.rs:218` is now
the **most-corroborated finding of the session** — it surfaces in dowiz V3 4.1, spectral-evolution V1 #4,
AND the gap-audit's drift-gate chain (§2). One ~5-line fix (`!rho.is_finite() ⇒ Unstable/reject`) closes
all three. → **Layer B, promoted to an owned DoD (§2).**

### 1.3 GitHub-maintenance action items

**dowiz (`GITHUB-MAINTENANCE-AUDIT-dowiz.md`) — Layer H (ops) + Layer I (docs):**
- **0 tags / 0 releases on the remote**; the fine-grained PAT **cannot see the private repo** (broaden
  scope or use a different token — not fixable from inside the audit).
- **61 remote branches, ~39% scratch/bot/backup/snapshot** — archive-then-delete, enable
  `deleteBranchOnMerge`, fix the `plane-maintainer` bot to stop leaving permanent branches.
- **Versioning scheme (recommended):** CalVer `YYYY.MM.PATCH` for the repo/product tag (first =
  `2026.07.0` on `main`) + an **independent in-code `KERNEL_PROTO_VERSION` / `MESH_WIRE_VERSION`** for the
  event-log/wire format — *not* blanket SemVer (no external consumer, `0.y.z` carries no compat promise).
- Seed `CHANGELOG.md` (Keep a Changelog) from wave history; one GitHub Release per CalVer tag.
- **Stale Repowise index in `.claude/CLAUDE.md`** (indexed 2026-06-14, still lists deleted
  `apps/`+`packages/`) — re-index or annotate superseded.
- 6 committed build-output dirs tracked in git (`temp/`, `dogfood-output/`, `graphify-out/`, `qa-shots/`,
  `qa-onboarding-shots/`, `playwright-report/`) — `git rm -r --cached`.
- Finish the CORE-ROADMAP consolidation (SUPERSEDED banners; one-page `docs/ARCHITECTURE.md`).

**hermes (`GITHUB-MAINTENANCE-AUDIT-hermes.md`) — NO Layer home (separate repo):**
- **The governing finding: there is NO operator-owned GitHub destination.** 16 unpushed commits
  (`feat/kernel-rust-rewrite`, the HK-00..06 Rust kernel) live **only on this disk**; `origin` is
  `NousResearch/hermes-agent` (READ-only, MIT, pushed *today*). → **H-0 operator decision (§4).**

### 1.4 Local-LLM / Mistral audit (`LOCAL-LLM-AGENTIC-INFRA-MISTRAL-AUDIT.md`) — Layer F (confirmation)

**Status: confirms Layer F's existing verdicts, no new work.** Ollama daemon is live (4 models pulled,
CPU-only, 30 GB RAM). **Mistral/Mixtral = ZERO code** (8 doc/skill mentions only). **Do NOT pull Mixtral
8×7B** — the host is **memory-bandwidth-bound** (`llama3.1:8b` ~9.2–10 tok/s flat across 1/2/4 concurrent),
MoE saves FLOPs not bandwidth (~13B active params still stream from RAM → *slower* per token), and Q4 fit
is hostile (~26–28 GB vs ~26 GB free). Keep the dense-model + typed-fallback stack; spend on the
answer-cache/build-time-compile path instead. The `LlmBackend`/`OllamaAdapter` stack builds + passes a
real non-mocked roundtrip against the live daemon (12 unit + 3 integration green). **MEMORY.md has 1
broken link** (`UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-2026-07-11.md` → repoint to the `-v3-` variant).

### 1.5 Fail-operational round-1 + round-2 — Layer B/C/D/E (already self-mapped)

**Status: already slotted itself into Layer A–I — the corpus is self-consistent here.** Round-1's
`BLUEPRINT-FAIL-OPERATIONAL-LAYOUT-VERSIONING-SYNTHESIS.md` **§6 is an explicit Layer A–I mapping table**
(NaN fix → Layer B applying Layer C's value-bound law; CI gate → Layer H enforcing Layer D doctrine;
UT-LAW → Layer B/C; zero-copy shims → Layer A/B; telemetry-tier → Layer D; FEC/RaptorQ → Layer E; G11
remediation → Layer G). Round-2's master synthesis carries this forward with the concrete
`LaneFrameHeader` build artifact and the same Layer placements:

- **Fable-A (FEC):** ADOPT-NOW on L1 QUIC-datagram / L2 BPv7-shard / L3 future-carrier lanes only
  (`reed-solomon-simd 3.1`, DECART done); never on ARQ streams, never an authenticity control → **Layer E**.
- **Fable-B (CSC-LAW / Pattern C′):** contained self-certifying bridge, fail-operational lanes only,
  **never red-line**; RC-2-narrow closed by construction, **RC-2-broad residual pinned open** (B-T4) →
  **Layer B/C**.
- **Fable-C (CWR):** confidence-weighted reconciliation, ADOPT-NOW infrastructure half (Kalman fusion of
  one admitted stream + kernel prediction, `ema_next` first consumer); `ConfidenceLevel`-as-wire-field
  REJECTED → **Layer D** (CapabilityClass envelopes) + **B** (kalman).
- **Fable-D (MMU / LaneFrameHeader):** MMU scheme ADOPT-EQUIVALENT (WASM SFI on phones, microVM tier on
  hub); the 32-byte `LaneFrameHeader` is the concrete build artifact; no Confidence, no CRC → **Layer E/B**.
- **Fable-E (DeltaPatch):** delta-kernel topology ADOPT-EQUIVALENT (K1/K2 already built); `DeltaPatch`/
  `PatchOp` + ABSOLUTE-OP LAW is NEW-small (ADOPT); DELTA-DETERMINISM LAW → **Layer B**. See §5 for the
  #58 reconciliation.

**The only missing step is a pointer FROM the canonical roadmap/index TO these docs** — round-1/round-2
did the Layer-mapping internally but SOVEREIGN and CORE-ROADMAP-INDEX do not yet reference them. That
pointer is the §6.1 / §6.2 proposed diff. No redundant re-mapping is done here.

### 1.6 Genuine structural-fit check (task's explicit question)

**Items with NO clean home in the Layer A–I structure:** the **hermes product-surface findings** (T1
prompt-injection, T2 rate limit) and the **hermes GitHub-home decision (H-0)**. This is **not a Layer A–I
gap to fix** — Layer A–I is by design the *dowiz/bebop2 kernel-product* altitude axis; hermes is a
separate repo included in scope only "where cited" (CORE-ROADMAP-STANDARD §1). Its findings correctly live
in the existing MEMORY arc `hk05-hk09-routing-status-2026-07-16.md`, not in Layer A–I. **Smallest honest
structural addition: none** — a one-line pointer in `CORE-ROADMAP-INDEX.md` §7 (cross-cutting arcs) noting
hermes is tracked out-of-band + the H-0 decision, rather than inventing a Layer. Everything else this
session (verification CRITICALs, GitHub-dowiz, local-LLM, fail-operational R1+R2) maps cleanly onto an
existing Layer, as shown above. **No new Layer is warranted.**

---

## 2. 🚨 URGENT — blocks already-shipped code (NOT routine backlog)

> This subsection is deliberately distinct from the §1 backlog because it is **a live correctness gap in
> code already on `main`**, not technical debt owed against a future build.

**The just-landed P-B `RetainedBase::admit` drift-gate is NaN-fail-open, because the NaN fix it
structurally depends on did not land with it.**

Fresh trace, all on `main @ 87da9ccd4` (re-verified via `git show` this pass):

1. `RetainedBase::admit` (`spectral_cache.rs:267`) rejects **only** `DriftClass::Unstable` via
   `classify_drift(&raw.to_dense())`.
2. `classify_drift` returns `Damped` when `rho = spectral_radius(a) < 1.0 - BAND`.
3. `spectral_radius` (`spectral.rs:218`) still masks NaN via `fold(0.0, f64::max)` → a NaN spectrum
   reports **ρ = 0.0 → `Damped` → admitted as a healthy retained base.**

P-B built the type-safe snapshot-admission gate exactly as designed (item landed `7f2fc6880`), and it is
**silently defeated** by the one upstream primitive left unfixed. No document in the corpus connected
these two before the gap-audit — and the round-2 corpus lists the NaN fix only as a round-1 carry-over
"still owed" (round-2 §4.6/§5.2), never noting a **round-2/P-B mechanism now depends on it**.

**Why this is now a precondition, not a nice-to-have:** fixing the NaN fold is a **correctness
precondition for a gate that already shipped**. It is also the session's most-corroborated finding
(dowiz V3 4.1 + spectral V1 #4 + this drift-gate chain — §1.2), and it is confirmed **absent from every
round-2 blueprint's own DoD** (only in the round-1 "still owed" bucket). **Action:** promote the
NaN-bearing-tile RED→GREEN test to an **owned Layer-B DoD item** (state/consistency lane), applying Layer
C's value-bound/self-termination law, and flag it as blocking/urgent in the roadmap — not filed as
routine. One ~5-line fix (`if !rho.is_finite() { /* Unstable / reject */ }`, mirroring the guard already
present on the `hydra.rs` path) closes it. RED-first test → `REGRESSION-LEDGER.md`.

---

## 3. Secondary delta-design gaps (GAP-AUDIT §3 — real, unspecified, not urgent)

Slotted under **Layer B** (delta/state consistency, Fable-E's `DeltaPatch`):

- **(b) same-`base_epoch` ordering + undefined "declared window":** the `base_epoch` gate rejects a patch
  "outside the lane's declared window" but "declared window" is never defined (single epoch? range?
  monotonic floor?). For two distinct patches with the same valid `base_epoch` from the same adapter,
  admission is **unspecified** (last-write-wins vs reject-second vs merge) — real gap.
- **(c) no single-writer-per-lane invariant:** grep for "single writer / lane owner / one adapter per
  lane" → zero hits. Lane-scope is a *per-adapter grant* (stops writing *outside* the grant) but does
  **not** preclude two adapters both granted the same `LaneId` both `Put`-ing to the same `(lane, key)`.
  The LaneFrameHeader/lane-scope design does **not** structurally prevent cross-adapter overlap — it
  bounds each adapter to its lanes but says nothing about lane exclusivity.
- **(a) per-op/per-patch byte cap unnamed:** op *count* is capped (`MAX_PATCH_OPS=256`) but
  `PatchOp::Put{value}` has no named per-op byte bound and no `MAX_LANE_PAYLOAD_BYTES` const exists —
  bounded only by the outer wire `max_frame_bytes` (a network-lane guard, not a lane-payload guard).
- **(d) snapshot-boundary replay:** bounded-safe for absolute ops (worst case = omission-staleness, not
  corruption) but under-documented and **contingent on (b)** being specified first.

---

## 4. Still awaiting operator decision — ONE consolidated docket (pointers only, not re-litigated)

Everything proposed this session but not yet ruled on, in one place:

| # | Open item | Where it lives | Blocks |
|---|---|---|---|
| **H-0** | **hermes GitHub home** — 16 unpushed commits live only on disk. **Immediate stopgap: `git bundle` backup off the single disk.** Durable: Path A `gh repo fork` to operator account (recommended; retains MIT link) vs Path B independent repo | `GITHUB-MAINTENANCE-AUDIT-hermes.md` §6 | any tagging/versioning of the rewrite; disk-loss risk NOW |
| **GH-tag** | **dowiz CalVer/SemVer tagging** — cut `2026.07.0` on `main`; adopt CalVer `YYYY.MM.PATCH` + independent in-code `KERNEL_PROTO_VERSION`/`MESH_WIRE_VERSION`; seed `CHANGELOG.md`; branch cleanup + `deleteBranchOnMerge`; broaden PAT scope; `git rm --cached` the 6 output dirs | `GITHUB-MAINTENANCE-AUDIT-dowiz.md` §2/§3/§6 | GitHub discoverability; no release history exists today |
| **RC-2-broad** | **residual acceptance** — a byte-perfect, canonical, in-bounds translation/patch of *wrong content* is information-theoretically undetectable by any type/structural mechanism; the design **bounds and makes visible** but cannot close it. Pinned open by executable test B-T4/E-T4 (one pin, two surfaces). Closure only via round-trip witness (bijective) or N-version (lossy) — both DEFER-WITH-TRIGGER | round-2 §5.1; Fable-B §3–§4; Fable-E §6 | acceptance ruling only — nothing builds around it |
| **NOBYPASSRLS** | **flag flip** — a SEPARATE parallel workstream, never folded into P30; hard-gates only the W4 product T4 write-path lane | `docs/ops/P8-NOBYPASSRLS-FLAG.md`; SOVEREIGN §8.12 (P30) | W4 product write-path only |
| **P06 `key_V`** | **signed done-gate** — `v1.rs` contract executable + tested (SOVEREIGN §8.4), but the real Ed25519/ML-DSA-65 verify-only impl behind it is still owed (behind Phase-3 C4b `mod_l`). `digest32` is a placeholder. Spectral V1 #5 confirms the Phase-A/Phase-B boundary is *documentation-only* until this lands | SOVEREIGN §8.4; INDEX §1 cross-cutting blocker | Layer C independent-verify · Layer G product-safety · E3-Phase-B · P30 signed DecisionUnit import |

**Cross-reference (not re-enumerated):** the pre-existing dockets remain open and are the authority for
their own items — SOVEREIGN §3 (O1/O3/O4/O5/O7/O9/O18b/O20), the P30 R-series (R-1 `0x12→0x13`
discriminant — reconfirmed UNRATIFIED by agentic-mesh V1 A3; R-2 budget-unit; R-3 `RootDelegationPolicy`;
R-4 money-flip), and the agentic-mesh canon-diffs CD-1..8. This docket adds the five session-new items
above; it does not replace those.

---

## 5. Loose-citation closure — Fable-E `DeltaPatch` vs ledger #58

**Ledger #58 verbatim** (`…MASTERWORK-SYNTHESIS-V2.md:394`):
> `| 58 | COO for gossip/patches, CSR for compute | **ALREADY-EQUIVALENT** | the edge-tuple contract IS the COO layer; from_edges→CSR; no new struct. B1 A1 | — |`

**Fable-E introduces:** `DeltaPatch { base_epoch, ops }`, `PatchOp::{Put, Remove}` (ABSOLUTE-OP LAW, no
increment arm), `MAX_PATCH_OPS=256` — verdicted by Fable-E itself as **NEW-small (ADOPT)**.

**Verdict: NO contradiction; #58 does NOT cover this layer; `DeltaPatch` is genuinely new-and-authorized,
NOT a rename of #58's "COO for patches."** The reasoning:

- **#58 governs the graph-edge sparse-matrix representation** — `(row, col, weight)` edge tuples that feed
  spectral compute and cross-node hashing (COO → `from_edges` → CSR). Its "no new struct" verdict is about
  the *tensor/edge* layer: the existing edge-tuple contract already IS the COO form.
- **Fable-E's `DeltaPatch`/`PatchOp` is a different data shape at a different layer** — a lane-scoped
  op-list of ABSOLUTE `(lane, key, value)` **state-mutation** operations emitted by an adapter against
  named kernel-state targets, with per-op lane-scope checking and all-or-nothing application. It is not a
  sparse-matrix of graph edges; it is an adapter→kernel write vocabulary.
- These are **complementary and non-overlapping.** #58's "no new struct" reaches the edge/tensor COO/CSR
  representation and stops there; it never reaches the state-patch op-list. So `DeltaPatch` is not the
  "COO for patches" concept under a different name — the word "patches" in #58 refers to gossip/anti-entropy
  *frame-set* reconciliation over edge tuples, a different "patch" than Fable-E's field-level op-list.

**Recorded disposition:** the un-joined citation is **closed as complementary, not equivalent.**
`DeltaPatch` is a legitimately new small struct that #58's verdict does not govern; #58 stands unchanged
for the edge/tensor layer. (This matches the gap-audit §5's own read: "genuinely different shapes… an
un-joined citation," now joined.) Separately confirmed non-issue: dowiz `DeltaPatch` vs bebop2
`anti_entropy::diff` are different architectural layers (intra-node adapter→kernel op-list vs
whole-signed-frame reconciliation) that compose by nesting — no shared-schema work, no operator input
needed.

---

## 6. Proposed diffs for the two live target documents (operator applies — NOT applied here)

> Reviewable insertions with exact anchor locations. **Not applied** — matching how the rest of this
> session's fail-operational/round-2 work was handled (proposal docs, not direct edits to shared/live
> files), and because live `/root/dowiz` has concurrent uncommitted edits to SOVEREIGN.

### 6.1 `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`

**Insertion A — new subsection, appended after §8.12 (Phase 30), before §9:**

```markdown
### 8.13 Session live-status refresh + verification/red-team + fail-operational (2026-07-18)

Status-only pass (no new phase). Full detail: `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md`.

- **GROUND-TRUTH is 41 commits stale.** `GROUND-TRUTH-2026-07-17.md`'s `origin/main=9f78b91d5` / 452
  tests is superseded by `main @ 87da9ccd4` (+41 commits: P-A T1–T8, P-B normalize-hash + drift-gate,
  P-C hysteresis, F DecisionUnit-family, web-G binding, P-H chaos+A1–A6+bench-gate). Test count must be
  re-run, not trusted at 452.
- **Verdicted-fix landings:** event_log exactly-once, normalize-before-hash, eig2x2, order_machine ρ=0,
  hydra hysteresis, eqc-rs A1/A2/A6 — ALL LANDED. Only two verdicted fixes remain live: the
  `spectral_radius` NaN fold (§8.13 URGENT below) and `ci-no-courier-scoring.sh trust_weight`.
- **🚨 URGENT / blocks shipped code:** the landed P-B `RetainedBase::admit` drift-gate is NaN-fail-open —
  `admit → classify_drift → spectral_radius` masks a NaN spectrum to ρ=0.0=Damped=admitted
  (`spectral_cache.rs:267` → `spectral.rs:218`). Fixing the NaN fold is now a **correctness precondition
  for a gate already on `main`**, not backlog. Most-corroborated finding of the session (dowiz V3 4.1 +
  spectral V1 #4 + this chain). Promote the NaN-bearing-tile RED→GREEN test to an OWNED Layer-B DoD.
- **Verification/red-team CRITICALs** (per-repo, `verification-2026-07-17/`): dowiz (E1 forged-total STILL
  LIVE → Layer G money-recompute; budget.rs NaN degrade-open → Layer C; money-arith overflow/panic
  cluster → Layer A), agentic-mesh (B-3 shippable secret-leaking RefSigner; A5 unbounded Sybil; 0x12
  UNRATIFIED, B2/B3 unbuilt → Layer D/E), hermes (T1 group-chatter prompt-injection — separate repo, no
  Layer home), spectral (Lyapunov primitive fail-open; E3 key_V boundary doc-only).
- **Fail-operational round-1+round-2** (FEC/CSC-LAW/CWR/LaneFrameHeader/DeltaPatch) already self-mapped to
  Layers B/C/D/E; RC-2-broad residual pinned open (B-T4/E-T4). Ledger #58 reconciled: `DeltaPatch` is
  new-and-authorized, complementary to #58's edge-COO layer, not a rename of it.
```

**Insertion B — new rows appended to the §3 operator-decision table (or §8.2's docket):**

```markdown
| H-0 | **hermes GitHub home** — 16 unpushed commits disk-only; stopgap `git bundle`, durable fork (Path A) vs independent repo (Path B) | rewrite versioning; disk-loss risk NOW | recommended Path A (fork, retains MIT link) |
| GH-tag | **dowiz CalVer tagging** — `2026.07.0` on main + in-code KERNEL_PROTO/MESH_WIRE version; CHANGELOG; branch cleanup; PAT scope | GitHub discoverability | no release history exists today |
| RC-2-broad | **residual acceptance** — well-formed-wrong translation/patch undetectable; bounded+visible, not closed (B-T4/E-T4) | acceptance ruling only | closure only via witness/N-version (both DEFER) |
```

### 6.2 `docs/design/CORE-ROADMAP-STANDARD-2026-07-17.md`

**Insertion C — a live-status note appended after the "Correction (2026-07-17, P-D audit…)" paragraph at
the end of §3, mirroring that note's pattern:**

```markdown
**Live-status note (2026-07-18, session verification pass).** Since this structure was written, the
Layer A–I work has advanced on `main` (now `87da9ccd4`, +41 commits vs GROUND-TRUTH's `9f78b91d5`):
Layer A (eqc-rs T1–T8 organs), Layer B (normalize-before-hash + RetainedBase drift-gate), Layer C
(hydra hysteresis), Layer F (DecisionUnit-family routing), Layer H (chaos + A1–A6 + bench-gate) have
landed. Two verdicted fixes remain live and are added to their Layer DoDs:
- **Layer B (URGENT, precondition for shipped code):** `spectral_radius` NaN fold (`spectral.rs:218`)
  defeats the landed `RetainedBase::admit` drift-gate — NaN→ρ=0.0→Damped→admitted. Owned Layer-B DoD:
  NaN-bearing-tile RED→GREEN, applying Layer C's value-bound law. Most-corroborated finding of the
  session (dowiz V3 4.1 + spectral V1 #4 + drift-gate chain).
- **Layer C:** `budget.rs` NaN/negative `estimate` flips degrade-closed→degrade-open (no `is_finite`/`≥0`
  guard) + relocated `.lock().unwrap()` poison-cascade; `noether.rs` Lyapunov primitive fail-open on NaN.
- **Layer F confirmed (no new work):** Mistral/Mixtral = zero code; do NOT pull Mixtral 8×7B
  (bandwidth-bound host, hostile RAM fit); keep the dense-model + typed-fallback Ollama stack.
- **Layer H:** GitHub hygiene (CalVer tag `2026.07.0`, in-code KERNEL_PROTO/MESH_WIRE version, branch
  cleanup, `git rm --cached` 6 output dirs, stale Repowise index in `.claude/CLAUDE.md`).
Full detail + proposed SOVEREIGN diffs: `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md`.
```

### 6.3 (Companion, optional) `docs/design/CORE-ROADMAP-INDEX.md`

**Insertion D — one row appended to §7 (cross-cutting arcs), so the fail-operational + hermes work is
reachable in ≤2 hops (the index's own maintenance rule):**

```markdown
| Fail-operational / layout-versioning (R1 + round-2 FEC/CSC-LAW/CWR/LaneFrameHeader/DeltaPatch) | [fail-operational-layout-versioning-2026-07-17/round-2/BLUEPRINT-ROUND-2-MASTER-SYNTHESIS.md](fail-operational-layout-versioning-2026-07-17/round-2/BLUEPRINT-ROUND-2-MASTER-SYNTHESIS.md) | Self-mapped to Layers B/C/D/E; RC-2-broad residual open (B-T4/E-T4); NaN fix owed to Layer B |
| hermes-agent-kernel-rewrite (dev-tooling, separate repo) | MEMORY `hk05-hk09-routing-status-2026-07-16.md` | Out of dowiz/bebop2 Layer scope by design; H-0 GitHub-home decision open; T1 prompt-injection MED-HIGH |
```

---

## 7. Honesty notes

- **Superseded (2026-07-18):** the bebop2 `CROSS-REPO-VERIFICATION-MASTER-SYNTHESIS.md`, flagged in the
  original pass of this document as not physically present anywhere, was a genuine, permanent loss
  (confirmed via git archaeology, not a hasty guess) and has since been regenerated fresh, adversarially
  self-reviewed, and pushed — see §1.2.
- Test count for `main @ 87da9ccd4` was **not re-run** here (no build in this doc pass) — stated as stale,
  not guessed.
- The corpus was found **partly self-consistent**: the fail-operational round-1 §6 and round-2 already
  mapped their actionables into Layer A–I, and the mesh-masterwork is already P30/INDEX-mapped — so §1.1
  and §1.5 report existing slotting rather than manufacturing redundant re-mapping.
- No code, no commits, no edits to the live target docs — per standing "поки жодних комітів" and worktree
  isolation.
```
