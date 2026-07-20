# dowiz — Chronological Master Roadmap

**This is the single canonical entry point for "what happened, when, and what's the status."**
Every other roadmap/ground-truth/master-plan document in `docs/design/` is now either (a) folded
into this one, or (b) explicitly marked historical below (§8) — read this file first, always.

> **Why this file exists.** By 2026-07-20 the project had accumulated 19 separate top-level
> "roadmap"-shaped documents (`GROUND-TRUTH-*`, `MASTER-ROADMAP-*`, `MASTER-*-PLAN`,
> `ROADMAP-*`) plus `CORE-ROADMAP-INDEX.md` and a 78-item space-grade track, several
> superseded-but-unmarked, dated close together, easy to confuse. This document replaces the
> need to read any of them to answer "where does the project stand." It does **not** replace
> `CORE-ROADMAP-INDEX.md`'s job as the detailed cross-reference table (§0–§10 there, ~124 rows,
> every blueprint linked) or the individual blueprint files (~93 files under
> `CORE-ROADMAP-2026-07-17/` + ~50 under the space-grade track) — those stay as the *detail
> layer*. This document is the *narrative spine*: read top to bottom once, chronologically, to
> understand the whole arc; then drop into `CORE-ROADMAP-INDEX.md` or a specific blueprint when
> you need depth on one item.

**How to read this:** each dated section is a handful of lines per wave/decision/landing, not a
copy of the underlying docs. Status tags used throughout: **LANDED** (real code, tested, on
`main`) · **PLAN** (design doc only, nothing built) · **RULED** (an operator decision recorded in
`DECISIONS.md`) · **SUPERSEDED** (dead, kept for audit trail only).

---

## 0. Current live status (as of 2026-07-20, verified fresh — not carried forward from an older snapshot)

- `main` HEAD: this document's own commit lineage, origin and local match exactly at push time.
- Kernel tests: **1137 passed / 0 failed / 8 ignored** (default features); **1310 passed / 0
  failed / 9 ignored** (`--features pq`).
- Engine tests: **128 passed / 0 failed**.
- Blueprint coverage: effectively complete — every P01–P96 either has a file or is P84
  (deliberately reserved); the space-grade track's 78 items are covered by its own doc + ~50
  per-item blueprints. See `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md` for the full delta audit.
- Open/pending work: tracked live in §7 below, not scattered across other docs.

## 1. Origin (2026-05-31 → 2026-07-10)

Project initialized 2026-05-31. Early phase (TypeScript/JS frontend + pnpm/turbo stack,
centralized `axum`+`rusqlite` server) — **entirely superseded**: the JS stack was fully deleted
2026-07-15 ("drop js", see `CLAUDE.md`), the centralized server dropped 2026-07-12 (**D1**,
below). Nothing from this era is load-bearing; mentioned only for completeness.

## 2. 2026-07-11 — first structured roadmap

- `ROADMAP-GROUND-TRUTH-2026-07-11.md` — **SUPERSEDED, unmarked until this pass** (see §8.1).
  First real "DONE vs PLANNED" ground-truth doc, spanning both `dowiz` (product) and `bebop`
  (protocol). Established the discipline this whole corpus still follows: verify against live
  disk, don't trust the brief.
- `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md` — **SUPERSEDED** (marked at the time, 07-17).
  Tier-0..5 spine targeting the since-deleted Node/TS stack.

## 3. 2026-07-12 — the six hard invariants (D0) + the protocol backbone rulings (D1–D9)

`DECISIONS.md` opens here and becomes the authoritative red-line record for the rest of the
project's life. All same-day:

- **D0 — RULED.** The six non-negotiable invariants, outranking all roadmap/feature pressure:
  **decentralized · local-first · post-quantum · crypto · mesh · reliability-over-latency.**
- **D1 — RULED.** Drop the centralized server (`server/`, axum+rusqlite) entirely — peer nodes
  only, no central DB, no Supabase, no Fly.
- **D2 — RULED.** `MANIFESTO.md` + `DECISIONS.md` live at repo root.
- **D3 — RULED.** Transport = DTN/BPv7 (RFC 9171) + QUIC/TCPCLv4 + BIBE custody, PQ envelope at
  the protocol layer regardless of underlay. `libp2p-gossipsub`/Zenoh/Reticulum rejected as
  primary substrate (latency-optimized or non-PQ).
- **D4 — RULED.** Post-quantum is a *protocol* (transit/signatures/at-rest/supply-chain/in-transit
  composed scope), not isolated primitives. Hybrid `X25519+ML-KEM-768` + `ML-DSA-65`.
- **D5 — RULED.** 3 autonomous node roles (owner/courier/customer); NOSTR/ActivityPub/MCP as
  bridges, never core transport, always PQ-enveloped first.
- **D6 — RULED.** Mesh machinery is NOW in-scope (operator override of C8's deferral).
- **D7 — RULED.** Every change ships a RED→GREEN falsifiable assertion — the discipline this
  whole project still runs on.
- **D8 — RULED.** Plan precedence: newest decision outranks older roadmap/blueprint on conflict.
- **D9 — RULED.** ANU QRNG is opt-in enhancement; native OS entropy is the default AND the
  fallback — a node must boot identically online or offline.

## 4. 2026-07-13 → 2026-07-14 — second-generation planning (all later superseded)

- `MASTER-EXECUTION-PLAN-2026-07-13.md` — **SUPERSEDED** (marked at the time). 4-phase altitude
  spine (Земля→ядро→поверхня→платформа), direct ancestor of the later Layer A–I axis.
- `MASTER-INTEGRATION-PLAN-2026-07-14.md` — **SUPERSEDED** (marked at the time). Best-absorbed of
  the early masters — every concrete item later got a named carrier in P01–P30.
- `MASTER-ROADMAP-10-PHASES-2026-07-14.md` — **SUPERSEDED** (marked at the time).
- `ROADMAP-GROUND-TRUTH-2026-07-14.md` (rev 3) — **SUPERSEDED** (marked at the time, by
  `GROUND-TRUTH-2026-07-17.md`).

## 5. 2026-07-16 — the canonical roadmap begins: P01–P30

`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (2873 lines) becomes canonical — "single
source of truth for the PATH," phases P01–P30. **LANDED subset (confirmed on `main` by
2026-07-19's audit, still true today):** P01, P02, P07, P18, P19. Rest of P01–P30 wired through
Wave 2/3. Same week: spectral energy-flow evolution arc (E1–E3) begins — **LANDED**: E1 (Laplacian
parity) + E2 (CLT), `6bd181a02`.

## 6. 2026-07-17 — the Layer A–I execution wave: P31–P46, quality-bar codified

- `CORE-ROADMAP-STANDARD-2026-07-17.md` — **PLAN (standing meta-doc)**, not superseded — codifies
  the operator's quality bar for ALL future planning as a durable constant, not a one-off.
- `GROUND-TRUTH-2026-07-17.md` — **SUPERSEDED, unmarked until this pass** (see §8.1), by
  `GROUND-TRUTH-2026-07-19-FINAL.md`.
- `CORE-ROADMAP-INDEX.md` + `CORE-ROADMAP-2026-07-17/` directory born — the Layer A–I execution
  structure, phases P31–P46, ~93 individual blueprint files. This becomes (and remains) the
  detail-reference layer this document sits above.
- Agentic mesh protocol arc (B1–B4) begins — Wave 0 **LANDED** (AgentBridge, `TokenBucket::release`,
  B4 crypto-forgery fix).
- CORE **~90%** by end of this wave: P31 (S0/S1/S2/S4 DONE), P32a DONE, P33 audit-only.

## 7. 2026-07-18 — R-3 ruling, launch-blocker research, P47–P74, WIRING WAVE, space-grade begins

- **D10 — RULED.** R-3 `RootDelegationPolicy` = Option A (`OperatorSigned` + per-anchor
  `IssuanceBudget`) — sovereign, P06-independent, closes the Batch-7 Sybil residual. Mechanism
  already built in the `bebop-repo` (`e08eb07`); this is a ruling record, not new code.
- `ROADMAP-LIVE-STATUS-2026-07-18.md` — **historical status snapshot**, correcting stale blueprint
  claims by re-verifying against live code (Hermes autopilot pass).
- `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md` — **historical session patch doc** (445 lines,
  produced in an isolated worktree, never edited the live target docs directly).
- `TELEGRAM-ROADMAP-SUMMARY-2026-07-18.md` — **narrow ops note**, not actually a roadmap despite
  the name — 7-message Telegram-delivery formatting summary. See §8.2.
- 5 Opus research passes → Fable synthesis → 2 canon-diffs (P38-rev, P39-rev) + **18 blueprints
  (P57–P74)** across 4 build waves (M1–M4): canvas text input, a11y-mirror, capability-cert chain,
  payment core (P60), notifications, catalog/multi-vendor, shell platform spike, intent/voice,
  dispatch orchestrator, data wallet, hub provisioning/supervisor, storefront checkout, courier
  surface, moderation/blocklist, food-court N-leg checkout.
- **WIRING WAVE — LANDED + PUSHED** (confirmed 2026-07-20 by this session's own verification,
  `17d65f315` is an ancestor of current `origin/main`): previously built-but-unwired CORE surface
  got real callers — P59 cap-verify, P74 moderation, P68 hub_supervisor drive, P66 wallet
  reconnect→outbox fix, P83 span-metrics + a real deadlock root-cause fix, P64 engine_loop
  intent-router, P89 field_modal live, P40 agent-executor proxy wiring native-spa-server to
  agent-loop with the money-law firewall intact.
- `MASTER-ROADMAP-SWARM-SAFETY-TELEMETRY-FIRST-2026-07-19.md` and the space-grade kernel
  architecture synthesis begin (thin coordination layer + the 78-item execution track that
  dominates the next two days — see §9/§10).

## 8. 2026-07-19 — P75–P96, Q-series governance, GAP-A1 closed, space-grade items 1–54

- 18 more blueprints (**P75–P83, P85–P91, P93–P94**) + P92 (M1 combined) written this wave — full
  P01–P96 blueprint coverage reached except deliberately-reserved P84.
- **GAP-A1-DISPOSITION-AUDIT-2026-07-19.md** — the one real gap the day's own gap-audit found
  (un-homed arc-units MESH-14/IP-01..07/09/17/18/21) — **closed same day**, every unit dispositioned.
- `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` — first full-corpus coverage audit: blueprint
  coverage declared effectively complete. Its own "GAP-A1 still open" line went stale within the
  same day (the disposition doc landed in a later commit) — corrected by this session's 2026-07-20
  delta audit.
- **Q-SERIES (governance layer over P75–P96) — Q1-a + Q2-G14 LANDED.** Claim-verification
  checkpoint (`DONE-VERIFIED` ledger status + `verified-by` pointer) + span-p99 telemetry consumer.
- `GROUND-TRUTH-2026-07-19-FINAL.md` — the authoritative state re-baseline of this era: kernel
  894/0 failed (3 ignored), engine 121/0 failed, main HEAD `5a97e1f6f`. Core roadmap declared
  complete on `origin/main` (`d8004a3c7`) at the time.
- `ROADMAP-RECHECK-SESSION-SYNTHESIS-2026-07-19.md` — consolidated "what's missing" recheck; G2–G5
  meta-gap findings all closed same-day.
- `MASTER-SYNTHESIS-CONSISTENCY-TELEMETRY-DIGITAL-TWIN-2026-07-19.md` — **PLAN, explicitly
  operator-gated no-execution-dispatch at the time** — specified space-grade items §K 55–72 (cost
  ledger, digital twin, epistemic-basis retrofits) and §L 73–78 (Governed Self-Evolution — the
  Gate-Root Invariant, red-line registry, change-proposal pipeline, self-healing/self-upgrading).
- **Space-grade items 1–31ish — LANDED** through this day and the item-execution wave that follows
  it directly into 07-20: zero-dep gate (item 1), toolchain-bump gate (item 14), hand-rolled
  logger/FDR replacing `tracing` (items 4+29), `regex` retirement — zero external deps reached
  (item 5), hardening checklist + `hardening-gate` CI (item 6), Kani proofs + native exhaustive
  contracts (item 7, `df92f0c16`+`23f583b3e`), PMU classifier-input stamps (item 27), plus
  verification/audit passes on items 2, 22, 30, 31.
- Item 26 (event-log/FDR/import_unit batching) — **measurement pass CLOSED as measurement-only**
  this day (real numbers: 1,513 ev/s, ~53× throughput available at batch-64, flagged
  BATCH-WORTHY-but-operator-gated) — superseded the next day when the operator authorized it (§10).

## 9. 2026-07-20 — governance rulings (D11–D14), autopilot execution wave, 2 synthesis waves, this audit

The single busiest day in the project's history. In rough order:

- **D11 — RULED.** Governed Self-Evolution (items 73–78) apply-token design: node-local
  human-only apply-token, 2-factor (`capability_cert` + SHA3-based rotating code), 24h default
  pending-TTL, the meta-governance boundary (AI may edit its own governance module under the
  human gate; core kernel authority + the circuit breaker are the one hard, non-negotiable line),
  row-removal via `DECISIONS.md` D-entries, 3-consecutive-window self-heal threshold, item
  77-before-78 sequencing. **Spec-plane only — does not authorize dispatching 73–78 to code.**
- **Space-grade autopilot execution wave — LANDED**, operator authorization: "continue on
  autopilot until all are done... give permission to work on all... if decision is needed, stop &
  ask." `SPACE-GRADE-VERIFIED-STATUS-LEDGER-2026-07-20.md` records real, acceptance-filter-verified
  landings: items 8 (Kani GCRA), 9 (breaker), 20 (P95 persistence), 21 (autonomic), 22–24 (mesh
  pq-signed), **26 (group-commit batching, `85022e49d`, supersedes the 07-19 measurement-only
  close)**, 48 (FDR blind-spot closure), 50 (K3 verdict cause), 54 (Sentinel), 55–56 (K3/epistemic
  retrofits), and more per the ledger's own tables. Separately this session: **cross-mesh
  replication — LANDED** (`kernel/src/mesh_replication.rs`, `307c3ead5`, tracked out-of-band as
  §M, "not one of the original 78 items," not "item 5" as one row briefly mislabeled and this
  audit corrected).
- **KnowledgeSpine wired to the memory/docs corpus — LANDED** (`28faa120d`), 705 files tracked,
  tamper-evidence proven end-to-end.
- **Product-surface wave — PLAN, 5 docs, all decisions RULED same-day (D13/D14):** spatial
  storefront/voice hub, concurrency architecture (**D13 — RULED: "async only where it brings
  value"** — `ToolPort`/`agent-loop`/every kernel port stay synchronous permanently; tokio's only
  future entry is the not-yet-built mesh-adapter layer, gated on a ~1,000–2,000-socket threshold),
  offline resilience, media/comms + granular per-layer agentic autonomy (**D14 — RULED:**
  Service-Worker+IndexedDB doctrine exception ratified, Human-only autonomy default with
  Agent-assisted opt-in, native PQ-hybrid ratchet for customer-leg chat, per-hub-configurable
  relay topology, Owner/Kitchen/Counter-Manager staff roles, local-disk-only media storage), and
  intent interface (corrects a false "one unified wave-equation" claim; `engine/src/intent.rs` +
  `compose_ui.rs` already implement the requested one-screen architecture, just never styled).
- **"Red lines are still only by humans" — RULED**, hardened beyond D14's deny-by-default framing
  to a structural, non-configurable exclusion: agent identities cannot hold Ledger/Auth/Secret/
  Migration capability, no grant path exists at all.
- **Remaining-queue wave — PLAN, 4 docs:** Telegram ops-hub build-order (item 1, `topics
  resources`, already **LANDED** `9f94547ca`); payment-adapter residual (headline finding: the
  online-fiat payment core is already **LANDED** and tested, `kernel/src/ports/payment_provider.rs`
  — residual scope is the out-of-kernel adapter crate + webhook infra + PSP selection);
  omnichannel order-intake (P48-INTAKE, builds the inbound mirror of P43); local-model wiring
  (smallest scope — locks in the existing sync HTTP-client architecture, closes one wiring gap).
- **This document + `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md` — the delta audit and cleanup
  this section is part of.** Fixed 4 orphaned/under-linked docs, 1 dangling link, 2 stale status
  cells, 1 mislabel in `CORE-ROADMAP-INDEX.md`; annotated (not silently fixed) that space-grade
  items 45/73/74 have real standalone CI-gate scripts not yet wired into any workflow; found and
  partially resolved a disk-full incident (99 leftover autopilot-swarm worktrees, 77 removed
  safely, 25GB freed) plus a live git-collision incident (recovered via an isolated worktree,
  Hermes's own concurrent `swave/integrate` work — implementing pieces of this session's own
  blueprints — restored intact); and — the reason this specific document exists — consolidated
  the whole 19-document mess into this one chronological spine, per direct operator instruction.

## 10. The space-grade 78-item track — how it relates to everything above

`SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` (1551 lines) is a **deliberately separate
track** from the P01–P96 CORE roadmap — zero-external-dependency, maximum-kernel-authority,
100%-determinism-where-possible hardening work, not product features. It is NOT a competing
roadmap; think of it as a fourth axis alongside Layer A–I / P-series / space-grade, all converging
on the same `main`. Status as of today: the large majority of items 1–61 have landed real code or
been closed as measurement/audit passes (§8–§9 above); items 62–72 (telemetry/cost-ledger/digital-twin)
and 73–78 (Governed Self-Evolution) remain spec-only pending D11's follow-on code-dispatch
decision. Full per-item detail lives in that doc and its ~50 companion `BLUEPRINT-ITEM-*` files —
not reproduced here.

## 11. Open / pending, right now (single source — don't look elsewhere for this)

- Space-grade items 62–72 and 73–78: **spec exists (D11 rules 73-78's design), no code dispatch
  authorized yet.**
- Space-grade items 45/73/74's CI-gate scripts: **real code on `main`, not wired into any GitHub
  Actions workflow** — a named, scoped follow-up, not yet actioned.
- Product-surface wave + remaining-queue wave: **8 PLAN docs total, zero code written** — Stage A
  of the intent-interface blueprint (wiring already-tested endpoints together) is flagged as the
  lowest-risk next build step if the operator wants to proceed past planning.
- 21 worktrees from the disk-cleanup incident (§9) intentionally left untouched pending manual
  operator review — not required for roadmap completion.
- `payment-adapters` crate, `intake-adapters` crate, and the `AiMode`→`compose.rs` wiring gap: all
  three named concretely in the remaining-queue wave's blueprints — the wiring gap has since
  started landing on the `swave/integrate` branch (see §9), not yet merged to `main`.

## 12. What every filename means now, at a glance

| If you're looking for... | Read |
|---|---|
| "What happened and when" (this doc) | `ROADMAP.md` (here) |
| Full P-number ↔ blueprint-file cross-reference, every arc, every layer | `CORE-ROADMAP-INDEX.md` |
| One phase/item's full design detail | Its own file under `CORE-ROADMAP-2026-07-17/` or the space-grade track |
| Red-line/architecture decisions with full rationale | `DECISIONS.md` (D0–D14) |
| Why the project exists, the six invariants | `MANIFESTO.md`, `DECISIONS.md` D0 |
| The 78-item hardening track specifically | `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` |
| Blueprint-coverage / landed-status audits | `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md` (this pass) → `-2026-07-19.md` (prior pass) |
| Anything else with "ROADMAP"/"GROUND-TRUTH"/"MASTER" in the name | §8 below — it's historical |

---

## 8. Historical / superseded documents — kept for audit trail, never plan against these

All 18 of the following are subsumed by this document + `CORE-ROADMAP-INDEX.md` +
`DECISIONS.md`. None should be read to determine current status.

### 8.1 — Not previously marked superseded (fixed by this pass: banner added to each)

| Doc | Was presenting as | Now marked |
|---|---|---|
| `GROUND-TRUTH-2026-07-17.md` | A live "single source of truth" snapshot | Superseded by `GROUND-TRUTH-2026-07-19-FINAL.md`, itself superseded by §0 of this doc |
| `ROADMAP-GROUND-TRUTH-2026-07-11.md` | "Canonical roadmap" | Superseded by everything after it — the oldest doc in the corpus |

### 8.2 — Already carried a SUPERSEDED banner (unchanged, just indexed here for completeness)

| Doc | Superseded by (per its own banner) |
|---|---|
| `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md` | `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` + `CORE-ROADMAP-INDEX.md` |
| `MASTER-EXECUTION-PLAN-2026-07-13.md` | same |
| `MASTER-INTEGRATION-PLAN-2026-07-14.md` | same |
| `MASTER-ROADMAP-10-PHASES-2026-07-14.md` | same |
| `ROADMAP-GROUND-TRUTH-2026-07-14.md` | `GROUND-TRUTH-2026-07-17.md` (itself now superseded, see 8.1) |

### 8.3 — Narrow-purpose, dated, session-scoped docs (not roadmaps despite the name; never superseded because they were never meant to be re-read as status — kept as-is)

`ROADMAP-LIVE-STATUS-2026-07-18.md`, `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md`,
`ROADMAP-RECHECK-SESSION-SYNTHESIS-2026-07-19.md`, `TELEGRAM-ROADMAP-SUMMARY-2026-07-18.md`,
`MASTER-ROADMAP-SWARM-SAFETY-TELEMETRY-FIRST-2026-07-19.md`,
`MASTER-SYNTHESIS-CONSISTENCY-TELEMETRY-DIGITAL-TWIN-2026-07-19.md`. Each was a one-time session
output (patch proposal, recheck pass, status correction, or a thin sequencing layer over other
docs) — historically interesting, never the thing to consult for "what's the status now." That's
this document, from today onward.

### 8.4 — Still current, not historical, not superseded

`CORE-ROADMAP-STANDARD-2026-07-17.md` (standing quality-bar doctrine, not a status doc),
`MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` (still the detailed P01–P30 reference,
subordinate to this doc for status but not dead), `CORE-ROADMAP-INDEX.md` (the detail
cross-reference layer this doc sits above), `GROUND-TRUTH-2026-07-19-FINAL.md` (superseded for
*live* status by §0 above, but its landing-wave narrative for that specific day remains accurate
history), `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` (the live 78-item track, see §10).

---

*Maintained going forward: every future dated wave gets one new §-numbered entry appended to §9's
successors (or a new §9+n section once 07-20 is no longer "today"), never a new competing
top-level doc. If you're about to create a new `MASTER-*`/`ROADMAP-*`/`GROUND-TRUTH-*` file,
stop — extend this one instead.*
