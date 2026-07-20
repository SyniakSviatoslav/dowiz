# dowiz — Master Roadmap (fully merged, single file)

**This is the one roadmap document in `docs/design/`.** Everything that used to live in 19
separate top-level "roadmap"-shaped files (`GROUND-TRUTH-*`, `MASTER-ROADMAP-*`, `MASTER-*-PLAN`,
`ROADMAP-*`) is now either fully inlined below (Parts II–V) or was genuinely dead weight and has
been deleted outright — git history keeps every deleted file recoverable, `git log --diff-filter=D
-- docs/design/<name>.md` finds the last version of any of them. Nothing outside this file and
`CORE-ROADMAP-INDEX.md` (the still-separate detail cross-reference to the ~140 individual
blueprint files) should be read to determine current roadmap status.

> **Structure.** Part I (this part) is the chronological narrative spine — read it top to bottom
> once. Parts II–V are the full, verbatim content of the four documents that were still current as
> of the merge (2026-07-20): the P01–P30 Sovereign Architecture roadmap, the standing quality-bar
> doctrine, the 2026-07-19 ground-truth re-baseline (historical narrative, superseded for *live*
> status by Part I §0 but kept verbatim for its landing-wave detail), and the 78-item space-grade
> execution track. Their own internal heading levels were demoted by one so they nest correctly
> under this document's top-level headings; their content is otherwise reproduced exactly as
> written, including any of their own internal status/superseded notes.

**How to read Part I:** each dated section is a handful of lines per wave/decision/landing, not a
copy of the underlying docs. Status tags used throughout: **LANDED** (real code, tested, on
`main`) · **PLAN** (design doc only, nothing built) · **RULED** (an operator decision recorded in
`DECISIONS.md`) · **DELETED** (content merged into Parts II–V or fully absorbed into this
narrative; the standalone file no longer exists on disk).

---

## 0. Current live status (as of 2026-07-20, verified fresh — not carried forward from an older snapshot)

- `main` HEAD: this document's own commit lineage, origin and local match exactly at push time.
- Kernel tests: **1137 passed / 0 failed / 8 ignored** (default features); **1310 passed / 0
  failed / 9 ignored** (`--features pq`).
- Engine tests: **128 passed / 0 failed**.
- Blueprint coverage: effectively complete — every P01–P96 either has a file or is P84
  (deliberately reserved); the space-grade track's 78 items are covered by Part V + ~50 per-item
  blueprint files. See `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md` for the full delta audit.
- Open/pending work: tracked live in §11 below, not scattered across other docs.

## 1. Origin (2026-05-31 → 2026-07-10)

Project initialized 2026-05-31. Early phase (TypeScript/JS frontend + pnpm/turbo stack,
centralized `axum`+`rusqlite` server) — **entirely superseded**: the JS stack was fully deleted
2026-07-15 ("drop js", see `CLAUDE.md`), the centralized server dropped 2026-07-12 (**D1**,
below). Nothing from this era is load-bearing; mentioned only for completeness.

## 2. 2026-07-11 — first structured roadmap

- `ROADMAP-GROUND-TRUTH-2026-07-11.md` — **DELETED 2026-07-20** (was superseded, unmarked, since
  2026-07-14). First real "DONE vs PLANNED" ground-truth doc, spanning both `dowiz` (product) and
  `bebop` (protocol). Established the discipline this whole corpus still follows: verify against
  live disk, don't trust the brief.
- `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md` — **DELETED 2026-07-20** (was superseded, marked
  at the time, 07-17). Tier-0..5 spine targeting the since-deleted Node/TS stack.

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

## 4. 2026-07-13 → 2026-07-14 — second-generation planning (all later deleted)

- `MASTER-EXECUTION-PLAN-2026-07-13.md` — **DELETED 2026-07-20** (was superseded, marked at the
  time). 4-phase altitude spine (Земля→ядро→поверхня→платформа), direct ancestor of the later
  Layer A–I axis.
- `MASTER-INTEGRATION-PLAN-2026-07-14.md` — **DELETED 2026-07-20** (was superseded, marked at the
  time). Best-absorbed of the early masters — every concrete item later got a named carrier in
  P01–P30.
- `MASTER-ROADMAP-10-PHASES-2026-07-14.md` — **DELETED 2026-07-20** (was superseded, marked at the
  time).
- `ROADMAP-GROUND-TRUTH-2026-07-14.md` (rev 3) — **DELETED 2026-07-20** (was superseded, marked at
  the time, by `GROUND-TRUTH-2026-07-17.md`).

## 5. 2026-07-16 — the canonical roadmap begins: P01–P30

The Sovereign Architecture roadmap (now Part II below) becomes canonical — "single source of
truth for the PATH," phases P01–P30. **LANDED subset (confirmed on `main` by 2026-07-19's audit,
still true today):** P01, P02, P07, P18, P19. Rest of P01–P30 wired through Wave 2/3. Same week:
spectral energy-flow evolution arc (E1–E3) begins — **LANDED**: E1 (Laplacian parity) + E2 (CLT),
`6bd181a02`.

## 6. 2026-07-17 — the Layer A–I execution wave: P31–P46, quality-bar codified

- The quality-bar doctrine (now Part III below) — **standing meta-doc**, not superseded — codifies
  the operator's quality bar for ALL future planning as a durable constant, not a one-off.
- `GROUND-TRUTH-2026-07-17.md` — **DELETED 2026-07-20** (was superseded, unmarked, since
  2026-07-19), by `GROUND-TRUTH-2026-07-19-FINAL.md` (now Part IV below).
- `CORE-ROADMAP-INDEX.md` + `CORE-ROADMAP-2026-07-17/` directory born — the Layer A–I execution
  structure, phases P31–P46, ~93 individual blueprint files. This remains the one detail-reference
  layer this document sits above (not merged — too large, too many individual files, and it's a
  cross-reference table, not itself a competing roadmap narrative).
- Agentic mesh protocol arc (B1–B4) begins — Wave 0 **LANDED** (AgentBridge, `TokenBucket::release`,
  B4 crypto-forgery fix).
- CORE **~90%** by end of this wave: P31 (S0/S1/S2/S4 DONE), P32a DONE, P33 audit-only.

## 7. 2026-07-18 — R-3 ruling, launch-blocker research, P47–P74, WIRING WAVE, space-grade begins

- **D10 — RULED.** R-3 `RootDelegationPolicy` = Option A (`OperatorSigned` + per-anchor
  `IssuanceBudget`) — sovereign, P06-independent, closes the Batch-7 Sybil residual. Mechanism
  already built in the `bebop-repo` (`e08eb07`); this is a ruling record, not new code.
- `ROADMAP-LIVE-STATUS-2026-07-18.md` — **DELETED 2026-07-20** (historical status snapshot,
  correcting stale blueprint claims by re-verifying against live code — content absorbed into this
  narrative, nothing unique lost).
- `ROADMAP-UPDATE-SESSION-SYNTHESIS-2026-07-18.md` — **DELETED 2026-07-20** (historical session
  patch doc, 445 lines, produced in an isolated worktree, never edited the live target docs
  directly).
- `TELEGRAM-ROADMAP-SUMMARY-2026-07-18.md` — **DELETED 2026-07-20** (narrow ops note, not actually
  a roadmap despite the name — 7-message Telegram-delivery formatting summary).
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
- `MASTER-ROADMAP-SWARM-SAFETY-TELEMETRY-FIRST-2026-07-19.md` — **DELETED 2026-07-20** (thin
  sequencing/gating layer over other docs, no unique content) — and the space-grade kernel
  architecture synthesis begin (the 78-item execution track, now Part V below).

## 8. 2026-07-19 — P75–P96, Q-series governance, GAP-A1 closed, space-grade items 1–54

- 18 more blueprints (**P75–P83, P85–P91, P93–P94**) + P92 (M1 combined) written this wave — full
  P01–P96 blueprint coverage reached except deliberately-reserved P84.
- **GAP-A1-DISPOSITION-AUDIT-2026-07-19.md** — the one real gap the day's own gap-audit found
  (un-homed arc-units MESH-14/IP-01..07/09/17/18/21) — **closed same day**, every unit dispositioned.
- `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-19.md` — first full-corpus coverage audit: blueprint
  coverage declared effectively complete. Its own "GAP-A1 still open" line went stale within the
  same day (the disposition doc landed in a later commit) — corrected by this session's 2026-07-20
  delta audit. (This audit doc is kept standalone — it's an audit report, not a roadmap.)
- **Q-SERIES (governance layer over P75–P96) — Q1-a + Q2-G14 LANDED.** Claim-verification
  checkpoint (`DONE-VERIFIED` ledger status + `verified-by` pointer) + span-p99 telemetry consumer.
- `GROUND-TRUTH-2026-07-19-FINAL.md` — **fully inlined as Part IV below**, no longer a standalone
  file. Authoritative state re-baseline of this era: kernel 894/0 failed (3 ignored), engine 121/0
  failed, main HEAD `5a97e1f6f`. Core roadmap declared complete on `origin/main` (`d8004a3c7`) at
  the time.
- `ROADMAP-RECHECK-SESSION-SYNTHESIS-2026-07-19.md` — **DELETED 2026-07-20** (consolidated "what's
  missing" recheck; G2–G5 meta-gap findings all closed same-day, content absorbed above).
- `MASTER-SYNTHESIS-CONSISTENCY-TELEMETRY-DIGITAL-TWIN-2026-07-19.md` — **DELETED 2026-07-20**
  (was PLAN, explicitly operator-gated no-execution-dispatch at the time — specified space-grade
  items §K 55–72 and §L 73–78, both still tracked live in Part V below, nothing unique lost).
- **Space-grade items 1–31ish — LANDED** through this day and the item-execution wave that follows
  it directly into 07-20: zero-dep gate (item 1), toolchain-bump gate (item 14), hand-rolled
  logger/FDR replacing `tracing` (items 4+29), `regex` retirement — zero external deps reached
  (item 5), hardening checklist + `hardening-gate` CI (item 6), Kani proofs + native exhaustive
  contracts (item 7, `df92f0c16`+`23f583b3e`), PMU classifier-input stamps (item 27), plus
  verification/audit passes on items 2, 22, 30, 31.
- Item 26 (event-log/FDR/import_unit batching) — **measurement pass CLOSED as measurement-only**
  this day (real numbers: 1,513 ev/s, ~53× throughput available at batch-64, flagged
  BATCH-WORTHY-but-operator-gated) — superseded the next day when the operator authorized it (§9).

## 9. 2026-07-20 — governance rulings (D11–D14), autopilot execution wave, 2 synthesis waves, this audit + full merge

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
  §M in Part V, "not one of the original 78 items," not "item 5" as one row briefly mislabeled and
  this audit corrected).
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
- **The delta gap-audit + first-pass consolidation.** Fixed 4 orphaned/under-linked docs, 1
  dangling link, 2 stale status cells, 1 mislabel in `CORE-ROADMAP-INDEX.md`; annotated (not
  silently fixed) that space-grade items 45/73/74 have real standalone CI-gate scripts not yet
  wired into any workflow; found and partially resolved a disk-full incident (99 leftover
  autopilot-swarm worktrees, 77 removed safely, 25GB freed) plus a live git-collision incident
  (recovered via an isolated worktree, Hermes's own concurrent `swave/integrate` work —
  implementing pieces of this session's own blueprints — restored intact). First produced a
  navigation spine over the still-fragmented 19 files; on operator follow-up ("merge all roadmaps
  into one"), fully inlined the 4 still-current docs as Parts II–V below and deleted the 13
  genuinely-dead ones outright — this is that final, fully-merged document.

## 10. The space-grade 78-item track — how it relates to everything above

Part V below (the space-grade 78-item roadmap) is a **deliberately separate track** from the
P01–P96 CORE roadmap — zero-external-dependency, maximum-kernel-authority,
100%-determinism-where-possible hardening work, not product features. It is NOT a competing
roadmap; think of it as a fourth axis alongside Layer A–I / P-series / space-grade, all converging
on the same `main`. Status as of today: the large majority of items 1–61 have landed real code or
been closed as measurement/audit passes (§8–§9 above); items 62–72 (telemetry/cost-ledger/digital-twin)
and 73–78 (Governed Self-Evolution) remain spec-only pending D11's follow-on code-dispatch
decision. Full per-item detail is in Part V and its ~50 companion `BLUEPRINT-ITEM-*` files (kept
separate — that's ~50 individual files, not itself a competing top-level roadmap doc).

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
| "What happened and when" (this doc, Part I) | `ROADMAP.md` §0–§11 (here) |
| What to build next, in what order, citing the real blueprint for each | `ROADMAP.md` §15 (here) |
| Full P-number ↔ blueprint-file cross-reference, every arc, every layer | `CORE-ROADMAP-INDEX.md` |
| One phase/item's full design detail | Its own file under `CORE-ROADMAP-2026-07-17/` or the ~50 space-grade `BLUEPRINT-ITEM-*` files |
| Red-line/architecture decisions with full rationale | `DECISIONS.md` (D0–D14) |
| Why the project exists, the six invariants | `MANIFESTO.md`, `DECISIONS.md` D0 |
| The full P01–P30 Sovereign Architecture detail | Part II below (this file) |
| The standing quality-bar doctrine | Part III below (this file) |
| The 2026-07-19 landing-wave narrative (historical) | Part IV below (this file) |
| The 78-item space-grade track, full detail | Part V below (this file) |
| Blueprint-coverage / landed-status audits | `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md` (this pass) → `-2026-07-19.md` (prior pass) — kept standalone, these are audit reports, not roadmaps |
| Anything else with "ROADMAP"/"GROUND-TRUTH"/"MASTER" in the name | It's gone — deleted 2026-07-20, content is in this file or was genuinely dead. `git log --all --diff-filter=D -- 'docs/design/<name>.md'` recovers it if ever needed. |

---

---

## 13. Verified-status corrections, P-series (2nd-pass audit, 2026-07-20, same day)

Following the full merge, the operator asked for a fresh, evidence-based (not doc-trusted) sweep
of every P-number's actual landed-status. Three parallel passes ran: P01–P96, space-grade 1–78
(corrections folded into Part V directly, see its reconciliation table + the 2nd-pass correction
block after it — items 61/62/66 were found *wrongly marked done* there), and a full
`docs/design/` reachability sweep (§14 below). This section carries the P-series findings; nothing
below duplicates what a prior audit already confirmed accurate — only genuine corrections and
newly-confirmed unresolved items are listed.

**Corrections — a live doc's claimed status did not match `main`'s real content:**

| P-number | Doc claimed | Verified reality | Evidence |
|---|---|---|---|
| **P37** | PLANNED (0% — "no dynamic HTTP server exists in-repo") | **LANDED.** Biggest single stale claim found in the sweep. | Commit `68d5c2874` landed `kernel/src/json_api.rs` + `tools/native-spa-server/src/api.rs` with `/api/order`, `/api/order/{id}`, `/api/order/{id}/advance` genuinely registered in the served binary's router. |
| **P04** | "Nothing has landed yet; `kernel/src/router.rs` and `kernel/src/dsu.rs` do not exist" (blueprint's own completion appendix) | **LANDED.** Both files exist. | Commit `86b1558fa`; doc never updated after the landing. |
| **P95** | Blueprint's own VERDICT: "HOLD — ready design, do NOT build yet," gated on precondition P95-C1 (a real caller must exist first); companion doc reaffirms "NO-GO if unmet." | **Built anyway** (`f16d603d7`, 2026-07-20), with no on-record confirmation the gate was re-checked or an explicit operator override was given. Feature itself is real/tested/on `main` — this is a **process/governance finding, not a code-quality one**: the documented gate appears to have been bypassed, not honored. `CORE-ROADMAP-INDEX.md` §9 still reads "P95 HOLD/NO-GO," now one day stale vs. the actual merge. | Flagged for operator awareness, not silently resolved either direction. |
| **P65** | Presented with equal LANDED weight alongside P66/P67/P68 in the WAVE-CLOSEOUT doc. | **Overstated.** Cited commit `bae2134` sits only on an unmerged bebop-repo branch (`feat/p65-dispatch-orchestrator`), not that repo's own `main`. Built, not merged. | bebop-repo branch state, checked directly. |
| **P28** | `CacheGraph` (`llm-adapters/src/cache_graph.rs`) described as built (operator-directed override of P26's rejects, "planned forward"). | **Does not exist.** Zero `.rs` hits anywhere in the repo for `cache_graph` or `CacheGraph`. | Repo-wide grep, zero matches. |
| **P26** | Two headline "ADOPTED" claims: `kernel/src/memory_budget.rs`/`MemoryBudget`, and `retrieval/ppr.rs` delegating to CSR. | **Neither holds.** `memory_budget.rs` doesn't exist at all; `ppr.rs` is still dense, not CSR-delegated. The one real fix in this phase (`llm-adapters/src/cache.rs::BoundedStore`) is entry-bounded, not byte-bounded as the doc describes. | Direct file/grep check. |
| **P48** | Single cell: "PLANNED — build-out open." | **Flattens two different realities.** The messenger-intake half (H1-H4) genuinely is 0% built (the doc's own ground-truth table already says so honestly). But P48's original CRUD-admin scope has since landed — under a *different* phase number: `kernel/src/ports/owner_surface.rs`'s own comments state it "supersedes P48 B1/B2/B3" — that's really P70's delivery, not left open under P48. | `kernel/src/ports/owner_surface.rs` header comments. |
| **P62** | WAVE-CLOSEOUT cites commit `422b45c95`. | **Typo, cosmetic only.** Not a valid git object — one hex digit off. Real commit is `422b45e95`. Code and its 19 tests are real and green regardless. | `git cat-file` lookup. |
| **P91** | "MERGED to main" (blanket). | **True for P91.0/P91.1 only** (ring-arithmetic fix, real). P91.2 (NIST ACVP KAT + constant-time tag-compare) remains open — `docs/audits/hardening/HOT-PATHS.tsv` itself carries `MISSING`/`KNOWN-RED` flags on `kem.rs`/`hybrid.rs`. The blanket phrasing overstates completeness. | `HOT-PATHS.tsv`'s own flags. |

**Confirmed accurate:** roughly 85 of 95 checkable P-numbers (all except P84, which is
deliberately reserved by design) had their claimed status directly verified against real
files/tests/commits — the large majority of the roadmap's own status claims hold up.

**Genuinely unresolved (honest, not guessed):**
- **P29** — the specific "shape C1 model-tier routing, 30-case fixture" pilot named in this
  document could not be located as a distinct artifact from this repo (general `DecisionUnit`/
  `Stale` infra exists; the named pilot doesn't show up in a targeted grep) — may exist on an
  unmerged branch not checked.
- **P03, P09, P10, P36, P65 (bebop side), P76, P78, P82, P85 (bebop portion), P92, P93, P94** —
  each blueprint states its own files live in the separate `/root/bebop-repo` (OpenBebop)
  repository, not in `dowiz`. Structurally unverifiable from this repo alone; not claimed either
  way here.

## 14. Reachability gaps found in `CORE-ROADMAP-INDEX.md` (2nd-pass sweep, 2026-07-20)

A full sweep of the remaining `docs/design/` + `docs/research/` corpus (~557 files, beyond the
P-series and space-grade tracks already covered above and in Part V) found **no missing
blueprints** — every real, proposed piece of work already has a real, DoD-shaped document
somewhere. What it found instead: `CORE-ROADMAP-INDEX.md`'s own stated guarantee ("every planning
document reachable in ≤2 hops") is false for a real, bounded set of still-substantive documents.
Fixed directly in `CORE-ROADMAP-INDEX.md` §9 (see that file for the actual new rows/links):

- **3 arc directories cited as MEMORY-only, despite having real on-disk blueprints** —
  `integration-ports/` (IP-01..21), `ecosystem-strategy/` (EC-01..20), `ops-reliability/`
  (OPS-01..22) — each has a real `BLUEPRINTS-*.md` with per-unit Мета/Межа/Форма/RED-контракт
  structure, unlike their 7 sibling arcs which already got direct links. Now linked directly.
- **`realtime-change-intelligence-2026-07-17/`** — the most load-bearing orphan found: live,
  post-JS-drop, cited by two docs `ROADMAP.md` itself already links (`BLUEPRINT-CACHE-REFERENCE-
  GRAPH-TENSOR-ARENA`, `BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE`), but never itself
  linked — 3 hops instead of 2. Now linked directly.
- **`hermes-kernel-rewrite-2026-07-15/`, `organism-status-2026-07-15/`,
  `tech-synthesis-2026-07-15/`** — each 3 hops instead of 2; one landed deliverable traced back to
  the first (`kernel/src/harmonic.rs`'s own doc comment confirms the port). Now linked directly.
- **A named subset of standalone docs** with real, non-superseded content and no path in at all:
  `AUTONOMOUS-ORGANISM-SYNTHESIS-2026-07-14.md`, the `BLUEPRINT-W17/W19/W20/W22-*.md` mini-wave
  (siblings W18/W21 already had partial linkage — this is link-hygiene, not undone work; the prior
  2026-07-19 audit already confirmed all of W17–W22 shipped/green), `launch-design-brief.md`,
  `spectral-graph-fsm.md` (self-labels "Roadmap item," genuinely grounded in `order_machine.rs`),
  `SWARM-QUANT-BLUEPRINT-2026-07-15.md`, `SYSTEMS-GPU-ML-KERNEL-SYNTHESIS-2026-07-16.md`,
  `WEB3-SYNTHESIS-INVISIBLE-AGENTIC-LOCAL-INFRA-2026-07-17.md`. Now linked directly.
- **In `docs/research/`**: `BLUEPRINT-W13-pgrust-adapter.md`, `BLUEPRINT-W14-mesh-discovery-
  gossip.md`, and 3 of 6 `AUDIT-2026-07-18-*` council critiques (ARCHITECT/HERZOG/TORVALDS — their
  sibling FEYNMAN critique was already linked, oddly leaving these three out). Now linked directly.
- **`KNOWLEDGE-SPINE-BLUEPRINT-2026-07-14.md`** — its own frontmatter still reads `status:
  proposed`, but `kernel/src/bin/spine_snapshot.rs` confirms it's actually landed on `main` — the
  opposite-direction staleness (work outran its own doc). Corrected in that file directly.

**Correctly excluded, verified not a gap:** 8 Triadic-Council closed-loop directories
(`cinematic-product-media/`, `dev-login-backdoor-hardening/`, `fee-courier-seed/`,
`golive-remediation/`, `owner-token-revocation/`, `p0-privacy-hardening/`, `soft-access-gate/`,
`telegram-notifications-actions/`) all govern TypeScript/JS code deleted wholesale in the
2026-07-15 "drop js" commit — correctly unreachable, indexing them would be actively wrong.
**One governance-hygiene note, not a code gap:** `dev-login-backdoor-hardening/` and
`p0-privacy-hardening/` each carry a `NEEDS-HUMAN-DECISION` item ("was the dev-login backdoor
actually exploited," "is there a breach-disclosure obligation") that was never affirmatively
answered — only mooted by the code's deletion. Flagged here for operator awareness; not resolved
by this pass, which is documentation-only.


## 15. Full build-out execution plan (2026-07-20, planning-only, zero code)

> **Status: PLAN. No implementation in this section — that is deliberate.** Operator directive:
> "plans & blueprints first for the roadmap, include them in ROADMAP.md — only after that Opus for
> implementing them based on proper specs, plans & blueprints for ALL roadmap items & plans,
> design, etc." Scope decisions locked in the same exchange: build as much of the roadmap as
> possible this session (not just a fix/polish pass), prioritized toward one concrete outcome — a
> food-court owner can place and track one real order end to end — because that is both the
> release gate (no public tag until this works) and the thing the operator wants to hand-test
> personally. Everything below cites an EXISTING blueprint rather than re-deriving specs that
> already exist (per this corpus's own no-duplicate-authorship discipline) — this section's job is
> sequencing and gap-flagging, not re-planning what's already planned.

### 15.1 Why a sequencing pass was needed at all

The audits earlier today (§13/§14) already confirmed something important: **almost nothing here
is unplanned.** Every phase below already has a real, DoD-shaped blueprint file. What was missing
was (a) a single ordering across four independent waves that all claim priority, and (b) an honest
flag on the handful of places where a blueprint's own claimed status turned out to be stale
(§13) or where two blueprints appear to overlap in scope without an explicit disposition (found
during this pass, listed in 15.4). Those are real planning gaps; this section closes them.

### 15.2 Phase 0 — MVP critical path (build this first; this is the release gate)

The minimal slice for "an owner creates a menu, a customer places one order, the owner sees it,
a courier delivers it, everyone gets notified, cash changes hands on delivery." Every piece below
either already exists (cite the evidence) or has a real blueprint (cite the file) — nothing here
needs new planning, only sequenced building.

| # | Piece | Status entering this plan | Blueprint / evidence | Build note |
|---|---|---|---|---|
| 0.1 | Kernel order FSM + money law | **LANDED, most mature part of the stack** | `kernel/src/order_machine.rs`, `money.rs` — core of the whole system, thousands of tests | No work needed |
| 0.2 | HTTP order surface | **LANDED** (§13 correction: the roadmap's own "0%/PLANNED" claim was stale) | `kernel/src/json_api.rs` + `tools/native-spa-server/src/api.rs`, commit `68d5c2874` — real `/api/order`, `/api/order/{id}`, `/api/order/{id}/advance` routes | Confirm still boots cleanly (an in-flight test-sweep agent is checking this right now — §15.5 notes to fold its result in before building starts) |
| 0.3 | Payment: cash-on-delivery rail | **LANDED** | `kernel/src/ports/payment.rs` (`PaymentPort`+`CashAttestation`+reconciliation), `kernel/tests/firewall_p47.rs`, commits `e6367ae73`/`de56a27d6` | Use this rail for the MVP — no PSP integration needed; matches D12 §4-D's own market framing and avoids the payment-adapter-residual blueprint's real complexity (Phase 0 doesn't need it) |
| 0.4 | Owner surface (menu management, order visibility) | **PLANNED** — rulings landed (WebGPU no-DOM exemption, hub model), build-out open | `CORE-ROADMAP-2026-07-17/BLUEPRINT-P70-owner-surface.md` (supersedes P48's CRUD-admin half per `kernel/src/ports/owner_surface.rs`'s own header comment — confirmed this session) | **Build this.** Owner needs a real menu-entry + order-queue view — this is the one genuinely new UI surface the MVP can't skip |
| 0.5 | Customer surface (browse, checkout) | **PLANNED**, blueprint is explicitly "M1 critical path" | `CORE-ROADMAP-2026-07-17/BLUEPRINT-P69-customer-storefront-checkout.md` | **Build this.** Pair with 0.4 — both ride the same intent-interface architecture (0.6) |
| 0.6 | UI architecture both 0.4/0.5 render through | **Built, tested, wired into the production loop — never connected to real content** | `engine/src/intent.rs` + `compose_ui.rs` (P64), `docs/design/BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md` Stage A ("wire two already-tested endpoints together, ~zero technical risk, 6 falsifiable RED→GREEN acceptance criteria already given") | **Build Stage A first** — it's the substrate 0.4/0.5 render through, and the blueprint already rates it near-zero risk |
| 0.7 | Order intake channel (how a customer's order reaches the kernel) | **PLANNED**, Phase 1 = "build first," self-serve, free | `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P48-INTAKE-omnichannel-order-intake-2026-07-20.md` Phase 1 (Telegram + Website) | For MVP: the **website path only** (0.5's own storefront → 0.2's `/api/order`) — the blueprint's own Phase 1 already scopes Telegram+Website together, but website has zero deniable/review gates and this environment already has `cloudflared` running (checked live), so Telegram bot intake is a plausible Phase-0-adjacent add, not a hard requirement. Recommendation: ship website-only for Phase 0, Telegram intake in Phase 1 (15.3) |
| 0.8 | Courier assignment + delivery flow | **STATUS CONFLICT FOUND THIS PASS — see 15.4-G1, resolve before building** | Two candidate blueprints found: `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P52-courier-working-surface.md` (P52, explicitly flagged "MVP-blocking: P50's gate cannot go green without it") and `BLUEPRINT-P71-courier-surface.md` (P71, Wave W3) | **Do not build until 15.4-G1 is resolved** — building the wrong one wastes the highest-risk piece of Phase 0 |
| 0.9 | Outbound order notifications (owner/courier/customer) | **Mostly built** — outbound-only send fabric already exists | `kernel/src/messenger.rs` (deep-link builders), `kernel/src/ports/notification.rs` (`Notifier` fan-out) | Wire existing sends to the new order-lifecycle events; small glue, not new design |
| 0.10 | First-order validation gate | **PARTIAL** — audit half done, gate open | `CORE-ROADMAP-2026-07-17/P50-COMPLIANCE-AUDIT.md`, commits `568ff51c4`/`788cbee5a`; depends on 0.3/0.4/0.8 (P47/P48/P34/P37/P38) | Closes naturally once 0.1–0.9 land — not separate new work |

**Explicitly deferred out of Phase 0** (real, planned, just not on the critical path for one order):
live map/routing (P51 — a customer can track by order-status text without a live map for v1),
food-court multi-vendor N-leg checkout (P72 — single-vendor first), the payment-adapter-residual
crate (real PSP integration — cash rail suffices for Phase 0), hub provisioning/claim automation
(P67 — the operator can provision the one test hub by hand for now), data wallet/offline drafts
(P66), dispatch orchestrator (P65 — direct courier assignment suffices at MVP scale).

### 15.3 Phase 1+ — continued build-out, priority order (after Phase 0 is green)

Everything else in the roadmap that's still PLAN, sequenced by dependency and leverage. Each row
cites its existing blueprint — no new specs written here.

1. **Telegram order intake** (`BLUEPRINT-P48-INTAKE...` Phase 1's Telegram half, deferred from 0.7) — cheapest next win, self-serve, free, zero deniable gates.
2. **Telegram ops-hub build-out Phases 2–5** (`docs/design/BLUEPRINT-TELEGRAM-OPS-HUB-REMAINING-BUILD-ORDER-2026-07-20.md`) — CI/CD digest, S0/S1 mirror-rule formalization, LOGS topic, KERNEL/MESH+MEMORY/DOCS topics. Pure ops-observability, zero product risk, already 5 DoD-bearing phases specified.
3. **Local-model wiring Phase 1** (`BLUEPRINT-LOCAL-MODEL-WIRING-TESTING-USAGE-2026-07-20.md`) — closing the `AiMode`→`compose.rs` gap. Small, already scoped, unblocks any future AI-assist feature honestly (fail-closed today).
4. **Intent-interface Stages B/C** (`BLUEPRINT-INTENT-INTERFACE-ONE-SCREEN-2026-07-20.md`) — gated on named operator decisions already recorded there (wgpu network-grant timing, native-companion-app ever) — do not build until those are re-confirmed live, not just recalled from the blueprint.
5. **Payment-adapter residual** (`BLUEPRINT-P60-payment-adapter-residual-2026-07-20.md`) — real PSP integration once Phase 0's cash rail has proven the order flow works; Phase 0 of that blueprint (PSP diligence) can start in parallel with anything above since it's pure research.
6. **Omnichannel intake Phases 1.5–3** (SimpleX, WhatsApp) — after Telegram (step 1) proves the pipeline; WhatsApp specifically requires the named Business-Verification shepherding decision from that blueprint's own D1.
7. **Map/routing (P51)**, **dispatch orchestrator (P65)**, **data wallet (P66)**, **hub provisioning automation (P67)**, **food-court N-leg checkout (P72)** — the Phase-0-deferred items, roughly in this order (each blueprint already exists under `CORE-ROADMAP-2026-07-17/`).
8. **Product-surface wave remainder**: offline resilience Phase A (Service-Worker+IndexedDB, already D14-ratified), media/comms build-out (`Media` capability resource + manifest layer over the already-80%-there `chunker.rs`/`backup.rs`), spatial-storefront-voice-hub build phases.
9. **Space-grade items 34–44** (toy-pilot-arc, already IN-PROGRESS per the Part V reconciliation table — 35/36/38 done, 34/37/39–44 remain) and **items 62–72** (telemetry/cost-ledger/digital-twin retrofits, mostly landed per Part V's reconciliation, a few gaps remain per that table).
10. **Space-grade items 73–78** (Governed Self-Evolution) — D11 has already ruled the full apply-token design; this is the one place a fresh operator "go" is explicitly required before ANY code lands (D11: "does not authorize dispatching items 73-78 to code" — a separate decision from everything else in this plan).
11. **Predictive RESOURCES telemetry + anomaly stability signals** (operator-requested 2026-07-20,
    [`BLUEPRINT-PREDICTIVE-RESOURCES-TELEMETRY-2026-07-20.md`](BLUEPRINT-PREDICTIVE-RESOURCES-TELEMETRY-2026-07-20.md))
    — real-time prediction over the `resources-summary.jsonl` history (p50/p99/jitter/watts/CO2e/
    CPU/mem/disk) via `kernel::kalman` + a generalized `kernel::markov` regime detector (the exact
    `tools/loop-signals/` detector shape, reused not reinvented) + graph-Laplacian cross-metric
    coherence via `kernel::spectral`; spikes surfaced as a new `resources-stability-alert.jsonl`,
    narrated by `llm-adapters` only when an alert fires, degrading closed to a template otherwise.
    Prerequisite schema fix inside the blueprint (§3.1): `mem_pct`/`disk_pct`/`load1_norm`/net
    throughput are computed every `topics resources` run today and silently discarded, never
    persisted — must be added to the JSON record before any of those three can be predicted.
12. **mesh-adapter / bebop-repo cross-repo type drift** (found + real-build-confirmed 2026-07-20,
    [`BLUEPRINT-MESH-ADAPTER-BEBOP-CROSSREPO-DRIFT-2026-07-20.md`](BLUEPRINT-MESH-ADAPTER-BEBOP-CROSSREPO-DRIFT-2026-07-20.md))
    — `bebop-delivery-domain` (companion `bebop-repo`) does not compile against the current
    `dowiz-kernel` (`OrderItem` gained `vendor_id`/`currency`, `OrderStatus` gained
    `Refunding`/`CompensatedRefund`; live `cargo check --features kernel-rlib` reproduces 2
    concrete errors across 5 call sites). **⚠ Needs an explicit operator decision before any code
    lands** — the blueprint fully specifies both a cross-repo PR path and a dowiz-side vendored-fork
    stopgap, but does not choose between them.
13. **`native-spa-server` listener-wide DoS hardening** (follow-up to `00436dedc`'s per-connection
    fix, [`BLUEPRINT-NATIVE-SPA-SERVER-LISTENER-HARDENING-2026-07-20.md`](BLUEPRINT-NATIVE-SPA-SERVER-LISTENER-HARDENING-2026-07-20.md))
    — a global `tokio::sync::Semaphore` connection cap + a per-IP `kernel::token_bucket`-based rate
    limiter, both fail-closed and both defense-in-depth alongside the existing `MAX_INFLIGHT_API`/
    `MAX_BODY_BYTES` layers (neither of which covers accept-time exhaustion). Not gated on anything
    above; independently buildable whenever picked up.

### 15.4 Gaps found while sequencing (resolve before implementation starts)

- **G1 — P52 vs. P71, courier surface: RESOLVED (2026-07-20 audit) — was a linking gap, not a missing disposition.** A real split already exists and was already recorded, just not linked from here. **P52** (`BLUEPRINT-P52-courier-working-surface.md`) is the kernel-side fold/law/capture design — K1 availability/duty, K2 claim inbox, K3 delivery-run relay, K4 PoD capture, K5 earnings fold, K6 invite/enrollment, K7 cash attestation, K8 conversation pane; its own text is explicit that it is "NOT a fourth rendering technology" and "NOT voice/turn-by-turn." **P71** (`BLUEPRINT-P71-courier-surface.md`) is titled "P52-rev" in its own text — the rendered, voice-primary, dispatch-wired courier app. P71 §1 is itself a full disposition note: it preserves K1/K4-K8 unchanged and supersedes exactly 3 named things from P52 (the 60s surface-owned offer-timeout → P65's 30s hub-owned deadline authority; the "NOT voice" anti-scope → voice-primary via P64; the CPU-stub render deferral → a real full-wgpu render), plus adds the actual `apps/courier` build. This disposition is also independently recorded at `ROADMAP.md:3196` (§18.3, dated 2026-07-18 — two days before this §15/G1 text was written, which is why the "no recorded disposition" claim above was itself stale the moment it was written). **Build order: P52's kernel-side folds first, then P71's render layer on top — do not retire either file (same keep-both precedent as P70/P48).**
- **G2 — Phase 0.2's exact current endpoint completeness is pending live verification.** A test-sweep agent is checking whether `native-spa-server` actually boots and serves real responses right now (§15.5) — fold that result in before starting 0.4/0.5, since the owner/customer UI has nothing to render against if the order API isn't actually reachable.
- **G3 — no genuinely unplanned work found.** Every other item in this section already has a real DoD-bearing blueprint. This plan's job was sequencing + the two gaps above, not authoring new specs.
- **G4 — space-grade item 3's third proof clause was silently uncovered — CLOSED this pass with a
  real blueprint.** The roadmap-wide gap audit (2026-07-20, successor to
  `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md`) found item 3 ("`order_machine` const-adjacency")
  was the one genuine gap across all 78 space-grade items — its "zero heap allocations under a
  counting allocator test" clause had no test and no dedicated blueprint, only incidental mentions
  inside items 6/7. See [`BLUEPRINT-ITEM-03-order-machine-zero-alloc-proof-2026-07-20.md`](BLUEPRINT-ITEM-03-order-machine-zero-alloc-proof-2026-07-20.md).
- **G5 — `wasm-bindgen` version pin: accepted external constraint, not a design gap.** The
  `wasm-bindgen` version in `kernel/Cargo.toml`/`wasm/`'s manifests is pinned to whatever `wgpu`
  requires for the WebGPU render engine (P38) and cannot be independently downgraded without
  breaking that dependency chain. This is a genuine upstream constraint, not something this
  codebase's own architecture can decouple — no blueprint is warranted unless a real decoupling
  path (e.g. wgpu dropping its wasm-bindgen version floor, or the render engine moving off wgpu
  entirely — not proposed here, no evidence either is imminent) is found in a future pass.
  Recorded here plainly so it is never mistaken for an unaddressed "PLANNED" item.

### 15.5 Execution discipline for the Opus implementation pass

- **Work in isolated `git worktree`s, never the shared `/root/dowiz` checkout.** Confirmed necessary twice today — a concurrent autonomous process actively mutates that checkout, including at least one local-only divergent reset discovered this session (see the git-collision handling embedded in this document's own commit history for the pattern to follow: fetch, isolated worktree off `origin/main`, verify fast-forward, push, never force).
- **Verify-before-build, every time.** Multiple items in this very plan (P37, P28's `CacheGraph`, P26's `MemoryBudget`) turned out to have stale claimed-status this session — always re-check against live code before assuming a blueprint's "LANDED"/"PLANNED" marker is still accurate.
- **RED→GREEN, per this repo's standing culture** — every implemented item ships a test proving the gap existed before and is closed after, not narrative.
- **Stop and ask when a real decision is needed** — G1 above is now resolved (was a linking gap,
  not a missing decision), but item 12's mesh-adapter/bebop-repo cross-repo drift (§15.3) is a live
  current example of the same discipline: the blueprint fully specifies both remediation paths but
  deliberately does not choose; the intent-interface Stage B/C gates and the WhatsApp D1
  shepherding decision are two more already named in their own blueprints. Do not resolve these by
  guessing.
- **Fold in the 5 in-flight test-sweep agents' findings** (kernel, engine/wasm, agent-lane, tools/apps, web/browser) before starting Phase 0 build work — they may surface bugs in exactly the surfaces (native-spa-server, the intent/compose_ui wiring) this plan depends on being solid.

---

*Maintained going forward: every future dated wave gets one new §-numbered entry appended after
§9 (or a new top-level Part if it's substantial enough to warrant its own detail section), never a
new competing top-level doc. If you're about to create a new `MASTER-*`/`ROADMAP-*`/
`GROUND-TRUTH-*` file, stop — extend this one instead.*

---

# Part II — Sovereign Architecture Roadmap (P01–P30, verbatim, 2026-07-16)

> Full content of the former `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`, inlined here
> 2026-07-20; the standalone file has been deleted. Headings below were demoted by one level to
> nest under this document's top-level structure; content is otherwise reproduced verbatim,
> including this doc's own internal status/superseded markers. Part I §5 has the one-paragraph
> summary and current landed-subset status; read this Part for the full P01–P30 detail.

## MASTER ROADMAP — Sovereign Architecture (dowiz + openbebop), 2026-07-16

> **Single source of truth for the TARGET:** `docs/design/ARCHITECTURE.md` §0 (MESH-FOUNDATION +
> SCOPE RULE) + §8 (honest gaps), cross-locked with `docs/design/STRATEGIC-VECTORS-LOCKED-2026-07-16.md`.
> **Single source of truth for the PATH:** this document + `sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md`
> (detailed reference table) + 19 per-phase blueprints in the same directory (execution detail).
>
> This document does not restate the architecture — it maps the shortest honest path from the
> repo's ACTUAL current state to that architecture, in full, with zero anchors deferred outside a
> phase and zero pre/post-MVP split, per the operator's 2026-07-16 directive. It supersedes
> `MASTER-ROADMAP-10-PHASES-2026-07-14.md`, `MASTER-EXECUTION-PLAN-2026-07-13.md`,
> `MASTER-INTEGRATION-PLAN-2026-07-14.md`, `MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md`
> (added 2026-07-17 — omitted from the original list; P-I consolidation audit §0 confirmed it is
> superseded like the rest), and `MASTER-ROADMAP-MVP-2026-07-12.md` (root) — all
>
> ⚠️ **CANON DRIFT — 2026-07-17.** §1.3/§1.6(a) "CI runs exactly 2 jobs / kernel+engine tests run nowhere
> / CONTRIBUTING.md:17 false / ARCHITECTURE.md §8 still claims Apache-2.0 mismatch" describe the pre-P01
> state and are FALSE vs HEAD. P01 `4b05ee588` added all seven CI gates; P18 corrected ARCHITECTURE.md
> §8 (LICENSE is AGPLv3 since `ac1caba40`; secret scrub CLOSED). Live repo is truth.
> pre-date the mesh-foundation pivot (`0d1935d96`) and are kept as history, not law, per
> `ARCHITECTURE.md`'s own "merge, never append" rule. It does **not** edit `ARCHITECTURE.md` itself;
> `BLUEPRINT-P02` is the proposed merge-diff, left for the operator.

---

### 0. Provenance — how this was built (so the result can be checked, not just trusted)

Following the operator's routing directive (Fable for synthesis grounded in code + research +
target architecture; Opus for blueprints), this roadmap was produced in three rounds, all
code-grounded, none self-certified without evidence:

1. **Round 1 — five parallel Fable gap-analyses**, each owning a disjoint slice of the 147 nominal
   architecture anchors (M1-12, V1-6, D1-8, S1-9, E1-62, F1-50), each grounded directly in the
   actual code (`/root/bebop-repo` for mesh/PQ-crypto, `/root/dowiz/kernel`+`engine` for
   service/compute, `/root/hermes-agent-kernel-rewrite` for dev-tooling routing), not just prior
   docs. Reports: `sovereign-roadmap-2026-07-16/R1-{A,B,C,D,E}-*.md`.
2. **Round 2 — one Fable merge pass** that resolved cross-cluster dependencies the five reports
   surfaced into each other into ONE dependency-ordered phase sequence, re-verifying every
   load-bearing claim (CI job list, LICENSE text, D5/D8 absence, E10/E36 duplication) against the
   live tree rather than trusting R1's paraphrase. Report: `sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md`.
3. **Round 3 — nineteen parallel Opus blueprints**, one per phase from Round 2's table, each a
   concrete, file-and-line-grounded design document (files touched, migration steps, falsifiable
   acceptance criteria) — not implementation. Reports: `sovereign-roadmap-2026-07-16/BLUEPRINT-P{01..19}-*.md`.

Total: 5 + 1 + 19 = 25 agent passes, all read-only research/design (no product code, CI config, or
canon file was edited by this effort — every finding below is a proposal for the operator or a
future implementation pass to act on).

---

### 1. What is actually true right now (headline findings, most load-bearing first)

These surfaced independently across multiple phases' code-grounded research and materially change
what "the roadmap" means — read this before the phase table:

1. **The PQ-crypto substrate is far more built than canon implies, but unconsumed.** `cargo test`
   across all five bebop2 crates is GREEN today (232+ tests). ML-DSA-65 is ACVP-verified. But it's
   a library nobody calls: dowiz has **zero** code-level dependency on the protocol (one stray
   comment at `kernel/src/domain.rs:524`), the wire is authenticated-but-**plaintext-readable**
   (`NoopPayloadEnc`, default feature literally named `insecure-tls`), and the trust root itself is
   classically forgeable (`roster.rs` delegation chain is Ed25519-only, making the ML-DSA leg
   vacuous against the exact quantum attacker it exists to stop). → Phases 3, 9.
2. **`apps/web`, `packages/ui` (all i18n), and `packages/domain` are fully deleted**, not merely
   decoupled (`79ef316f6`/`db766de47`/`fce5738b0`; `git ls-files 'apps/*'` = 0 at HEAD and on
   `origin/main`). Every "re-plumb the product onto the mesh" framing in older docs is stale — this
   is a rebuild, with the As-Built Summary and DOWIZ-INTERFACES checklist as the feature-inventory
   ledger. → Phase 16.
3. **CI is regressed, silently.** A gitleaks gate existed (`b10a7bfe3`) and was dropped when
   `f9ab28ff1` rewired CI to drop all JS/TS. `.github/workflows/ci.yml` today runs exactly 2 jobs;
   kernel's 337 tests and engine's 47 tests run **nowhere** in CI. Every downstream "GREEN" claim
   in this repo has been unverifiable since that commit. `CONTRIBUTING.md:17` additionally makes a
   false claim (a DCO check that doesn't exist). → Phase 1, correctly sequenced as the global
   Wave-0 precondition for trusting anything downstream.
4. **The one mandated global control doesn't exist, and the thing that shares its name is its
   opposite.** M9 requires a unilateral operator kill-switch; the only "KillSwitch"-named
   construct in the codebase (`guard.rs`) is a ≥2/3 **consensus** vote registry — a governance
   mechanism, not a kill-switch, and architecturally the inverse of what M9 specifies. → Phase 10.
5. **V1 (split-identity + adversarial verification) is 0% built while its entire substrate sits
   idle.** All "verification" today is self-context (the same session that made a change judges
   it) — this is the self-certification pattern BRAIN-TOPOLOGY research already flagged as this
   project's dominant failure mode. The ML-DSA/hybrid-gate/genesis-loader primitives V1 needs are
   already finished. → Phase 6.
6. **Canon itself is stale in three checkable ways**, independent of any roadmap work: (a) LICENSE
   is **already AGPLv3** (flipped `ac1caba40`, 2026-07-14) — `ARCHITECTURE.md` §8 still claims an
   Apache-2.0 mismatch; (b) the force-push/history-scrub question is **substantively resolved**
   (H8 runbook closed, origin already at the scrubbed tip) — canon still lists it as open; (c) the
   count "**147 locked anchors**" is off by at least 2 — **D5 and D8 are referenced in the "D1-8"
   total but defined nowhere** in any revision of either canon document (verified by git
   archaeology across all four introducing/amending commits). None of these are edited here —
   `BLUEPRINT-P02` is the proposed merge-diff. → Phase 2.
7. **A real, still-open HIGH-severity finding survives from the prior crypto pass**: the `mod_l`
   constant-time side-channel (C4b, flagged 2026-07-14, never closed) sits on the same Ed25519
   code path that Phase 6's identity ceremony and Phase 10's kill-switch signing would both use —
   this is why Phase 3 (crypto hardening) gates both of them, not just wire work.
8. **Two real design contradictions block dispute/escrow (F44)**, not a missing spec: the only
   existing design (`fable-protocol-2026-07-11/F2-dispute-arbitration.md`, a complete 6-state
   machine with a written RED test) proposes a reputation-scoring jury (contradicts M12's
   NO-COURIER-SCORING law) and UMA/Kleros external arbitration (contradicts M6's zero-dependency
   law). This needs an explicit operator ruling, not a silent pick. → Phase 2 (ruling), Phase 14 (build).

---

### 2. THE 19-PHASE ROADMAP

Full detail — anchors, current→target gap, dependencies, falsifiable done-tests — lives in
[`sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md`](sovereign-roadmap-2026-07-16/R2-MERGED-PHASE-ROADMAP.md)
§2. This table is the navigation index; each row links to its execution-grade blueprint.

| # | Phase | Anchors | Depends on | Blueprint |
|---|---|---|---|---|
| 1 | CI Truth Floor | V2,V3,V5,S3,S6,D6,E2,E3,E40,E52,E58,E62 | — | [BLUEPRINT-P01](sovereign-roadmap-2026-07-16/BLUEPRINT-P01-ci-truth-floor.md) |
| 2 | Canon Repair + Operator Decision Batch | D5,D8,V4,V6,E35,E42,E53,E55 | — | [BLUEPRINT-P02](sovereign-roadmap-2026-07-16/BLUEPRINT-P02-canon-repair-operator-decisions.md) |
| 3 | PQ Trust-Root Hardening | M2,M4,M12,E10(≡E36),F19,F21,F24,F26 | — | [BLUEPRINT-P03](sovereign-roadmap-2026-07-16/BLUEPRINT-P03-pq-trust-root-hardening.md) |
| 4 | Kernel Product-Math Primitives | F45,E61 | — | [BLUEPRINT-P04](sovereign-roadmap-2026-07-16/BLUEPRINT-P04-kernel-product-math.md) |
| 5 | Routing Organism Wiring | E13,E14,E15,E19,E20,F6 | — | [BLUEPRINT-P05](sovereign-roadmap-2026-07-16/BLUEPRINT-P05-routing-organism-wiring.md) |
| 6 | V1 Split-Identity + Adversarial Verifier | V1,E9 | 1, 3 | [BLUEPRINT-P06](sovereign-roadmap-2026-07-16/BLUEPRINT-P06-v1-split-identity-verifier.md) |
| 7 | Money-Law Closure | S5,S9 | 1 (strengthened by 6) | [BLUEPRINT-P07](sovereign-roadmap-2026-07-16/BLUEPRINT-P07-money-law-closure.md) |
| 8 | Typed Local Observability | M8,S7,S8,D7,E46,E47,F29,F31,F32,F36,F39,F40 | 1 | [BLUEPRINT-P08](sovereign-roadmap-2026-07-16/BLUEPRINT-P08-typed-local-observability.md) |
| 9 | Confidential, Self-Healing Wire | M3,M6,M7,D2,E31-34,E38,F11-13,F15,F16,F18,F20,F22,F23,F25,F30 | 3, 4 | [BLUEPRINT-P09](sovereign-roadmap-2026-07-16/BLUEPRINT-P09-confidential-self-healing-wire.md) |
| 10 | Hub Runtime: Policy-as-Data, Kill-Switch, Boot | M5,M9,E37,F1,F2,F5,F8,F28 | 3, 6, 9 | [BLUEPRINT-P10](sovereign-roadmap-2026-07-16/BLUEPRINT-P10-hub-runtime-kill-switch-boot.md) |
| 11 | Compute Budget & Cache | E21-25,F33-35 | 1, 8 | [BLUEPRINT-P11](sovereign-roadmap-2026-07-16/BLUEPRINT-P11-compute-budget-cache.md) |
| 12 | Durable Storage, Deploy & Ops Floor | D1,S1,S2,E11,E26-30,E48-50,F37,F38 | 7 | [BLUEPRINT-P12](sovereign-roadmap-2026-07-16/BLUEPRINT-P12-durable-storage-ops-floor.md) |
| 13 | Delivery on Protocol | M1,M10,S4,E1,E39,F17,F41-43,F46,F50 | 4, 7, 9, 10 | [BLUEPRINT-P13](sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md) |
| 14 | Dispute/Escrow + Per-Hub Graph-Wiki | E8,E16,E51,F44,F48 | 2 (HARD), 13 | [BLUEPRINT-P14](sovereign-roadmap-2026-07-16/BLUEPRINT-P14-dispute-escrow-graph-wiki.md) |
| 15 | Living Organism Unbounded | M11,E17,E18,F3,F4,F7,F9,F10,F27 | 10 (HARD), 5, 9 | [BLUEPRINT-P15](sovereign-roadmap-2026-07-16/BLUEPRINT-P15-living-organism-unbounded.md) |
| 16 | Product UI Rebuild | D4,E12,E41,E43,E44,F49 | 4, 13 | [BLUEPRINT-P16](sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md) |
| 17 | Demo, Splat Tiers & GPU-Unlock Closure | E4,E45,F14,F47 | 11, 16, +GPU-unlock | [BLUEPRINT-P17](sovereign-roadmap-2026-07-16/BLUEPRINT-P17-demo-splat-gpu-unlock.md) |
| 18 | Public-Flip Readiness & Execution | D3,E5,E54,E59 | 1, 2 | [BLUEPRINT-P18](sovereign-roadmap-2026-07-16/BLUEPRINT-P18-public-flip-readiness.md) |
| 19 | Ecosystem Growth Engine | E6,E7,E56,E57,E60 | 18 | [BLUEPRINT-P19](sovereign-roadmap-2026-07-16/BLUEPRINT-P19-ecosystem-growth-engine.md) |

**No pre/post-MVP split exists in this table.** Every phase is a real build-dependency step toward
the same one architecture; "later" phases are later because something earlier must exist first
(crypto before wire, wire before hub-runtime, hub-runtime + money before the product rides the
protocol), never because a phase was deemed optional or deferred by preference.

#### Waves (maximum parallelism)

```
WAVE 0 (start immediately, mutually independent): P1 P2 P3 P4 P5
WAVE 1: P6◄(1,3)   P7◄(1)        P8◄(1)         P18-prep◄(1,2)
WAVE 2: P9◄(3,4)              P11◄(1,8)       P12◄(7)
WAVE 3: P10◄(3,6,9)
WAVE 4: P13◄(4,7,9,10)
WAVE 5: P14◄(2,13)   P15◄(10,5,9)   P16◄(4,13)
WAVE 6: P19◄(18)                              P17◄(11,16,+GPU-unlock)
```

**Critical path: P3 → P9 → P10 → P13 → {P14, P16} → P17** (crypto correctness → confidential wire
→ hub runtime → delivery spine → dispute/UI → demo). P1 and P2 gate everything *epistemically*
(nothing downstream is trustworthy without them) but are cheap and fully parallel — start them
first regardless of what else is picked up. P5, P8, P11, P12, and P18-prep are off-critical-path
lanes that should be fanned out early to use idle capacity.

Full adjacency list: `R2-MERGED-PHASE-ROADMAP.md` §3.

**Wave-admission classification (added 2026-07-17, per Phase 25 —
[BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md](BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md)).**
The diagram above states *dependency* width; this note adds *resource* width, because this host is
4 physical cores × 2 SMT (8 vCPU, live `lscpu`), and the two kinds of parallel work in these waves
have different bounds:

- **Every phase is two-faced at execution time.** Its agent work (research, design, writing code
  text against a worktree) is **I/O-bound-dispatch** — lanes mostly blocked on the LLM API, CPU is
  not the bound; up to **D_max = 16** such lanes run concurrently (gated by memory PSI and the
  per-workflow `min(16, cores−2)` cap, not by core count). Its verification steps (each
  blueprint's `cargo build`/`cargo test`/bench done-check) are **CPU-bound-local** — bounded by
  **4 strict-core slots** (`taskset -c 0,2,4,6`, `nice 10`; one uncapped cargo build consumes all
  4). So WAVE 0's five phases genuinely CAN fan out five-wide (and wider) on the agent face, while
  their build/test done-checks queue through the shared 4-slot CPU budget — stagger the *builds*,
  never the *agents*.
- **Predominantly CPU-bound-local at verification:** P1 (kernel 337 + engine 47 tests), P3 (5
  bebop2 crates + CT tests), P4, P5, P6, P7, P8, P9, P10, P11, P12, P13, P24 (ring + criterion
  benches) — every phase whose done-check is a Rust build/test run, which the mandatory-benchmark
  doctrine (AGENTS.md) makes nearly all of them.
- **Predominantly I/O-bound-dispatch end to end:** P2 (canon repair — doc work), P18-prep, P14's
  design/ruling half, P20's doc/asset units, P22's blueprint-stage work, and all research/blueprint
  fan-outs (the 25-pass production run of this very roadmap was this class — it ran fine on 4
  physical cores because cores were never its bound).
- **Local-inference (third class, easy to mislabel):** P21's resident-agent runtime and P15
  E13-cpu (post-O18b) wait on local Ollama — that wait IS this host's CPU doing inference, so
  these count against the CPU budget (concurrency delegated to `OLLAMA_NUM_PARALLEL`, auto ≤ 4),
  never against the 16-lane dispatch budget.
- Admission is dynamic: a pure, local, µs-scale predicate over PSI/procfs (consuming P24's gauge
  surface — never a network call, per the LOCAL-DECISION rule). Full model, thresholds, and the
  proposed standing AGENTS.md rule: the Phase 25 blueprint.

---

### 3. Operator decisions required (19 items) — the real gate on Wave 3+

This roadmap intentionally does **not** resolve these. Each lives inside a numbered phase (that IS
its coverage — an anchor blocked on a ruling is not a dropped anchor), but real engineering on
several critical-path phases cannot proceed with full confidence past Phase 2 without them. Full
Descartes-quadrant treatment (options, tradeoffs, a flagged-overridable recommendation where one is
safe to offer) is in `BLUEPRINT-P02`. Highest-leverage first:

| # | Decision | Blocks | Stakes |
|---|---|---|---|
| O3 | **F44 dispute/escrow mechanism** — the only spec contradicts M12 (NO-COURIER-SCORING) and M6 (zero-dep) | Phase 14 | Not silently resolvable; candidates exist (operator-gated arbiter capability; staked Schelling voting) |
| O4 | **F48 merge semantics** — content-address-only vs. CRDT for per-hub graph sync | Phase 14 | A dormant `crdt-fence` guard was found to fence CRDT out of money/order code ONLY — genuinely open for the knowledge-wiki |
| O1 | **D5/D8** — define (candidates found in root `DECISIONS.md`, but on a *colliding* numbering scheme — adopting them accepts the collision explicitly) or renumber the whole D-series | Every "147"/"146" claim | Affects every anchor-count statement in every doc, including this one |
| O5 | **D2/iroh** — land the crate for real, or amend canon to "quinn primary + named unlock trigger" | Phase 9 | iroh does not exist in the codebase today; canon currently claims otherwise |
| O7 | **E1/F41 "hub-ring"** — ratify the consistent-hash reading (a literal star-hub would contradict M7 no-SPOF) | Phase 13 | Two words with no formal spec anywhere until ratified |
| O19 | **I-FINAL proof home** — bebop consensus path vs. dowiz `tools/eqc`; the file the old blueprint cited doesn't exist at either candidate location (it exists at a third, legacy path) | Phase 13 (F46 full closure) | Sharper than originally scoped — see BLUEPRINT-P13 |
| O9 | **V1-B verifier isolation bar** — fresh worktree vs. separate machine vs. different model family | Phase 6 | One sentence of canon closes this |
| O8 | **F10 max sub-hub recursion depth** | Phase 15 | Numeric value only |
| O2, O6, O10-O17 | Cheap/mechanical: E10≡E36 ratification, E35 "3-tier locality" definition, S7/S8 split, M12/F25 replay bound, "BD" expansion, M9 subtree-kill semantics, E13-20 numbering, canon rewordings, EUTM brand choice, public-flip go | Bookkeeping / Phases 8,9,10,14,15,18 | `BLUEPRINT-P02` offers a recommended default for each — operator can accept or override |
| O18 | **SPLIT 2026-07-16** (was one "GPU-unlock" trigger; a triple-confirmed self-critique pass — see §7 below — found this bundled two unrelated things): **O18a** `graphics-unlock` (network `cargo add wgpu` succeeding — verified RED/403 as of 2026-07-16) stays external/environment-gated; **O18b** `model-weights-unlock` (llama.cpp CPU tier — GGUF fetch + local server) is verified GREEN-ish on this host today and requires only a DECART report + operator go, **not** an external trigger | P17 (O18a only); P15 E13-gpu (O18a); P15 E13-cpu (O18b — actionable now) | Not cheap/mechanical — O18b is the single highest-leverage unblocked item in the whole roadmap right now |

**Practical read:** Waves 0-2 (Phases 1-5, 7, 8, 11, 12) need **no operator input** to start — pure
engineering against already-diagnosed gaps. Wave 3 onward (Phases 6, 9, 10, 13, 14, 15) benefit
from or hard-require O1, O3-O9 being ruled on. Getting O1/O3/O4/O5/O7/O9 answered early is the
single highest-leverage non-engineering action available — it unblocks the entire critical path's
back half without costing any engineering time.

---

### 4. Anchor accounting — zero exceptions, proven not asserted

- Nominal canon count: **147** (M1-12 + V1-6 + D1-8 + S1-9 + E1-62 + F1-50).
- **E10 ≡ E36** — identical text ("ML-DSA hybrid") at two different anchor numbers, re-verified
  against `STRATEGIC-VECTORS-LOCKED-2026-07-16.md:89,98`. Counted once; E36 recorded as alias. → 146.
- **D5, D8 undefined** in every revision of both canon documents (git-archaeology-verified across
  all four introducing/amending commits, independently re-checked in Round 2). Carried as
  **operator-decision placeholders inside Phase 2** — that is their coverage, not a gap in this
  roadmap. → **146 distinct IDs: 144 defined + 2 placeholders**, all 147 nominal IDs accounted for.
- **One seam anchor** (E16, "spectral+BD memory") was not claimed by any of the five Round-1
  clusters; the merge pass assigned it explicitly to Phase 14 rather than let it drop silently.
- Full per-anchor → phase mapping (all ~147 IDs, one row per series): `R2-MERGED-PHASE-ROADMAP.md` §5.

**Zero anchors deferred outside a phase. Zero anchors silently dropped.** This claim is checkable,
not asserted — the accounting table it rests on is reproducible by grep against the two canon
documents plus the phase table above.

---

### 5. What this roadmap is not

- **Not an implementation.** Every one of the 19 blueprints is explicitly a planning document —
  none of the 25 research/design passes that produced this roadmap wrote or edited product code,
  CI config, or canon files. Turning a blueprint into a merged diff is the next, separate unit of
  work per phase.
- **Not a canon edit.** The corrections in §1 items 6-7 and the full decision docket in §3 are
  proposals (concretely, `BLUEPRINT-P02`'s diff) for the operator to merge into `ARCHITECTURE.md`
  by hand or by explicit delegation — this document does not touch that file, honoring its own
  "merge, never append" rule.
- **Not a re-prioritization of the architecture.** No anchor was judged more or less important than
  another; phase ORDER here reflects only build-dependency reality (what must exist before what),
  never a value judgment about which parts of the architecture matter more.

### 6. Next steps

1. Operator rules on O1, O3, O4, O5, O7, O9 (the ones that actually gate engineering, per §3) —
   everything else in the decision docket has a safe default already proposed in `BLUEPRINT-P02`.
2. Wave 0 (Phases 1-5) starts immediately — no ruling required, each is independently actionable
   from its blueprint today.
3. `BLUEPRINT-P02`'s canon-diff gets merged into `ARCHITECTURE.md` (the LICENSE/scrub/DCO
   corrections in particular are cheap, checkable, and currently make the canon assert three false
   things).
4. As each phase's implementation lands, its blueprint's falsifiable done-test is the closure
   criterion — not a subjective "looks done."

---

### 7. Follow-up pass (2026-07-16, same day) — self-critique + harness/LLM-infra research

Per the operator's standing session-closing ritual (now codified in `AGENTS.md` — the 2-question
doubt check), this roadmap was immediately subjected to an independent adversarial review, plus two
research passes the operator requested on the agent harness itself. All in
`sovereign-roadmap-2026-07-16/`:

- **[SELF-CRITIQUE-2Q-DOUBT-AUDIT.md](sovereign-roadmap-2026-07-16/SELF-CRITIQUE-2Q-DOUBT-AUDIT.md)**
  — a decorrelated Opus pass answering "what are you least confident about" (7 items, each
  investigated to a verdict, not just listed) and "what's the biggest thing missing." **Two items hit
  the big-deal threshold**, both confirmed by live probes, not just doubted: (1) Phases 5/15 gated
  ALL self-hosted LLM execution on an external "GPU-unlock" trigger, but llama.cpp is CPU-first by
  design and needs no GPU — this was a real category error, now corrected (§3 of this document, and
  in `BLUEPRINT-P05`/`BLUEPRINT-P15`/`R2-MERGED-PHASE-ROADMAP.md` directly). (2) The critical path
  (P3→P9→P10→P13→...) makes the quantum-mesh substrate load-bearing-first, while G11 ("first real
  order" — the only proof the product is wanted) sits as a late done-test rather than a Wave-0 gate —
  **this is flagged, not resolved**; it is an operator-level charter question this roadmap does not
  prejudge.
- **[HARNESS-RESEARCH-revfactory-and-agentic-teams.md](sovereign-roadmap-2026-07-16/HARNESS-RESEARCH-revfactory-and-agentic-teams.md)**
  — dowiz's telemetry *consumers* (EV/Kelly model routing, false-claim meter) are already ahead of
  the open-source agent-team-harness genre (incl. `revfactory/harness`, 8.2k★), but starving on
  hand-written data (9 rows in `track_record.jsonl`).
- **[LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md](sovereign-roadmap-2026-07-16/LLM-INFRA-RESEARCH-wandr-vllm-llamacpp.md)**
  — independently reaches the same llama.cpp/GPU-gating verdict, with a concrete `LlmBackend`
  Trait-as-Port design (managed-API default, llama.cpp/vLLM as hub-chosen adapters, per M5).
- **[HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md](sovereign-roadmap-2026-07-16/HARNESS-IMPROVEMENT-SYNTHESIS-PLAN.md)**
  — the combined, sequenced fix: **H1** auto-harvest the governance ledgers from session transcripts
  (cheapest, highest leverage — zero new deps), **H2** sovereign per-repo Telegram coverage for
  dowiz/openbebop/hermes (contract-shared, code-independent — cross-repo calls explicitly rejected as
  a central-SPOF shape), **H3** the `LlmBackend` port + operator-gated llama.cpp Tier-1 rollout.
- **[OPEN-SOURCE-CREDITS-LIST.md](sovereign-roadmap-2026-07-16/OPEN-SOURCE-CREDITS-LIST.md)** — every
  repo/service/tool integrated, borrowed from, or reverse-engineered across dowiz + bebop-repo +
  hermes-kernel's full history (~150 active dependencies, 32 design influences, 21 specs/standards,
  26 evaluated-and-rejected), for the operator to star/credit.

`BLUEPRINT-P05` §8, `BLUEPRINT-P15` §9/§10, and `R2-MERGED-PHASE-ROADMAP.md`'s O18 row / Phase-5 /
Phase-15 rows / dependency graph have already been corrected to reflect the O18 split (E13-cpu vs.
E13-gpu). `ARCHITECTURE.md:34` still needs the operator's canon-merge pass (per `BLUEPRINT-P02`'s
mechanism) — not edited here, same boundary as the rest of this roadmap.

---

### 8. Second follow-up pass (2026-07-17) — four new phases, one cross-phase addendum, a
### completeness audit, native-cleanup tracking

Same rule as §7: every claim below is either re-derived from live code/tests this session or
named as an open decision, never asserted from a prior doc's authority.

#### 8.1 Four new phases (P20–P23)

None of these existed in the 19-phase table in §2. Each has an execution-grade blueprint already
written (research + DECART + 2-question doubt audit + Anu/Ananke check, same protocol as P01–P19).
Adding them here is bookkeeping, not new design work — the design already exists in the cited file.

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 20 | Demo & Marketing Pipeline Refactor | [DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md](DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md) | 7 (its DM-2 offer-redemption ledger hard-depends on P07's replay-dedup fix), 18 (all publication gated behind public-flip, mirroring P19's own boundary) | 7 work units (DM-1..DM-7); no new crate, reuses engine `compose` + a committed glyph atlas |
| 21 | Local AI / Local Agents (resident-agent plane) | [LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md](LOCAL-AI-LOCAL-AGENTS-RESEARCH-2026-07-17.md) | 5 (routing organism, done) | Extends the already-shipped `LlmBackend`/Ollama port (harness-2026-07-16) with a plan→act→observe loop; zero new external deps per its own DECART; shares sequencing with the agentic-mesh arc (separate branch) but does not depend on it landing. **2026-07-18: full standalone blueprint now exists — [CORE-ROADMAP-2026-07-17/BLUEPRINT-P21-local-llm-hermes-native.md](CORE-ROADMAP-2026-07-17/BLUEPRINT-P21-local-llm-hermes-native.md)** (native `/models` via Ollama `/api/tags` + Hermes routing-lane ruling + P25 L-class/P26 MemoryBudget consumption + real-time bench/eval harness via P45 §4b.3; Mixtral verdict + MoA deferral recorded there with live-measured numbers). **Part 2 (same day, same file, §11): Tiered Intelligence architecture evaluated against the operator's own proposal — Tier-0-router REJECTED (two deterministic routers already exist), Ollama stays (neither server has priority queueing), model verdict = resident code/general pair + existing remote lane as "Tier 2", not a new local heavy model — see §15 below.** |
| 22 | Multi-Platform Social Auto-Posting | [BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md](BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md) | 1 (CI floor) | New `SocialPoster` port mirroring the `LlmBackend` port pattern exactly; Wave 0 = Telegram (no operator decision needed); Wave 1 (Viber) blocked on **O-SOC-1** (public media-hosting location); Wave 2 (Meta) gated on its own approval-process calendar, not a build dependency |
| 23 | Device Auth + 2FA | [BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md](BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md) | none for its P1 (a zero-dep `totp.rs` primitive, buildable today); **P3 (full HTTP wiring) depends on a dynamic admin HTTP surface that does not exist anywhere in this roadmap yet** — a real gap, named here rather than assumed away. Native Rust primitive chosen over Better Auth because the JS/TS stack (Better Auth's runtime) is fully deleted from `origin/main`, not merely paused |

**Wave placement:** P20–P23 are Wave-0-eligible in the same sense P1–P5 are (P21's `totp.rs` and
P22's Telegram adapter need no operator ruling; P20 and P23 have a named phase dependency but no
operator-decision dependency). None of them sit on the P3→P9→P10→P13 critical path (§2); they are
off-critical-path lanes exactly like P5/P8/P11/P12/P18-prep, fan out whenever capacity is idle.

#### 8.2 One cross-phase addendum, not a new phase — Hub design vs. vendor market research

[HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md](HUB-DESIGN-VENDOR-MARKET-RESEARCH-2026-07-17.md)
compared dowiz's hub design against general vendor-facing delivery-platform market patterns
(menu-as-data, external-channel bridging, multi-fleet dispatch, store-state/kitchen-load modeling,
multi-location semantics) and found six gaps (G1–G6). **This is deliberately not phase 24** — every
gap-closing item (D1–D6) extends an *existing* phase (10, 13, 15, 16) rather than standing alone, per
the addendum's own scope decision (no new phase, no new mechanism, every fix reuses something already
in the roadmap). One new operator decision came out of it:

| # | Decision | Blocks | Stakes |
|---|---|---|---|
| O20 | **Multi-location semantics** — is a "location" a sub-hub (P15 agent-recursion) or a flat intra-hub row? The legacy schema had organizations→locations; the mesh model never ruled this | Phase 16 (multi-location UI), the D6 addendum | Recommended default: intra-hub row for v1, sub-hub-as-target for later scale — flagged overridable, not forced |

The doc's own counterweight, restated because it is easy to lose in a gap list: dowiz is **ahead** of
every platform it was compared against on offline resilience, customer-data ownership, zero take-rate
economics, and courier dignity (NO-COURIER-SCORING is a genuine differentiator, not a compliance
cost). The gaps are real; so is the lead.

#### 8.3 Roadmap completeness audit (2026-07-17)

A dedicated pass re-verified every blueprint in this roadmap plus H1–H4 (hermetic-remediation) and
B1/B3/B4/E1/E2 (the agentic-mesh and spectral-evolution arcs, separate worktrees) against live
code/tests, and appended a "Planning-protocol completion appendix" to 25 files where the blueprint's
own claims had drifted from what is actually built — always append-only, nothing rewritten, nothing
committed by the audit itself. Two findings apply across the whole roadmap, not to any one phase:

1. **"Landed" is branch-implicit.** The same claim ("still open" vs. "already built") can be true or
   false depending on which branch/worktree is checked out — confirmed independently on P07's dedup
   fix, B3's `TokenBucket::release`, and B4's crypto bench, each of which exists only on its own
   feature branch. Every appendix now names its branch explicitly; a reader who doesn't check the
   branch is the single most common way this roadmap goes stale in practice.
2. **The dominant staleness direction is "assumed-unbuilt, actually built."** Nine blueprints (P01,
   P07, P08, P12, H1, H2, E1, E2, and P06's own `v1-verify` gate contract — see §8.4) had real
   implementation land after their blueprint's evidence pass, with no header update recording it.
   The fix in each case was appending the landed state, not rewriting the design.

Two operator-decision items surfaced by this pass are not yet in §3's table and are added here:
**O2b** (P14's reproduced F2 dispute table silently dropped a phrase load-bearing to Contradiction A —
needs a re-derivation, not a ruling) and a note that **O18** and **O19**'s "resolved" status in three
2026-07-16 docs contradicts MEMORY.md's own record of them as still-open blockers — named, not
adjudicated, exactly per this roadmap's own rule of recording contradictions rather than picking a
side silently.

#### 8.4 P06's merge-gate contract is now executable (found during the audit, not separately built)

`tools/ci-truth/src/v1.rs` implements BLUEPRINT-P06 §2–§5's anchor loader, TLV
encode/decode, and merge-gate policy as a real, tested Rust module — with signing
behind an explicit `Signer` trait whose only production implementation
(`UnsignedSigner`) honestly emits `"signed":false`, exactly mirroring
`main.rs:423`'s existing placeholder. This is *not* a violation of P06's own hard
precondition ("no signing until Phase 3 closes C4b on `mod_l`") — it contains no
signing, only the policy the signing eventually plugs into.

STATUS (2026-07-17):
- (a) **DONE** — the module now has a `#[cfg(test)]` suite (8 contract tests:
  TLV round-trip, K≠V load invariant, and the §5 merge-gate policy covering the 3
  mandated RED cases — missing attestation note, key_K==key_V self-sign, residue
  missing — plus red-line-touch honesty and GREEN-required-on-red-line). `ci-truth
  v1-verify <sha>` is wired and runnable; verified 27/27 ci-truth tests green,
  0 warnings. The contract is now falsifiable by this roadmap's own bar.
- (b) **OPEN (operator-gated)** — per P06-EXECUTION-PLAN-2026-07-17.md §2, the
  dowiz-side verifier still needs a *real* Ed25519/ML-DSA-65 verify-only
  implementation behind this contract. `v1.rs`'s `digest32` is explicitly a
  placeholder (`git hash-object`, not sha3-256), named as such in its own comment —
  not a finished crypto primitive. The `Signer` trait slot is left open for the
  bebop2 hybrid (Ed25519⊕ML-DSA) implementation that lands after Phase 3 closes C4b.
  Until then, `v1-verify` correctly emits RED on any commit lacking the two git
  notes, which is the honest Phase-1 behavior.

#### 8.5 Native-only cleanup — tracked, not fully executed

Per the operator's standing direction (no Python/Node runtime code outside adapters/bridges): this
session deleted genuinely dead artifacts (14 one-off `audit/*.py` scripts that manually poked
`apps/api` endpoints deleted with the rest of the JS/TS stack; two stale root-level duplicates of
`eval-layer/{metrics,openrouter_judge,eval_runs}.py`; an unused `.venv-paddle/` OCR experiment
directory). **Not deleted, and why:**

- `tools/eqc/eqc.py` — actively wired into `.github/workflows/ci.yml`'s `eqc-proofs` job. Deleting it
  without a Rust replacement would break CI. Tracked as a named follow-up: port to Rust under Phase 1
  (CI Truth Floor) once someone picks it up — not silently left, not silently deleted.
- `tools/skillspector-rs/gen_rules.py` and `tools/skillspector/src/skillspector/`'s Python source —
  this is a legitimate bridge, not a dinosaur: `gen_rules.py` parses the Python analyzer source as its
  "source of truth" (its own comment) and generates `skillspector-rs`'s `rules.rs` from it. The Python
  never runs in production; it is a code-generation input, the exact "adapter, not runtime" exception
  the operator's own direction allows.
- `tools/loop-signals/transcript_events.py`, `tools/telemetry/test_ser.py`, `kernel/benches/bench_track.py`
  — not re-audited this pass; flagged here so they are a known open question, not an assumed-clean
  item.

#### 8.6 One more phase (P24) — native runtime telemetry (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 24 | Native Runtime Telemetry — ring-buffer flight recorder + explainable latency events | [BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md](BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS-2026-07-17.md) | 8 (consumes P08's typed-schema/local-sink design and generalizes P08 §4's claim-latency anomaly pattern from CI commit latency to any runtime latency — decided from P08's own text, not assumed; P24 re-owns none of P08's anchors) | Linux-technique port (procfs snapshot-of-byproduct counters, perf/eBPF sample+aggregate-in-place, SPSC acquire/release rings, PSI cause attribution, RRD max-preserving tiers); one new kernel module (`ring.rs`, SPSC-no-CAS per the RCI H1 lesson), zero new external deps; every anomaly logged as an explained capsule (baseline+rule+PSI+prelude), never a bare "spike detected". Off-critical-path lane like P5/P8/P11/P12 |

#### 8.7 One more phase (P25) — wave scheduling & resource-classed admission (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 25 | Wave Scheduling & Concurrent Agentic Execution — resource-classed admission control | [BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md](BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION-2026-07-17.md) | 24 (soft — consumes P24 W1b's PSI-extended gauge surface as the admission signal; pre-P24 the same `/proc/pressure` files are read directly) | Corrects the "8 cores" premise (live `lscpu`: 4 physical cores × 2 SMT); splits all wave work into CPU-bound-local (4 strict-core slots, `taskset -c 0,2,4,6`, `nice 10`), I/O-bound-dispatch (D_max = 16 default — C10K/work-stealing grounding: lanes blocked on LLM API don't occupy cores; bound is memory-per-agent + API limits), and local-inference (Ollama = CPU load, delegated to `OLLAMA_NUM_PARALLEL`). Two binding operator rules named: LOCAL-DECISION (admission computed natively from local procfs/PSI state, µs-scale, never a network round-trip) and CORE-BOUND (CPU work on real cores only by default). Retroactive wave classification appended to §2; proposed AGENTS.md standing rule in blueprint §6 (operator merges, not applied). One proposed pure module `kernel/src/admission.rs`; zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24 |

#### 8.8 One more phase (P26) — memory optimization & flow analysis (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 26 | Memory Optimization & Flow Analysis — raising the D_max ceiling | [BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md](BLUEPRINT-MEMORY-OPTIMIZATION-FLOW-ANALYSIS-2026-07-17.md) | 25 (consumes its D_max formula/admission predicate as the integration point; most units startable now) + 24 (soft — VmRSS/PSI gauges) | Web-grounded memory research applied to P25's memory-bound concurrency ceiling. ADOPTED: `kernel/src/memory_budget.rs` (`MemoryBudget`, TokenBucket's byte-budget sibling — reserve/release, no time-refill), byte-bounded LRU cache store behind the existing `BlockStore` trait (the exact-match LLM cache is unbounded today), `FileBlockStore` index-not-mirror (its `open` currently loads the whole store into RSS), and `retrieval/ppr.rs` dense→CSR delegation (O(n²)→O(nnz), removes a dual authority — the genuine reuse of the existing sparse machinery). KEPT: system allocator (no `#[global_allocator]` exists; Rust 1.32 precedent; measured trigger + `MALLOC_ARENA_MAX` fallback named). REJECTED honestly: bumpalo/hand-rolled arenas (ns saved vs 10-second network waits), PPR/graph-scored cache eviction (no production lineage, no entry graph exists), Tucker/CP/tensor-train (no embedding matrix exists), ARC (patent history, unneeded adaptivity). DEFERRED with named triggers: int8 embedding quantization (4×/~99% retention, fetched numbers — trigger: Layer-B index >100 MB), W-TinyLFU admission sketch (trigger: >10⁴ entries + measured hit-rate loss). Net effect: D_max's `MEM_PER_AGENT` becomes measured+enforced (`try_reserve` per lane) instead of an estimate — the mechanism behind P25's "raiseable to 24+". Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24/P25 |

#### 8.9 One more phase (P27) — fault-isolated decentralized architecture (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 27 | Fault-Isolated Decentralized Architecture — audit closure + circuit-breaker/bulkhead/supervision discipline | [BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md](BLUEPRINT-FAULT-ISOLATION-DECENTRALIZED-ARCHITECTURE-2026-07-17.md) | none hard; soft: 24 (breaker-snapshot surface), 26 (owns the A3 cache-cap fix — convergent double-detection recorded in both), GapWire W1 (transition→GapEvent wiring), 9/10 (per-peer breakers, not schedulable yet) | Two-part artifact: (1) a ranked 16-finding stability/performance audit of kernel/engine/tools/llm-adapters (worst: A1 head-of-line blocking wedges the live Telegram alerting pipeline forever — `rust-spool/src/main.rs:240-247` retries the queue head with no send-failure deadletter; also A2 `FileBlockStore::put` panics on disk I/O via an infallible port, A4 the Dispatcher's `workers` bound is dead code, A6 zero compaction on every append-only store with `metric.jsonl` at 2.7 MB growing live); (2) one new primitive `kernel/src/breaker.rs` (`CircuitBreaker`, `TokenBucket`'s failure-exposure sibling: EMA trip filter via `geo.rs::ema_next` + min-calls floor, Open/HalfOpen hysteresis, transition-only event emission) + bulkheads at audited seams + OTP-grade restart-intensity policy for drainers + the "every port is fallible" rule. Research grounded in Armstrong/OTP supervision, Fowler breaker, Hystrix-deprecation lesson, reliability block algebra (series→parallel under verified independence), RFC 6298 EWMA + φ-accrual (deferred, named triggers). Proposes an AGENTS.md "Fault Containment" standing section (§6 there, operator merges) tied to the existing `.specify`/openspec SDD pipeline. Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24/P25/P26 |

#### 8.10 One more phase (P28) — cache reference graph + hybrid tensor decomposition + bump arena (2026-07-17, same protocol as §8.1; OVERRIDES three P26 verdicts by explicit operator direction)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 28 | Cache Reference Graph + Hybrid Tensor Decomposition + Bump Arena — living-memory pattern applied to the LLM cache | [BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md](BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA-2026-07-17.md) | 26 (W3/W4 consume its M2 `BoundedStore` eviction seam; overrides its §0.3/§0.7/§0.8 verdicts — P26 carries a dated addendum pointing here); soft: Layer B (`HARNESS-LLM-BACKEND.md` §3.3 — semantic edges + tensor rung 3 activate when it builds), 24 (bench/telemetry surfaces) | Operator-directed override of P26's three rejects, planned forward. (1) `CacheGraph` (`llm-adapters/src/cache_graph.rs`): node = existing sha3-256 cache key interned by insertion order (chronology); edges = co-access (v1, sliding window, aggregation free via `Csr::from_edges` duplicate-summing), derivation (v1.1, `derived_from` provenance), semantic (at Layer B); query = existing deterministic `personalized_pagerank` seeded from recents (recall) → PPR-primary/LRU-tie-break eviction scorer with a replay A/B falsifier vs plain LRU — the living-memory blueprint §7 "cache prefetch" layer instantiated, HippoRAG-precedented (NeurIPS 2024). (2) Hybrid tensor ladder: (entry × entry × relation) tensor per RESCAL (X_k ≈ A·R_k·Aᵀ, ICML 2011) coupled with an (entry × feature) matrix (embeddings + PPR/degree/recency) per CMTF (Acar–Kolda–Dunlavy 2011); rung 1 buildable NOW = new deterministic `kernel/src/lowrank.rs` (fixed-K power iteration + deflation over existing `Csr::spmv`, fills `spectral_cache::Decomp`'s empty basis slot — `spectral.rs` is eigenvalues-only, live-verified); SQ/PQ quantization stays the complementary rung-4 track. (3) `kernel/src/arena.rs` `BumpArena` (zero-dep, `Vec<u8>` region, O(1) reset, `T: Copy`, degrade-closed heap fallback) at the graph/spectral rebuild site (≈2n+7 allocs/CSR rebuild, ≈n²+O(n)/dense charpoly call), claim stated on its own terms: ~2k malloc/free pairs → ≤8 bumps + 1 reset per pass, criterion A/B + Miri + byte-identical-output falsifiers. Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24–P27 |

#### 8.11 One more phase (P29) — latency elimination: Decision Compiler + measured latency levers (2026-07-17, same protocol as §8.1)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 29 | Latency Elimination — Decision Compiler (LLM-as-one-time-compiler → native DecisionUnits) + measured dispatch-latency levers | [BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md](BLUEPRINT-LATENCY-ELIMINATION-RESEARCH-AND-BRAINSTORM-2026-07-17.md) | GapWire/orchestrator arc (soft — DecisionUnit `Stale`-on-`GapEvent` invalidation rides its `triage` routing; the pilot can land advisory-only before it); 25 (soft — the cache-write stagger lands at its admission point); 21/G3 (soft — the draft-local/verify-remote trial shares its small-model precondition P-2) | Measured ground truth from today's own 1,000 API calls: p50 4.9 s / mean 10.6 s / p90 26.2 s per call, **99.3% prompt-cache-read already**, avg 1,232 output tokens ⇒ decode volume dominates (85–90%), network ≤1–2 s ceiling — so the operator's local-AI hypothesis inverts (local 4.8–10.5 tok/s is 5–15× slower than API decode) and the operator-ruled primary is the **Decision Compiler**: recurring question *shapes* compiled ONCE by the LLM into tested, provenance-stamped native Rust `DecisionUnit`s (`Decision::{Answer,Escalate}`, degrade-closed), invalidated by GapWire events (`skillspector-rs` `build.rs` rerun-if-changed semantics promoted to runtime), never self-certified (independent replay per RC-2/P7; red-line shapes operator-gated). Four in-repo precedents cited as proof of pattern (`is_redline` ci-truth `main.rs:237`, mesh `scope.rs:244`, hermes `gov_route` EV table, skillspector rules pipeline). Pilot = shape C1 model-tier routing (<1 µs decide, 30-case fixture vs policy v3.4, Stale-path test). Secondary adopt-now levers: output-token discipline + `effort: low` doer lanes, wave-dispatch cache-write stagger, 1-h TTL/pre-warm, doc-edit/wave separation (AGENTS.md rule — operator merges). Distillation deferred with named trigger; mesh cache-sharing filed to B2. Speculative section (S1–S8) kept clearly non-decided. Zero new external deps. Off-critical-path lane like P5/P8/P11/P12/P24–P28 |

#### 8.12 One more phase (P30) — Bebop2 mesh masterwork synthesis (2026-07-17, same protocol as §8.1; consolidates the 9-batch tensor/state/safety/consensus/network/equations/product audit)

| # | Phase | Blueprint | Depends on | Note |
|---|---|---|---|---|
| 30 | Bebop2 Mesh Masterwork — 9-batch synthesis: equations-first kernel organs, exactly-once/hysteresis correctness closure, arena/breaker/eigenvector substrate, capability-Sybil-proof mesh composition, staged product→kernel migration | [bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md](bebop2-mesh-tensor-hermetic-2026-07-17/BLUEPRINT-BEBOP2-MESH-MASTERWORK-SYNTHESIS.md) (synthesis over batches 10–18 in the same directory; `INDEX.md` there navigates) | **None hard for Waves 1–2** (startable now: eqc-rs→`geo.rs ema_next` wiring, `event_log.rs:330` `append_raw` (line corrected 2026-07-18; was `:359`, file changed, symbol unchanged) exactly-once port — a LIVE money-red-line bug on this branch, `hydra.rs` hysteresis, `order_machine` ρ=0 const, householder eig2x2 dedup, wasm-boundary clamps, then arena.rs/breaker.rs/eigenvector-R1-R3/gossip-epoch). **P06 (key_V)** — the standing 3-way blocker gains a 4th consumer: the DecisionUnit *signed* import-verdict form and the tamper-leg closure both plug into P06's `Signer` slot (unsigned local-replay import gate builds earlier; synthesis §6). **P28** — co-owned substrate: P30 W2 *builds* P28's `arena.rs` and rung-1 solver per the eigenvector-refactor plan (no second arena, no lowrank.rs). **P29** — design authority for DecisionUnit gossip (= Decision Compiler; P30 adds only epoch/import-gate/rollback-in-same-log). **RLS NOBYPASSRLS** (`docs/ops/P8-NOBYPASSRLS-FLAG.md`) — a SEPARATE parallel workstream, never folded in; it hard-gates only the W4 product T4 write-path lane. New operator rulings docketed: **R-1** 0x12→0x13 discriminant, **R-2** budget-unit semantics, **R-3** `RootDelegationPolicy`, **R-4** money-law eqc flip + S2 integer basis-points (+ optional C8 bilateral-memory flag). Operator verdicts applied as binding: Sybil-proof via asymmetric anchor-rooted capability issuance (Batch 7 PROVEN-VIABLE — Cheng–Friedman's own asymmetric escape class; `verify_chain` already implements it), reputation/scoring/watchdogs/proxies rejected on physics+red-line. Verdict ledger: 14 ADOPT (+4 gated), 10 EXTEND, 17 ALREADY-EQUIVALENT, 16 DEFER-with-numeric-trigger, 19 REJECT-on-physics; zero concepts dropped (Batch 6 §5.1 completeness sweep is the spine). Zero new external deps in Waves 1–2. Off-critical-path lane structure like P5/P8/P11/P12/P24–P29, but W1-L2 (exactly-once port) is a correctness red-line item and should not idle |

---

### 9. Consolidation pass (2026-07-17, Layer I) — the altitude axis, the master index, and the
### would-be-lost ledger

Appended by the CORE-ROADMAP Wave-3 consolidation pass (same append-only rule as §7/§8). Full
execution detail: `CORE-ROADMAP-2026-07-17/BLUEPRINT-P-I-consolidation.md`; ground-truth audit:
`CORE-ROADMAP-2026-07-17/P-I-audit-cross-repo-consolidation.md`.

#### 9.1 The Layer A–I altitude axis and the master index

The numeric phases **P01–P30 in this document remain the sole execution numbering — nothing is
renumbered.** The CORE-ROADMAP effort's letter groupings are ratified as **`Layer A..I`** (the
former "P-A..P-I" spelling is retired from prose to kill the P-D/P04 lexical collision; on-disk
`BLUEPRINT-P-X-*.md` filenames keep their provenance names). Each Layer is an **altitude lens over
a cluster of numeric phases**, not a phase itself — the full crosswalk table (Layer ↔ numeric
phases ↔ blueprints ↔ arcs) and the navigation map of the whole planning corpus live in
**[`CORE-ROADMAP-INDEX.md`](CORE-ROADMAP-INDEX.md)**, which is this roadmap's companion index
(this doc stays the canonical WHAT/WHEN; the index is the canonical WHERE). The Layer A–I axis
descends directly from `MASTER-EXECUTION-PLAN-2026-07-13.md`'s ground→core→surface→platform
spine — lineage stated, not re-derived.

The five older master docs (`MVP-2026-07-12` root, `BUILD-SEQUENCE-UPDATED-2026-07-11`,
`INTEGRATION-PLAN-2026-07-14`, `10-PHASES-2026-07-14`, `EXECUTION-PLAN-2026-07-13`) now carry
SUPERSEDED banners pointing here — preserved in full as audit trail, never planned against.

#### 9.2 Would-be-lost ledger (P-I audit §3) — all six dispositioned, zero silent drops

| ID | Item | Disposition (executed 2026-07-17) |
|---|---|---|
| L1 | Update-blob **code-signing** (ML-DSA verify vs pinned root; `kernel/src/pq/codesign.rs` is live on this branch) | Folded into **Phase 10** — boot/update-integrity unit note appended to `BLUEPRINT-P10` |
| L2 | Transport bake-off rationale (Zenoh/Reticulum/TCPCLv4/BIBE; libp2p rejected) | `docs/transport-research-2026-07-12.md` **restored from git blob `94e257fe9`** + cross-linked from `BLUEPRINT-P09` |
| L3 | Courier out-of-app notification/wake path (`NotifyHub`/VAPID lineage) | Folded into **Phase 13** — delivery semantics dissolved-by-mesh (courier node receives `MeshEvent`s directly); out-of-band device-wake kept as a P13 sub-unit, xref P08 alerting |
| L4 | Anonymous `.onion`/Tor tier | **E53-form waiver** — what: anonymity/Tor access tier; why-suspended: no vendor-node tier exists and no anonymity demand demonstrated; trigger to revisit: vendor-node tier ships AND a venue requires anonymity. **→ ACTIVATED 2026-07-18** (direct operator request supersedes the trigger — recorded, not silent): phase **P53**, §14 below; blueprint `CORE-ROADMAP-2026-07-17/BLUEPRINT-P53-tor-onion-integration.md` |
| L5 | "Lost reports" honesty ledger (13 + ~20 pre-2026-07-12 reports) | Closed-as-lost, decisions survive in `UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3`; not resurrected (would violate ground-truth discipline) |
| L6 | Self-development research queue (causal/do-operator, category-theory functorial mapping, info-geometry, integer laws) | **Deliberately NOT a numeric phase** — separate always-running axis; indexed from `CORE-ROADMAP-INDEX.md` → MEMORY `physics-math-exploration.md` |

---


### 10. Ecosystem-Component Consolidation (2026-07-18) — the second axis, the swarm-ready phase
### set, and the critical path to first-deployable dowiz

Appended by the 2026-07-18 ecosystem-consolidation pass (same append-only rule as §7/§8/§9).
Companion index registration: `CORE-ROADMAP-INDEX.md` §0 (component axis) and §9 (orphaned-arc
absorption table).

#### 10.0 Why this section exists

The operator found ~12 separate planning arcs (150+ blueprint units: field-ui-engine,
dowiz-interfaces, rust-engine-rewrite, integration-ports, ecosystem-strategy, ops-reliability,
mesh-real, docker-swap, math-first-architecture, hydraulic-loop-v2) scattered outside canonical
navigation — several with ZERO reference from `CORE-ROADMAP-INDEX.md`, some carrying stale or
false "already done" claims. The operator's instruction, verbatim: **"один роадмап, не декілька…
нічого не загубити, нічого не добавляти що не критично… чітким розподілом на частини
екосистеми"** — ONE roadmap, nothing lost, nothing added beyond the critical, clearly divided by
ecosystem part, organized so a swarm can execute waves WITHOUT re-deriving anything
(self-sufficient DoD/anti-scope per phase), with the product supporting three operating modes
(no-AI / local-offline-AI / connected-AI) and the core delivery flow (orders/courier/money) never
requiring AI at all. Every claim in §10.1–10.5 was live-verified against running code/tests on
2026-07-18 by parallel research passes before being written here — not inherited from any arc
doc's own claims, several of which turned out to be false or stale at the source.

**Integrity-check notes (assembly pass 2026-07-18 — reported, not silently fixed):**

- Phase headings P31–P46: every number present exactly once across §10.5 — no gaps, no
  duplicates. Two structural notes: **P38** exists only as the deliberate split **P38a/P38b** (no
  bare P38 heading; rolled up as one P38 row in §10.2), and PROTOCOL numbered its second phase
  **P34B** (capital-letter sub-phase of P34 — same sub-letter convention as P31a/P38a, rolled
  into the P34 row in §10.2).
- Two source units are deliberately split in half across two phases, each half explicit at both
  ends (intentional, NOT double-counting): **MESH-09** (BPv7 half → P34; iroh half → P34B) and
  **P21** (executor half → P40; mode/degradation half → P41).
- Units with NO absorbing phase found (open audit items) — **RESOLVED 2026-07-19**, see
  `GAP-A1-DISPOSITION-AUDIT-2026-07-19.md` for full reasoning: **MESH-14** needs one small
  docs+CI-lint blueprint (its live-test-citation rule folds into the Q1 claim-verification
  checkpoint rather than duplicating it). Of the IP-* set: **IP-01/02/03/04/05/07** are already
  covered or already satisfied by P40/P42 (cross-reference-only fix, no new blueprint); **IP-06**
  (`QualityGovernor`) is a genuine future-blueprint candidate, deferred until engine rendering
  work is next active (nothing to degrade from yet — no live GPU pipeline); **IP-09** overlaps
  today's P95 living-memory work, redirect there; **IP-17/IP-18** correctly stay operator-gated
  (crypto red-line, by their own text — not actioned without explicit sign-off); **IP-21** is
  verification scaffolding, inherently downstream of the items it aggregates.

#### 10.1 The Ecosystem-Component axis — a THIRD lens

Orthogonal to BOTH the P01–P46 numeric phases AND the Layer A–I altitude axis (§9.1) — a third
navigation lens, not a replacement for either. Row order below IS the critical path.

| Component | Mission | Owns | Position on critical path | Current completion (live-verified 2026-07-18) |
|---|---|---|---|---|
| **CORE** | decide/fold Law, event-log, money, capability primitives, spectral math, self-tuning control loops | P31–P33 | 1st — the foundation, already sufficient for everything downstream | ~90% done, not the bottleneck; dominant failure mode is built-but-unwired code |
| **PROTOCOL** | mesh, capability issuance, crypto, transport, delivery-domain (bebop2) | P34–P36 | 2nd — THE wiring lever | ~70% built and PROVEN but 100% stranded from dowiz's own kernel — the single biggest lever in the whole roadmap |
| **DELIVERY** | the dowiz product surface: UI, order/courier/payment flow, auth, demo/marketing, app-shell | P37–P39 | 3rd — the first-deployable target | ~0% deployable (no HTTP server, no rendered UI, no live deployment anywhere) but the underlying math/domain logic is mostly done in CORE+PROTOCOL — wiring-heavy, not a from-scratch build |
| **AGENT** | local/network AI, tool-use loop, MCP; three operating modes (no-AI / local-offline / connected) | P40–P42 | 4th — scaffolds in parallel with DELIVERY, offline-first by construction | substrate (LlmBackend/Ollama) shipped, but the executor loop connecting it to anything is 0% — a chat backend today, not an agent |
| **ECOSYSTEM/OPS** | external integrations, deployment, monitoring, multi-product platform | P43–P46 | 5th — explicitly and deliberately LAST | near-zero built, correctly so — nothing exists yet to integrate/deploy/monitor |

**The critical path to first-deployable dowiz:** CORE (already sufficient) → PROTOCOL **P34**
(wire the proven mesh delivery-domain into the dowiz kernel — THE highest-leverage single phase
in this entire document) → DELIVERY **P37+P38** (thin HTTP surface + WebGPU render, both largely
wiring once P34 lands) → AGENT **P40+P41** (tool-loop + three-mode proof, can scaffold in
parallel with DELIVERY, offline-first by construction) → ECOSYSTEM/OPS **P43–P46** (only once
something is live). **P34 is the single most important next action across the entire unified
roadmap** — bigger leverage than any other phase, because it converts ~70% of already-built,
already-tested protocol code from stranded to load-bearing.

#### 10.2 Full P31–P53 index (swarm fast-lookup; sub-letter detail lives in §10.5, P47–P53 full sections in §11–§14)

> Extended P47–P53 on 2026-07-18 by the consolidation/consistency pass §11's note anticipated
> ("a later consolidation pass reconciles that table" — this is that pass). Same-day swarm
> landings folded into the P40/P41/P42 status cells with commit hashes.

| Phase | Component | Name | Status | Absorbs | Depends on | Blocks |
|---|---|---|---|---|---|---|
| P31 | CORE | Math-First Kernel (S0–S7 + Master Integration Tier A/B) | DONE-heavy: P31a DONE-ledger; P31b WIRING-GAP; P31c PARTIAL; P31d PLANNED; P31e PARTIAL | 17 units: S0–S7, A1–A3(mip), A6, B1–B5(mip) | nothing hard (P31d targets frozen P31a organs) | nothing on critical path |
| P32 | CORE | Hydraulic-Loop-v2 Wiring (self-tuning control loops) | P32a DONE; P32b/P32c WIRING-GAP; P32d PLANNED | 10 units: BP-01/02/05–10/22 + cross-model critic | P32d impl soft-deps AGENT LlmBackend wiring | nothing |
| P33 | CORE | CORE Ledger Hygiene (cleanup + status audit) | PLANNED (audit/flag only, zero build) | 15 units: BP-03/04/11/12–21/23 + JS-spike artifacts | nothing | P33b should precede new P32 sub-letter claims |
| P34 | PROTOCOL | Wire mesh-real's proven delivery-domain into the dowiz kernel (+ P34B planned mesh halves) | P34 WIRING-GAP (all absorbed units DONE); P34B PLANNED | 13 units: MESH-01–13 (MESH-09 halves split P34/P34B; MESH-14 unaccounted — §10.0) | CORE Law (sufficient today); P34B also P36 DoD-2 + P34 | DELIVERY P37, P13 wire-side, P34B; formalizes AGENT-arc's MESH-11 borrow |
| P35 | PROTOCOL | Docker-swap: zero-OCI runtime home (registration + finish) | PARTIAL (DK-01/02/03/07 DONE; DK-04/05/06/08 PLANNED) | 10 units: DK-01–10 | independent lane — no ordering dep on P34 | DK-06 feeds P45 deployment; wasm-host feeds AGENT tool ports |
| P36 | PROTOCOL | Bebop 5-expert remediation (2 live regressions first) | 🔴 REGRESSION ×2 (no_std wasm32 RED; insecure-TLS default-on); remainder PARTIAL | 5 units: bebop review P0–P4 (C1–C7 closures inventoried) | parallel to P34 — never serialize P34 behind it | DoD-2 blocks P34B iroh; DoD-1 blocks wasm32 consumers of bebop2-core |
| P37 | DELIVERY | Minimal HTTP/API surface for orders | PLANNED (0% — no dynamic HTTP server exists in-repo) | 1 unit: RW-09 (+ unblocks P23-P3; supplies wire half of P13) | P34 for mesh-backed data (can start against local delivery-domain) | P23-P3, P13, P39, AGENT P40 real tool target, P45 (hard) |
| P38 | DELIVERY | WebGPU render engine (P38a) + Sea & Sheet surfaces (P38b) | P38a PARTIAL (math substrate DONE, GPU path 0%); P38b PLANNED (0%) | 27 units: FE-04(=RW-04)/05–07/10–16 + RW-01/05/10/11 (P38a) · DZ-01–12 (P38b) | O18a graphics-unlock (hard, environment-gated); P38b ← P38a + P37/P34 | P38b; P17 splat-tier closure |
| P39 | DELIVERY | App-shell: installability + capability-auth wiring + offer math | PLANNED (installability undecided; P23-P1 and P20 DM-1 unblocked today) | 1 new unit (installability gap) + hosts P23-P1/P3 wiring and P20 DM-1 (their numbers unchanged) | P37 (auth wiring); P38a/b (installable target) | P17/P20 demo credibility; step-up auth for AGENT flows |
| P40 | AGENT | AgentLoop executor + tool-calling capability wiring | PARTIAL — 2026-07-18 swarm landed `kernel/src/agent/loop.rs` AgentLoop (fail-closed, `626236886`/`e25e9fed8`); was "PLANNED (loop 0 grep hits)" | P21 (executor half) | P37 for the real read-order tool (scaffold now against a stub) | P41, P42 |
| P41 | AGENT | Three-mode operation: no-AI / local-offline / connected | PARTIAL — 2026-07-18 swarm landed `kernel/src/ports/llm.rs` AiMode + BackendConfig::from_env (fail-closed, default Off, `e74fc3e4f`/`4d8e292b0`); parity proof + full degradation contract still open | P21 (mode/degradation half) + operator three-mode directive | P40 (DoD-1 no-AI proof landable today, before P40) | P42 |
| P42 | AGENT | MCP port + agent-as-capability boundary | PARTIAL — 2026-07-18 swarm landed `kernel/src/ports/mcp.rs` + `ports/tool.rs` capability-scoped tool boundary (`575a75a20`/`09b2c7edd`); was PLANNED | 1 unit: IP-08 | P40 + P41 | ECOSYSTEM external consumption of AGENT tools |
| P43 | ECOSYSTEM/OPS | External integration ports (messenger/marketing/export/backup/hosting) | PLANNED (+1 live QRNG-endpoint bug fixable now) | 6 units: IP-11/12/13/14/19/20 (IP-10/15/16 → existing P22, not renumbered) | DELIVERY P37/P38; PROTOCOL P34 | nothing on critical path |
| P44 | ECOSYSTEM/OPS | Cache layers (EC-05) + own-RAG/own-inference scale-out | PLANNED (0/5 layers) — LOW PRIORITY / FAR-FUTURE | ~9 units: EC-05 + own-inference/RAG/chunking/gossip units of EC-03/04/06/08/12–15 | AGENT P40/P41 real traffic; DELIVERY P37 load | nothing; nothing waits on it |
| P45 | ECOSYSTEM/OPS | Deployment + monitoring floor (minimum viable ops) | PARTIAL — barely; the arc's own premise (attic) is gone | 22 units: OPS-01–22 | HARD-blocked by P37; data-layer items gated on pgrust-rebuild /council | P46 |
| P46 | ECOSYSTEM/OPS | Multi-product platform ("dowiz Local" + marketplace) | PLANNED (0%) — FURTHEST FUTURE | EC-17 + multi-product/marketplace remainder of the EC arc | everything above: P37/P38 live, P45 green (incl. off-site backup), P43 ≥1 port | nothing — terminal node of the roadmap |
| P47 | DELIVERY | Payment & settlement rails (cash → crypto → processors) | PARTIAL — ruling LANDED (Wave 0 = cash, §11); Wave-0 rail code landed 2026-07-18: `kernel/src/ports/payment.rs` PaymentPort + CashAttestation + reconciliation + `tests/firewall_p47.rs` (`e6367ae73`/`de56a27d6`) | none — genuinely new | P37 (order surface to settle against) | nothing on wiring critical path; prerequisite for P50's first-real-order gate |
| P48 | DELIVERY | Owner/Admin operational surface (omnichannel hub) | PLANNED — rulings LANDED (WebGPU no-DOM-exemption; hub model, §11); build-out open | none — new (silence-ledger item 2) | P37 (auth + API); P38a only conditionally (ruling made it unconditional) | P50's first-real-order gate (a real venue needs a managed menu) |
| P49 | DELIVERY | Customer identity, notification & tracking UX | PARTIAL — ruling LANDED (deferred to 5–50 real clients; Wave-0 default = per-order capability grant, §11); grant identity code landed 2026-07-18: `kernel/src/ports/customer.rs` (option 2, privacy-minimal, `f55ff8911`/`69bdb2a71`) | customer-side closure of P43's corrected claim (§10.5.5) | P37 (wire), P38a/b (tracking render), P43 DoD-2 (send path) | P50's first-real-order gate ("real customer" leg) |
| P50 | ECOSYSTEM/OPS | Legal/compliance & first-order validation gate | PARTIAL — audit half ON DISK (`CORE-ROADMAP-2026-07-17/P50-COMPLIANCE-AUDIT.md`, `568ff51c4`/`788cbee5a`); first-order gate open | G11 + old-stack legal-surface audit obligation | audit: nothing (startable now); gate: P47/P48/P49 + P34/P37/P38 critical path | P46 (and any scale-out) |
| P51 | DELIVERY | Open map + routing: OSM vector, field-rendered routes, pin-drop, live tracking | PLANNED (blueprint ON DISK; kernel router landed pre-phase in P04) | none — feeds/closes P04 router + `route_js` gap, P49 DoD-4 supply, splatting Stage-1 | P38a (render legs; CPU compose works today), P34/P37 (wire/asset ride) | P49 DoD-4 (TrackFrame consumer), splatting arc Stage-1; feeds P50 audit (ODbL row) |
| P52 | DELIVERY | Courier working surface: shift, claims, run, PoD, earnings | PLANNED (blueprint ON DISK; protocol side already the most-built part of stack) | none — executes DZ-08, MVP-audit M1/M4/M10 seams | P34, P38a, P51, P37, P39 (K6), P48 (roster), P47 (attestation) | nothing downstream, but itself MVP-blocking: P50's gate cannot go green without it |
| P53 | DELIVERY | Tor/onion integration: anonymous-access tier, Onion-Location + QR | PLANNED (blueprint ON DISK; W0 buildable today) | fold-in ledger L4 (activated 2026-07-18) | W0: nothing; W1 (live onion service): P37 + P45, operator-run | nothing — feeds P48 share panel, P52 K6 (QR encoder), P50 audit (privacy-tier row) |
| P54 | AGENT | LLM/agent behavioral verification: adversarial probes, money-trust fence, fine-tuning gate | PLANNED (blueprint ON DISK; fine-tuning explicitly DEFERRED, zero LoRA/QLoRA built) | none — new phase, consumes P21/P40/P41/P42 | P21 (backend), P40 (AgentReasoner seam), P56 (storage/scheduling substrate) | none downstream; strengthens P54→P56 alerting only |
| P55 | PROTOCOL/CORE | Protocol/ecosystem testing: regression taxonomy, proptest/mutation, chaos-injection | PLANNED (blueprint ON DISK; proptest confirmed already-live dev-dep, 400-case suite) | none — new phase, extends P24/P27/P36 | P27 (CircuitBreaker), P24 (flight-recorder spans), P56 (storage/scheduling) | none downstream; feeds P36/P34 regression coverage |
| P56 | ECOSYSTEM/OPS | Verification-harness shared infrastructure: storage, scheduling, meta-verification | PLANNED (blueprint ON DISK; 4 meta-detectors designed, `hetzner:dowiz/test-results/` sync policy) | none — new phase, shared substrate for P54+P55 | P25 (admission control, extended not forked), P45 (alerting, extended not forked), disk-cleanup pass (local storage now unblocked) | P54, P55 (both consume this as their storage/scheduling substrate) |

#### 10.3 Cross-cutting invariants (binding across components; each stated once)

1. **Three-mode operation (no-AI / local-offline-AI / connected-AI).** DELIVERY's core
   order/courier/money flow NEVER requires AI to function — already true by construction (CORE's
   decide/fold Law is pure deterministic Rust; LLM is "a feeling at the edge," never in the
   decision path). If every AGENT phase were deleted tomorrow, orders would still place, couriers
   would still match (deterministic HRW), money would still settle. P41 is the enforcement phase;
   the invariant binds DELIVERY and CORE too — they must never introduce an AI dependency in the
   critical order/money path.
2. **Offline-first / solo-island** (ARCHITECTURE.md F12; proven live by
   `delivery-domain/intake.rs::ac6_solo_island_full_flow_no_peers` — full order→delivery with
   ZERO peers). Binding on DELIVERY P37 (order placement must not require network — P37 DoD-5)
   and AGENT P41 (mode-2 network-isolated proof — P41 DoD-4).
3. **Capability-cert auth model.** Capability certificates (proto-cap, ML-DSA-signed,
   `HybridGate`/`verify_chain`/`RevocationSet` — all already built) are the PRIMARY auth model;
   conventional password+TOTP is never primary (D3 device-bound keypair primary; TOTP/WebAuthn
   are step-up only). Binding on DELIVERY P37 DoD-4 and P39.
4. **WebGPU/field-render UI, never DOM-first — and its input complement, the intent-interface.**
   The UI is a WebGPU/WASM render of backend physics-field state; DOM survives ONLY as FE-15's
   invisible AccessKit mirror for screen-reader/IME input. Binding on DELIVERY P38a/P38b.
   *Reframed 2026-07-18 (operator directive — owner/client/courier must never need to think
   long, dig in, google, or click through menu trees):* the input half of this invariant is the
   **intent-interface** — every modality (touch today; voice/gesture at DZ-10's unchanged
   Phase-9b slot) is the SAME `Intent{FieldPos, magnitude}` → `S`-field-impulse mechanism
   (IP-05's 8-parameter operator, INTENT→`S`; IP-07 superposition `S₁+S₂`), and this is WHY the
   field-render (P38) and the local-agent loop (P40) exist: the surface answers INTENT rather
   than requiring conventional menu-tree navigation. Load-bearing UX philosophy from day one
   (`Intent`/`InputSource` land in P38b DoD-1); voice later only ADDS a backend to the already
   load-bearing mechanism — sequencing unchanged (P38b DoD-3 stands), framing corrected (see
   the DZ-10 framing note in `docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md`).
5. **Compilation-firewall pattern (repo-wide).** Consumers reach protected surfaces only through
   a facade whose lack of direct kernel imports is proven by `cargo tree` + a committed
   red-proof. Three instances, one pattern: PROTOCOL's KernelFacade
   (`proto-cap/src/facade.rs:123 submit_intent` — line corrected 2026-07-18, file grew, symbol
   unchanged, MESH-02), AGENT's ToolPort (P40 DoD-1), and the
   MCP layer (P42 DoD-3).
6. 🔴 **Two live regressions** demanding attention regardless of critical-path sequencing (P36
   DoD-1/DoD-2): bebop's `no_std` wasm32 build is RED right now
   (`cargo build --target wasm32-unknown-unknown --no-default-features -p bebop2-core` fails with
   `E0425` at `at_rest.rs:74`, regression from `d23e7aa` post-dating the remediation doc's own
   GREEN claim), and `proto-wire` ships `default=["insecure-tls"]`. Fix opportunistically —
   don't let them block P34, but don't let them rot either.

#### 10.4 Demo/offer pipeline note (operator: do not lose)

P20 (Demo & Marketing Pipeline, DM-1..DM-8) stays live-numbered as-is — this consolidation
neither renumbers nor absorbs it. Its unblocked entry point **DM-1 (kernel discount math)** is
additionally hosted inside DELIVERY P39's DoD as the concrete next actionable step, since P39
already needs `compute_order_total` (`kernel/src/domain.rs:129`) extended. DM-2..DM-8
(publishing/marketing content pipeline) remain P20's own scope, gated behind P18 public-flip,
untouched by this consolidation.

#### 10.5 Full component sections

The five component drafts, pasted verbatim below (headings demoted one level to nest here; zero
content trimmed; each drafted and live-verified by an independent parallel pass on 2026-07-18).

#### 10.5.1 CORE — decide/fold Law, event-log, money, capability primitives, spectral math, self-tuning control loops

**Position on the critical path:** CORE is ~90% done and is NOT the bottleneck. The critical path runs CORE → PROTOCOL (wire stranded mesh to the dowiz kernel — the biggest lever) → DELIVERY (HTTP server + auth + UI render, ~0%) → AGENT (tool-use loop) → ECOSYSTEM/OPS. This section's job is to number and index what exists so nothing is lost, and to name the few real remaining gaps. The dominant failure mode in CORE is not missing code — it is **built-but-unwired code** (status `WIRING-GAP` below): modules that compile, pass tests, and are called by nothing.

**Absorbed source arcs:** math-first-architecture (S0–S7), Master Integration Plan Tier A/B (2026-07-14), hydraulic-loop-v2 (BP-01..23).

---

##### P31 — Math-First Kernel (S0–S7 + Master Integration Tier A/B)

###### P31a — Math-first DONE ledger
**Absorbs:** S0, S1, A3(mip), S2, S4, S7, A1(mip), A2(mip), B1, B2, B4, B5
**Status:** DONE
**Role & responsibility:** The completed body of the math-first rewrite. Indexed here so every old unit ID resolves to a P-number; no further build work.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` (§11 has full Layer-A DoD for A1/A2/A3/A6); `docs/design/BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN-2026-07-17.md`; `docs/design/MASTER-INTEGRATION-PLAN-2026-07-14.md`.
One-line ledger:
- **S0** eqc equation-compiler — DONE. `tools/eqc-rs/` (16/16 tests) → generated organs in `kernel/src/eqc_gen.rs` (`ema_next_f64`=A2, `apply_tax_exclusive_int`/`apply_tax_inclusive_int`=A3). Already numbered A1/A2/A3/A6 in the Layer-A canon.
- **S1 + A3(mip)** eigensolver consolidation — DONE via `kernel/src/spectral.rs::topk_symmetric` + `kernel/src/householder.rs::eigh_contig`/`eig2x2`. The old "3 duplicate Jacobi solvers, dowiz+bebop2 dual-authority" hazard is resolved.
- **S2** money=integer — DONE (`kernel/src/money.rs`, `domain.rs`, `cart.rs`, all i64).
- **S4** zero-copy bridge — DONE on the CORE side (`engine/src/bridge.rs::VertexBridge`, real CPU staging copy). GPU path stub is DELIVERY's concern, not CORE's.
- **S7** mesh-shaped substrate — PARKED by its own plan; becomes PROTOCOL's concern when it happens. Not a CORE item.
- **A1(mip)** "fix 3 broken backup scripts" — MOOT: scripts deleted with the `apps/` purge. Superseded by B4.
- **A2(mip)** living-knowledge recall — DONE in a different, better shape than planned: native Rust `kernel/src/retrieval/{ppr,bm25,recall,diffusion,spine,memory_store,index}.rs` (2879 lines, 9 tests), wired at `kernel/src/lib.rs:151,176`, consumed by the self-improvement loop via `recall.rs::PrimaryRecall`. (The JS spike it replaced is dead — see P33a.)
- **B1** Kalman filter — DONE (`kernel/src/kalman.rs`; `geo.rs::ema_next` confirmed as its bit-identical 1D special case).
- **B2** micrograd-autodiff — DONE (`kernel/src/micrograd.rs::Value::backward`).
- **B4** Rust-native backup organ — DONE (`kernel/src/backup.rs`, 702 lines, Buzhash-CDC dedup, crash-atomic). ⚠ Never exercised end-to-end — there is no live deployment to back up yet. That exercise is an **OPS/DELIVERY dependency**, not a CORE gap; ECOSYSTEM/OPS must schedule a real end-to-end backup run once minimal DELIVERY exists.
- **B5** trigram search — DONE (`kernel/src/trigram.rs` + `retrieval/index.rs::TrigramIndex`).
**Anti-scope:** Do not touch any of the above. Do not "improve" DONE organs while wiring gaps remain elsewhere.
**Depends on / blocks:** B4 end-to-end exercise blocked by DELIVERY/OPS deployment existing. Nothing here blocks the critical path.

###### P31b — CORDIC int-mode emission
**Absorbs:** A6 (Layer-A canon residual)
**Status:** WIRING-GAP
**Role & responsibility:** CORDIC exists (`tools/eqc-rs/cordic.rs`) but eqc-rs's Sin/Cos int-mode emission does not route through it. Close the last Layer-A gap so trig in integer mode is compiled, not floated.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P-A-kernel-primitives.md` §11 (A6 DoD already written there — reuse it, do not re-derive).
**DoD:**
1. An eqc-rs equation containing Sin/Cos compiled in int-mode emits calls into `cordic.rs` routines (verifiable by inspecting generated output).
2. eqc-rs test suite stays green (16/16 baseline) plus at least one new test asserting int-mode Sin/Cos output matches CORDIC reference values.
**Anti-scope:** Do not rewrite CORDIC — it exists and is correct. Do not add new trig ops beyond Sin/Cos. Do not touch float-mode emission.
**Depends on / blocks:** Depends on nothing. Blocks nothing on the critical path; pure Layer-A completeness.

###### P31c — no_std kernel
**Absorbs:** S3 (residual half; SIMD half of S3 is DONE — `kernel/src/simd.rs`, AVX2 softmax with bit-identical fallback)
**Status:** PARTIAL
**Role & responsibility:** Make the kernel crate `no_std`-capable so it can live on constrained/bare-metal targets (Kernel-as-MCU north star) and embed cleanly in WASM/mesh contexts. Today the crate is plain `std` — zero `#![no_std]` anywhere.
**Blueprint:** none dedicated; math-first plan S3 entry is the source. Write the alloc-boundary inventory as the first DoD step, not a separate blueprint doc.
**DoD:**
1. `kernel` compiles with `#![cfg_attr(not(feature = "std"), no_std)]` (or equivalent feature-gated split) — `cargo build --no-default-features` green.
2. Full existing test suite still green under the default `std` feature — zero behavioral drift.
3. Modules that genuinely need `std` (I/O-bound: `backup.rs`, `retrieval/memory_store.rs`, etc.) are feature-gated, not rewritten.
4. CI has a `--no-default-features` build check so the property can't silently regress.
**Anti-scope:** Do not chase `no_std` for the SIMD module beyond what falls out naturally (it's already done and bit-identical). Do not rewrite I/O organs to be allocation-free — gate them behind `std`. Do not block any other phase on this; it is not on the critical path.
**Depends on / blocks:** Soft prerequisite for PROTOCOL-side embedding of the kernel into mesh/WASM substrates and the long-horizon Kernel-as-MCU thesis. Blocks nothing in DELIVERY/AGENT.

###### P31d — Verification ladder (z3/kani proofs)
**Absorbs:** S5
**Status:** PLANNED (zero hits repo-wide — genuinely not started)
**Role & responsibility:** Machine-checked proofs for the invariants the repo already treats as red-lines: money-integer arithmetic, tax organs, fold determinism. Converts "VERIFIED-BY-MATH" from a discipline into an artifact.
**Blueprint:** none — needs one before build. First deliverable IS the blueprint (candidate-invariant inventory + tool choice kani-vs-z3 with a falsifiable comparison per the tech-selection rule).
**DoD:**
1. A blueprint doc exists naming ≥5 candidate invariants ranked by (red-line severity × proof cheapness), with tool selection justified by an honest comparison, not appeal to authority.
2. At least one proof harness lands and runs in CI (e.g. kani proof that `apply_tax_exclusive_int`/`apply_tax_inclusive_int` never overflow/never go negative for the documented input domain).
3. The proof is demonstrated RED-able: a deliberately introduced off-by-one in a scratch branch makes it fail.
**Anti-scope:** Do not attempt whole-kernel verification — ladder means cheapest-rung-first, money organs before anything else. Do not add proof tooling as a hard build dependency (CI-only). Do not let this delay PROTOCOL/DELIVERY work; it is hardening, not critical path.
**Depends on / blocks:** Depends on P31a being frozen (proofs target stable organs). Blocks nothing; strengthens everything.

###### P31e — Equation-IR at runtime + online learner bridge
**Absorbs:** S6, B3 — **these are the same open item; counted once here, never double-count**
**Status:** PARTIAL
**Role & responsibility:** eqc-rs today is build-time-only codegen; `kernel/src/online.rs` (LinearSGD/ScalarAdam online learner) exists and is tested, but the two are connected only by a comment. The gap is a runtime representation of eqc's IR that the online learner can adjust parameters of — closing the loop from "equations compiled once" to "equations that learn."
**Blueprint:** none dedicated — math-first plan S6 entry + Master Integration Plan B3 entry are the sources. A small design note (runtime-IR shape, which parameters are learnable, determinism guarantees) should precede code.
**DoD:**
1. A runtime-IR type exists in the kernel that can represent at least the already-generated organs' equation class (EMA + tax forms).
2. `online.rs::LinearSGD` or `ScalarAdam` demonstrably updates a parameter of a runtime-IR instance across ≥1 test scenario, with the pre-update evaluation bit-identical to the corresponding `eqc_gen.rs` compiled organ.
3. The comment-only link between `online.rs` and eqc is replaced by an actual call path (grep-verifiable: a real symbol reference, not prose).
4. Determinism preserved: with learning disabled, runtime-IR evaluation == compiled-organ output, bit-identical.
**Anti-scope:** Do not port all of eqc-rs into the kernel — only the minimal IR subset the existing organs need. Do not make runtime-IR the default execution path; compiled organs stay canonical, IR is the learning surface. Do not invent new optimizers — SGD/Adam exist.
**Depends on / blocks:** Depends on P31a (S0 compiler as the IR source of truth). Feeds P32 (self-tuning loops get a learnable substrate) and long-term AGENT self-improvement. Blocks nothing on the critical path.

---

##### P32 — Hydraulic-Loop-v2 Wiring (self-tuning control loops)

Code lives in bebop-repo (`bebop2/core/`, `crates/bebop/`) plus one kernel-side organ; blueprints in `docs/design/hydraulic-loop-v2/` (`HYDRAULIC-LOOP-v2-PLAN.md`, `BLUEPRINTS.md`). The pattern across this arc: nearly everything was BUILT and TESTED; almost nothing was WIRED. P32's work is connection, not construction.

###### P32a — Hydraulic DONE+wired ledger
**Absorbs:** BP-05, BP-06, BP-08, BP-09, BP-10, BP-22
**Status:** DONE
**Role & responsibility:** The hydraulic-loop items that are complete AND connected (or correctly resolved). Indexed for the record.
**Blueprint:** `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` + `BLUEPRINTS.md`.
One-line ledger:
- **BP-05** PID redesign — DONE. `crates/bebop/src/governor.rs:180-233`, Jury-stable defaults (kp=1.03, ki=0.22, kd=0.20); old unstable Kd=1.5 demoted to named `default_legacy()`, kept only as a RED regression fixture.
- **BP-06** entropy-budget ledger — DONE. `crates/bebop/src/entropy_ledger.rs`, invariant `0 ≤ D_t ≤ H_max`, registered.
- **BP-08** admit() intake (Tikhonov well-posedness) — DONE AND LIVE-WIRED. `kernel/src/intake.rs` (744 lines), registered `kernel/src/lib.rs:84`, real consumer `kernel/src/loops.rs:11`. The one hydraulic item actually connected to something — the wiring template for P32b/P32c.
- **BP-09** survival-analysis persistence — DONE. `crates/bebop/src/persistence.rs` (Hungarian matching + D*=⌈log_p α⌉ Bonferroni), wired via `lib.rs:58`, consumed by `memory.rs`/`instrument_panel.rs`.
- **BP-10** orthogonometer (Goodhart decorrelation) — DONE. `crates/bebop/src/orthogonality.rs`, consumed by `loop_runtime.rs:421`.
- **BP-22** TS↔Rust reconcile — RESOLVED DIFFERENTLY: `agent-governance/resonator.ts` deleted entirely; no TS port left to reconcile; superseded by native `agent-governance-wasm`. Closed, not open.
**Anti-scope:** Do not touch. In particular do not "modernize" `default_legacy()` — it is a deliberate RED fixture.
**Depends on / blocks:** BP-08's intake→loops wiring is the reference pattern for P32b/P32c.

###### P32b — Resonator + arccos metric wiring
**Absorbs:** BP-01, BP-02
**Status:** WIRING-GAP
**Role & responsibility:** `resonator` is registered (`bebop2/core/src/lib.rs:328 pub mod resonator;` — the original plan's "still frozen, 1-line fix" framing is stale, it was unfrozen at some point) but has ZERO call sites: compiled+tested standalone, driving nothing. Likewise `algebra.rs:56 geodesic_distance` (acos) exists+tested with zero callers outside `algebra.rs` and was never plugged into resonator's `Metric` trait. The work is: pick ONE real, existing control loop and make resonator's output an input to it, with the arccos metric as its distance function.
**Blueprint:** `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` (BP-01/BP-02 entries) — reuse the math there; only the wiring decision is new.
**DoD:**
1. `geodesic_distance` implements resonator's `Metric` trait (grep-verifiable trait impl, not a free function sitting nearby).
2. Resonator has ≥1 call site in a live loop consumer (candidates, in preference order: `crates/bebop/src/loop_runtime.rs`, `kernel/src/loops.rs` via the BP-08 pattern) — grep for `resonator::` outside `resonator.rs`/tests returns ≥1 hit.
3. A test exercises the loop→resonator→loop round trip (not resonator in isolation — that coverage already exists).
4. Evidence the wiring changes behavior: with resonator's contribution zeroed, at least one loop-level test result differs (guards against decorative wiring).
**Anti-scope:** **Do not rebuild resonator** — only wire its existing output into a real caller. Do not redesign the `Metric` trait. Do not invent a new consumer loop just to have a caller; if no existing loop genuinely benefits, stop and report that instead — a finding, not a failure.
**Depends on / blocks:** Independent of P32c (parallelizable). Serves the AGENT self-improvement loop (these control loops govern the agent's own loop). Blocks nothing in PROTOCOL/DELIVERY.

###### P32c — Online DMD wiring
**Absorbs:** BP-07
**Status:** WIRING-GAP
**Role & responsibility:** `dmd.rs` (OnlineDMD, rank-1 RLS, complex eigenpairs) is built, tested, and registered (`bebop2/core/src/lib.rs:316`), but only *referenced* — not called — from `field.rs:328`. Same stranded pattern as resonator. Wire its mode estimates into a live consumer so the dynamic-mode decomposition actually informs something.
**Blueprint:** `docs/design/hydraulic-loop-v2/HYDRAULIC-LOOP-v2-PLAN.md` (BP-07 entry).
**DoD:**
1. `field.rs:328`'s reference becomes an actual call: OnlineDMD is instantiated and updated with real field-state samples in a non-test code path.
2. At least one downstream decision reads DMD output (dominant eigenvalue / mode) — grep-verifiable consumer.
3. Round-trip test: feed a synthetic signal with a known dominant mode through the wired path and assert the consumer sees it (reuses `dmd.rs`'s existing test fixtures where possible).
**Anti-scope:** **Do not rebuild or extend OnlineDMD** — rank-1 RLS is done. Do not add higher-rank updates, forecasting, or new spectral features. Wiring only.
**Depends on / blocks:** Independent of P32b (parallelizable). Same consumer landscape as P32b — if both wire into `field.rs`/loop_runtime, coordinate to avoid hot-file collision (sequential landing, parallel prep).

###### P32d — Cross-model critic
**Absorbs:** the cross-model critic from hydraulic-loop-v2's original 7 math corrections (load-bearing, never assigned a BP number)
**Status:** PLANNED (not built — no multi-model-voting code found anywhere)
**Role & responsibility:** A decorrelated multi-model check on control-loop decisions — the mechanism behind the arc's math-correction discipline, and the one named item from the 7 corrections with no code at all. Distinct from the harness-level review agents (those review diffs; this critiques loop outputs).
**Blueprint:** none — needs a short design note first: what gets critiqued (which loop outputs), decorrelation requirement (different model/provider, per the research-verifier precedent), and advisory-only posture (signals, never gates — GROUND-TRUTH-over-PROXY rule).
**DoD:**
1. Design note exists specifying critic inputs (≥1 concrete loop output type), decorrelation constraint, and advisory-only integration point.
2. Minimal implementation: one loop output critiqued by ≥2 decorrelated judges with disagreement surfaced as a signal (logged/ledgered), not a gate.
3. RED-provable: a deliberately corrupted loop output triggers a critic disagreement signal in a test.
**Anti-scope:** Do not build a general "AI council" framework — one loop output, minimal voting, advisory only. Do not let critic output gate anything deterministic (violates GROUND-TRUTH-over-PROXY). Do not couple it to the AGENT phase's LlmBackend wiring timeline — design note can proceed now; implementation may reuse AGENT's LlmBackend once wired.
**Depends on / blocks:** Implementation (not design) soft-depends on AGENT's LlmBackend being wired to consumers. Blocks nothing.

---

##### P33 — CORE Ledger Hygiene (cleanup + status audit)

###### P33a — Dead JS-spike artifact deletion flag
**Absorbs:** JS living-knowledge spike closure (successor of the A2(mip) lineage; the live replacement is P31a's native Rust retrieval)
**Status:** PLANNED (flag only — deletion is an operator/lead call, not this phase's to execute unilaterally)
**Role & responsibility:** The JS living-knowledge spike is FULLY DEAD — all source deleted (`f9ab28ff1`, "drop ALL JS/TS per operator"). Orphaned build artifacts remain on disk: `spikes/living-knowledge/out/semantic-cache.json` (7.7 MB) plus eval-result JSON files — unregenerable dead weight with no source left to produce them. Per the standing Anu/Ananke discipline: confirmed-dead legacy gets actually deleted, after verifying nothing consumes it.
**Blueprint:** n/a — housekeeping.
**DoD:**
1. Verified no CI job, script, or codegen step reads `spikes/living-knowledge/out/**` (grep across repo + CI config, zero hits).
2. Files deleted in a single revertable commit, or an explicit operator decision recorded to keep them.
**Anti-scope:** Do not resurrect any part of the JS spike. Do not delete anything outside `spikes/living-knowledge/out/` under this flag. Do not treat this as license to sweep other directories — one target, one commit.
**Depends on / blocks:** Nothing. Anytime task.

###### P33b — Unconfirmed hydraulic BP status audit
**Absorbs:** BP-03 (Francis QR complex eigenvalues), BP-04 (diffusion sign fix), BP-11 (renormalizer), BP-12..23 (security/integration wave items)
**Status:** PLANNED (audit to-do, NOT a build task)
**Role & responsibility:** These BP items were not individually verified this session — their status is **unconfirmed**, neither DONE nor OPEN. Do not guess. A fresh check must classify each as DONE / WIRING-GAP / OPEN against live source, the same way BP-01/02/05–10/22 were classified above. Note BP-22 is already resolved (see P32a) — the audit range is BP-03, BP-04, BP-11, BP-12..21, BP-23.
**Blueprint:** `docs/design/hydraulic-loop-v2/BLUEPRINTS.md` (the authoritative BP list to audit against).
**DoD:**
1. Each of BP-03/04/11/12..21/23 has a live-verified status (file:line evidence for DONE/WIRING-GAP; explicit "no code found" for OPEN) recorded in this roadmap.
2. Any newly discovered WIRING-GAP items get folded into P32 as new sub-letters; any OPEN items get an explicit build/park decision — neither silently dropped.
3. Zero code changes made during the audit itself.
**Anti-scope:** Audit only — do not fix, wire, or build anything found during the check; file findings back into the roadmap instead. Do not mark anything DONE without file:line evidence (the BP-01 "still frozen" staleness above is exactly the failure mode this guards against).
**Depends on / blocks:** Should complete before any new P32 sub-letters are claimed by swarm agents (prevents duplicate/stale work). No external blocks.

---

**CORE cross-reference summary:** every source unit accounted for — S0→P31a, S1→P31a, S2→P31a, S3→P31c, S4→P31a, S5→P31d, S6→P31e, S7→P31a(parked→PROTOCOL); A1(mip)→P31a(moot), A2(mip)→P31a, A3(mip)→P31a, B1→P31a, B2→P31a, B3→P31e(=S6), B4→P31a, B5→P31a, A6→P31b; BP-01→P32b, BP-02→P32b, BP-05→P32a, BP-06→P32a, BP-07→P32c, BP-08→P32a, BP-09→P32a, BP-10→P32a, BP-22→P32a, cross-model-critic→P32d, BP-03/04/11/12..21/23→P33b, JS-spike-artifacts→P33a. Nothing dropped.

#### 10.5.2 PROTOCOL — mesh, capability issuance, crypto, transport, delivery-domain (bebop2)

**Position on the critical path:** CORE (~90% done) → **PROTOCOL** → DELIVERY → AGENT → ECOSYSTEM/OPS. PROTOCOL is the single biggest lever in this roadmap: mesh-real's core delivery logic is **~70% already built, proven, and tested** in `/root/bebop-repo` — and **100% stranded**, with zero code-level connection from dowiz's own kernel to it. Wiring PROTOCOL to CORE (P34) is the #1 recommended next move for the whole project, bigger than building anything new.

**Connective-tissue finding (the most important sentence in this section):** the current P09/P10/P13 blueprints claim *"dowiz today has ZERO code-level dependency on the bebop protocol"* — true as a wiring statement, but **misleading as a build statement**: MESH-01 (delivery-domain), MESH-02 (KernelFacade), MESH-04 (claim_machine), and MESH-05 (matcher) already exist as ready-to-consume prerequisites in the sibling repo, and downstream work (agentic-mesh-protocol-2026-07-17 builds directly on MESH-11) already absorbs mesh-real informally without citing it. Nothing in this section starts from scratch; almost all of it is *registration and wiring* of finished code.

---

##### P34 — Wire mesh-real's proven delivery-domain into the dowiz kernel
**Absorbs:** MESH-01, MESH-02, MESH-03, MESH-04, MESH-05, MESH-07, MESH-09 (BPv7 half), MESH-10, MESH-11, MESH-12 — plus the wiring gap itself.
**Status:** WIRING-GAP (all absorbed units DONE; the connection is the only missing piece)
**Role & responsibility:** Make dowiz's kernel the consumer of the already-built bebop2 delivery protocol, closing the single largest value gap in the ecosystem. delivery-domain was *designed* to reuse dowiz-kernel as its decider (the KernelFacade `submit_intent` seam is exactly the compiled wire→Law→money boundary), so this phase is consumption, not construction. Once wired, DELIVERY's HTTP surface becomes trivial because the order lifecycle already exists here.
**Blueprint:** `/root/dowiz/docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` + `/root/dowiz/docs/design/mesh-real/MESH-REAL-PLAN.md` (MESH-12 resolution: `MESH-12-RESOLVED-2026-07-14.md`). Reuse; do not rewrite.

**Done inventory (one line each, verified live this session):**
- MESH-01 delivery-domain crate — `bebop2/delivery-domain/{lib.rs,intake.rs,pod.rs,finalization.rs,hub_ring.rs}`, 1844 lines, incl. proven solo-island offline test `intake.rs::ac6_solo_island_full_flow_no_peers` (full order→delivery with ZERO peers — directly satisfies the operator's "offline agent" requirement).
- MESH-02 KernelFacade — `bebop2/proto-cap/src/facade.rs:123` `submit_intent` (line corrected 2026-07-18; was `:64`, file grew, symbol unchanged); the compilation-firewall pattern (any port importing dowiz-kernel directly fails to build).
- MESH-03 event vocabulary — `proto-cap/src/event_dict.rs:278-299`, `DeliveryEvent::{OrderPlaced,ClaimOffered,ClaimAccepted,ClaimReleased,SettlementRecorded}`.
- MESH-04 claim_machine — `proto-cap/src/claim_machine.rs:85` `assert_transition`.
- MESH-05 matcher — `proto-cap/src/matcher.rs:63` `assign()` HRW rendezvous-hash + `hub_ring.rs:62`; deterministic, NO courier-scoring.
- MESH-07 Sync·Pull+Merkle — `proto-wire/src/sync_pull.rs:422` `MerkleLog`, 1181 lines, real.
- MESH-09 BPv7 half — `proto-wire/src/bpv7.rs`, 611 lines, hand-rolled custody/retry/expiry.
- MESH-10 WSS+rustls — `proto-wire/src/wss_transport.rs:423,511`, real TlsAcceptor/Connector.
- MESH-11 Revocation+H2 — `revocation.rs:49-114` + `hybrid_gate.rs:188-206,571` (agentic-mesh-protocol builds on this).
- MESH-12 node_id+genesis — `node_id.rs`, `H(pq_pub‖classical_pub)`, fail-closed `load_genesis`.

**DoD (falsifiable):**
1. A cargo dependency edge exists from a dowiz workspace member (kernel-adjacent adapter crate, not the web app) to `bebop2` `delivery-domain`/`proto-cap`, and it builds in CI — falsified by `cargo tree` showing no such edge.
2. The MESH-02 compilation firewall survives the wiring: dowiz consumes the protocol **only** through `KernelFacade::submit_intent` (`facade.rs:123`, line corrected 2026-07-18); a committed red-proof demonstrates that adding a direct dowiz-kernel import to any port fails the build.
3. Event-vocabulary round-trip: dowiz's order lifecycle maps 1:1 onto the five `DeliveryEvent` variants (`event_dict.rs:278-299`); an integration test folds a complete dowiz order through `claim_machine.rs:85 assert_transition` with zero illegal transitions.
4. Matcher consumption: at least one dowiz-side integration test calls `matcher.rs:63 assign()` for courier assignment and asserts determinism (identical inputs → identical assignment). No scoring, ranking, or reputation input is added (standing rejection).
5. Offline proof re-anchored: the `ac6_solo_island_full_flow_no_peers` scenario runs green **driven from the dowiz-kernel decider side** — full order→delivery with zero peers, using dowiz's Law as the fold.
6. Blueprint reconciliation: P09/P10/P13 are amended to cite MESH-01/02/04/05 by unit ID and file path; the "ZERO code-level dependency" claim is deleted or date-scoped to pre-P34. Falsified by grep still finding the unqualified claim.

**Anti-scope:** Do NOT fork or rewrite delivery-domain inside dowiz — consume the sibling crate. No new event variants. No courier-scoring/reputation (rejected as echo chamber; trust = signed capability only). No per-node storage (P34B). No transport/crypto hardening (P36). No HTTP server (DELIVERY drafter's scope).
**Depends on / blocks:** Depends on CORE decide/fold Law (~90% done — sufficient today). **Blocks DELIVERY** (its HTTP server becomes thin once this lands), blocks P34B, and formalizes what AGENT's agentic-mesh arc already borrows (MESH-11).

---

##### P34B — Finish the planned mesh halves: per-node storage, CRDT fence, iroh, ML-KEM KAT
**Absorbs:** MESH-06, MESH-08, MESH-09 (iroh half), MESH-13
**Status:** PLANNED
**Role & responsibility:** Close the four mesh-real units that were designed but never built. These are genuinely 0-30% (unlike the P34 units) and none of them gate the P34 wiring. The crypto-hygiene item (MESH-13) is real but not urgent-critical: ML-DSA-65 signing is the actually-used-today primitive; the ML-KEM path is not.
**Blueprint:** `/root/dowiz/docs/design/mesh-real/BLUEPRINTS-MESH-REAL.md` (same doc as P34; these are its unbuilt waves).

**DoD (falsifiable):**
1. MESH-08 CRDT compile-fence: a build-level mechanism (not design comments) makes introducing a CRDT merge on kernel-owned state fail compilation; red-proof committed.
2. MESH-13 ML-KEM: the current schoolbook impl (self-labeled "alternative") passes official FIPS-203 KAT vectors, or is replaced by one that does; `zeroize` applied to secret material (currently absent from the entire workspace).
3. MESH-09 iroh half: `proto-wire/Cargo.toml:51 iroh = []` is either implemented (gated behind secure TLS — hard-depends on P36 DoD-2) or formally retired with a dated decision note. An empty stub feature persisting is a fail.
4. MESH-06 per-node storage: a written decision + blueprint exists for per-node **local-first** storage. It MUST cite and explicitly distinguish `/root/dowiz/docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` — that blueprint covers dowiz's CANONICAL-repo **hub** storage, a related-but-distinct concern. Today there are zero pgrust references anywhere in bebop-repo; conflating the two is the failure mode this DoD item exists to prevent.

**Anti-scope:** Do not block or serialize P34 behind any of this. Do not treat the hub pgrust rebuild as satisfying MESH-06 (different node role, different consistency model). No new KEM primitives beyond FIPS-203 compliance of the existing path.
**Depends on / blocks:** Depends on P34 (wiring reveals the real per-node storage shape) and on P36 DoD-2 (iroh must not inherit insecure-TLS). Blocks nothing on the critical path.

---

##### P35 — Docker-swap: give DK-01..03 a real home + finish DK-04..08
**Absorbs:** DK-01..DK-10
**Status:** PARTIAL (DK-01/02/03/07 DONE; DK-04/05/06/08 PLANNED)
**Role & responsibility:** Register and finish the zero-OCI runtime subsystem. **Omitted-finding, stated plainly: docker-swap has REAL, TESTED, WORKING code — yet it is entirely unreferenced by CORE-ROADMAP-INDEX.md and MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE.md; no P-number owns it.** This phase is that home. The working half: `bebop2/ports/telegram/src/lib.rs` is a real `wasm32-wasip2` component (zero ambient authority, one import), and `bebop2/wasm-host/src/lib.rs` is a real Scope→WASI deny-by-default host (wasmtime feature-gated) — not stubs.
**Blueprint:** `/root/dowiz/docs/design/docker-swap/BLUEPRINTS-DOCKER-SWAP.md` + `DOCKER-SWAP-PLAN.md`; DK-07 already resolved by `/root/dowiz/docs/design/microvm-isolation/ADR-NO-SANDBOX-AGENT-GOVERNANCE.md`.

**DoD (falsifiable):**
1. Registration: the unified roadmap index lists P35 as owner of DK-01..10, with DK-01/02/03/07 marked DONE and file-cited. Falsified by grep finding no DK-* reference in the index.
2. DK-06 Firecracker: an actual microVM boots and runs a workload in a test. Today `kernel/src/isolation/microvm.rs` has only a `kvm_available` probe and its own comment admits "actual VMM launch is the next unit" — that comment being still-true is the fail condition.
3. DK-08 supply chain: a CI workflow produces an SBOM (syft) and signs artifacts (cosign). Zero hits exist in any workflow file today; a run producing both artifacts is the pass.
4. DK-04 native static-server: `tools/native-spa-server` is either promoted to a deployed serving path (proven by one staging deploy) or descoped with a dated note. No third state.
5. DK-05 pgrust-as-native-systemd: decision note written, cross-referencing P34B DoD-4 (hub deployment vs per-node storage — again related-but-distinct).

**Anti-scope:** Do NOT reopen DK-07 — the no-sandbox-for-agent-governance ADR is decided. No Kubernetes (standing rejection). Do not rewrite the working DK-01/02/03 code to "modernize" it.
**Depends on / blocks:** Independent lane — runs parallel to P34, no ordering dependency on the wiring. DK-06 feeds ECOSYSTEM/OPS deployment; the wasm-host feeds AGENT's sandboxed tool ports. Note the P36 cross-wire: P36 DoD-1's `no_std` regression is on `wasm32-unknown-unknown` for `bebop2-core`, a different target than these `wasip2` components — related toolchain, not the same build.

---

##### P36 — Bebop 5-expert remediation: kill the two live regressions, close remaining P1/P2
**Absorbs:** P0-P4 of the bebop excellence review
**Status:** 🔴 REGRESSION (2 items live) — remainder PARTIAL
**Role & responsibility:** Finish the 5-expert-review remediation. Two items are not "planned work" but **actively broken/dangerous right now** — a shipped-default insecure-TLS feature and a build regression that post-dates the remediation doc's own GREEN claim (i.e., the doc is currently lying about build state). These two outrank everything else in this section for time-sensitivity.
**Blueprint:** `/root/bebop-repo/docs/design/EXCELLENCE-REVIEW-AND-REMEDIATION-2026-07-14.md` (amend its status table per DoD-1; do not rewrite the review).

**Done inventory (one line each):** C1 ML-KEM decap constant-time (`core/src/pq_kem.rs:730-746` + dudect test) · C2 getrandom nomem removed (`core/src/rng.rs:278-282,329-332`) · C3 deterministic-keygen behind test-only cfg · C4/C4b Ed25519 scalar-mul + mod_l constant-time (`sign.rs:668-739`, closed 2026-07-17/18 — this IS the already-known C4b closure, not a new item) · C6 hybrid identity from two independent seeds (`pq_dsa.rs:1135 derive_pq_seed`) · C7 serde_json bounded + TLV on the signed path · **P4 clean-slate publish DONE** — `github.com/SyniakSviatoslav/OpenBebop` public since 2026-07-14 (root `f38f2c5`, main now 67 commits ahead).

**DoD (falsifiable — regressions FIRST, in order):**
1. 🔴 **no_std regression:** `cargo build --target wasm32-unknown-unknown --no-default-features -p bebop2-core` is green. It fails RIGHT NOW with `E0425` at `at_rest.rs:74` — a regression introduced by commit `d23e7aa` (2026-07-17), AFTER the remediation doc claimed GREEN. Pass additionally requires (a) this target build added to CI so it cannot silently regress again, and (b) the remediation doc's status corrected.
2. 🔴 **C5 insecure-TLS default-on:** `proto-wire/Cargo.toml` currently ships `default=["insecure-tls"]` — a live security footgun in the default build, not a doc gap. Pass = insecure-tls removed from defaults (opt-in only) + a fence that fails CI if it ever re-enters the default feature set.
3. P1 field-math consolidation: the duplicate Chebyshev/VSA singleton in `rust-core/src/lib.rs:25-159` is removed or re-exported from the single core; grep finds exactly one implementation.
4. P2 Node/Docker cruft: root `package.json`, `docker-compose.sovereign.yml`, and the 4 unported `.mjs` CI gates are ported or deleted — after verifying none are CI-wired or codegen inputs (standing verify-before-delete directive).
5. P2 QRNG: production keygen either draws through the SeedPool (`proto-cap/src/entropy.rs::AnuQrng` is off-by-default today; real keygen uses a separate OS-only `core::rng::entropy_provider()`) or a dated decision records OS-only entropy as accepted. Per standing stance, QRNG **seeds/mixes, never replaces** OS entropy.

**Anti-scope:** No new crypto primitives. Do not re-report C4b as new work. Do not reopen P4 (publish is done). No batch-verify performance work — the batch-accept honesty question is settled (every accept re-verifies singly; correctness over speed).
**Depends on / blocks:** Runs parallel to P34 — do NOT serialize the wiring behind remediation. DoD-2 blocks P34B DoD-3 (iroh). DoD-1 blocks any `no_std`/wasm32 consumer of `bebop2-core`. Nothing here waits on any other drafter's scope.

---

*Draft note for the assembling editor: P34's DoD-6 (blueprint reconciliation) is the one item in this section that touches documents owned by other drafters (P09/P10/P13) — keep it here, since the false "zero dependency" claim is a PROTOCOL fact, but flag it during final assembly.*

*Assembly resolution (2026-07-18): confirmed — §10.5.3 DELIVERY's P13 status row and P34 DoD-6 agree (both date-scope the stale "ZERO code-level dependency" claim and name P34 as the supplier); no cross-draft conflict remained to resolve.*

#### 10.5.3 DELIVERY — Product Surface (P37–P39)

**Position on the critical path:** CORE (~90% done) → PROTOCOL (mesh-real ~70% built, being wired in P34) → **DELIVERY is next**. The blunt truth: the product surface currently has **zero deployability** — no fly.toml, no live deployment, no HTTP order/API server anywhere in the repo (the only axum server, `tools/native-spa-server`, is static-file-only with zero dynamic routes). Once P34 lands, DELIVERY's order/courier/payment logic is mostly a **wiring** job (delivery-domain already has the proven flow), not new design.

**Already-landed substrate (DONE, listed for completeness, not re-scoped):** FE-01 zero-copy bridge (`engine/src/zerocopy.rs`, `engine/src/bridge.rs::VertexBridge` — caveat carried into P38a: `wasm/src/lib.rs` still returns copied `Vec`s, not the real ptr/len boundary), FE-02 SoA store (`engine/src/widget_store.rs`), FE-03 fixed-timestep loop (`engine/src/loop_.rs`), FE-08 motion/critical-damping (`engine/src/motion.rs`), FE-09 money-never-tween guard (`engine/src/money_guard.rs`); RW-02 (`kernel/src/analytics.rs::channel_ledger_js`), RW-03 (legacy money.ts/JS confirmed absent), RW-06 (`kernel/src/geo.rs`), RW-07 (`kernel/src/cart.rs`), RW-08 (`kernel/src/messenger.rs` + `money.rs`).

**Process note (one line, not a phase):** FE-17/RW-12's prescribed island-by-island migration was bypassed — `apps/web` was wholesale-deleted (`79ef316f6`, 2026-07-13) before those blueprints existed. No harm done (greenfield `web/` replaced it cleanly), but record it as a process deviation for future migrations.

**Already-numbered phases — corrected status (cross-reference only, numbers unchanged):**

| Phase | True status (verified this session) |
|---|---|
| P13 Delivery on Protocol | Its blueprint's "ZERO code-level dependency on the bebop protocol" claim is **stale** — PROTOCOL's P34 directly supplies what P13 needs. P13 becomes a wiring exercise once P34 lands; content unchanged, dependency corrected. |
| P16 Product UI Rebuild | Remains the master roadmap's home for this work; **P38a/P38b below are what actually fill it in** — no duplicate scope. |
| P17 Demo/Splat/GPU-Unlock Closure | Shares the same O18a `graphics-unlock` trigger as P38a's `cargo add wgpu`; one unlock serves both. |
| P18 Public-Flip | Landed on main; no DELIVERY correction needed. |
| P20 Demo & Marketing Pipeline (DM-1..DM-8) | **CONFIRMED 0% built** — `kernel/src/offer.rs`, `OfferKind`, `OfferRedeemed`, `PromotionType`: 0 grep hits; `compute_order_total` (`kernel/src/domain.rs:129`) explicitly excludes discounts ("No discounts in this scope"). DM-1 is the unblocked entry point (hosted in P39). |
| P22 Social Auto-Posting | **CONFIRMED 0% built** (no `SocialPoster` trait, no adapters, no `social-adapters` crate), but reuse substrate (`Spool`/`TokenBucket`/`ChannelLedger`) exists in kernel — cheap once started. Belongs more naturally in ECOSYSTEM's scope (external messenger ports); stays P22 as-is, status corrected here only. |
| P23 Device Auth+2FA | P1 (`totp.rs`, zero deps) buildable **today**, nothing blocks it. P3 (full HTTP wiring) **blocked — confirmed live** — on "no dynamic admin HTTP surface exists anywhere in this repo". P37 is the unblock. |

---

##### P37 — Minimal HTTP/API surface for orders
**Absorbs:** RW-09 (thin-shell boundary codify — the wire adapter is the second shell over the same kernel, subject to the same rule). Unblocks P23-P3; supplies the wire half of P13. No FE/DZ units live here.
**Status:** PLANNED (0% — the only axum server in the repo is static-file-only, zero dynamic routes)
**Role & responsibility:** The #1 literal blocker of the entire DELIVERY layer: expose delivery-domain's already-proven order lifecycle over a wire. This is explicitly a **thin** surface — just enough dynamic routes to place/advance/read an order — not a REST API design exercise; the order flow, state machine, and money math already exist and are tested, the server merely transports intents to `decide` and serves `fold`-derived state.
**Blueprint:** No dedicated blueprint (deliberately — the scope is "thinnest possible adapter"). The two documents that name this exact gap and constrain it: `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P13-delivery-on-protocol.md` and `docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (whose P3 is blocked on this surface). Reuse them; do not write a REST spec.
**DoD:**
1. A dynamic HTTP server exists in-repo (extend `tools/native-spa-server` or a sibling crate) serving both the static `web/` assets and dynamic order routes from one binary. Falsifiable: `grep` finds ≥1 non-static route handler; the binary boots and answers a dynamic request.
2. An integration test drives one full order lifecycle **over the wire** (place → accept → pickup → deliver) and asserts the final fold-derived state matches the same sequence run directly against delivery-domain. Red today (no server), green at close.
3. Thin-shell invariant (RW-09): zero domain logic in handlers — every state change routes through kernel `decide`/`fold`; falsifiable by review gate: no order-state mutation outside kernel calls in the server crate.
4. Mutating routes authenticate via **capability certificates** (proto-cap, ML-DSA-signed, PROTOCOL's `HybridGate`/`verify_chain`/`RevocationSet` — all already built). Falsifiable: a request with a forged or revoked cert is rejected (401/403) in a test; a valid chain passes.
5. **Offline parity (ARCHITECTURE.md F12, canon-locked):** the HTTP server is NOT the only way to place an order. The WASM-in-browser local decide/fold path that `web/src/app.mjs`'s beachhead already uses is extended to real order placement — a test places an order with the server absent and the fold is identical; rejoin/sync is PROTOCOL P34's job, not P37's.
6. The server binary is runnable locally with one documented command. (Deploy packaging — fly.toml, monitoring — is ECOSYSTEM/OPS scope, P40+; P37 only guarantees a bootable binary.)
**Anti-scope:** Do NOT build a conventional REST+session/password login — auth is capability-cert-based per canon and `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` D3 (device-bound keypair primary; TOTP/WebAuthn are step-up only). Do NOT design a full resource-oriented REST API, pagination, versioning, or an admin CRUD surface. Do NOT put any pricing/discount/state logic in handlers. Do NOT make network the required path for order placement (F12).
**Depends on / blocks:** Depends on PROTOCOL P34 for real mesh-backed order data (the server can land against local delivery-domain first). **Blocks** P23-P3 (its named live blocker), P13 wire-side wiring, P39b, and any AGENT (P4x) flow that needs an API to call.

##### P38a — WebGPU render engine completion
**Absorbs:** FE-04/RW-04 (particle→wgpu, single unit, counted once), FE-05 (SDF pipeline + GPU design-token table), FE-06 (MSDF text), FE-07 (layout field), FE-10 (Green's-function feedback), FE-11 (potential wells), FE-12 (spectral φ₂φ₃ embedding), FE-13 (constraint solver), FE-14 (lazy-render-on-settle), FE-15 (a11y mirror), FE-16 (WebGL2/SIMD fallback), RW-01 (`dowiz-engine` Cargo workspace), RW-05 (shell crate reshape — closes FE-01's caveat), RW-10 (web toolchain), RW-11 (view→wgpu migration).
**Status:** PARTIAL (math substrate DONE; GPU path and pipelines 0%)
**Role & responsibility:** Turn the tested, bit-deterministic physics-field substrate into actual pixels. This requires **no redesign**: `engine/src/field_frame.rs::compose()` already renders physics state to RGBA (real, tested, bit-deterministic), and `VertexBridge` has a real CPU staging copy — its `new_gpu()` is a stub only because the `wgpu` dependency is a network-gated `cargo add` (O18a `graphics-unlock`, verified RED/403 as of 2026-07-16 — a ONE-TIME unlock shared with P17, not an architecture question).
**Blueprint:** `docs/design/field-ui-engine/` + `docs/design/field-ui-engine.md` (FE-01..17), `docs/design/rust-engine-rewrite/` (RW-01..12), `docs/design/sovereign-roadmap-2026-07-16/BLUEPRINT-P16-product-ui-rebuild.md` (P16's home, filled by this phase), `docs/design/BLUEPRINT-W21-field-ui-gpu-blocked.md` (the gpu-blocked record). Reuse; don't rewrite.
**DoD:**
1. `wgpu` added (O18a unlock); `VertexBridge::new_gpu()` real — staging buffer uploads to a GPU vertex buffer. Falsifiable: headless pixel readback matches `field_frame::compose()`'s RGBA reference (the bit-deterministic oracle already exists — use it, don't invent a new one).
2. FE-04/RW-04 particle renderer draws N particles from `widget_store` (FE-02) at the fixed timestep (FE-03) with `motion.rs` damping (FE-08). Note: the source `particle-cloud.js` no longer exists (only a README survives) — this is a reimplement-from-spec against the engine SoA store, not a port.
3. FE-05: SDF pipeline (`sdf.rs`/`scene.rs` primitives exist) gains the GPU design-token table. FE-06: MSDF glyph atlas + text draw (currently zero glyph code anywhere).
4. FE-07 real force-layout (today only a partial spectral-decode helper) + FE-10/11/12/13 each land with at least one deterministic test against kernel math.
5. FE-14 lazy-render-on-settle: falsifiable — frame callbacks stop within k ticks of field settle in a test.
6. FE-15: DOM survives ONLY as an invisible AccessKit mirror for screen-reader/IME text input; falsifiable — screen-reader tree exposes order state while zero visible DOM nodes render UI. FE-16: WebGL2/SIMD fallback flags are functional, not the current empty stubs.
7. RW-05 + FE-01 caveat closed: `wasm/src/lib.rs` exposes the real ptr/len JS boundary (no copied `Vec<u8>`/`Vec<f32>` returns); mixed-in retrieval exports separated out. RW-01: kernel/engine/wasm unified into a `dowiz-engine` Cargo workspace (currently path-deps only). RW-10: `web/package.json` graduates from bare Node script runner to a real toolchain. RW-11: the view layer that emerges here is wgpu-native from day one (no interim DOM view to migrate).
**Anti-scope:** Do NOT build a DOM admin panel — the UI is a WebGPU/WASM render of backend physics-field state (canon; DOM only per FE-15's invisible mirror). Do NOT redesign the math substrate — compose/zerocopy/widget_store/loop_/motion/money_guard are done and tested. Do NOT tween money (FE-09 guard is landed and binding). Do NOT treat `web/src/app.mjs` (204 lines, console-only, 24/24 kernel exports bound) as throwaway — it is the confirmed deliberate first step; its own header names the DOM/FieldSim pass ("G3") as a later unit that reuses these bindings.
**Depends on / blocks:** Depends on O18a `graphics-unlock` (hard, environment-gated; same trigger P17 waits on). Blocks P38b entirely and P17's splat-tier closure. Independent of P37 — the two can proceed in parallel.

##### P38b — Sea & Sheet product surfaces (dowiz-interfaces)
**Absorbs:** DZ-01..12 — all twelve, none dropped. DZ-10 (voice) is absorbed **as deliberately deferred**: fully built+tested for the old deleted stack (49/49 tests, real Whisper ASR), deleted in the 2026-07-13 purge, and intentionally re-placed at the arc's own Phase 9b ("optional integrations", after the order-critical path) — an intentional deprioritization, not an oversight. Gesture control (one checklist bullet) shares that tail.
**Status:** PLANNED (0% code — no `Intent`/`FieldPos`/`InputSource` structs exist anywhere; `web/src/app.mjs` has zero DOM/canvas)
**Role & responsibility:** The actual product interfaces built on P38a's pipelines: Sea (ambient-field client surface) and Sheet (brand-SDF) — the customer storefront and order flow as field-render, wired to real order data. This is where a customer first *sees* dowiz in the new stack.
**Blueprint:** `docs/design/dowiz-interfaces/` (DZ-01..12, Sea & Sheet). Reuse; don't rewrite.
**DoD:**
1. `Intent`/`FieldPos`/`InputSource` structs exist and are exercised by tests (currently 0 grep hits).
2. Sea and Sheet each render via P38a pipelines against real kernel state; one end-to-end pass shows an order placed through the Sea surface reaching delivery-domain fold state (via P37's wire or the F12-canon local WASM path).
3. DZ-01..09/11/12 each traceable to landed code or an explicit deferral note; DZ-10 voice + gesture remain at Phase-9b priority — pulling them forward is a scope violation, not initiative.
**Anti-scope:** No DOM-first screens (same canon as P38a). Do not resurrect the old Whisper voice stack ahead of the order-critical path. Do not fork a second design-token system — FE-05's GPU token table is the single source.
**Depends on / blocks:** Hard-depends on P38a (pipelines) and, for real order data, P37 + PROTOCOL P34. Blocks nothing downstream except demo polish (P17/P20 visual units benefit but aren't gated).

##### P39 — App-shell: installability + capability-auth wiring + offer math
**Absorbs:** The **installability gap** — the one genuinely new phase-item in this section, with no prior unit ID: the old stack had a full Svelte PWA + service worker AND a Tauri desktop installer (`apps/bootstrap-installer`), both deleted in the purge; the new stack has ZERO PWA/installability work and NO canon decision locking it in or rejecting it. It is not covered by FE-*/DZ-* (those are rendering, not app-shell packaging). Also hosts the DELIVERY-side wiring of P23 and P20's DM-1 — both keep their own numbers; P39 does not claim them.
**Status:** PLANNED (installability 0% + genuinely undecided; P23-P1 unblocked today; P20 DM-1 unblocked today)
**Role & responsibility:** The remaining product-surface pieces once P37/P38 (the real leverage points) exist: make the product installable, wire device-bound capability auth into the live surface, and give the kernel real offer/discount math so demos and marketing have something true to show.
**Blueprint:** Installability: none exists — first deliverable is the canon decision itself (PWA vs native wrapper vs both vs rejected), recorded before code. Auth: `docs/design/BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` (reuse — it already correctly targets D3 device-bound keypair primary, TOTP/WebAuthn step-up only). Offers: `docs/design/DEMO-MARKETING-PIPELINE-REFACTOR-2026-07-17.md` (P20; DM-1 is the entry point).
**DoD:**
1. A canon decision on installability exists (ADR-style, in docs/design), and — if accepted — the chosen shell (manifest + service worker, or wrapper) installs the `web/` surface and launches offline-capable per F12. Falsifiable: install + airplane-mode launch reaches the local-decide order path.
2. P23-P1 (`totp.rs`, zero deps) landed; P23-P3 wired onto P37's routes once P37 exists — step-up only, never primary auth.
3. P20 DM-1 landed: kernel discount math exists and `compute_order_total` (`kernel/src/domain.rs:129`) no longer carries "No discounts in this scope"; property tests pin money invariants (FE-09/money_guard discipline applies).
**Anti-scope:** Do not build a conventional password+TOTP login as primary — capability certificates (proto-cap, ML-DSA) are the auth model, per canon; TOTP/WebAuthn are secondary step-up only. Do not let the service worker introduce a network dependency for ordering (F12). Do not expand into P20's publishing/marketing pipeline (DM-2+ stays P20's own scope, publication gated behind P18 public-flip) or P22 social posting (ECOSYSTEM-adjacent, stays P22).
**Depends on / blocks:** P39 auth-wiring depends on P37 (P23-P3's named blocker); installability depends on P38a/P38b having something worth installing, though the canon decision and manifest skeleton need nothing. Blocks P17/P20 demo credibility (real offers, installable demo) and provides the step-up auth AGENT (P4x) flows will assume.

#### 10.5.4 AGENT — local/network AI, tool-use loop, MCP

> **Scope boundary (locked invariant):** DELIVERY's core order/courier/money flow NEVER requires AI to function. This is already true by construction — CORE's decide/fold Law is pure deterministic Rust (the standing "НЕ-AI у ядрі" invariant from math-first-architecture's 7 invariants; LLM is "a feeling at the edge," a resonator-style pure-fn concern, zero I/O in the core, never in the decision path). Every phase below is additive assistance on top of an already-complete deterministic system. If every AGENT phase were deleted tomorrow, orders would still place, couriers would still match (deterministic HRW), money would still settle.
>
> **Naming discipline — two different "agents," do not conflate:**
> - **This section's agent** = the local delivery-operations assistant: `LlmBackend` (`kernel/src/ports/llm.rs`) + a to-be-built tool-use loop acting on order/courier operations.
> - **`AgentBridge`** (`kernel/src/ports/agent/{admission,cap,manifest,scope}.rs`, consumed by `agent-adapters/{cache,dispatch,mcp}.rs`) = foreign-agent admission/caging for the mesh — PROTOCOL's scope, part of the agentic-mesh-protocol arc. Zero code links it to `LlmBackend` today, and that separation is intentional.
>
> **Out of scope, flagged for awareness — self-mod effector (bebop-repo):** `bebop2/core/src/self_mod.rs` + `self_mod_loop.rs` exist with a header claiming "ACTIVATED (operator, 2026-07-16)" (commits `3696caa`/`dd431b5`). It is DORMANT (called only from its own unit tests, not wired into any live loop), narrow (mutates one in-memory Kalman q-scaler parameter, capability-gated, hard-refuses all red-lines), and it is a code-self-modification actuator, not a delivery-operations assistant. It stays outside this section's phase numbering. Caveat: its activation claim is self-asserted in commit messages and deserves independent operator confirmation before anyone treats it as authorized-live.

**Critical-path position:** CORE (~90%) → PROTOCOL (P34) → DELIVERY (P37/P38) → **AGENT (this section)** → ECOSYSTEM/OPS. AGENT depends on DELIVERY's P37 order-API surface existing (a tool loop needs something to call), but design and scaffolding proceed in parallel — see per-phase dependency notes.

---

##### P40 — AgentLoop executor + tool-calling capability wiring
**Absorbs:** P21 (resident-agent plane, executor half) · follow-on to the shipped harness-llm-backend arc (`feat/harness-llm-backend`, Ollama port Wave 0+1+consumer-wiring DONE)
**Status:** PLANNED (its substrate is DONE — the gap is everything above it)
**Role & responsibility:** Build the plan→act→observe executor that turns the existing raw chat-completion backend into an agent that can DO things. Today `LlmBackend` is real and consumed (only by `llm-adapters/src/{dispatch,cache,compose,ollama}.rs` and its own tests/benches), but `AgentLoop`/any executor has **0 grep hits anywhere in the repo** — this is the single biggest gap in AGENT's scope: a chat backend with no callers connecting it to orders. P40 also defines the tool-port interface behind a KernelFacade-style compilation firewall and un-pins tool-calling at the capability level: `Caps.tool_calling` is HARD-PINNED `false` at `llm-adapters/src/ollama.rs:59` — it is not even wired at the flag level yet.
**Blueprint:** `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md` covers the backend layer and already anticipates tool-calling — as a `Caps` probe ("Tool-calling/structured-output support differs per backend/model — a `Caps` probe, not assumed," §2.2 Quirks item 5) and as a `tools` field in the exact-match cache key (§3.2) — but contains **no loop design**. Build on that doc's port/adapter/firewall conventions; the loop itself needs a first design pass here.
**DoD:**
1. A `ToolPort` trait exists in the kernel-ports layer (plain structs, no serde/HTTP, mirroring `llm.rs` conventions); the loop crate consumes tools ONLY through it — `cargo tree` shows the loop crate does not import `dowiz-kernel` directly (same firewall done-check the LLM blueprint already uses: kernel shows no HTTP client, no adapter crates).
2. `Caps.tool_calling` is no longer hard-pinned `false` for Ollama — it is set by a live per-model probe, fail-closed (probe fails ⇒ `false`).
3. Exactly ONE tool ships: **read order status by ID**. A local Ollama model, given a natural-language request, calls it and returns the correct status for one test order, proven end-to-end by one test. (Deliberately minimal — this is the falsifiable "the agent can DO something" gate, not a framework.)
4. The loop is bounded: hard max-iteration cap, every tool call and result logged, a tool error surfaces as a typed loop outcome (never a silent retry-forever).
**Anti-scope:** No multi-tool framework, no tool registry, no write/mutating tools, and absolutely no money/auth/RLS/migration tools in this phase. No streaming, no re-design of `LlmBackend` (it is shipped; extend, don't rewrite). Do not touch `AgentBridge`/`agent-adapters` — that is PROTOCOL's mesh-admission surface. No autonomy: the loop executes one user-initiated request to completion, it does not schedule itself.
**Depends on / blocks:** Needs DELIVERY's P37 order-API surface for the real read-order tool target; until P37 lands, DoD items 1–2 and a no-op/echo tool loop are buildable now against a stub — do the scaffold in parallel. Explicitly does NOT require PROTOCOL's P34/P35 to be complete: a minimal read-only tool loop must work on a solo offline node, since offline-first is a hard requirement. Blocks P41 (mode parity needs a loop to be parity OF) and P42 (MCP re-exposes P40's tool port).

##### P41 — Three-mode operation: no-AI / local-offline / connected — one tool interface, swappable backend
**Absorbs:** P21 (mode/degradation half) · operator three-mode directive (verbatim requirement, the spine of this section)
**Status:** PARTIAL (backend swappability largely shipped; mode-parity proof and degradation contract are the gap)
**Role & responsibility:** Make the three operating modes an enforced, tested property rather than an intention. Mode 1 (no-AI) requires **zero new code** — CORE+PROTOCOL are AI-free by design and this phase only locks that in as a regression-proof invariant. Modes 2 and 3 must differ ONLY in which `LlmBackend` impl is selected (Ollama local vs managed/remote — both adapter families already exist per the blueprint's Tier-0 `ManagedApiAdapter` / Tier-1 Ollama split and `dispatch.rs`), never in the tool-loop shape: one port, swappable backend, no second tool-calling implementation. *Extended 2026-07-18 (operator BYO-AI directive):* mode 3 "connected" explicitly includes the owner's OWN AI subscription — any OpenAI-compatible endpoint + owner-supplied key, same `ManagedApiAdapter`/`Quirks::managed_api` path, no vendor list, a config-provenance sub-distinction (managed-default vs BYO) rather than a fourth mode; the fresh-venue DEFAULT PRESET is written-explicit mode 2 (local Ollama) — BYO is the opt-in upgrade, local-first is the zero-owner-config default; the owner-facing settings surface lives with P48's hub (cross-reference only, designed in P48's own lane).
**Blueprint:** `docs/design/harness-2026-07-16/HARNESS-LLM-BACKEND.md` §2.2 (one `OpenAiCompatTransport` + per-adapter `Quirks`) is the swappable-backend half; the degradation contract and the BYO-AI/default-preset extension are designed in `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P41-three-mode-ai-operation.md` (§3.5, §3.6).
**DoD:**
1. **No-AI proof:** CORE/PROTOCOL test suites pass with AGENT crates absent from the build graph (`cargo tree -p dowiz-kernel` shows no llm/agent-loop crates — the blueprint's existing firewall check, promoted to a mode-1 invariant). Consistency anchor: PROTOCOL's `ac6_solo_island_full_flow_no_peers` already proves the full flow with no peers; mode 1 is that plus no-AI, and it must stay green untouched.
2. **Mode parity:** P40's single-tool test passes with the backend swapped by configuration only — zero source diff in the loop or tool port between local and connected runs.
3. **Graceful degradation:** with Ollama stopped (typed `LlmError::Unavailable`) and no network, the order/courier flow is provably unaffected and the agent surface returns a typed "assistant unavailable" outcome — never a hang, never a blocked order.
4. **Local-offline proof:** mode-2 run passes with all remote endpoints unreachable (network-isolated test), consistent with the solo-island guarantee.
5. **BYO/preset (2026-07-18 addendum):** a BYO endpoint+key composes the IDENTICAL connected stack as the managed-default case (provenance is attribution metadata, never behavior), and the fresh-venue provisioning preset resolves to explicit local mode through the normal config path — per BLUEPRINT-P41 §3.6.
**Anti-scope:** No new code for mode 1 (writing any is a design smell — reject it in review). No second tool-loop implementation for remote backends. No "smart" auto-escalation from local to remote without explicit configuration. Routing help from the model stays advisory-only — the deterministic HRW matcher remains the sole courier-assignment authority in every mode.
**Depends on / blocks:** Depends on P40 (needs the loop and one tool to prove parity over). DoD item 1 is provable TODAY, before P40 — land it first as the locked baseline. Independent of PROTOCOL P34/P35 completeness by construction (offline-first). Blocks P42 (MCP exposure must inherit the same three-mode contract).

##### P42 — MCP port + agent-as-capability boundary
**Absorbs:** IP-08 (MCP-server + agent-as-capability port — 0% built, no code found under this name)
**Status:** PLANNED
**Role & responsibility:** Give P40's tools a standard exterior: the agent calls tools via MCP, and each tool is a capability-scoped port — never a direct kernel import. This mirrors PROTOCOL's KernelFacade compilation-firewall pattern on the AGENT side: the same architectural move, applied to tool-calls. Deliberately lighter than P40/P41 — it is follow-on work that standardizes a pattern only after P40/P41 have proven it on one tool.
**Blueprint:** no existing blueprint, first design pass needed here. Reference-adjacent code exists — `agent-adapters/mcp.rs` — but it serves PROTOCOL's mesh `AgentBridge` (foreign-agent admission), not this port; study its conventions, do not repurpose it or couple to it.
**DoD:**
1. P40's read-order-status tool is additionally callable through an MCP server endpoint, same behavior, one test.
2. Capability scoping is enforced fail-closed: a tool invocation outside the granted capability scope is refused with a typed error, proven by one negative test.
3. Firewall holds: the MCP layer imports only the `ToolPort`/facade surface — `cargo tree` shows no direct `dowiz-kernel` dependency from the MCP crate.
**Anti-scope:** No foreign-agent admission, caging, or mesh exposure — that is `AgentBridge`, PROTOCOL's scope; P42 serves the LOCAL agent only. No tool-catalog expansion (still the one proven tool). No transport invention — MCP as-specified. Forward-looking cross-reference only, not scoped work: IP-05's "multimodality = superposition of intents" (voice+touch composing, not conflicting) becomes AGENT-relevant if voice ever becomes an agent input channel — that lives with DELIVERY's DZ-10 (voice, deliberately Phase-9b-deprioritized), and P42 must not front-run it.
**Depends on / blocks:** Depends on P40 (tool port) and P41 (three-mode contract it must inherit — the MCP surface must degrade exactly as gracefully). Needs no PROTOCOL P34/P35 completion: a local MCP endpoint on a solo node is the baseline case. Blocks nothing on the critical path — ECOSYSTEM/OPS integration work that wants to consume AGENT tools externally should wait for P42 rather than importing anything deeper.

#### 10.5.5 ECOSYSTEM/OPS — External Integrations, Deployment, Multi-Product Platform

> **Sequencing verdict (the most important sentence in this section):** ECOSYSTEM/OPS is **explicitly LAST on the critical path** — CORE → PROTOCOL (P34) → DELIVERY (P37/P38) → AGENT (P40/P41) → **then this**. This is not a priority judgment about the work's worth; it is a statement of physical reality: there is currently **zero live deployment** (no `fly.toml`, no pgrust binary installed, `attic/` and the old `apps/` stack physically deleted). Deployment, monitoring, external integrations, and multi-product platforming only make sense once there is something real to deploy, monitor, and integrate. Building a monitoring stack for a service that does not exist is waste, and every phase below carries an anti-scope rule enforcing that.

> **Audit finding (largest silently-dropped cluster in the whole roadmap audit):** neither the integration-ports arc (IP-01..21) nor the ecosystem-strategy arc (EC-01..20) is referenced *at all* by `CORE-ROADMAP-INDEX.md` or `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` — zero hits for either arc name. Roughly **38 of the 41 combined external-facing IP+EC unit IDs had no living tracking artifact anywhere in current canon** before this section. This section is their new (and only) home. The absorption ledger:
> - **IP-10 / IP-15 / IP-16** (social/messenger marketing) → **ABSORBED INTO existing P22**, not renumbered (see below).
> - **IP-11 / IP-12 / IP-13 / IP-14 / IP-19 / IP-20** (messenger/marketing/hosting/backup/export/automation ports) → **P43**.
> - **EC-05** (five cache layers) and the own-inference / own-RAG / chunking / gossip-flows units of **EC-03/04/06/08/12–15** → **P44**.
> - **All OPS-01..22** (ops-reliability arc) → **P45**.
> - **EC-17** and the multi-product/marketplace remainder of the EC arc (incl. "dowiz Local") → **P46**.
> - EC's shared **KernelFacade** concept is already built in PROTOCOL — cross-referenced there, not re-claimed here. IP-01..09/17/18 are PROTOCOL's/AGENT's/CORE's scope and are covered by those drafters.

##### Existing P22 — Social Auto-Posting (confirmed home, not renumbered)
P22 is **confirmed 0% built** — no `SocialPoster` trait, no `TelegramAdapter`/`ViberChannelAdapter`, no `social-adapters` crate anywhere. But its blueprint (`docs/design/BLUEPRINT-SOCIAL-AUTO-POSTING-2026-07-17.md`) already correctly cites **IP-10/IP-15/IP-16 as prior art**, which means P22 is *already* the correct numbered home for those three units. **They are ABSORBED INTO existing P22 — do not renumber, do not duplicate under P43.** Note for whoever starts it: the reusable substrate already exists in kernel (`Spool`/`TokenBucket`/`ChannelLedger`), making Wave-0 (Telegram) cheap once DELIVERY gives it something to post about.

**Scope expansion (2026-07-18 operator directive — blueprint §11, same file):** P22 additionally owns:
1. **Content generation, dual-path**: a native template renderer (deterministic, zero-AI — works in P41 mode 1/`AiMode::Off`) AND an `LlmBackend`-drafted path (modes 2/3, via the existing Harness/Dispatcher), both producing the same reviewed `MasterPost` type so downstream posting cannot tell which path authored a draft. Post types are a closed set of five (daily special, sold-out, offer announcement — render-only over P20 DM-1/DM-7 objects, hours/area change, aggregate social proof with a ≥10-count privacy threshold).
2. **Posting modes**: manual owner approval is the **DEFAULT** for every draft from every source; agentic auto-posting is a per-venue, per-post-type **opt-in** behind an earned-autonomy ratchet (first-10-always-reviewed, 10-consecutive-clean counter, dedicated 1/day/platform `TokenBucket`, revoke-on-`Rejected`, kill switch). Drafting is exposed to the P40 agent loop as a future `ToolPort` extension (**P42-gated** — no P40 enum changes now); **publish/approve are never model-callable actions** at any autonomy level.
3. **The campaign lane** for recipient-list channels: **mailing lists + SMS** ride the absorbed IP-15 `ChannelAdapter` shape under this phase's number — sharing P22's drafts/approval/outbox/`?ch=` attribution but **not** the `SocialPoster` trait (per-recipient fan-out + consent/unsubscribe ledger; recipient lists are PII, so the lane is blocked on its own consent-ledger mini-blueprint). **SMS is per-message PAID via any provider** (Twilio/TurboSMS-class), unlike free Telegram/Viber posting — preflight must show `recipients × unit_cost`. Transactional sends (order-status/OTP over messenger/SMS/email) are **NOT** P22 — they stay P43 DoD-2 + P49.

##### P43 — External Integration Ports: Messenger / Marketing / Export / Backup-Export / Hosting
**Absorbs:** IP-11, IP-12, IP-13, IP-14, IP-19, IP-20. (IP-10/15/16 → ABSORBED INTO existing P22, not renumbered.)
**Status:** PLANNED (with two false premises corrected and one small live bug)
**Role & responsibility:** All customer/operator-facing external channels that are not social auto-posting: messenger delivery-notification ports, marketing/channel-tracking, data export, and hosting/automation ports. These follow the arc's core-immutable/integrations-as-ports doctrine: adapters at the edge, never leaking into kernel Law. **Boundary vs P22 (clarified 2026-07-18):** P43's messenger/SMS/email surface is **transactional** — order-status notifications and OTP (the DoD-2 send path, consumed customer-side by P49). Marketing **campaign** sends to opted-in recipient lists (mailing lists, SMS campaigns) belong to P22's campaign lane (the absorbed IP-15 `ChannelAdapter` — see P22's 2026-07-18 scope expansion and blueprint §11.5); the two may eventually share a low-level provider adapter, but the producer pipelines (order events here vs owner-authored/AI-drafted content there) never merge.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P43-external-integration-ports.md` (2026-07-18): Telegram-first `ChannelSend` port; **httpSMS** as the recommended own-infra SMS default (paid Twilio-class optional); **WhatsApp Cloud API** transactional adapter with honest per-template cost model (free customer-initiated 24h windows exploited structurally); **SimpleX Chat** as the architecturally-preferred ADDITIONAL privacy channel (CLI-sidecar WebSocket bot, one-time-invitation onboarding, self-hosted SMP relay = optional P45 ops item); `?ch=` tracking, export port, native media-import port. Source arc: `/root/.claude/projects/-root-dowiz/memory/integration-ports-reactive-arc-2026-07-13.md` (do not rewrite the arc doc).

**Two arc claims CONFIRMED FALSE (wrong even when written — correct the record, do not build on them):**
1. *"`?ch=` channel-tracking spine already exists at `Storefront.svelte:93`"* — **false.** `Storefront.svelte` does not exist anywhere in the current repo (it lived in the old, deleted `apps/` stack). Current `web/src` is a greenfield rebuild with zero `?ch=` code. Channel tracking is a from-scratch item, not a "wire-up" item.
2. *"Telegram already has full push+OTP as the primary messenger"* — **false.** What exists is `kernel/src/messenger.rs:33 telegram_link()`, an explicitly **non-sending** deep-link string builder (its own comment says "never sends"), plus a completely separate `tools/telemetry/*` Telegram bridge that is the OPS/governance alerting channel (heartbeat monitors) — **not** a customer/courier-facing channel. Do not conflate the two; a real customer-facing Telegram send path does not exist.

**One concrete small bug, actionable now (bug ticket, not a phase):** the QRNG entropy port is split oddly — `bebop-repo/bebop2/proto-cap/src/entropy.rs` (`AnuQrng`/`SeedPool`) uses the current ANU endpoint with fail-closed tests, while `dowiz/kernel/src/pq/entropy.rs:37-122` is a simpler feature-gated version pointing at a **deprecated legacy ANU endpoint URL**. Fix = align the dowiz copy to the current endpoint (or delete it in favor of the proto-cap one). Small, mergeable independently of this phase's gating.

**DoD:**
1. QRNG endpoint mismatch fixed: `kernel/src/pq/entropy.rs` no longer references the deprecated ANU URL, verified by its own fail-closed test.
2. One real customer-facing messenger send path exists (Telegram first) that actually transmits — falsified by `messenger.rs` still being the only "messenger" code.
3. `?ch=` channel tracking exists in the *new* `web/src` and is asserted by at least one E2E check.
4. One data-export port (orders/menu) produces a file an operator can download from a live deployment.
**Anti-scope:** Do NOT build any adapter before DELIVERY P37/P38 gives it a live order flow to notify about — a messenger port with nothing to send is dead code. Do NOT re-implement social posting here (P22 owns it). Do NOT build marketing-campaign / mailing-list / SMS-campaign tooling here — that is P22's campaign lane (2026-07-18 expansion); P43's SMS/email use is transactional-notification only. Do NOT touch the `tools/telemetry` Telegram bridge; it is OPS plumbing, not a product channel.
**Depends on / blocks:** Depends on DELIVERY P37/P38 (live order/courier flow) and PROTOCOL P34 (capability-gated egress). Blocks nothing on the critical path. QRNG bug fix (DoD-1) has no dependency and may land any time.

##### P44 — Cache Layers (EC-05) + Own-RAG / Own-Inference Scale-Out — LOW PRIORITY / FAR-FUTURE
**Absorbs:** EC-05; own-inference-beyond-Ollama, own-RAG, chunking, and gossip-flows-as-kernel-properties units of EC-03/04/06/08/12–15.
**Status:** PLANNED (0 of 5 cache layers built)
**Role & responsibility:** The ecosystem-strategy arc's flagged "only gap": five cache layers (embedding cache, Merkle re-index, prefix-disk tier, pipeline cache, semantic cache) plus eventual self-hosted inference/RAG scale-out. Verified current state: exactly **one** basic exact-match sha3-keyed cache exists (`llm-adapters/src/cache.rs`); none of the five planned layers, no own-RAG, no chunking pipeline.
**Blueprint:** none — source arc: `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`. Do not write one yet.
**DoD (deliberately minimal — this is optimization work for a service that does not yet exist):**
1. A measured baseline exists (cache hit-rate + latency on real AGENT-loop traffic) *before* any layer is built — no layer ships without a number it improves.
2. Each layer lands only with a benchmark showing net win over the existing sha3 exact-match cache; a layer that doesn't beat it gets deleted, not kept.
**Anti-scope:** **This is explicitly NOT where swarm effort should go soon.** Do NOT build any cache layer before AGENT P40/P41 produces real inference traffic to cache — cache design against imagined workloads is the definition of premature optimization. Do NOT stand up own-inference infra while the existing Ollama port (already built, AGENT's scope) is unsaturated.
**Depends on / blocks:** Depends on AGENT P40/P41 (real traffic) and DELIVERY P37 (real product load). Blocks nothing; nothing waits on this.

##### P45 — Deployment + Monitoring Floor (minimum viable ops)
**Absorbs:** OPS-01..22 (entire ops-reliability arc).
**Status:** PARTIAL — but barely; the arc's own premise is gone
**Role & responsibility:** The minimum viable ops floor for whenever DELIVERY has something live: deploy path, dead-man's-switch monitoring, backup with off-site immutability, secrets handling. Honest inventory of what actually exists versus what the arc assumed:
- **Real:** `.github/workflows/heartbeat-monitor.yml` — a genuine external dead-man's-switch (polls `webhook.dowiz.org` every 10 min, Telegram-alerts on failure). Caveat: it watches a Cloudflare Tunnel webhook endpoint, **not "the app"** — there is no app running. The *pattern* is proven; the *target* doesn't exist yet.
- **Real:** `kernel/src/backup.rs` (702 lines, Buzhash-CDC dedup) — a native backup primitive, but a **different design** than the arc's WAL-G/rsync.net proposal, and never exercised end-to-end (nothing to back up yet).
- **Real-but-not-this:** `tools/telemetry/` (hetzner-exporter etc.) is the self-improvement loop's own harness telemetry — not a product-facing metrics stack. No Prometheus/`remote_write` anywhere.
- **Not built:** zero VictoriaMetrics / Grafana / Netdata / Gatus / SOPS / WAL-G / OpenTofu / Dokploy / PgBouncer / Cloudflare-Tunnel-config in the repo — all future-tense doc mentions only. `docs/ops/P8-SINGLE-PANE-SPEC.md` self-labels every signal `[SPEC]` and states plainly: "No canonical prod target exists."

**Superseded — resolve explicitly:** the arc's RLS-fix approach was "resurrect attic's 140 TS migrations." That path is **dead twice over**: (a) `attic/` is physically deleted, so the premise no longer exists; (b) it is **formally superseded** by `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (committed 2026-07-18), whose §0 states "NOT a TS/Supabase migration… the old attic/packages-db 140 migrations are quarantined and dropped; we do not revive them," and whose §5 DECART table formally rejects attic-revival in favor of a native Rust/sqlx adapter. **The native pgrust rebuild is current canon.** It is already registered in `CORE-ROADMAP-INDEX.md` §7 as a separate red-line track, gated on operator `/council` review — **cross-referenced here, not renumbered into P45.**

**Blueprint:** `docs/ops/P8-SINGLE-PANE-SPEC.md` (monitoring, `[SPEC]`), `docs/design/BLUEPRINT-P-NATIVE-PGRUST-TENANT-REBUILD.md` (data layer, separate gated track). Reuse both; write nothing new until unblocked.
**DoD:**
1. A deploy artifact exists and is reachable at a canonical prod URL (falsifies "no canonical prod target exists").
2. The heartbeat dead-man's-switch is retargeted from the tunnel webhook to the live app's health endpoint, and a deliberately induced outage produces a Telegram alert within 10 minutes.
3. `kernel/src/backup.rs` exercised end-to-end against real tenant data: backup → restore → byte-identical verification.
4. **Off-Hetzner immutable backup exists** — the arc's own #1 flagged risk, still completely unaddressed. A restore from the off-site copy succeeds with Hetzner unreachable. This is the one item that stays red until proven.
**Anti-scope:** **This phase is BLOCKED — not merely sequenced after — on DELIVERY P37 existing.** Do not stand up VictoriaMetrics/Grafana/any observability stack before there is a service emitting signals; do not write OpenTofu/Dokploy config for infrastructure that hosts nothing; do not revive attic migrations (canon forbids it and the files are gone); do not fold the pgrust rebuild into this phase's numbering (it is an operator-gated red-line track in CORE-ROADMAP-INDEX §7).
**Depends on / blocks:** Hard-blocked by DELIVERY P37 (something to deploy). Data-layer items depend on the pgrust tenant-rebuild track clearing its `/council` gate. Blocks P46 (no multi-product platform without a deployed first product). DoD-4 (off-site backup) should be first in line the moment P37 produces state worth protecting.

##### P46 — Multi-Product Platform: "dowiz Local" + Marketplace — FURTHEST FUTURE
**Absorbs:** EC-17 and the multi-product/marketplace remainder of the ecosystem-strategy arc, including "dowiz Local" (the planned second product intended to prove multi-product reuse — never shipped, zero grep hits in the repo).
**Status:** PLANNED (0%)
**Role & responsibility:** The ecosystem endgame: prove the CORE/INFRA/FLOWS decomposition by shipping a second product on the same kernel, then (and only then) generalize toward a marketplace. Nothing exists; nothing should, yet.
**Blueprint:** none — source arc: `/root/.claude/projects/-root-dowiz/memory/ecosystem-strategy-arc-2026-07-13.md`. No blueprint until the gate below is met.
**DoD:**
1. A second product ("dowiz Local" or successor) runs on the unmodified kernel with zero kernel forks — falsified by any product-specific patch to CORE.
2. Reuse is measured, not asserted: the second product's non-kernel code line count is published against the first product's.
**Anti-scope:** **Do not start this before a single product has second-tenant proof** — the original EC plan's own sequencing wisdom warned against building marketplace infrastructure before proving reuse empirically, and that warning is honored here as a hard gate, not a suggestion. No marketplace scaffolding, no plugin registry, no partner API before DoD-1 of this phase is even startable, which itself requires DELIVERY P37/P38 live with real tenants.
**Depends on / blocks:** Depends on literally everything above: DELIVERY P37/P38 live, P45 ops floor green (including off-site backup), P43 at least one working external port. Blocks nothing — it is the terminal node of the entire roadmap.

---

### 11. Gap-closing phases (2026-07-18, found by the §10 end-state-vision pass)

Appended by the 2026-07-18 end-state-vision follow-up pass (same session as §10; same
append-only rule as §7/§8/§9/§10). **This section extends the phase index from P31–P46 to
P31–P50.** §10.2's index table originally still read "P31–P46" and was deliberately left
untouched here (a parallel pass may have been editing nearby text); the anticipated later
consolidation pass extended it through P53 on 2026-07-18. This section remains the full-text
authority for P47–P50. Blueprint — ONE combined file for all four (deliberately; see its own header for why):
`docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P47-P50-gap-closing-phases.md`.

#### 11.0 Why this section exists

These four phases exist because the end-state-vision pass — walking concrete user scenarios ("a
customer orders dinner and pays," "an owner changes a price," "a regulator asks a question,"
"the first real order happens") through the full P01–P46 set — found real functional gaps that
no existing phase owns, recorded in that pass's silence ledger: no phase names how money
physically enters or leaves the system (its own words: "the largest silence in the entire
end-state"); no phase owns the owner's operational surface (P37's anti-scope explicitly
excludes "an admin CRUD surface," P38b's Sea & Sheet are customer-facing, and no P31–P46 DoD
says "owner edits a menu item and sees it live"); no phase specifies how an anonymous customer
orders, tracks, and re-identifies (capability certs are specified for couriers/operators/
devices only), nor real customer notifications, nor a tracking UX over the geo math that
already exists; and no phase carries the legal/compliance surface the old stack had (GDPR
routes) or promotes the "first real order" proof (§7's own G11 flag, unresolved) from a late
done-test to a tracked gate. These are not speculative scope creep — they are structural
absences the roadmap's own scenario walk revealed, added under the operator's paired directive
("знайти прогалини, сліпи зони у роадмапі і добавити, розширити" AND "нічого не добавляти що не
критично"): exactly these four, and nothing else.

##### P47 — Payment & settlement rails (DELIVERY component; extends the P37–P39 range)
**Absorbs:** none — genuinely new; no prior unit ID anywhere names a payment rail (grep for
payment/stripe/liqpay/cash-on-delivery across `kernel/`, `engine/`, `web/`, `llm-adapters/` and
bebop2's `delivery-domain`/`proto-cap`: zero non-test hits, verified live 2026-07-18).
**Status:** PLANNED — decision RESOLVED (2026-07-18, operator ruling), build-out open.
*Correction (2026-07-18, later same day): PARTIAL — the wave swarm landed the Wave-0 cash rail:
`kernel/src/ports/payment.rs` (PaymentPort + CashAttestation + reconciliation) +
`kernel/tests/firewall_p47.rs` (`e6367ae73`/`de56a27d6`). Design-vs-implementation
reconciliation deliberately not done here.*
**Role & responsibility:** `SettlementRecorded` exists as a wire event
(`bebop2/proto-cap/src/event_dict.rs:122,279` — payload + variant, verified this pass) and
money math is airtight range-checked `i64` (`kernel/src/money.rs`) — but nothing names how
money physically enters or leaves. P47 owns that boundary: a payment-provider port trait in the
kernel-ports layer (`kernel/src/ports/`, mirroring `llm.rs` conventions) behind a
capability-scoped adapter under the same compilation-firewall pattern as KernelFacade (§10.3
invariant 5). **Cash-on-delivery is the recommended Wave-0 rail, named explicitly:** it is the
only rail with zero external dependency, zero vendor, and zero central authority — exactly the
mesh's own local-first stance — with the courier's signed cash-collected attestation as the
`SettlementRecorded` source. Card/digital rails are a later, more complex addition requiring a
real payment-processor integration decision this roadmap does NOT make unilaterally — ⚠
OPERATOR DECISION (see §11.2-1).

> **RESOLVED (2026-07-18, operator ruling):** rail sequencing decided in three waves.
> **Wave 0 = cash** — the blueprint's own recommendation is now CONFIRMED by the operator, not
> merely recommended. **Wave 1 = crypto** — explicitly ordered BEFORE conventional payment
> processors ("у планах крипта, та останнє уже платіжні системи"). This ordering is not
> arbitrary: a crypto payment is a signed transaction, which fits the mesh's own
> capability-cert / PQ-signature settlement model (signed `CashAttestation`-style events,
> `verify_chain`/`RevocationSet` reuse) far more naturally than a centralized-processor
> integration — the rail extends machinery the stack already trusts instead of importing a
> foreign trust model. **Wave 2 (last) = Stripe / Payoneer / Google Pay / Apple Pay**, and for
> this wave the operator BINDS an explicit constraint: use OFFICIAL, PROVEN THIRD-PARTY
> LIBRARIES — no custom native reimplementation ("варто застосовувати готові і перевірені
> бібліотеки без власного нативного коду"). This is a DELIBERATE, NAMED EXCEPTION to the repo's
> native-Rust / re-derive-first default (memory: `rust-native-bare-metal-decision-2026-07-14` —
> which itself demands honest falsifiable comparison, not purity): payment-processor
> integration is high-liability, PCI-DSS-adjacent compliance surface where reinventing audited,
> certified handling in native code is a real security/liability risk, not a purity concern.
> Official SDKs exist precisely because this territory is solved and certified. Verified live
> on crates.io 2026-07-18: Stripe publishes NO first-party Rust SDK; the de-facto crate is
> community-maintained `async-stripe` (1.0.0-rc.6, actively maintained) — so Wave-2 candidates
> are `async-stripe` OR Stripe's official REST API directly, and Google Pay / Apple Pay via
> their standard web/native Payment Request APIs. Final vendor pick within this constraint
> stays a build-time engineering choice — the operator did not pick a vendor and neither does
> this note.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P47-P50-gap-closing-phases.md` §2.
**DoD:**
1. A `PaymentPort` trait exists in the kernel-ports layer (plain structs, no HTTP/serde in
   kernel, per `ports/llm.rs` conventions); rail adapters live outside the kernel; `cargo tree`
   shows the kernel has no payment-adapter dependency (same firewall proof as §10.3 invariant
   5), red-proof committed.
2. Cash-on-delivery wired end-to-end: an integration test drives place → deliver → courier
   cash-collected attestation → `SettlementRecorded` folded, over P37's wire, all amounts `i64`.
3. Reconciliation property test: folded settlement totals equal fold-derived order totals
   exactly (integer equality, no epsilon), across arbitrary order sequences.
4. Card/digital rail: a dated operator decision note (vendor, geography, fee model — operator
   judgment) exists BEFORE any card-rail adapter code lands; adapter code present without the
   note is the fail condition.
   — *RESOLVED-in-part (2026-07-18): the ruling above IS the sequencing + constraint note
   (waves fixed, official-libraries-only for Wave 2). The specific Wave-2 vendor pick is
   delegated to build time WITHIN that constraint; geography/fee-model specifics still surface
   to the operator when a concrete vendor is proposed.*
5. *(added 2026-07-18, per ruling)* Wave-1 crypto rail: a design note maps crypto settlement
   onto the existing signed-event model (attestation-style signed transaction →
   `SettlementRecorded` fold, `verify_chain`/`RevocationSet` reuse, all amounts `i64`) BEFORE
   any crypto adapter lands; gated behind DoD-2 (cash rail green first).
6. *(added 2026-07-18, per ruling)* Wave-2 processor rail: adapters wrap an official/proven
   third-party library only — candidates to evaluate: `async-stripe` (no first-party Stripe
   Rust SDK exists; verified crates.io 2026-07-18) or Stripe's official REST API directly;
   Google Pay / Apple Pay via their standard Payment Request APIs. RED check: any custom
   native implementation of processor-side payment cryptography or card-data handling is the
   fail condition.
**Anti-scope:** Do NOT build a custom payment processor. Do NOT touch the money
integer-arithmetic law — it is CORE's scope and already correct. Do NOT couple to any specific
geography's payment rails (bank APIs, national schemes) without an operator ruling. No
card/digital adapter before DoD-4's note exists. *(2026-07-18 addendum: "no custom payment
processor" is now reinforced and extended by the Wave-2 ruling — no native reimplementation of
processor SDK territory either; official libraries are binding there, a named exception to the
native-Rust default.)*
**Depends on / blocks:** Depends on P37 (an order surface to settle against). Blocks nothing on
the wiring critical path — deliberately late-critical-path: needed before real revenue (P50's
first-real-order gate names it a prerequisite), not before the wiring proof.

##### P48 — Owner/Admin operational surface (DELIVERY component)
**Absorbs:** none — new; makes concrete the workflow implied by menu-as-data + capability certs
(silence-ledger item 2), which every existing phase implies and none owns.
**Status:** PLANNED — decision RESOLVED (2026-07-18, operator ruling), build-out open
**Role & responsibility:** The venue owner's working surface: menu editing, live order
visibility, and staff/courier roster management. Today this is owned by nobody — P37's
anti-scope explicitly excludes "an admin CRUD surface," P38b's Sea & Sheet are customer-facing,
and no P31–P46 DoD contains "an owner edits a menu item and sees it live." The blueprint's
FIRST open question — named here, not decided: is the admin surface WebGPU-rendered like the
customer surface (§10.3 invariant 4), or does it get a DOM exemption on FE-15-adjacent
reasoning (the a11y mirror already establishes that DOM survives where WebGPU genuinely cannot
serve; admin UIs are data-dense and form-heavy)? ⚠ OPERATOR DECISION (see §11.2-2).

> **RESOLVED (2026-07-18, operator ruling):** two decisions in one ruling. **(a) Rendering:
> WebGPU, NO DOM exemption.** The interface logic is the same as everywhere else in the
> product — "продовження рендер бекенду через фізику," a continuation of the backend rendered
> through physics. §10.3 invariant 4 holds uniformly; FE-15's a11y mirror remains the only DOM
> survivor. **(b) The role itself is bigger than the open question assumed: the admin surface
> IS a HUB architecture.** The operator's own framing: the owner manages and processes the
> food vendor and its orders arriving from MULTIPLE INTAKE CHANNELS — social media, websites,
> bots, etc. — all funneling into ONE hub, with agentic support ("тут власне уся суть, що
> замовити може будь-хто і з різних входів"). Omnichannel order intake is therefore not a
> P22/P43 nice-to-have — it is what P48's hub architecture actually IS: every intake channel
> maps into the SAME order pipeline, i.e. the same
> `DeliveryEvent::OrderPlaced(OrderPlacedPayload)` wire vocabulary P34 already defines
> (`bebop2/proto-cap/src/event_dict.rs:279` variant, `:106` payload — verified live
> 2026-07-18). Agentic support ties to P40's tool loop: an agent can plausibly help the owner
> triage/process orders arriving from different channels. Boundary note: INBOUND channel
> intake belongs to P48's hub; the OUTBOUND notification send path stays P43's (unchanged).
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P48-owner-hub-surface.md`
(standalone, promoted 2026-07-18 — carries this entry's DoD 1–7 forward and adds the
same-day scope expansion: two-way messenger order flow, adaptive notification channels,
Google-Maps reviews ingestion, event-log-first hub sync; the original resolved-decision
text remains in `BLUEPRINT-P47-P50-gap-closing-phases.md` §3 as provenance).
**DoD:**
1. Rendering-approach decision recorded (operator ruling, dated) before surface build-out.
   — *✅ RESOLVED (2026-07-18): WebGPU, no DOM exemption — see the ruling note above. Surface
   build-out is unblocked.*
2. An owner edits a menu item and sees the change reflected in a live order-flow test: edit → a
   subsequently placed order's fold-derived state carries the change. (The sentence no P31–P46
   DoD contains; this phase's reason to exist.)
3. Live order visibility: the owner surface lists current orders as a read-only projection of
   fold-derived state — no shadow state.
4. Roster: an owner grants and revokes a courier capability cert through the surface,
   exercising the existing proto-cap issuance + `RevocationSet`
   (`bebop2/proto-cap/src/revocation.rs:49`, verified this pass); a revoked courier's next
   mutating request is rejected in a test.
5. Auth: the surface authenticates with the SAME capability-cert model as P37 (owner-scoped
   cert); a negative test proves no password-based admin login path exists.
6. *(added 2026-07-18, per ruling)* Omnichannel intake, Wave-0: at least TWO concrete
   non-native intake channels land as candidates — (i) a social-media DM/message intake
   adapter and (ii) a simple web-form intake — BOTH mapping into the same
   `DeliveryEvent::OrderPlaced(OrderPlacedPayload)` vocabulary
   (`bebop2/proto-cap/src/event_dict.rs:279`/`:106`, verified live 2026-07-18). RED check: an
   intake channel minting its own order representation instead of `OrderPlaced` is the fail
   condition — channels differ, the pipeline does not.
7. *(added 2026-07-18, per ruling)* Agentic support: a design note ties hub triage to P40's
   tool loop (agent-assisted processing of orders across channels); advisory at Wave 0, not a
   gate on DoD-2/3/4.
**Anti-scope:** NO separate admin-password system — a second, weaker auth path for the most
privileged user is an anti-pattern, explicitly rejected (capability certs are the auth model
per §10.3 invariant 3; TOTP/WebAuthn are step-up only per P39). Do NOT build a general-purpose
admin framework — scope is exactly the named menu/order/roster operations. No
analytics/marketing dashboards (P20/P22/P43 territory). *(2026-07-18 correction, per ruling:
the P22/P43 boundary above governs dashboards and the outbound send path only — INBOUND
omnichannel order intake is P48's own hub scope, not deferred territory.)*
**Depends on / blocks:** Depends on P37 (auth + API surface); on P38a only if the rendering
ruling picks WebGPU. *(2026-07-18: the ruling picked WebGPU — the P38a dependency is now
unconditional.)* Blocks P50's first-real-order gate (a real venue needs a managed menu).

##### P49 — Customer identity, notification & tracking UX (DELIVERY component)
**Absorbs:** the customer-side closure of P43's corrected claim (§10.5.5 confirmed "Telegram
already has full push+OTP" FALSE — a real customer-facing send path does not exist); otherwise
no prior unit ID.
**Status:** PLANNED — decision RESOLVED (2026-07-18, operator ruling): planned-but-deferred;
simple Wave-0 default now, mechanism revisited at 5–50 real clients.
*Correction (2026-07-18, later same day): PARTIAL — the wave swarm landed the Wave-0 default:
`kernel/src/ports/customer.rs` per-order capability grant identity (option 2, privacy-minimal,
`f55ff8911`/`69bdb2a71`). Design-vs-implementation reconciliation deliberately not done here.*
**Role & responsibility:** Three inseparable customer-facing concerns. (a) **Identity** — how an
anonymous customer places, tracks, and re-identifies to an order WITHOUT a device-bound
capability cert: certs are specified for couriers/operators/devices, and requiring a customer
to enroll a hardware identity to order food is not plausible — extending certs to customers
must be justified, not assumed. (The old stack solved this with `softVerifyAuth` anonymous
order tracking — commit `c3bd16cf9`, deleted with the purge — a real precedent, not a design
from nothing.) (b) **Notifications** — real order-status delivery to the customer's channel:
P43 DoD-2 builds the transmitting send path; this phase is its customer-side consumer, closing
the correction from the customer's perspective. (c) **Live tracking UX** — the existing
Kalman/EMA geo math (`kernel/src/kalman.rs`; `kernel/src/geo.rs:39 ema_next`, verified this
pass) rendered through P38's pipelines; no §10 phase specifies this today. The identity
mechanism is ⚠ OPERATOR DECISION (see §11.2-3) with three named candidates, none picked here:
(1) short-lived session token bound to a device fingerprint; (2) a lighter capability grant
scoped to a single order (reuses proto-cap machinery, no hardware enrollment); (3) magic-link
via email/SMS.

> **RESOLVED (2026-07-18, operator ruling):** "варто спланувати, та узагалі некритично і
> відкладається до перших 5/50 реальних клієнтів" — worth planning at design level, NOT
> critical, the mechanism decision is DEFERRED until the first 5–50 real clients exist. The
> operator gate on the mechanism pick is LIFTED and demoted to a build-time engineering
> choice: pick a simple pragmatic default from the three named candidates as a Wave-0 minimal
> default WITHOUT extensive validation (the blueprint's own table already notes candidate 2 is
> pure proto-cap reuse and best offline-fit — but the pick stays with the build, not this
> note), then revisit properly once real usage data exists. Do not over-engineer or block
> anything on perfecting identity now. **Urgency context (operator, same date, recorded as
> context not decision):** "потрібен, перший клієнт тестував і чекає на оновлену частину, ще
> декілька клієнтів також ЧЕКАЮТЬ" — a first real client has already tested the product and is
> waiting for the updated version, and several more clients are also waiting. That is why
> "simple default now, don't perfect it" is the right call: the roadmap needs a working simple
> version FASTER than a perfect one. Cross-reference: this feeds P50's first-real-order gate
> directly (blueprint §5.3) — that milestone is not hypothetical; real clients are already
> waiting on it.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P47-P50-gap-closing-phases.md` §4.
**DoD:**
1. Identity-mechanism decision recorded (one of the three candidates, or an operator-supplied
   better one), dated, before build-out.
   — *✅ RESOLVED (2026-07-18): the operator ruling above replaces the mechanism ruling with a
   deferral — build picks a simple default from the three candidates (build-time engineering
   choice, no further operator gate), records THAT pick as a dated note, and the proper
   mechanism decision is re-opened at 5–50 real clients. Build-out is unblocked.*
2. Build-out once decided: an anonymous customer places an order and later re-identifies to
   track it, over P37's wire, with no durable customer account created — one integration test.
3. One real notification reaches the customer's channel on an order state change (rides P43
   DoD-2's send path; stays RED until that path actually transmits).
4. A live tracking view renders real geo state (Kalman/EMA output) through P38a's pipelines,
   with a deterministic test against kernel math per P38's own convention.
**Anti-scope:** No customer account/profile system beyond what one order needs — no loyalty, no
CRM, no marketing identity. Do NOT conflate customer identity with courier/operator identity
(device-bound certs stay theirs). Do NOT build a second notification transport — P43 owns the
send path.
**Depends on / blocks:** Depends on P37 (wire), P38a/P38b (tracking render), and P43 DoD-2 (a
transmitting messenger path). Blocks P50's first-real-order gate (its "real customer" leg).

##### P50 — Legal/compliance & first-order validation gate (ECOSYSTEM/OPS component; extends the P43–P46 range)
**Absorbs:** G11 (§7's self-critique flagged "first real order" as the only proof the product is
wanted, sitting as a late done-test — unresolved, operator-level) + the audit obligation implied
by the old stack's deleted legal surface.
**Status:** PLANNED
**Role & responsibility:** Two distinct things deliberately bundled, because both are "did we
forget something structurally important" GATES rather than build-heavy phases. (a) **Compliance
audit:** the old stack had real GDPR machinery — `attic/apps-api/src/routes/owner/gdpr.ts`,
`attic/apps-api/src/workers/anonymizer-gdpr.ts`, `attic/apps-api/src/public/admin/gdpr.html`
(deleted `f9ab28ff1`) and `packages/shared-types/src/contracts/owner/gdpr.ts` (deleted
`79ef316f6`) — verified in git history this pass; `attic/` itself is no longer on disk, so git
history is the source. The new roadmap never mentions the topic. The audit proves the pivot did
not silently drop a legal obligation — it is NOT a full compliance program. (b) **First-order
gate:** promote "one real order through the full stack, end to end, for a real transaction"
from a late incidental done-test to an explicit Wave-0-style gate the roadmap tracks as a
first-class, dated milestone — separate from, and prior to, any scale-out work.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P47-P50-gap-closing-phases.md` §5.
**DoD:**
1. A written audit compares the old-stack legal/compliance surface (recovered from pre-purge
   git history — the four files above are the starting inventory; tax/food-safety handling is
   greppable the same way) against new-stack status, with EVERY item marked exactly one of:
   ported / deliberately-dropped-with-reason / genuinely-missing. No item left unclassified.
2. Every audit item requiring real legal judgment is flagged ⚠ OPERATOR/COUNSEL — this phase
   does NOT self-certify compliance claims (the standing anti-self-certification rule applies
   especially hard to legal claims).
3. A named, dated milestone — not just a test — exists for "first real order, real money, real
   courier, real customer," with an explicit go/no-go checklist whose prerequisites are P47 (a
   way to pay), P48 (a managed menu), and P49 (a customer who can order and track), on top of
   the P34→P37 critical path.
**Anti-scope:** Not a legal-implementation project — audit-and-gate only; no compliance
framework, no policy generators, no legal-department process. No self-certified compliance
claims, ever. Do NOT let the milestone decay back into a test-suite line item — it is an
operator-visible go/no-go event.
**Depends on / blocks:** The audit half depends on nothing — git history exists today; it is
the one genuinely unblocked item in this section, startable now. The gate half depends on
P47/P48/P49 plus the P34/P37/P38 critical path. Blocks P46 (and any scale-out): the
first-real-order gate must be green before multi-product work means anything.

#### 11.1 Silence-ledger cross-reference (nothing orphaned)

| # | Silence-ledger item (end-state-vision pass, its own words) | Closing phase |
|---|---|---|
| 1 | "no phase names how money physically enters or leaves — no payment-provider port, no cash-handling flow, no fiat leg… the largest silence in the entire end-state" | **P47** |
| 2 | "P37's anti-scope explicitly excludes 'an admin CRUD surface'; P38b's Sea & Sheet are customer-facing; no P31–P46 DoD says 'owner edits a menu item and sees it live'" | **P48** |
| 3 | "How an anonymous customer orders, tracks, and re-identifies… is unspecified" + customer notifications exist only as P43's to-be-designed send path + no §10 phase specifies the tracking UX over the existing Kalman/EMA geo math | **P49** |
| 4 | "the old stack had GDPR routes; the new roadmap never mentions the topic" + G11: the first-real-order proof "sits as a late done-test, not a Wave-0 gate. Unresolved, operator-level" | **P50** |

#### 11.2 Operator decisions introduced by this section (3 — same convention as §3)

> **ALL THREE RESOLVED 2026-07-18 (operator ruling; full text in each phase's RESOLVED note
> above — original framings preserved below, per convention).**

1. **P47** — which card/digital payment rail (vendor, geography, fee model), if any, follows
   cash-on-delivery. The Wave-0 cash rail itself needs no ruling — it has no vendor to choose.
   — *✅ RESOLVED (2026-07-18): waves fixed — cash (confirmed) → crypto → processors last;
   Wave-2 binds to official/proven third-party libraries, no native reimplementation; specific
   Wave-2 vendor delegated to build time within that constraint.*
2. **P48** — admin-surface rendering: WebGPU per §10.3 invariant 4, or a DOM exemption on
   FE-15-adjacent reasoning for a data-dense/form-heavy surface.
   — *✅ RESOLVED (2026-07-18): WebGPU, no DOM exemption; plus the role is a multi-channel
   intake HUB with agentic support — see the P48 ruling note.*
3. **P49** — customer identity mechanism: device-fingerprint session token vs one-order
   capability grant vs magic-link email/SMS (or an operator-supplied alternative).

---

### 12. Operator-directed phases (2026-07-18, appended after §11)

Appended by a separate 2026-07-18 pass (same append-only rule as §7-§11). **This section
extends the phase index from P31-P50 to P31-P51.** It is deliberately NOT folded into §11:
§11.0's own charter is "exactly these four, and nothing else" (the end-state-vision pass's
silence ledger), and P51 comes from a direct operator directive, not from that pass — a
different provenance deserves a different section. (§10.2's index table was extended through
P53 on 2026-07-18 — the consolidation pass §11's note anticipated.)

##### P51 — Open map + routing: OSM vector data, field-rendered routes, pin-drop, live tracking (DELIVERY component)
**Absorbs:** none — genuinely new phase; it *feeds and closes* existing seams rather than
absorbing units: P04's landed in-kernel router (`kernel/src/router.rs` — Dijkstra/A*/CH +
`road_graph_from_ways`, whose own doc names OSM parsing "a downstream concern" — P51 IS that
concern), P04's never-landed `route_js` wasm line (0 grep hits in `wasm/src/lib.rs`, verified
2026-07-18), P49's DoD-4 tracking-view supply side, and the gaussian-splatting arc's Stage-1
pin-drop (supplied, not re-litigated).
**Status:** PLANNED
**Role & responsibility:** Operator directive (2026-07-18, verbatim intent): OpenStreetMap
with pin-drop + route tracking, or better a physics-render of the route/map from satellite
data — hard constraints non-paid, non-vendor-lock-in. The blueprint's cited 2026 research
verdict: satellite-based street rendering is infeasible without cost (free global optical
tops out at Sentinel-2's 10 m/px — a road is one pixel; every sub-meter source is paid,
non-commercial, or country-patchwork; imagery-tile ToS forbid derivative offline use), which
independently re-confirms the splatting arc's own satellite rejection from a new angle. The
chosen design delivers the operator's "better and more interesting" branch honestly: **OSM
vector data (ODbL) rendered through the existing field engine** — roads and building outlines
as `SdfShape::LineSegment` scene layers, the planned route as a field *source term* whose
glow is `compose()`'s own diffusion (the physics-render, by construction), courier marker as
a P38-G2 particle, routing via the already-landed zero-dep kernel router, live tracking via a
`kalman.rs` constant-velocity configuration + `geo.rs` route snap/ETA, pins via
`nearest_road_node` + `point_in_polygon` zone gating. Fully offline-capable (F12): one
content-addressed MapPack per venue region, no tile server, no routing server, no geocoder at
runtime. A spectral/Laplacian *layout* of the road network was explicitly rejected (topology
≠ geography; a navigator needs geographic fidelity) — the field integration is real, not
decorative.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P51-open-map-routing.md` (full
20-point-standard blueprint: research citations, DECART engine comparison — OSRM/Valhalla/
GraphHopper honestly compared and rejected Wave-0 on substrate, with Valhalla pre-named as
the dynamic-costing fallback boundary — MapPack format, seven build items M1-M7, DoD,
benches, ODbL compliance).
**DoD (summary — falsifiable detail in the blueprint §6):**
1. MapPack pipeline: deterministic extractor (`tools/map-pack`, byte-identical reruns) +
   fail-closed kernel parse (bit-flip ⇒ typed refusal, truncation fuzz panic-free).
2. Pin-drop: on-street pin snaps via `nearest_road_node`; out-of-zone pin refused before
   `decide`; unroutable tail honest (raw pin + walk distance, never fabricated).
3. Route render: composed frame byte-identical across runs; route glow localized to the
   route polyline; unreachable destination ⇒ typed `NoRoute` + labeled straight-line hint —
   a fabricated road path is unrepresentable.
4. Live tracking: event-sequence tests (drive ⇒ `[Updated…, Snapped…, Arriving]`; detour ⇒
   `OffRoute×K ⇒ RerouteNeeded`); GPS noise burst absorbed by the KF without false off-route;
   teleport/out-of-order samples rejected with bit-stable filter state.
5. One `TrackFrame`, two consumers: courier surface and customer live view (this supplies
   P49 DoD-4 — P49 cites, does not re-implement); wasm ptr/len exports close P04's gap.
6. Privacy: `CourierPositionUpdated` (≤32 B, ≤0.5 Hz) emittable ONLY between
   assignment-accept and delivery-complete — asserted at the emit site.
7. ODbL: "© OpenStreetMap contributors" rendered on every map view (a11y-mirror path now,
   MSDF when P38-G3 lands); MapPacks published under ODbL; no proprietary geometry ever
   inside a pack (collective-database invariant); P50 audit row added.
**Anti-scope:** NO paid mapping/geocoding/imagery API ever, including as fallback (hard
operator constraint — a Google/Mapbox/HERE import is a scope violation regardless of test
state). No turn-by-turn voice (AGENT/DZ-10 Phase-9b territory). No text-address
geocoding/autocomplete Wave-0 (pin-first; self-hosted Photon/Nominatim is the named future
unit). No tile servers, no planet scale, no live-traffic dynamic costing (Valhalla self-host
pre-named at that boundary). No satellite texture work (Sentinel-2 10 m ambient backdrop
recorded as deferred-decorative in the blueprint, not scope). Does not touch splatting
Stage-2, money, or any red-line.
**Depends on / blocks:** Depends on P38a (G2/G3 render legs; CPU compose path works today —
map/route/track math and tests are GPU-independent), P34/P37 (the wire the position event and
MapPack asset ride; local-first paths work without them per F12), and nothing else. Blocks
P49 DoD-4 (its tracking view consumes P51's `TrackFrame`) and the splatting arc's Stage-1
dependency; feeds P50's audit with its ODbL row.
   — *✅ RESOLVED (2026-07-18): deferred until 5–50 real clients; simple Wave-0 default picked
   at build time from the three candidates, no further operator gate; real clients already
   waiting elevates urgency of the simple version (see P49 ruling note).*

---

### 13. Audit-minted phases (2026-07-18, appended after §12)

Appended by a separate 2026-07-18 pass (same append-only rule as §7-§12). **This section
extends the phase index from P31-P51 to P31-P52.** Provenance: the same-day MVP audit
(`docs/design/DELIVERY-MVP-FEATURE-COMPLETENESS-AUDIT-2026-07-18.md`) found exactly one
MVP-blocking ownership vacuum (§6 M1, "the largest single omission this audit found") and the
operator directed minting a phase for it. P52 is DELIVERY-component work and belongs
conceptually beside P37-P39/P47-P49 — it is appended HERE rather than inside §10.5.3 because
the append-only convention (§12's own precedent: P51 is DELIVERY too and got its own tail
section) beats section-thematic placement. (§10.2's index table was extended through P53 on
2026-07-18 — the consolidation pass §11's note anticipated.)

##### P52 — Courier working surface: shift, claims, run, proof-of-delivery, earnings (DELIVERY component)
**Absorbs:** none — genuinely new phase. It *executes and closes* existing seams rather than
absorbing units: DZ-08's courier interaction design
(`docs/design/dowiz-interfaces/BLUEPRINTS-DOWIZ-INTERFACES.md:225` — designed in the arc,
executed by nobody: P38b is customer-facing by its own §10.5.3 text), the MVP audit's M1
(courier surface), M4 (matcher candidate-set supply — `matcher.rs:63 assign(order,
candidates, max)`'s `candidates` has no producer; grep for shift/on_duty/availability across
delivery-domain + proto-cap: zero hits, re-verified 2026-07-18), and M10 (the P48-DoD-4 ↔
P23-P2 courier-invite handoff seam, "implied by both DoDs and named by neither").
**Status:** PLANNED
**Role & responsibility:** The courier's own working surface — the third leg of the one
physics-render pattern (customer = P38b Sea & Sheet, owner = P48 hub, courier = P52), on the
SAME P38a substrate under the P48 rendering ruling (WebGPU, no DOM exemption) — for the actor
whose PROTOCOL side is the most built part of the stack (claim_machine, HRW matcher, k-of-n
PoD, settlement events — all landed and tested in bebop2) and whose SCREEN was owned by
nobody. Seven build items: K1 availability (the Wave-0 candidate-set rule stated as law —
all certified-unrevoked couriers, pull-based claims — plus a node-local duty fold + cap-gated
toggle; deliberately NOT a new proto-cap wire variant), K2 claim inbox consuming
`DeliveryEvent::Claim` (`Action::ClaimOffered/ClaimAccepted/ClaimReleased`,
`bebop2/proto-cap/src/scope.rs:94-98`, `event_dict.rs:294-297` — relayed intents only, claim
Law legality stays receiver-side), K3 delivery-run screen consuming P51's
`map_scene`/`TrackFrame` (routing/tracking 100% P51's, zero re-design), K4 proof-of-delivery
capture — the UI for the BUILT k-of-n hybrid-signed `DeliveryClaim`
(`bebop2/delivery-domain/src/pod.rs:62-74`; its `location` is opaque bytes with NO photo/
signature/GPS-fence concept — P52 pins the 12-byte micro7 geo encoding and gates `Delivered`
on `is_settled()`), K5 earnings as a derive-only second reader over `SettlementRecorded`
folds (D5 pattern; zero new money logic), K6 the concrete invite handoff (owner mints a
short-lived single-use DOMAIN_DELEGATION-scoped enrollment capability → QR/deep-link → the
courier's un-enrolled device redeems it through P39's `enroll_device` and comes out
cert-enrolled; manual operator ceremony documented as the courier-#1 MVP fallback), K7 the
cash-collected attestation input (P47 Wave-0's `SettlementRecorded` source — hub-derived
amounts, witness-typed emit site). Phase-level falsifier: one end-to-end test from
un-enrolled device to statement row.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P52-courier-working-surface.md`
(full 20-point-standard blueprint: live-verified ground truth incl. the availability-gap and
PoD-shape findings, K1-K7 build items, adversarial sets, DoD, budgets, ledger rows).
**DoD (summary — falsifiable detail in the blueprint §5):**
1. Availability: bootstrap rule (empty duty fold ⇒ all certified-unrevoked candidates) +
   toggle exclusion + duty≠claim decoupling, all tested; revoked courier's toggle 403s.
2. Claim flow: offer→inbox fold; accept/decline event sequences; 60s expiry → `ClaimReleased`
   → `primary_for` requeue (never-drop re-asserted); illegal accept = typed refusal, no
   shadow state; island accept = queued-unconfirmed intent, never a fabricated `Claimed`.
3. Run screen: renders P51 `TrackFrame`; SwipeToComplete never-fake-success (completes only
   on receiver-confirmed fold); stale track labeled, never presented live.
4. PoD: capture → k-of-n signature collection → `is_settled()` gates the `Delivered` intent;
   below-threshold/tamper/duplicate-signer arms asserted at the surface; photo evidence
   content-addressed and NOT signature-load-bearing (gated `#[ignore = "M3-blob-path"]`).
5. Earnings: courier-scoped statement, integer-exact, reconciles against the ledger fold
   (P47 DoD-3's property shape shared).
6. Invite handoff: mint → redeem → cert passes `verify_chain` and admits a courier route
   end-to-end; expired/spent/bad-chain/revoked-issuer all refuse; single-use enforced.
7. Cash attestation: emittable only from a Delivered-pending run (witness type); amount
   hub-derived, never UI-supplied; double-tap idempotent.
**Anti-scope:** NO fourth rendering technology (P38a pipelines only; P48's WebGPU ruling
inherited; zero visible DOM). NO new proto-cap `Action`/`Resource` variants and NO matcher/
claim-Law changes (P34's lane — P52 is a consumer). NO map/routing/Kalman code (P51's lane).
NO payment/settlement semantics (P47's lane). NO owner/hub features (P48) or customer
identity (P49). NO courier scoring/rating/reputation in any form, ever (structural +
CI-locked; the gate extends over P52's modules). NO multi-order batching, NO tipping (each
needs its own operator ruling before existing anywhere).
**Depends on / blocks:** Depends on P34 (wire vocabulary + fold path), P38a (render
pipelines; CPU compose path usable today, GPU legs behind O18a like everyone else), P51
(routing/tracking/`TrackFrame`), P37 (routes + cap middleware for duty/claim/attestation),
P39 (`enroll_device` for K6), P48 (roster grant as K6's input), P47 (attestation semantics
K7 feeds). Blocks nothing further downstream — but it is itself **MVP-blocking** per the
audit §7 ("the courier cannot see, accept, or attest a delivery without SOME surface"): P50's
first-real-order gate cannot go green without it, so it sits on the first-transaction
critical path beside P47/P48/P49.

---

### 14. Operator-directed phases, second batch (2026-07-18, appended after §13)

Appended by a separate 2026-07-18 pass (same append-only rule as §7-§13). **This section
extends the phase index from P31-P52 to P31-P53.** Provenance: direct operator directive
activating fold-in ledger item **L4** (§9.2 — "Anonymous `.onion`/Tor tier", E53-form
waiver). The waiver's trigger ("vendor-node tier ships AND a venue requires anonymity") is
SUPERSEDED by the operator's direct request, recorded explicitly rather than silently: the
demand-signal leg is satisfied by the request itself; the vendor-node-tier leg is honored
by the phase's own wave split (code now, live onion service only WITH P37+P45). (§10.2's
index table was extended through P53 on 2026-07-18 — the consolidation pass §11's note
anticipated.)

##### P53 — Tor/onion integration: anonymous-access tier, Onion-Location + QR convenience (DELIVERY component, PROTOCOL cross-ref)
**Absorbs:** fold-in ledger **L4** (§9.2) — the only ledger item still in waiver form, now
activated. Otherwise genuinely new; it extends seams rather than absorbing units: P37's
`build_router`/headers-middleware extension point (`tools/native-spa-server/src/lib.rs:93-106`,
verified live 2026-07-18), the `deploy/` operator-run systemd tier (pgrust precedent), and
P52 K6's QR handoff (shared encoder).
**Status:** PLANNED
**Role & responsibility:** Operator directive (2026-07-18, verbatim): "можливість tor, onion
інтеграції і взаємодії — зручної" — a CONVENIENT Tor/onion access tier, standard privacy
networking (the BBC/ProPublica/SecureDrop pattern: clients reach the hub without exposing
their network identity; a hub can serve without publishing its location). The blueprint's
2026 research verdict, DECART'd against primary sources: **onion-service HOSTING = system C
`tor` daemon as an optional deploy-tier sidecar** (two torrc lines forwarding to a loopback
listener; production-grade PoW DoS defense since tor 0.4.8) — NOT embedded `arti`, because
the Tor Project's own docs mark arti's service-side hosting "suitable for testing and
experimentation only" with DoS protection unimplemented as of Arti 2.5.0 (Jun 2026); the
arti migration is a named trigger (its experimental warning drops + service-side PoW
lands), and the torrc shape is chosen to translate 1:1 to arti's `proxy_ports` when it
fires. Convenience layer: the standard **`Onion-Location` response header** (Tor Browser ≥
9.5 shows a one-tap ".onion available" pill on the clearnet site) emitted by a new tower
layer beside the existing `security_headers` middleware, plus a pure in-kernel **QR
encoder** (`kernel/src/qr_code.rs`, no new deps) feeding a two-QR share panel on P48's hub
surface: primary QR = clearnet URL (works everywhere; Tor Browser users get the pill),
secondary labeled QR = the onion URL — nobody ever types a 56-char address. **Honest
latency boundary (not oversold):** onion circuits are six relays with 0.5-1.5 s rendezvous
setup and high-variance RTT — ordering and menu browsing over Tor work well within P37/P51
budgets; the customer tracking view works labeled-degraded (~1-2 s lag); courier live
navigation is NOT offered over Tor; and hub-to-hub mesh transport over Tor is
designed-and-deferred (PROTOCOL cross-ref): Tor carries TCP streams only, so the quinn/QUIC
carrier physically cannot ride it — a future `TorTransport` is a sibling of the wss
carrier behind the same M6 Trait, deferred until a hub actually needs location-hidden or
censorship-resistant inter-hub links (trigger named in the blueprint §5.3). Trust model
untouched: a Tor client authenticates with the same capability certs on the same routes —
anonymity is network-layer only, never an auth bypass; Tor adds a privacy layer and
substitutes for none of the PQ wire security.
**Blueprint:** `docs/design/CORE-ROADMAP-2026-07-17/BLUEPRINT-P53-tor-onion-integration.md`
(full 20-point-standard blueprint: live-verified ground truth, cited 2026 research pass —
arti release line, Onion-Location spec/adoption, Tor Browser 15.0.x Android status, onion
latency measurements — DECART tables, build items O1-O6, adversarial sets, DoD, deploy
checklist).
**DoD (summary — falsifiable detail in the blueprint §6):**
1. `OnionAddress` validated type: v3-only parse, typed refusals (charset/length/v2), the
   sole path to the header value (injection unrepresentable).
2. `Onion-Location` layer: exact header on clearnet-listener responses, ABSENT on the
   onion listener (spec) and absent with no config (bit-identical responses — zero-config
   regression guard); invalid config = startup refusal, never warn-and-serve.
3. QR encoder: deterministic bit-exact fixture matrices, capacity refusal (never
   truncation), verified once by an INDEPENDENT decoder (no self-certification).
4. Deploy tier: torrc fragment (PoW ON) + operator checklist incl. 🔴 onion-key custody;
   sidecar-kill drill proves clearnet unaffected; all host steps operator-run.
5. Auth unchanged: P37's cap-gated 401/403 tests pass identically on BOTH listeners; a
   grep-lint proves listener identity never reaches auth code.
6. Two-QR share panel spec consumed by P48; onion QR pins `http://` scheme.
7. W1 (with P37+P45, operator-run): live onion service reachable via Tor Browser; measured
   Tor latency recorded once at deploy time (no fake network benches in CI).
**Anti-scope:** NO auth/trust-model change for Tor clients, ever (an auth difference between
listeners = NOT done regardless of green totals). NOT a substitute for PQ wire crypto
(MESH-10/P36 untouched). NO moderation-evasion machinery — one router serves both ingresses,
same content, same policy, same gates. NO arti or qrcode cargo dep without the named
trigger/unlock. NO mesh `TorTransport` code (deferred, PROTOCOL-owned when triggered). NO
onionbalance/bridges/single-onion mode/vanity addresses Wave-0 (each named-deferred or
rejected in the blueprint).
**Depends on / blocks:** W0 code depends on nothing (buildable today; O5 waits for P37's
api tests to exist). W1 (live onion service) depends on P37 (the HTTP surface) + P45
(deployment) and is operator-run. Feeds P48 (share panel), P52 K6 (shared QR encoder), and
P50's audit (privacy-tier row: onion access collects no client IPs by construction). Blocks
nothing — a hub without the sidecar loses only the onion mirror.

---

### 15. Verification/benchmark/research harness + local-LLM tiering (2026-07-18, appended after §14)

Appended by a same-day follow-up pass (same append-only rule as §7-§14). **This section
extends the phase index from P31-P53 to P31-P56** (§10.2's table already carries the P54-P56
rows as of this edit). Provenance: two direct operator directives — (a) a detailed technical
proposal for local-LLM resource tiering on this machine's real hardware, requiring the
already-written `BLUEPRINT-P21-local-llm-hermes-native.md` to grow a Part 2; (b) a mandate for
"a research team of top engineers/data-scientists"-grade verification harness spanning the
LLM/agent layer, the protocol/ecosystem layer, and dowiz/bebop2/openbebop jointly, grounded in
a supplied ML/CS-fundamentals glossary used as a falsifiable checklist, not prose to summarize.

#### P21 Part 2 — Tiered Intelligence architecture (extends the existing P21 phase, no new number)
Read the full design in `BLUEPRINT-P21-local-llm-hermes-native.md` §11 — this entry records
only the verdicts, per this file's own index-not-duplicate convention. The operator's
Tier-0/Tier-1/Tier-2 proposal was evaluated point-by-point against live evidence, not adopted
wholesale: **Tier-0-as-a-model REJECTED** (two deterministic routers already exist —
HK-05 `classify_complexity`, live per-turn; the planned G3 router — and P25's own DECART
already rejected LLM-in-the-loop dispatch as a category; an always-resident router model would
regress every axis it claims to improve). **Ollama stays** over raw `llama.cpp`-direct (neither
server has priority queueing, the operator's own headline reason to switch; priority lands
client-side over P25's existing admission design instead, with a named reopening trigger).
**Model verdict:** not one general model, not three local tiers — this session's own real
workload (kernel Rust with money red-lines, a dozen full blueprints, per the day's own git log)
exceeds every viable local candidate, so "Tier 2" is the *existing* remote/connected lane, not
a new local heavy model; local splits into resident code-vs-general roles; `mistral:7b`
(confirmed v0.3, 4.4GB, disk-unblocked by the same-day cleanup pass) enters as an eval-gated
challenger. **Mixtral rejection now doubly grounded**: disk (the original P21 finding) AND the
operator's own RAM math, sharpened by one more live fact — this box has 0 swap, so the failure
mode is OOM, not merely degraded latency.

#### P54 — LLM/agent behavioral verification harness (AGENT component)
**Absorbs:** none — new phase. Consumes P21 (backend), P40 (`AgentReasoner` seam), P56 (storage/scheduling substrate it runs on).
**Status:** PLANNED (blueprint ON DISK, 822 lines)
**Role & responsibility:** Adversarial/absurd-case prompt probes and behavioral verification
for the agent loop, grounded in the operator-supplied ML/CS glossary applied as a literal
checklist: tokenization failure-mode probes (letter-counting, leading-space sensitivity,
arithmetic inconsistency) mapped to concrete falsifiable tests; the glossary's own
"signals against fine-tuning" criteria applied honestly to this project's real maturity.
**DoD (summary):** a money-arithmetic-trust probe, two-pronged — (1) a *structural* always-green
fence proving no money/tax tool exists in the agent's tool namespace at all
(`MONEY_DECISION_CONSUMPTIONS_MAX=0`, proven in-process against `apply_tax()`), so a wrong
LLM-computed figure is provably unconsumable regardless of what the model says; (2) a
*behavioral* divergence-tracking probe (`money_freetext_divergence_cents`, observed not
gating) that is safe to fail precisely because prong 1 already makes it inert. Fine-tuning
readiness: **DEFERRED**, every one of the glossary's own "signals against" criteria fires
(no ≥500-example labeled corpus, no measured prompt-only baseline, CPU-only/0-VRAM hardware
independently confirms QLoRA impracticality) — `TRIGGER-FINETUNE` named (≥500 verified
examples AND measured baseline AND found insufficient AND a GPU host), zero LoRA/QLoRA
infrastructure built ahead of that trigger. Native Rust only (`agent-probe` crate, seeded
bounded wave runner, `criterion` reused for micro-benches per convention) — no Python/Bash
eval framework. 9 new `dowiz_agent_*` metric IDs extend P45 §4b.3; results feed the existing
`RegressionGate`/P32d critic/Markov detector as structured `ProbeRow`s, `rclone move`d to
`hetzner:dowiz/agent-verification/`.
**Anti-scope:** does not attempt to make the LLM compute money correctly (unachievable and
the wrong goal — the structural fence is what matters); no LoRA/QLoRA build ahead of the
named trigger; no Python/Bash test runner.
**Depends on / blocks:** depends on P21, P40, P56. Blocks nothing downstream.

#### P55 — Protocol/ecosystem testing: regression taxonomy, property/mutation testing, chaos injection (PROTOCOL/CORE cross-cutting)
**Absorbs:** none — new phase. Extends P24 (flight-recorder spans), P27 (CircuitBreaker/fault-isolation), P36 (bebop remediation).
**Status:** PLANNED (blueprint ON DISK)
**Role & responsibility:** Systematic, repo-wide testing discipline answering "how do we stop
this exact failure class" for a regression-class taxonomy (RC-1..RC-4) derived directly from
**four real bugs this same session found by hand**: RC-1 advisory-CI-gate-exists-but-unenforced
(the no_std wasm32 regression); RC-2 unexercised feature-flag combination (the `kernel-rlib`
E0004 regression — live inventory found ~14 named non-default configs, CI exercises 3);
RC-3 unsafe security default (`insecure-tls` default-on); RC-4 silently-stopped automation
(this same day's disk-cleanup finding: a memory-claimed cron job that isn't actually
scheduled). Each RC gets a concrete native-Rust mechanism with a RED-first falsifier proven
against the real historical incident (a required-checks liveness ratchet, a checked-in
feature-matrix coverage gate, a resolved-default security auditor over `cargo metadata`, a
heartbeat ledger).
**DoD (summary):** `proptest` — confirmed **already a live kernel dev-dependency** with a real
400-case suite (`ports/payment.rs:644`) — extended to `order_machine::assert_transition`,
`domain.rs::compute_order_total`, `claim_machine::assert_transition`, `matcher::assign` (all
four signatures re-verified live), and to bebop2/proto-cap where a trigger was already
recorded. `cargo-mutants` adopted as a **scheduled CI dev-binary** (local crates.io returned
403; GitHub runners have real egress — routed around the local sandbox limit rather than
skipping the tool), scoped to the deterministic core, advisory-then-gated. Chaos/network
injection extends an **already-built** `chaos.rs` (not a new mechanism) plus a weekly
netns-scoped `tc`/`netem` lane (`sch_netem` confirmed present); the offline-first
`ac6_solo_island_full_flow_no_peers` test serves as the partition oracle.
**Anti-scope:** no foreign-language test framework (Python `hypothesis` etc. rejected, Rust
`proptest`/`cargo-mutants` only); does not re-instrument what P24 already covers, only adds
the new spans this task needs; does not redesign P27's fault-isolation primitives, only adds
chaos-injection on top of them.
**Depends on / blocks:** depends on P27, P24, P56. Blocks nothing downstream; feeds tighter
regression coverage back into P34/P36.

#### P56 — Verification-harness shared infrastructure: storage, scheduling, meta-verification (ECOSYSTEM/OPS component)
**Absorbs:** none — new phase, the shared substrate P54 and P55 both consume; owns no specific probes itself.
**Status:** PLANNED (blueprint ON DISK)
**Role & responsibility:** The machinery underneath P54/P55 — result storage, async wave
scheduling, cross-platform/multi-device test-dimension modeling, and (the hardest, most novel
piece) **meta-verification**: checking that the tests/measurements themselves aren't reaching
false conclusions, not merely checking their results.
**DoD (summary):** four typed meta-verification detectors, each itself a registered probe with
a mandatory known-RED canary — **FlakyProbe** (differing verdicts on an identical
`(probe_id, probe_version, env_fingerprint, seed)` key, quantified via the kernel's own
`stats.rs::wilson_interval`, excluded from trend input so a harness bug never pages as a
product regression); **InstrumentTooNoisy** (a benchmark whose own measured noise band exceeds
the regression threshold it polices gets suppressed to `Inconclusive`, never a false
positive/negative); **StaleGround** (every probe carries content-addressed `grounds` —
fixture hashes, contract-enum shapes, doc anchors — a moved referent demotes a GREEN
structurally; **the worked fixture is this same day's own P34 case**, where the kernel gained
two new `OrderStatus` variants after being marked "proven" — this session's single most-
repeated failure mode, now encoded as the canonical test the meta-layer must catch
automatically, not found by hand again); **DeadProbe** (a probe with no known-RED canary
registers a panic, and canaries re-fire every 8 waves — GREEN-on-canary means the probe checks
nothing). Result schema is `event_log.rs`'s exact content-addressed pattern
(`TestRunEvent{schema_v, prev-chain, wave_id, probe_id+version, EnvDims, git_sha, seed,
metrics, verdict, meta}`); local storage is a 64MB-bounded index only, `rclone move`d to
`hetzner:dowiz/test-results/` after every wave (the disk-cleanup pass's `hetzner:dowiz`
remote, confirmed live 2026-07-18) — results never accumulate on local disk. A new Telegram
topic (`Testing-Research`) extends `tools/telemetry/lib.sh`'s existing `tg_send` mechanism for
ongoing best-practices research digests, per the operator's explicit request, rather than a
new bot/channel.
**Anti-scope:** does not design the specific LLM probes (P54's job) or protocol/chaos tests
(P55's job) — shared machinery only; does not invent a new scheduler (extends P25's L-class
admission) or a new alerting mechanism (extends P45's `Severity`/topic/noise-floor design);
does not pretend multi-OS/GPU testing is achievable on this single machine today (named gap,
not papered over).
**Depends on / blocks:** depends on P25 (admission), P45 (alerting), the disk-cleanup pass
(local storage headroom). Blocks nothing directly — P54 and P55 both consume it as their
storage/scheduling substrate, named as a soft dependency each.

---

### 16. Deployment topology + operating-model decisions (2026-07-18, dialogue pass)

Appended by a separate 2026-07-18 pass (same append-only rule as §7-§15). Provenance:
directly following the 5-persona hostile audit (§synthesis in `docs/research/AUDIT-2026-07-18-
SYNTHESIS-SCORECARD.md`, GO/NO-GO = NO-GO), the operator restated dowiz's target end-state in
their own words and asked for a **dialogue-format** clarification pass (not another silent
document dump) — a sequence of `AskUserQuestion` rounds, each answered directly, before any
further build work. This section is the **decisions record** of that pass; it does not
introduce new numbered phases — it constrains and cross-links existing ones (P37/P38a/P39/P52,
Sea&Sheet, P40/P41's AiMode) and settles topology questions no prior section had pinned down.

#### 16.1 Hosting topology — three modes, deliberately not mixed
Operator's own framing: *"хостинг на cloudflare pages або hetzner - щоб не змішувати +
self-host, self-app."* Three clean deployment targets for the same open hub software, chosen
per-venue, never blended within one deployment:
1. **Cloudflare Pages** — edge delivery for static/brand content (the Sheet layer, client-app
   assets). Global CDN, zero server management, fits the "installable or domain-hosted link"
   requirement for `dowiz.org`-served client apps directly.
2. **Hetzner** — dowiz-operated managed hub hosting, the default for venues that don't want to
   self-host. This is where the mesh-hub backend (kernel, event log, capability-cert store,
   the single in-hub agent) actually runs.
3. **Self-host** — the identical open hub software run on the venue's own hardware/devices.
   Not a degraded tier; the same binary, same protocol, same capability-cert model as Hetzner —
   only the physical host differs.
**Consequence for existing blueprints:** none of P37/P38a/P39/P52 assumed a single fixed host;
this section makes the three-mode split explicit so none of them silently bake in a
Hetzner-only or Cloudflare-only assumption going forward.

#### 16.2 Remote access to self-hosted/Hetzner hubs — Cloudflare Tunnel, unconditionally
Operator's ruling: *"dowiz Cloudflare Tunnel з коробки."* A venue that self-hosts on hardware
inside their own premises still needs the owner/courier to reach the hub from outside — without
the venue ever hand-configuring port-forwarding. The hub software bundles `cloudflared` and
provisions a tunnel to the operator's own Cloudflare account automatically at install time. This
is the **same** mechanism used for Hetzner-hosted hubs (no separate ingress design needed) — one
Cloudflare-side mechanism covers both non-CF-Pages hosting modes uniformly. **Open engineering
question, not yet closed**: per-venue tunnel provisioning/credential lifecycle (one Cloudflare
account fronting N independently-owned hubs — isolation between tenants at the tunnel layer)
needs its own design pass; flagged here as a named gap, not designed in this section.

#### 16.3 Courier model — venue brings its own, dowiz stays protocol-only
Operator's ruling (Recommended option, confirmed): *"Заклад приводить своїх кур'єрів."* dowiz
does not recruit, employ, or centrally pool couriers. Each venue onboards its own couriers
through the existing capability-cert flow; dowiz is dispatch protocol, not a labor marketplace.
**Confirms, does not change,** P52's existing courier-onboarding design (`BLUEPRINT-P52-
courier-working-surface.md`) — no rework needed, this closes the "which onboarding model"
question P52 had left implicit.

#### 16.4 In-hub agent — exactly one, assistant not autopilot
Operator's ruling, verbatim: *"один активний агент, для багатьох речей підійдуть автоматизовані
скрипти і автоматизації, немає потреби в окремих агентах суто на постинг чи аналітику - тут
власне агент це не автопілот, а права рука, помічник власника, щоб розвантажувати його, а не
приймати за нього рішення."* Exactly one active agent per hub (local Ollama or a connected
backend — the existing three-mode `AiMode`, unchanged). Routine/repetitive work (posting,
analytics) is handled by deterministic automation scripts, **not** separate specialized agents —
avoids an agent-per-function sprawl no one asked for. The role framing itself is load-bearing:
the agent offloads work FROM the owner, it does not make decisions FOR the owner. This is the
same structural boundary P40/P41 already enforce (AI excluded from money/order-confirm/cancel
authority) — this section extends that boundary from "money" specifically to "the owner's
decisions" generally, and settles that no per-hub arbitration/locking mechanism is needed since
there is only ever one active agent to arbitrate between.

#### 16.5 Order intake — every channel is a full-featured adapter, one kernel order-flow
Operator's ruling (Recommended option, confirmed): every intake channel (WhatsApp, Telegram,
web link, httpSMS, etc.) gets the *same* full capability — menu, payment, tracking — not a
"lightweight" subset. One order-flow lives in the kernel; each channel is a thin
transport/adapter translating its native format into the same kernel calls (ports/adapters,
already this repo's standing pattern — IP-* integration-ports arc). No channel-specific
business logic, no channel-tiering to design or maintain.

#### 16.6 Mesh topology — isolated hubs, `dowiz.org` as directory (MVP), federation named-deferred
Operator's ruling (Recommended option, confirmed): each hub is a fully autonomous, isolated
instance (own data, own couriers, own clients). `dowiz.org` is a **directory of links**, not a
federation/discovery protocol — a customer or courier does not, in the MVP, search or route
across multiple hubs simultaneously. This is the deliberately simple reading of "decentralized
mesh hubs": decentralization means no dowiz-owned central data store or control plane, **not**
inter-hub network discovery. Federation is explicitly named as a possible later addition, not
designed here — adding it later must not require re-architecting the isolated-hub model, since
each hub is already self-sufficient by construction.

#### 16.7 Auto-posting review — hybrid, owner-configurable per venue
Operator's ruling: *"гібрид, на розсуд користувача."* Consistent with §16.4's agent-role
framing (posting is a decision with brand-visible consequences, not a background operation): the
owner configures, per venue, whether posts queue for their approval before publishing or publish
autonomously from a one-time template/ruleset. Both modes must be supported; this is a setting,
not an architecture fork — no separate design path needed for each.

#### 16.8 `dowiz.org` access model — web-try-first, install as the daily-user upgrade
Operator asked directly for a recommendation (*"твоя думка? загалом для мобілок"*) rather than
choosing between options. Recommendation given and not yet contested: a web link
(`dowiz.org/s/venue-slug`, matching the existing public storefront pattern) is the zero-friction
"look, then try" path on any device — this is literally what "переглянути та спробувати"
requires. The installable client (Tauri, already Wave-0 per P39's operator-reversed verdict) is
the upgrade path for daily/repeat users — the owner managing a hub continuously, and the courier
who needs push notifications, offline resilience, and native GPS. Mirrors the standard
food-delivery UX split (DoorDash/Uber Eats: web always works, the app is for return visitors).
The operator's own mobile emphasis reinforces this rather than contesting it — Tauri 2.x's
mobile targets (confirmed earlier this session, `BLUEPRINT-AUTH-DEVICE-2FA-2026-07-17.md` §5.3b)
already carry native NFC/biometric plugins.

#### 16.9 Brand customization — confirmed as-is, no change
The operator's "kастомізовувати інтерфейс під власний бренд у межах визначеного дизайну" maps
directly onto the already-designed Sea&Sheet 5-token brand model (accent/ink/paper/type/radius) —
Sea (dowiz-owned ambient physics field/narrative layer) stays fixed, Sheet (brand content) is
customizable within that 5-token envelope. No new design work triggered by this dialogue pass;
recorded here only so the mapping is explicit and citable.

#### 16.10 Fly.io — fully retired, not deferred
Operator's ruling, twice-confirmed: kill the Fly zombie now (*"вимкнути зараз, клієнт
повідомлений про нову версію"*), remove Fly from the codebase entirely (*"прибирай з коду
повністю"*). Actioned this same pass:
- `.env` mode 666→600 (unrelated pre-existing audit action, done same session, unblocking
  nothing about Fly specifically but recorded for the same commit's provenance).
- Stale `dowiz.fly.dev` references in live-behavior-driving config updated to reflect
  Hetzner+Cloudflare-only: `.mcp.json` (`VITE_BASE_URL`), `openspec/config.yaml` (tech-stack +
  Mandatory Proof Rule target), `.claude/CLAUDE.md` (both SUSPENDED-section Fly mentions).
  `fly.toml` and the old TS backend (`apps-api`/`apps-worker`/`packages-db`) were already
  quarantined to `attic/` in an earlier commit (`fce5738b0`) — this pass only had stale
  *references* left to clean, not a live deploy pipeline.
- **Actual teardown is blocked on operator action**, not a design question: this sandbox holds
  no prod Fly credential (only a `dowiz-staging`-scoped token was ever intentionally saved, per
  `staging-fly-access` memory — prod tokens were deliberately never persisted). The operator
  must run `flyctl auth login` interactively (`! ~/.fly/bin/flyctl auth login`) before teardown
  can proceed.
- A **pre-existing runbook already covers this exact teardown**:
  `docs/red-team/2026-07-13/PART1-LIVE-PROD-DECOMMISSION.md` — written 5 days before this
  dialogue pass, already scoped as "NOT EXECUTABLE FROM THIS HOST" for the same credential
  reason. Its Step A (rotate the seeded `test@dowiz.com` owner credential in the live prod
  Supabase DB, confirmed live/owner-privileged by the 2026-07-13 red-team synthesis) is a
  prerequisite BEFORE Step B's `fly scale count 0` / `fly apps suspend` teardown, so the
  teardown window itself can't be abused. Step A is a live-prod auth/money-adjacent DB write —
  **not executed without separate explicit operator confirmation**, same red-line standard as
  every other prod-DB action this session.
**Depends on / blocks:** blocks nothing else in this roadmap — the new stack's build (Tier 3 web
UI, tracked via the audit triage's `#10`/`#11`) proceeds independently of when the Fly teardown
itself executes.

#### 16.12 Vendor onboarding — self-serve, automatic
Operator's ruling (Recommended option, confirmed): a new vendor registers through `dowiz.org`'s
directory and the hub is provisioned automatically on submission (Shopify-style), not a manual
curated approval queue. Chosen explicitly for scalability — the operator does not want to be a
bottleneck on every new venue. **Consequence:** the hub-provisioning path (whichever hosting
mode §16.1 offers) must itself be a fully automated, unattended flow — this is now a hard
requirement on whatever builds the self-serve signup, not an optional nicety.

#### 16.13 Payment — online-mandatory from Wave-0, multi-provider adapter layer
Operator's ruling: online payment is **mandatory from the start** (not deferred, not
cash-on-delivery-only) — reverses what would otherwise have been the simpler MVP default.
Provider choice: **multi-provider via an adapter layer from day one**, not a single
Stripe-only integration — mirrors the §16.5 channel-adapter pattern and P51's own
no-vendor-lock-in stance on mapping providers. **Consequence:** this promotes payment-gateway
integration to a Wave-0 blocking dependency (was previously deferred in the audit triage's
Tier 3), and the payment layer needs a port/adapter boundary analogous to the order-channel
one — not yet blueprinted, named here as a gap for the next blueprint pass.

#### 16.14 Offline-hub behavior — no central dowiz state, honest client-side status, venue-side fallback preferred
This resolved a real self-contradiction the dialogue surfaced: §16.6 committed to "isolated
hubs, no dowiz-owned central data store," but the operator's first answer on offline-hub
behavior ("fallback/queue at dowiz.org") would have required exactly that central store.
Operator's own correction, verbatim: *"без центрального тоді узагалі, показувати чесно або ж
добавити фолбеки на стороні самого закладу (це імпонує)."* Resolved cleanly in favor of the
stronger invariant: **dowiz.org/the client holds zero server-side order state, ever.** When a
hub is unreachable, the client shows an honest "hub offline" status — no disguised retry, no
central queue. Any resilience beyond that (e.g., capturing an attempted order locally and
retrying once the hub is reachable again) lives on the venue's own hub side or the customer's
own device, never on a dowiz-operated server. This is now the strongest, most explicit
statement of the "no central data store" invariant in this roadmap — future sections must not
reintroduce a central queue/buffer without an explicit, named reopening of this decision.

#### 16.15 Hub ↔ vendor cardinality — one hub can serve multiple vendors (food-court model)
Operator's ruling: a single hub is not strictly one-vendor — it can host **multiple vendors
sharing one delivery/courier pool** (food court, or several small locations under one roof).
This settles the earlier "small vs large vendor" framing from the operator's original vision
statement: cardinality is a hub-configuration choice, not a vendor-size tier. A chain with
multiple physical locations still maps to multiple hubs (one per location, per §16.1's
per-venue framing); the food-court case is the genuine one-hub/multi-vendor scenario.
**Consequence:** the in-hub data model needs a vendor-scoping layer (per-vendor menu/catalog,
shared courier/delivery pool) — not yet designed, named as a gap.

#### 16.16 Monetization — fixed per-hub subscription, no transaction percentage; self-host economics differ
Operator's ruling: dowiz charges a **fixed subscription per hub**, not a percentage of order
value — vendors keep 100% of their payment volume, simplifying the §16.13 payment-adapter
design (no split/settlement logic needed inside the payment path itself). **Self-host has
different economics**, confirmed as a follow-up: a one-time license fee or fully free/
open-source, not a recurring subscription — Hetzner-hosted hubs pay recurring for hosting +
protocol/updates/support; self-hosted hubs pay (if anything) once, for the software itself.
Exact self-host pricing (one-time-paid vs. fully free) is left open — named as a business,
not architecture, decision.

#### 16.17 Menu/catalog schema — fully vendor-defined, no fixed dowiz schema
Operator's ruling (Recommended option, confirmed): vendors define their own categories,
modifiers, and variants freely — dowiz does not impose a fixed schema (no hardcoded
"appetizers/mains/desserts" structure). This is what makes the platform viable for non-typical
food businesses, and by extension any small business beyond food (a "flowers" or "goods"
vendor fits without a schema exception). **Consequence:** the catalog data model needs to be
schema-flexible (vendor-authored category/modifier trees), which the old TS stack's
`AllergenEditor`/`Recipe BOM editor` (referenced in stale Repowise index entries) may partially
inform but does not dictate — those were built against the now-retired centralized stack.

#### 16.18 Multi-hub owner view — client-side aggregation, never server-side
Follows directly from §16.6's hub isolation and §16.14's "no central dowiz state" invariant: an
owner running multiple hubs (a chain, per §16.15) sees them together via their own device/app
connecting to each hub independently and merging the view locally — never via a dowiz-operated
aggregation server. Confirmed as the Recommended option specifically because it extends the
same invariant §16.14 just hardened, rather than opening a new exception for owners. **Consequence:**
the owner-facing Tauri client (P39) needs a genuine multi-hub connection mode (hold N
capability-certs, one per hub, fan out reads/writes, merge client-side) — not yet designed,
named as a gap against P39/P48.

#### 16.20 Target market — multi-language/multi-market from day one
Operator's ruling: no single-market MVP restriction (the stale openspec Albania/`sq` framing
is explicitly superseded) — i18n architecture from Wave-0, though the *first real order* can
still land in whichever single market is fastest; the requirement is architectural (no
hardcoded locale/currency), not a demand for N markets simultaneously live at launch.

#### 16.21 `dowiz.org` public role — pure infrastructure, no public vendor catalog
Operator's ruling (Recommended option, confirmed), reversing this section's own earlier
§16.6 framing: *NOT even a directory of links.* `dowiz.org` publicly lists no vendors at all.
Each venue gets its `/s/:slug` link and markets it entirely through its own channels (social
media, QR codes, the §16.7 auto-posting automation) — dowiz never inserts itself between a
venue and its own customers as a discovery layer. `dowiz.org`'s public surface is a
product/demo page for **prospective venue owners** evaluating the platform (the original
"переглянути та спробувати" framing was about trying the *product*, not browsing food) plus
the self-serve signup flow (§16.12) and installable-client hosting (§16.8). **Supersedes**
§16.6's "dowiz.org as directory" line — recorded as a correction, not silently overwritten:
§16.6's isolated-hub topology stands, only the "directory" characterization of `dowiz.org`
itself is retracted.

#### 16.22 Push notifications — hub-owned, no central token store
Operator's ruling (Recommended option, confirmed): each hub pushes to APNs/FCM directly and
stores its own push tokens locally — no dowiz-central token store, extending §16.14's
zero-central-server-state invariant to notifications specifically, closing what would
otherwise have been a real architectural exception to that rule.

#### 16.23 Customer identity — client-side data wallet, no dowiz account, no per-venue re-entry
Operator's ruling: neither a separate account per venue (friction) nor a central dowiz account
(would violate §16.6/§16.14 isolation) — a **client-side data wallet**. The customer's own
device/browser/app stores their details (name, address, payment method) locally and offers to
autofill them at any new hub's checkout, with no server-side account anywhere, dowiz-operated
or per-venue. This is the same client-side-aggregation principle §16.18 already established for
the owner's multi-hub view, applied symmetrically to the customer side — a second confirmation
that "no central state" resolves these UX-friction questions via the client, not via a new
server-side exception each time one comes up.

#### 16.24 Courier payout — fully the venue's responsibility, dowiz touches no courier money
Operator's ruling (Recommended option, confirmed): dowiz does not facilitate courier payout in
any form — consistent with §16.3 (venue brings its own couriers) and §16.16 (dowiz takes no
transaction percentage, vendor keeps 100%). dowiz remains a dispatch **protocol**, never a
money intermediary for courier compensation. The payment adapter (§16.13) needs no split-payout
logic — this closes what could otherwise have become scope creep into the payment design.

#### 16.26 Courier matching, reviews, and cold-start discovery
Three related closes in one round:
- **Courier matching stays HRW-automatic**, scoped to the venue's own courier pool (§16.3) —
  the existing rendezvous-hash mechanism and its no-scoring red line are unchanged by the
  "venue brings its own couriers" model; only the candidate set shrank from "all couriers" to
  "this venue's couriers," the algorithm itself didn't change.
- **Reviews/ratings are per-hub in MVP** — visible only within the brand that earned them, no
  cross-hub or dowiz-wide reputation system, consistent with the whole brand-isolation stance
  (§16.4, §16.6, §16.23).
- **Cold-start discovery is NOT purely the vendor's problem**, refining §16.21: dowiz actively
  helps each venue's `/s/:slug` page get found — SEO, AEO/GEO (answer/generative-engine
  optimization for AI search surfaces — ChatGPT/Perplexity/Google AI Overviews), plus the
  already-scoped auto-posting (§16.7), personalized mailings, and chat/channel/bot presence.
  This does **not** reopen §16.21 (dowiz.org itself still hosts no public catalog) — the help is
  per-venue technical/content tooling, not a dowiz-run discovery surface. Extends §16.4's
  agent-as-assistant role explicitly into marketing/growth, not just operations.

#### 16.27 Self-host durability — built-in encrypted auto-backup and auto-update with rollback
Two related operator rulings closing the self-host reliability gap:
- **Backup**: self-hosted hubs get a built-in encrypted auto-backup to `hetzner:dowiz` (or the
  vendor's own S3-compatible target) — dowiz never sees plaintext data, but a venue whose
  hardware fails can still recover. Extends the disk-cleanup pass's already-confirmed
  `hetzner:dowiz` remote (`BLUEPRINT-DISK-OPS-CLEANUP-2026-07-18.md`) to a new purpose (per-hub
  encrypted backup target, not just dev-tooling result storage).
- **Updates**: auto-update by default (keeps the mesh from fragmenting into stale protocol
  versions), with an explicit owner-triggered rollback to a prior version — balances
  ecosystem-wide protocol consistency against an individual owner's need for control after a
  bad update.

#### 16.29 Media storage and dispute handling
- **Media storage**: vendor-uploaded menu photos/video default to Cloudflare R2/Images (already
  in the stack per §16.1 — no new vendor lock-in), with an easy vendor-side option to connect
  their own storage instead — the same "managed default + easy opt-out" shape as hosting
  (§16.1) and payment providers (§16.13).
- **Disputes/refunds**: fully the vendor's and payment provider's responsibility (Recommended,
  confirmed) — extends §16.16/§16.24's "dowiz touches no money beyond the adapter" stance;
  refund execution runs through the payment adapter's own refund API, dowiz is not a
  dispute-resolution party.

#### 16.30 UI rendering approach — full wgpu, not DOM-for-forms + field-for-ambience
Operator's ruling: the physics/field engine (wgpu, Sea&Sheet, SDF text) renders the **entire**
UI — menu, checkout, admin dashboard — not just ambient/hero moments with conventional DOM for
forms and lists. **This is the single most scope-expanding decision in this dialogue so far,
flagged honestly rather than absorbed silently**: a bare wgpu canvas has zero native
accessibility (no screen-reader tree, no native keyboard focus/tab order, no browser
find-in-page). P51 already established the needed pattern for one surface — an "a11y-mirror
path" (a parallel accessible DOM tree kept in sync with the rendered canvas) for the map — this
decision means that pattern must now cover **every** screen, not just the map, which is
substantially more engineering than a conventional-DOM Tier-3 rebuild would have needed.
Recorded as a hard requirement, not softened — but the a11y-mirror-everywhere design itself is
NOT yet done and belongs in the Tier-3 blueprint, not this section.

#### 16.31 Voice/gesture control — Wave-0 requirement, not deferred
Operator's ruling: basic voice navigation/ordering ships in Wave-0 — this was scoped as an
audit-evaluation criterion in the original 5-persona audit prompt, and the operator now
confirms it is a real build requirement, not a "nice to have re-evaluated later." Combines with
§16.30 to substantially raise Tier-3's real scope beyond what the audit triage's `#10`/`#11`
line items assumed when they were written (those predate this dialogue pass).

#### 16.32 Vendor onboarding mechanism — claim pre-generated demo hubs, not live self-serve provisioning
Refines §16.12: rather than a hub being provisioned live and automatically at signup time (which
would need a fully unattended infra-provisioning pipeline as a Wave-0 dependency), the operator's
actual plan is a **claim mechanic** — pre-generated, ready-to-use demo hub instances exist ahead
of time; a prospective vendor claims one and starts using it immediately (ownership assignment,
not live provisioning, which is a much smaller/safer piece of engineering). A parallel,
non-mandatory path exists for cases outside the pre-generated pool: a signup/interest form on
`dowiz.org` notifies the operator directly, for manual follow-up. **`dowiz.org`'s own homepage
can be minimal** — a landing page with the signup form and links to the GitHub repository.
**New fact surfaced**: dowiz's codebase (or some part of it) is intended to be publicly visible
on GitHub — consistent with, and now more concretely confirmed than, §16.16's "self-host = one-time
license OR free/open-source" branch. Does not reopen §16.21 (still no public vendor catalog) —
the GitHub link is to the *product's source*, not a vendor directory.

#### 16.33 What this section deliberately does not resolve
Per the operator's own instruction (an extended, ~150-question-total progressive dialogue now
requested — "ще 100 питань" added on top of the original ~50, spanning interface/design as well
as remaining architecture — tracked outside this file, ~47 answered as of this checkpoint),
many real sub-questions remain open: the Cloudflare Tunnel multi-tenant credential-isolation
design (§16.2), the payment-adapter port/adapter design (§16.13), the in-hub multi-vendor data
model (§16.15), the owner multi-hub client mode (§16.18), the client-side data-wallet's concrete
implementation (§16.23), the SEO/AEO/GEO tooling's concrete design (§16.26), the a11y-mirror
architecture for full-wgpu UI (§16.30), the voice/gesture command surface (§16.31), the
claim-mechanic's concrete implementation (§16.32), and the full remainder of the question set
(interface/design details not yet asked). This section grows via the same append-only
convention as further rounds settle each one — it is not a final architecture document.

#### 16.34 wgpu text input, voice recognition locality, and courier-app rendering parity
Three related closes:
- **Text input inside canvas is fully custom** (Recommended-against option not taken) — no HTML
  `<input>` overlay hybrid. This is, honestly, one of the hardest sub-problems in UI
  engineering: a from-scratch text editor (cursor, selection, clipboard, IME composition for
  non-Latin scripts given §16.20's multi-language requirement) plus its own accessibility
  exposure (an AccessKit-style bridge, not a browser's native text-field a11y for free).
  Recorded as accepted scope, not softened — the §16.30 a11y-mirror pattern now has to cover
  live text editing state, not just static content.
- **Voice recognition runs locally/offline** (e.g. whisper.cpp or equivalent, wasm or native
  Tauri-side) — consistent with §16.4's no-AI-first/local-first stance; this is client-side
  transcription, a different category from the AI-decision-authority boundary §16.4 draws.
- **Courier app is also full wgpu** — chosen for rendering-architecture consistency across the
  whole product over the battery/simplicity tradeoff a lighter native courier UI would have
  offered. Named as a real tradeoff accepted deliberately, not overlooked: a courier on a bike
  running a GPU-rendered UI for a full shift is a genuine battery-life question the Tier-3/
  P52 build needs to benchmark, not assume away.

#### 16.35 UI paradigm — intent-driven generative rendering, not a button/menu interface (RESEARCH DIRECTION)
Operator's framing, verbatim intent: *"моє бачення передбачає мінімальну і майже відсутню
кількість кнопок чи елементів, замість цього наміри на які фізика + AI рендерить і показує
заготовлені речі через функції, або узагалі малює з нуля."* Explicitly requested to be
recorded as research, not a simple ship/defer checkbox — and treated that way here. The
proposal: rather than a conventional button/menu-driven admin UI, the interface is
**intent-driven** — the user expresses an intent (plausibly via the §16.31 voice channel, or a
minimal gesture/input), and the field engine + local AI (§16.4's assistant, not a decision-maker)
renders the appropriate response — either composing pre-built UI functions/fragments, or
generating the visual entirely from scratch procedurally. **Stated honestly, not softened**:
this is a substantially more ambitious paradigm than any known production food-delivery-scale
admin surface, layered on top of §16.30's already-large full-wgpu-UI and §16.34's
already-large custom-text-input commitments. It is recorded here as a genuine design direction
the operator wants researched and prototyped, not as settled Wave-0 buildable scope — the
Tier-3 blueprint needs to name a concrete fallback (a conventional, function-driven admin
surface) if the generative-rendering research doesn't converge in time, so this ambition
doesn't silently block the first real order.

#### 16.36 Admin dashboard scope timing — orders/menu/couriers Wave-0, marketing Wave-0 (basic), analytics deferred
- **Analytics/reports**: may wait for v2, after the first real order — the owner reads raw order
  data directly in admin (per §16.35's framing, likely via the generative/intent interface
  rather than a dedicated chart-heavy screen) until a purpose-built dashboard exists.
- **Marketing/auto-posting panel** (§16.7, §16.26 SEO/AEO): **Wave-0**, at least a basic
  auto-posting capability from day one — raises Tier-3 scope further; needs the social/channel
  bot integrations live before the first real order, not deferred to v2 as analytics is.

#### 16.37 Checkout flow — multi-step wizard, framed as a small narrative journey
Operator's ruling: multi-step (menu → cart → delivery → payment, separate screens), described
as already present in existing design docs/artifacts as *"невелика пригода"* (a small
adventure). **Confirmed existing reference, not a new concept**: `BLUEPRINT-P38-webgpu-render-
engine.md` already names a "narrative-cinematic reading of the Sea: the order-lifecycle pacing
arc with named beats" (P38 §, live-verified this pass) — the checkout wizard is this arc applied
concretely to the order flow, not a separate new design. Fits §16.23's client-side data wallet
naturally: each step can autofill from the wallet without a separate account per venue.

#### 16.38 Localization mechanism — local open-source translation model, not hand-maintained i18n tables
Operator's correction to §16.20's original framing: multi-language does not require
hand-maintained per-locale string tables (the old stack's SQ/EN/UA switcher pattern). Instead,
translation runs through a **local open-source model** (HuggingFace-class, run via the same
Ollama/local-LLM infrastructure as §16.4's assistant and §16.34's voice recognition) —
consistent with the whole local-first stance rather than a new exception. Removes what would
otherwise have been an ongoing manual-translation maintenance burden as the vendor/venue count
grows across markets.

#### 16.39 Typography — physics/math-generated glyphs, not font files (RESEARCH DIRECTION, extends §16.35)
Operator's framing, verbatim: *"ще дикіша ідея - малювання шрифтів через математику і
фізику."* Goes beyond §16.30's already-planned SDF-rendered text (glyphs from real font files,
rendered via signed-distance fields — an established technique) to something categorically
further: glyphs generated procedurally from the same field-engine math (the Laplacian/wave
primitives already driving the rest of the UI) rather than sourced from any font file at all.
**Stated honestly**: this is typography R&D with no known production precedent — recorded as a
research direction under the same §16.35 umbrella, not settled Wave-0 buildable scope on its
own.

#### 16.40 Intent-driven UI — full replacement from day one, no traditional-navigation fallback; schedule risk knowingly accepted
Sharpens §16.35: the operator confirmed **full replacement of traditional navigation from day
one**, not a layer over a conventional fallback (reversing this section's own earlier
recommendation, which had proposed keeping conventional screens as the safety net). Combined
with §16.30 (full wgpu), §16.34 (custom canvas text input), and §16.39 (physics-generated
type), this is now one of the most technically ambitious UI stacks recorded in this roadmap —
flagged directly to the operator as a real risk to the audit triage's own top priority (first
real order, Tier 3). **Operator's explicit, informed response, verbatim: "Один шлях: повний
набір одразу, графік зсувається як є"** (one path: the full set at once, the timeline shifts
accordingly) — a deliberate, informed acceptance of schedule risk in exchange for building the
complete vision as one coherent system rather than a phased/hedged rollout. Recorded as a
closed decision, not to be re-litigated on schedule-risk grounds alone; the Tier-3 blueprint
should still name concrete milestones so slippage is visible early, not discovered late.

#### 16.41 What this section deliberately does not resolve
Per the operator's own instruction (an extended progressive dialogue — originally ~50, extended
to ~150 total, now also covering interface/design in depth — tracked outside this file, ~53
answered as of this checkpoint), substantial ground remains open: the Cloudflare Tunnel
multi-tenant credential-isolation design (§16.2), the payment-adapter port/adapter design
(§16.13), the in-hub multi-vendor data model (§16.15), the owner multi-hub client mode
(§16.18), the client-side data-wallet's concrete implementation (§16.23), the SEO/AEO/GEO
tooling's concrete design (§16.26), the a11y-mirror architecture for full-wgpu UI (§16.30),
the intent-driven generative-UI research program itself (§16.35, §16.39-§16.40 — this is a
program of work, not a single decision), and the full remainder of the extended question set.
This section grows via the same append-only convention as further rounds settle each one — it
is not a final architecture document.

#### 16.42 Named design philosophy — *ad fontes*: math/physics primitives over library dependencies
Operator's own framing for §16.30/§16.35/§16.39-§16.40's UI direction, worth naming precisely
rather than leaving implicit: *"це велике спрощення у відмові від надбудов і залежностей -
математика і фізика, значно більш контрольована і класичніша за будь-які бібліотеки, окрім
цього простір безмежний"* — a deliberate rejection of UI-library superstructure in favor of
mathematical/physical primitives, framed as *simplification* (fewer external dependencies,
more directly controlled behavior) rather than added complexity, with an explicitly unbounded
design space as the payoff. Named **ad fontes** ("to the sources") per the operator's own
Renaissance-humanist reference — returning to first-principles primitives rather than
inherited/derivative UI-library abstractions. This is the coherent philosophical throughline
behind the whole §16.30/§16.34/§16.35/§16.39/§16.40 cluster of decisions, recorded here as a
named principle so future sections can cite it directly instead of re-deriving the rationale.

#### 16.43 Ad fontes scope — UI/rendering/interaction layer only
Operator's ruling (Recommended option, confirmed): §16.42's *ad fontes* principle applies to the
UI/rendering/interaction layer only — crypto, protocol, storage, and networking keep using
established, vetted crates (ML-DSA-65 and the rest of the existing kernel's dependency set are
correct as-is; a from-scratch crypto primitive would be a genuine safety regression, not
simplification). Prevents §16.42 from being over-read as a blanket minimal-dependency mandate
across the whole Cargo workspace — it is not; the existing kernel architecture is unaffected.

#### 16.44 Friction for consequential actions — encoded as field state, not a discrete confirm dialog (RESEARCH DIRECTION)
Operator's framing, verbatim: *"наміри та відображення інтерфейсу, його динамічна зміна у
амплітуді хвиль, інтенсивності середовища, кольорів й самого ритму - інтерфейс слугує
продовженням і відображенням органічно стану бекенду і ядра."* Deliberate friction for
money-moving/destructive actions (confirm order/payment, cancel) is not a discrete modal
dialog in this design — it is encoded directly in the field's own dynamics: wave amplitude,
environmental intensity, color, and rhythm shift in response to an action's real stakes,
because the interface is designed to be an organic, continuous reflection of the backend/
kernel's actual state rather than a separate presentation layer bolted on top of it. **Recorded
honestly as a real design principle, not yet a buildable spec**: this still needs a concrete
answer to how a specific field-state variable maps numerically to a specific stake (money
amount, irreversibility), what completes vs. cancels a gesture in that language, and how a
first-time user acquires the intuition for it before any learned association exists — those
are Tier-3-blueprint-level questions, not resolved by this dialogue pass. Directly answers the
original 5-persona audit's Herzog-lens "friction-as-a-feature for destructive actions"
checklist item (`AUDIT-PROMPT-TEMPLATE-2026-07-18.md`) with a concrete (if not yet fully
specified) mechanism, rather than leaving it unaddressed.

#### 16.45 Cloudflare Tunnel multi-tenancy — one dowiz-operated CF account for all hubs
Closes §16.2's named gap (Recommended option, confirmed): every hub — Hetzner-hosted or
self-hosted — tunnels through a single dowiz-operated Cloudflare account, not a
per-venue CF account the vendor registers themselves. Zero CF setup burden for the vendor
(matches §16.32's low-friction claim mechanic), at the cost of dowiz owning tenant-isolation
between tunnels/routes/credentials on its own CF account — named as real engineering work for
whichever blueprint builds hub provisioning, not yet designed here. Scoped explicitly to
Wave-0; the operator flagged this as revisitable if hub count grows enough to strain one
account's practical limits.

#### 16.46 Food-court checkout — unified cart across vendors, one delivery, split payment required
Closes part of §16.15's named gap (Recommended option, confirmed): a customer ordering from
multiple vendors inside one food-court hub gets a single unified cart and one delivery — not
separate per-vendor checkouts/deliveries. **Consequence for §16.13's payment adapter**: it now
needs split-payment/settlement logic to divide one payment across multiple vendors within the
same hub — this reopens part of §16.24's "no split-payout logic needed" framing, but only for
the intra-hub food-court case, not for courier payout (§16.24 stands unchanged for couriers).
Named as a concrete new requirement on the payment-adapter blueprint, not yet designed.

#### 16.47 Data wallet portability — device-resident with Signal-style QR device-linking, self-custody
Closes §16.23's remaining gap: the wallet lives on-device by default (no cross-device sync
service, consistent with §16.14/§16.23's no-central-server stance), with a **Signal-style QR
device-linking transfer** for moving it to a new device — one-time codes, generated once, loss
is explicitly the user's own responsibility. This is the same self-custody framing already
used for crypto/authenticator-style secrets elsewhere in this ecosystem (capability-certs,
HybridSigner keys) — applied consistently to customer data rather than inventing a new trust
model for this one case.

#### 16.48 Owner multi-hub credentials — a root/delegating capability-cert, not N flat per-hub certs
Refines §16.18/§16.32: neither "auto-issued via a dowiz.org account" (conflicts with §16.14's
no-dowiz-account stance) nor "manual QR-import per hub" (too much friction per hub) — the
operator wants a **root credential the owner holds themselves**, capable of self-service
adding, modifying, and revoking hub nodes under it. This is a hierarchical/delegating
capability-certificate pattern (a root cert that can mint or authorize child hub-certs) rather
than a flat set of N independent per-hub certs — fits naturally with the existing capability-
cert architecture (ML-DSA-65 hybrid signing) already used for couriers and hub identity
elsewhere in this roadmap, extended one level to support owner-side delegation. **Not yet
designed**: the concrete cert-hierarchy/revocation mechanics belong in the P39/P48 blueprint
work, not this dialogue-decisions section — recorded here as the shape of the answer, not the
full spec.

#### 16.49 Payment call site, courier-unavailable handling, tax responsibility
Three closes:
- **Payment calls happen client-side, hub never sees card data** (Recommended, confirmed) — a
  PCI-standard SDK pattern (card data flows directly from client to provider); the hub receives
  only a token/confirmation. Reduces the hub's PCI compliance burden substantially versus a
  server-side-secret-key design, at the cost of §16.46's split-payment logic needing to live in
  the provider's own split/Connect-style API rather than inside hub code.
- **No courier available at order time**: the order is accepted and waits/queues for a courier
  (Recommended, confirmed) — extends §16.14's honest-status principle to this case too: the
  order is real and pending, not silently rejected, but also not fabricated as "assigned" before
  a courier actually exists.
- **Tax/VAT**: fully the vendor's responsibility (Recommended, confirmed) — consistent with
  §16.29's dispute/refund stance; the vendor sets their own rate inside the free-form menu
  schema (§16.17), dowiz calculates and tracks nothing tax-related.

#### 16.50 Friction accessibility, voice as one of several equal intent channels, implicit onboarding
Three closes, all within the §16.35/§16.40 generative-UI cluster:
- **Friction is multi-modal, with full audio support** — not a color/visual-only signal (which
  would fail colorblind users). Operator adds a broader claim worth recording precisely but
  **not yet treated as proven**: that physics/math-driven rendering inherently solves much of
  cross-device/cross-platform portability without per-platform adapters, because the same
  equations render identically wherever wgpu runs. Consistent with this repo's own
  verified-by-math culture ([[verified-by-math-2026-07-07]]), this is recorded as a claimed
  benefit to validate during the Tier-3 build (real devices, real platforms), not asserted as
  already true.
- **Voice is one of several equal-standing intent channels** (voice + touch/gesture on canvas),
  not a primary channel that others merely supplement — the user picks whichever channel fits
  the moment (quiet vs. hands-busy environments, e.g. a courier mid-ride).
- **Onboarding is implicit/adaptive, embedded in the field itself** — no separate modal or
  text how-to screen; the interface teaches its own use through the same state-communication
  mechanism §16.44 already established for friction, rather than a bolted-on tutorial layer.

#### 16.51 Order cancellation authority, device baseline, content moderation
Three closes:
- **Cancellation**: the customer may cancel freely before the vendor confirms the order; once
  the vendor has confirmed (started preparing), only the vendor can cancel further — cancellation
  after confirmation is a money action (triggers a refund) and routes through §16.29's
  vendor+payment-provider dispute channel rather than a unilateral client action.
- **Device baseline — no UI-paradigm fallback, but a real optimization target**: §16.40's "one
  path" decision stands (no separate legacy UI mode), but the operator clarifies the same
  physics/math rendering must run efficiently on older/budget devices through lean Rust/wasm/
  kernel implementation — *"без тяжких бібліотек"* — rather than by degrading to a different,
  simpler interface. Consistent with, not an exception to, §16.42's *ad fontes* stance: the
  fallback is engineering discipline (an efficient kernel), not a second UI to design/maintain.
- **Content moderation**: full vendor trust, no pre-publication review, for Wave-0 (Recommended,
  confirmed) — consistent with §16.12/§16.17's self-serve/free-schema automation stance. A
  post-hoc report/blocklist mechanism for abuse is implied as necessary but not yet designed —
  named as a gap for the Tier-3/moderation blueprint, not resolved here.

#### 16.52 Agent model sourcing, SMS/email fallback, offline checkout resilience
Three closes:
- **Agent model**: BYO-model is an option, dowiz-fixed local model is the default (Recommended,
  confirmed) — the same pattern already established for hosting (§16.1), payment (§16.13), and
  storage (§16.29): a managed default with an easy opt-out, applied consistently to the AI layer
  too rather than inventing a fourth different shape for this one adapter.
- **SMS/email is a mandatory Wave-0 fallback**, not push+in-app-only (reverses the Recommended
  option) — a customer without the installed app, with push disabled, or on the web client
  still needs to receive order-status updates. **Consequence**: an SMS/email adapter is now a
  Wave-0 dependency on the hub's notification path, alongside §16.22's hub-owned push.
- **Offline checkout resilience**: if network drops mid-checkout, the in-progress cart/data-
  wallet-filled fields are held as a local draft and restored automatically on reconnect — no
  lost progress, payment simply doesn't fire until the client is back online. Extends §16.14's
  honest-status and client-side-state principles to the checkout flow specifically.

#### 16.53 Courier-in-motion voice priority, spam rate-limiting, hub heartbeat monitoring
Three closes:
- **Voice is the practical primary input while a courier is actively delivering (in motion)** —
  a safety-driven exception to §16.50's general "equal channels" stance, not a paradigm change:
  hands-busy/eyes-on-road contexts specifically favor voice, other contexts keep all channels
  equal.
- **Spam/fake orders**: the mandatory online-payment gate (§16.13) is the primary defense (no
  card, no order), but the operator adds an explicit **rate-limit at the hub level** on top of
  it — covering attempted/abandoned-checkout spam that never reaches a successful payment (a
  DoS/nuisance vector distinct from fraudulent-but-paid orders, which the payment provider's own
  fraud filtering already covers per §16.13).
- **Hub heartbeat monitoring**: dowiz receives a heartbeat/liveness signal from every hub via
  the CF Tunnel layer (§16.2/§16.45) and can alert if one silently drops — a deliberate,
  narrow exception to §16.14's data isolation (liveness only, never hub data), justified as
  operational pragmatism over architectural purity. Enables an operator alert for the exact
  §16.14 offline-during-order scenario's *silent, unexplained* variant, distinct from the
  already-designed honest-status UX for a hub the client can see is unreachable.

#### 16.54 Open-source scope, demo-hub fixtures, full offline Tauri client
Three closes, all Recommended options confirmed:
- **GitHub open-source scope**: the hub software (kernel/protocol/UI-rendering — whatever a
  self-host vendor actually installs) is open source; `dowiz.org`'s own infrastructure (the
  claim mechanic, CF-tenant-isolation, the directory-of-nothing landing site itself) stays
  closed — that is dowiz's own operating infrastructure, not the product a vendor runs.
  Sharpens §16.32's "some part of the codebase" into a precise boundary.
- **Demo hubs are pre-populated with fixtures** (test menu, test couriers) rather than empty —
  a claimed hub demonstrates real value immediately, the vendor replaces fixtures with their
  own data at their own pace rather than starting from a blank screen.
- **The installed Tauri client is cache-first with full offline functionality** — menu,
  statuses, and draft orders are cached locally and usable with zero network, syncing on
  reconnect. Extends §16.52's offline-checkout-draft principle to the whole installed client,
  not just the checkout step.

#### 16.55 SEO/AEO crawlability without any DOM — separate bot-facing files, not the a11y-mirror
Resolves the §16.30-vs-§16.26 tension flagged this pass, with an answer sharper than either
option offered: **no DOM at all, for anyone** — not even a hidden/off-screen a11y-mirror serving
double duty as SEO content. Instead, crawlers and AI/answer-engine bots get **purpose-built
static machine-readable files** (`robots.txt`, `manifest.json`, and by the same logic
`sitemap.xml`, schema.org JSON-LD data, and an `llms.txt`-style feed for AI crawlers
specifically) served alongside the canvas, with zero rendered markup. **This is a genuinely
different mechanism from §16.30's a11y-mirror**, worth keeping distinct rather than conflating:
the a11y-mirror serves a human using assistive technology who needs an interactive tree; these
bot-facing files serve a crawler that only needs facts, not interactivity. Two audiences, two
purpose-built solutions, neither one a DOM. Concrete file formats/schema are a Tier-3 design
task, not resolved here.

#### 16.56 `dowiz.org` landing page — also full wgpu, no static-page exception
Operator's ruling: the landing page (§16.32 — minimal, Cloudflare-Pages-hosted) is **also**
full wgpu, for consistency — no exception for dowiz's own marketing surface. Reinforces §16.40's
"one path" stance: the *ad fontes* commitment applies uniformly across the whole product,
including the surface most tempted to cut as "just a landing page." §16.55's bot-facing static
files remain the SEO answer for this page too, unchanged by this ruling.

#### 16.57 Abandoned demo-hub claims stay with the vendor permanently; hub-software license confirmed AGPLv3+TM+DCO
Two closes:
- **No reclaim policy** — a claimed demo hub that a vendor never finishes configuring (or never
  reopens) stays theirs indefinitely, not returned to the claimable pool after inactivity.
  Simpler operationally, at the cost of the pre-generated demo pool depleting over time with no
  automatic recycling — the claim-supply pipeline (§16.32) needs to account for net consumption,
  not assume any pool refresh from abandonment.
- **License confirmed**: the hub software uses the same AGPLv3 + trademark + DCO plan already
  adopted for kernel/protocol open-sourcing ([[open-source-goal-adr020-2026-07-03]]) — no
  separate licensing decision needed. AGPLv3's network-copyleft closes the obvious gap a
  competing hosted-SaaS fork of the hub software would otherwise exploit; the trademark clause
  protects the "dowiz" name specifically, separate from the code license.

#### 16.58 RTL deferred to v2, vendor-responsible GDPR deletion with dowiz tooling, live brand preview
Three closes, all Recommended options confirmed:
- **RTL languages (Arabic, Hebrew) deferred to v2** — Wave-0 targets LTR languages; the
  architecture doesn't exclude RTL, it's simply not built for Tier-3's first cut. Named
  explicitly so the Tier-3 blueprint doesn't have to guess whether this was an oversight or a
  deliberate scope cut — it's the latter.
- **GDPR right-to-be-forgotten**: the vendor is legally responsible for deletion requests
  against their own hub's order history (consistent with §16.29's full-vendor-responsibility
  stance on disputes/data), but dowiz provides a built-in "delete everything about customer X"
  tool in the hub software itself, so the vendor isn't left building this from scratch. The
  client-side data wallet (§16.47) is already trivially self-deletable by the customer directly.
- **Brand customization gets a live draft/staging preview** before publishing — a vendor sees
  their 5-token changes (§16.9) rendered in the real field engine before customers do, avoiding
  a published-then-regretted color/font combination. Adds a draft-vs-live state to the brand
  config, not yet designed in detail here.

#### 16.59 Client-app license, vendor offboarding grace period, no vendor quality bar
Three closes, all Recommended options confirmed:
- **Client/courier Tauri apps use the same AGPLv3+TM+DCO** as the hub software (§16.57) — one
  unified license across everything a claim-vendor touches, not a separate closed license for
  end-user apps.
- **Vendor cancellation (Hetzner-hosted hub)**: a grace period follows cancellation, during
  which the hub stays read-only-accessible so the vendor can export or migrate to self-host —
  no immediate hard shutdown. Coheres with §16.54 (hub software is open source, so self-host
  migration is genuinely available, not a hollow option).
- **No vendor quality bar at all** — extends §16.26's courier no-scoring red line to vendors:
  dowiz does not gate, exclude, or rank vendors by quality/performance/rating. Per-hub reviews
  (§16.26) are a signal visible to that hub's own customers, never a dowiz-side gate.

#### 16.60 Pickup/dine-in is part of order-flow from the start; agent excluded from customer PII by default
Two closes:
- **Pickup/dine-in** is part of the unified order-flow (§16.5) from Wave-0, not a
  delivery-only platform — an order without courier-matching (the customer collects it
  themselves) is the same order-flow minus the dispatch step, not a separate product mode.
  Broadens "малих і великих фуд вендорів" (the operator's original vision framing) to include
  vendors whose model doesn't need delivery at all.
- **The in-hub assistant agent (§16.4) is excluded from customer PII by default** — it sees
  aggregated/anonymized data (order counts, popular items) rather than a specific customer's
  name or address, tightening the money/decision-authority boundary already established into a
  data-visibility boundary too. Narrows the attack/leak surface specifically for the case
  where §16.4's BYO-model option (§16.52) points the agent at a connected, non-local backend.

---

### 17. Long-term ecosystem decisions (2026-07-18, dialogue pass continued)

Appended by a continuation of the §16 dialogue pass, per the operator's explicit request to
cover *"екосистеми у цілому, зокрема довгострокові"* (the ecosystem as a whole, specifically
long-term aspects) — a deliberate register shift from §16's Wave-0 implementation questions to
multi-year sustainability, governance, and survivability. Same append-only, decision-record
convention as §16.

#### 17.1 Protocol governance — BDFL now, open to revision later
Operator's ruling (Recommended option, confirmed): the operator decides protocol/kernel changes
unilaterally for now (AGPLv3, §16.57, permits this — copyleft governs redistribution, not
decision-making authority), with no formal RFC/maintainer-council process imposed prematurely.
Explicitly **open to revision once the ecosystem has external contributors or independent
self-host vendors who'd need a real voice** — recorded as a deliberate, revisitable choice, not
a permanent structural decision. Named here so a future governance change has a clear "why now"
anchor rather than looking like an unexplained reversal.

#### 17.2 Crypto-agility — a rotation plan from day one, not deferred
Operator's ruling (Recommended option, confirmed): given ML-DSA-65 and other algorithms in the
capability-cert stack (§16.48 root/delegating certs, HybridSigner) could be broken or
deprecated over a 10-20 year horizon, and thousands of independently self-hosted, isolated
hubs (§16.6) would each need to migrate, a **crypto-agile architecture is required from Wave-0**
— versioned capability-certs, an algorithm-migration path that doesn't require a hard fork of
the whole mesh. This is a real, non-trivial addition to the certificate design (already
partially aligned with the existing hybrid ML-DSA-65⊕Ed25519 scheme's own precedent of running
two algorithms simultaneously) — named as a concrete requirement for the P39/P48/capability-cert
blueprint work, not designed in full here.

#### 17.3 Ecosystem survivability if dowiz (the company) ceases to exist
Operator's ruling (Recommended option, confirmed): **claimed hubs must keep working
independently of dowiz's continued existence** — the hub software is AGPL-open (§16.57) so the
code itself survives regardless, and this section adds the deployment-level guarantee: no new
hub can be created without dowiz.org (§16.32's claim mechanic is dowiz-run infrastructure), but
an **already-running hub must not silently lose remote accessibility** if dowiz disappears.
**Real contradiction caught and resolved this pass**: §16.45 committed all hubs to tunnel
through one dowiz-operated Cloudflare account, which would have made every self-hosted hub's
remote access a hard dependency on dowiz's continued operation — directly undermining this
section's own survivability goal. **Resolution (Recommended option, confirmed)**: the hub
software must support **switching its tunnel target to the vendor's own Cloudflare account (or
another tunnel provider)** — a portable escape hatch, not the Wave-0 default. §16.45's
dowiz-operated-by-default design stands unchanged for the common case; this adds the fallback
path that makes "hubs survive without dowiz" an actual mechanism rather than only a stated
intent. Named as a concrete requirement for the hub-provisioning/tunnel blueprint, not designed
in full here.

#### 17.4 Business model, federation, and agent-autonomy boundary — all long-term stances
Three closes:
- **Fixed subscription, no transaction percentage, is a principle forever** — not a Wave-0
  starting point subject to later monetization creep. Revenue growth comes only from hub count
  and tariff tiers for capability/support, never from taking a cut of vendor transaction volume.
  Hardens §16.16 from an initial choice into a durable commitment.
- **Hub isolation (§16.6) is the permanent default; an optional federation phase is possible
  later, never mandatory.** A vendor who wants their hub discoverable by/interoperable with
  others could opt into a future federation protocol (ActivityPub-style, named only as an
  analogy, not a commitment to that specific protocol) — but isolation-by-default is not
  something later scale is expected to erode. This is the long-term counterpart to §16.6's own
  MVP-scoped framing — now explicitly durable, not just "not yet built."
- **The agent's "does not decide for the owner" boundary (§16.4/§16.40's design philosophy) can
  loosen — but only under the individual owner's own explicit control**, never as a platform-
  wide default shift. An owner may personally grant their own agent more autonomy over time;
  dowiz does not unilaterally relax the boundary for everyone. **Implies a graduated-consent
  mechanism** (the owner explicitly expanding their own agent's authority, presumably
  reversible) that is not yet designed — named as a gap for whichever blueprint eventually
  specifies the agent's permission model in detail.

#### 17.5 AR/VR readiness now, regulatory compliance stays vendor-side, opt-in anonymous ecosystem telemetry
Three closes:
- **Build in AR/VR/new-form-factor readiness now** (reverses the Recommended "not priority yet"
  option) — consistent with this dialogue's established pattern (§16.40, §16.56: the operator
  consistently picks the more ambitious, no-deferred-exception path over a scoped-down default).
  The *ad fontes* physics/math foundation (§16.42) is claimed to extend naturally to spatial
  interfaces; this ruling means that extension is designed for now, not left as a theoretical
  future compatibility claim. Adds real scope to the already-large §16.30/§16.35 UI research
  program — named honestly, consistent with how §16.40's schedule-risk acceptance was recorded.
- **Regulatory compliance (food safety, courier labor/gig-economy law, payment regulation)
  stays permanently the vendor's/venue's responsibility** (Recommended, confirmed) — coherent
  with dowiz's protocol-not-operator stance throughout this roadmap (§16.3 couriers, §16.29
  disputes, §16.49 tax). The free-form menu schema (§16.17) lets a vendor add whatever
  region-specific compliance fields their jurisdiction requires without dowiz hardcoding
  per-region rules.
- **Ecosystem-wide health visibility**: opt-in anonymous aggregate telemetry (order counts, no
  PII), vendor-controlled opt-out (Recommended, confirmed) — gives dowiz a real signal on
  whether the ecosystem is growing or stagnating without violating §16.14's data-isolation
  invariant; extends §16.53's heartbeat (liveness-only) with an explicit, consent-gated
  aggregate-metrics layer, kept clearly distinct from it.

#### 17.6 Fork-competition accepted as the price of openness; courier protocol is species-agnostic
Two closes:
- **A competing protocol forked from the AGPL-open hub software is an accepted outcome, not a
  risk to design against** (Recommended, confirmed) — coherent with §17.1/§17.3's own openness
  commitments; AGPLv3's network-copyleft (§16.57) still requires any such fork's modifications
  to stay open, so a fork can't out-close the original, but it can genuinely compete. Recorded
  explicitly so this isn't later treated as an unforeseen threat.
- **Drone/robot couriers use the same courier protocol, not a new one** (Recommended, confirmed)
  — the capability-cert and HRW-matching design (§16.3) are already agnostic to human-vs-machine;
  a courier is any agent holding a valid cert. No separate autonomous-delivery protocol is
  anticipated as a future need — this is a durability property of the existing design, not a
  new build item.

#### 17.7 Root-of-trust decentralization, legacy-version security posture, optional open-standard interop
Three closes:
- **Each hub can be its own self-signed capability-cert root** (Recommended, confirmed) — closes
  a real potential dowiz-forever dependency for cryptography, exactly parallel to §17.3's
  CF-tunnel-portability fix, and caught by asking the same class of question. dowiz may still
  optionally sign/verify roots as part of the claim flow (§16.32) for convenience, but a hub
  never *needs* dowiz's continued existence to be trusted — its own root is self-sufficient.
  This is now the second of two dowiz-forever dependencies found and resolved this dialogue
  pass; §17.3's tunnel fix and this cert-root fix together are what make §17.3's "hubs survive
  without dowiz" claim actually load-bearing rather than aspirational.
- **Legacy/unpatched self-host versions**: vendor-responsibility (Recommended, confirmed,
  consistent with §17.5's regulatory stance) — dowiz publishes security advisories (CVE-style)
  for transparency, but does not force-patch or reach into a vendor's own deployment.
- **Open data standards** (GS1 barcodes, Open Food Facts ingredient/allergen data): optional
  support, never required (Recommended, confirmed) — the free-form menu schema (§16.17) gains
  opt-in fields for vendors who want interoperability with external systems, without dowiz
  inventing a competing closed taxonomy or mandating external-standard compliance.

#### 17.8 Cloudflare itself abstracted as a swappable tunnel layer; governance form deferred
Two closes:
- **Cloudflare-the-company is a third dependency in the same risk class** as the dowiz-account
  CF Tunnel (§17.3) and the dowiz-signed cert root (§17.7) — caught by asking the same class of
  question a third time. **Resolution (Recommended, confirmed)**: the tunnel mechanism is
  abstracted as a port, Cloudflare Tunnel is the Wave-0 default implementation, but alternative
  tunnel providers (WireGuard, other relay options) are technically substitutable — the same
  adapter-port pattern already used for hosting (§16.1), payment (§16.13), storage (§16.29),
  and now the AI model (§16.52), applied one more time. **This closes the third and (so far)
  final dowiz/vendor-forever dependency found this dialogue pass** — together with §17.3 and
  §17.7, the mesh's survivability no longer rests on any single company's continued existence
  by design, not merely by stated intent.
- **Governance's eventual form (informal BDFL vs. a formal non-profit foundation) is
  deliberately undecided** (Recommended, confirmed) — consistent with §17.1's own "open to
  revision, not designed now" framing; no premature foundation structure is being built ahead
  of the actual need for one.

#### 17.9 Hetzner also abstracted as a swappable VPS port; synthesis of the four forever-dependency fixes
Hetzner gets the identical abstraction (Recommended, confirmed): an abstract VPS-hosting port
with Hetzner as the Wave-0 default, not hardcoded into the architecture — completing the
symmetry with §17.8's Cloudflare-tunnel fix.

**Synthesis, worth stating plainly**: this dialogue pass systematically applied one question —
*"what happens to a running hub if this specific company/account stops existing?"* — across
every infrastructure dependency in turn, and found four real single-points-of-failure, all now
resolved the same way (an abstract port with a Wave-0-default implementation, never a hardcoded
dependency): the dowiz-operated Cloudflare account for tunneling (§17.3), the dowiz-signed
capability-cert root (§17.7), Cloudflare-the-company as tunnel provider (§17.8), and Hetzner-
the-company as VPS provider (this entry). Combined with the already-existing adapter-port
pattern for payment (§16.13), storage (§16.29), and the AI model (§16.52), **every named
infrastructure dependency in this entire roadmap is now a swappable port with a Wave-0 default
— none is architecturally permanent.** This is the concrete, falsifiable form of §17.3's "hubs
survive without dowiz" claim, not just a restated intention.

---

### 18. Launch-blocker research → synthesis → blueprint program (2026-07-18)

Appended as the closing pass of the same-day dialogue that produced §16-§17. Once the operator's
~150-question dialogue settled the architecture, the explicit next instruction was: prioritize
for "first real order," dispatch Opus research per priority cluster grounded in the settled
architecture, synthesize with Fable into one build plan, then have Opus write swarm/wave-ready
blueprints — full interface ambition preserved, not truncated, with product-designer rigor.

#### 18.1 Research phase — 5 parallel Opus passes
Each grounded in the relevant §16/§17 sections, researching real 2026 prior art via WebSearch
(not guessed), written to `docs/research/`:
- **R1** `OPUS-R1-INTERFACE-RENDERING-2026-07-18.md` — wgpu UI, a11y, text input, typography,
  voice, deployment, SEO, AR/VR. Flagship pass. Found: full GPU app UI is production-real
  (GPUI/Zed); AccessKit native-ready but web/canvas backend planning-only; cosmic-text/parley
  solve text shaping (not greenfield); no precedent for physics-generated glyphs (stated
  honestly); caught a real contradiction (§16.34 vs. P38's own planned IME-input DOM overlay).
- **R2** `OPUS-R2-PAYMENT-MONEYFLOW-2026-07-18.md` — payment adapter, PCI-vs-canvas, split
  payment. Found: canvas card capture is PCI-impossible (SAQ-D); three compliant paths
  identified; surfaced the merchant-of-record fork for food-court.
- **R3** `OPUS-R3-HUB-PROVISIONING-IDENTITY-2026-07-18.md` — CF Tunnel automation, capability-
  cert hierarchy, crypto-agility, Hetzner provisioning. Found: dowiz's ML-DSA-65⊕Ed25519 hybrid
  is already IETF composite suite OID `1.3.6.1.5.5.7.6.48`; the 1,000-tunnel/account cliff;
  biscuit-style chains over the existing HybridSigner, not SPIRE (no PQC support).
- **R4** `OPUS-R4-ORDERFLOW-COURIER-NOTIFICATIONS-2026-07-18.md` — notifications, offline
  drafts, unified order machine, wallet transfer. Found: the HRW matcher and unified pickup/
  delivery order machine already exist in code; CRDTs explicitly not warranted for drafts;
  iOS-Safari-web-push-only-for-installed-PWAs is a real coverage gap.
- **R5** `OPUS-R5-MULTIVENDOR-ECOSYSTEM-OPS-2026-07-18.md` — backup, auto-update, food-court
  data model, brand preview, moderation, licensing. Found: `age`-style envelope over
  rclone-crypt; `self_update` has no rollback (A/B-slot supervisor needed); row-scoped
  single-DB food-court model; brand preview is a uniform-buffer swap, not a second renderer.

Two real cross-report contradictions surfaced this pass and were resolved directly with the
operator (not silently defaulted): food-court merchant-of-record (resolved: **each vendor is
their own MoR** — dowiz never becomes a party to the money) and R1's IME-vs-§16.34 conflict
(resolved: **Latin+Cyrillic only for Wave-0**, non-Latin/IME scripts deferred to v2, consistent
with §16.58's existing RTL ruling).

#### 18.2 Synthesis phase — Fable
`docs/design/CORE-ROADMAP-2026-07-17/SYNTHESIS-LAUNCH-BLOCKERS-2026-07-18.md` reconciles all
five reports against §16-§17 and the two rulings above into: a cross-cutting dependency map
(12 resolutions, X1-X12 — e.g. AccessKit-web being planning-only forces a hand-rolled a11y-mirror
on every web screen; the payment-path and desktop-shell choices are one coupled decision), a
build sequence to four milestones (**M1** first real pickup order, **M2** first delivery order,
**M3** first claimed/automated hub, **M4** first food-court order), four further operator
decisions (raised and closed the same pass — card-capture surface/desktop shell, backup
break-glass severity, abandoned-hub suspension, food-court market scope), and the blueprint
breakdown consumed by §18.3.

**Four decisions closed in this pass** (beyond the two research-phase rulings): (A) card capture
via **hosted redirect** (Path C) for web+desktop — frees desktop to pure `winit`+`wgpu`+
AccessKit with no webview; mobile keeps Tauri for a native provider SDK sheet. (B) **No
break-glass, ever** — self-custody is absolute across wallet, backup, and cert-root loss alike;
consistent with §16.47. (C) An abandoned claimed hub **may be suspended-but-preserved** (compute
released, state retained, still the vendor's) — qualifies §16.57 without reopening it.
(D) Food-court N-leg checkout proven first in **Eurozone/EUR via Stripe Connect + Adyen**,
a feature-scope limit, not a platform-architecture one (§16.20 stays market-agnostic).

#### 18.3 Blueprint phase — Opus, swarm/wave-ready
2 canon-diffs (dated append-only corrections, not rewrites) + 18 new blueprints across 4 build
waves, each written against the 20-point standard with live-verified ground truth (real
file:line citations against actual kernel/engine code, not assumed):

| Unit | File | Wave | One-line finding |
|---|---|---|---|
| P38-rev | `BLUEPRINT-P38-webgpu-render-engine.md` §12 | canon-diff | Strikes the planned IME `<input>` overlay (superseded by the Latin+Cyrillic ruling); adds 4 AR/VR insurance constraints as hard requirements now |
| P39-rev | `BLUEPRINT-P39-app-shell-installability.md` §1.2 | canon-diff | Records desktop = `winit`+`wgpu`+AccessKit, no webview (from ruling A); mobile keeps Tauri |
| P57 | `BLUEPRINT-P57-canvas-text-input.md` | W1 | Zero text-input code exists anywhere (verified by grep); two-lane build, cosmic-text, Latin+Cyrillic only |
| P58 | `BLUEPRINT-P58-a11y-mirror-everywhere.md` | W1 | Mirror doesn't exist yet; engine has no semantic layer — designs a new `engine/src/semantics.rs`; owns the ARIA-textbox convention |
| P59 | `BLUEPRINT-P59-capability-cert-chain.md` | W1 | A biscuit-style chain is already ~70% built in `cap.rs`; found the real gap (signing is Ed25519-only, not hybrid) |
| P60 | `BLUEPRINT-P60-payment-adapter-core.md` | W1 | Reuses the kernel's existing compile-firewall pattern for the no-PAN guarantee; N-leg atomicity as an event-sourced saga with proptest proof |
| P61 | `BLUEPRINT-P61-notification-fabric.md` | W1 | The send path is already scaffolded (`NotificationRouter`, one ignored test); resolves a real scope overlap with the existing P43 |
| P62 | `BLUEPRINT-P62-catalog-multivendor-data-model.md` | W1 | The existing catalog is currency- and vendor-blind — that gap *is* the X7 leaf invariant; owns it as an unrepresentable-if-invalid type |
| P63 | `BLUEPRINT-P63-shell-platform-spike.md` | W1 | A measurement blueprint, not a feature — 4-valued verdict system (Confirms/Refines/Contradicts/Blocked) forbids asserting unmeasured numbers |
| P64 | `BLUEPRINT-P64-intent-engine-friction-voice.md` | W1 | `Money` is type-proven not a `FieldValue` — the field animates, the payment amount never does; `CommitToken` unconstructible without the full held gesture |
| P65 | `BLUEPRINT-P65-dispatch-orchestrator.md` | W2 | Primitives are cross-repo (bebop-repo + dowiz); no-scoring red line enforced by a byte-identical offer-sequence test, zero new order-FSM edges |
| P66 | `BLUEPRINT-P66-data-wallet-offline-drafts.md` | W2 | Wallet-transfer crypto reuses existing kernel primitives (zero new deps); animated-QR transport chosen with a real size-budget justification |
| P69 | `BLUEPRINT-P69-customer-storefront-checkout.md` | W2 | M1 critical path; designs the redirect-payment suspend/resume `Journey` state machine; bot-pack generated from catalog state with an anti-drift test |
| P70 | `BLUEPRINT-P70-owner-surface.md` | W2 | Precise supersede/extend split against the existing P48; GDPR delete solved via crypto-erasure (resolves append-only-log vs. right-to-erasure by math) |
| P67 | `BLUEPRINT-P67-hub-provisioning-claim.md` | W3 | Caught a real bug before it was code: per-hub tunnel token/root key must be injected at first boot, never baked into the shared golden snapshot |
| P68 | `BLUEPRINT-P68-hub-supervisor-update-backup.md` | W3 | Confirms zero-new-crypto-deps reuse (like P66); structurally forbids a dowiz recipient key; separates plaintext-local vs. encrypted-offsite snapshots |
| P71 | `BLUEPRINT-P71-courier-surface.md` | W3 | Found and resolved a real numeric conflict (P52's 60s offer-timeout vs. P65's 30s) in P65's favor; kept the battery DoD honestly conditional on P63 |
| P73 | `BLUEPRINT-P73-dowiz-org-landing.md` | W3 | The landing hero *is* the live field engine demoing itself; vendor-listing is made type-level unreachable, not just policy-forbidden |
| P72 | `BLUEPRINT-P72-foodcourt-checkout-nleg.md` | W4 | Pure composition of P60+P62+P69; "dowiz never becomes a payment party" encoded as a type, not prose |
| P74 | `BLUEPRINT-P74-moderation-reports-blocklist.md` | W4 | The HRW matcher lives in a crate with no dependency edge to the kernel at all — the strongest possible form of the no-scoring red line |

**Milestone gates**: M1 = P69+P70(minimal)+P61+P60 on one manually-provisioned hub. M2 = +P65+P71.
M3 = +P67+P68. M4 = +P72. Full dependency ordering, single-owner shared-contract assignments
(idempotency → P60, catalog invariant → P62, golden-image → P67/P68, a11y harness → P58, intent
grammar → P64, cert wire format → P59), and Track-R (procedural glyphs, XR backends, deepened
intent-UI research — permanently off the critical path) are in the synthesis document, not
duplicated here. This section is the index; the blueprints and the synthesis are the source.

#### 18.4 Architectural tension found at commit time — resolved, §16/§18 wins

While committing §18's blueprint program, a concurrent swarm's merge to `main`
(`588188efad0` — `merge(feat/g3-dom-fieldsim)`) landed **DOM-based web UI work**
(`web/index.html`, `web/src/app.mjs`, wasm-driven, Layer-G/"G3" naming from the pre-dialogue
Layer A-I axis) — a `<div>`/`<script type="module">` browser page consuming kernel wasm exports
directly, with **no wgpu, no canvas, real DOM elements throughout**.

**This directly contradicts §16.30/§16.40**: the entire UI (menu, checkout, admin, courier)
renders through the wgpu field engine, canvas-only, zero DOM anywhere, no legacy-DOM fallback
mode — a closed decision the operator reaffirmed knowingly even after being shown its real
schedule cost (§16.40's "one path, full set at once, timeline shifts accordingly").
`feat/g3-dom-fieldsim`'s DOM approach is an **older, parallel track** (its base commit sits
before this session's UI-paradigm dialogue even started) — it was not built in defiance of
§16.30, it simply predates it and nobody had yet told that branch's own swarm the axis had
changed.

**Resolution, stated plainly so no future reader or swarm has to re-derive it**: **§16/§17/§18
is the current, prioritized axis.** Where `feat/g3-dom-fieldsim` (or any pre-dialogue Layer-G/
wasm+DOM work) conflicts with §16.30's wgpu-only mandate, **§16.30 wins** — the DOM-based
`web/` prototype is legacy, not canon, and must not be extended further as if it were the
product's real UI track. Its wasm-export-binding discipline ("kernel is the sole FSM/money
authority, no JS reimplementation of geo/spectral/FSM math") is a genuinely good property worth
preserving conceptually — but the rendering layer itself (DOM, `<script type="module">`,
`app.mjs`) is superseded by P38/P57-P74's wgpu-canvas program. This is **not** a request to
delete `feat/g3-dom-fieldsim`'s work outright (it may still be useful as a reference/fallback or
for non-product tooling) — it is a priority statement: new product-UI work builds on §16/§18's
wgpu track, not the DOM track, and any future reconciliation should migrate `web/`'s
wasm-binding logic onto the canvas substrate rather than growing the DOM path further.

**Also found the same commit-time pass**: several long-lived feature branches have drifted from
`main` — `feat/kalman-organ` (1052 ahead / **677 behind**, the most stale), `feat/agentic-mesh-*`
(226 ahead / ~0-1 behind), `feat/spectral-energy-*` (228 ahead / ~0-1 behind), and the
`research/*-verify-redteam-*` branches (162-228 ahead / 0-2 behind). Reconciliation against
`main` (which now carries the full §16-§18 axis) is tracked as separate operational work, not
folded into this roadmap section — see the isolated reconciliation branch this same pass.

---

### 19. Perf, physics and mesh research wave — status-ledger registration (2026-07-19)

Appended after §18, same append-only rule. The 2026-07-18→19 session ran ~20 dispatched Opus
investigations + 5 synthesis/blueprint passes over the kernel/engine performance surface, the
bebop2 mesh auth layer, and two product levers (ETA, living memory), then consolidated the
whole day into **one status ledger** rather than N scattered docs. Per the standing research-only
directive, this wave wrote **zero product code** — the deliverable is a dependency-ordered plan
(18 blueprints still to write, 4 already fully blueprinted, 12 closed scans) plus a small set of
**already-existing local-only code artifacts** produced in the surrounding session, all unpushed
and several process-blocked (§19.3). This section is the index; the ledger, the meta-gap audit,
and the 22 `BLUEPRINT-P{75-96}` files are the source — not duplicated here.

**Source of truth (do not re-derive from this entry):**
[`CORE-ROADMAP-2026-07-17/MASTER-STATUS-LEDGER-2026-07-19.md`](CORE-ROADMAP-2026-07-17/MASTER-STATUS-LEDGER-2026-07-19.md)
— §1 one-glance status table (every item, status vocabulary, source cites), §3 the merged
dependency sequence, §5 the 15 outstanding operator decisions. Adversarial coverage check:
[`CORE-ROADMAP-2026-07-17/META-GAP-AUDIT-2026-07-19.md`](CORE-ROADMAP-2026-07-17/META-GAP-AUDIT-2026-07-19.md)
(4 HIGH + 13 hygiene findings against 8 dimensions). Cross-cutting QA governance:
[`CORE-ROADMAP-2026-07-17/BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md`](CORE-ROADMAP-2026-07-17/BLUEPRINT-Q-SERIES-VERIFICATION-OBSERVABILITY-2026-07-19.md).
All three already carry index rows in [CORE-ROADMAP-INDEX §10](CORE-ROADMAP-INDEX.md).

#### 19.1 What the wave covered — four clusters + a positive-negative closed set

| Cluster | Items | One-line |
|---|---|---|
| **Mesh auth-layer refactor** | **M1**, **P92**, **P93**, **P94** | Real RFC-5705 exporter binding (M1 — an open red-team correctness item on the live path), verify-once channel-bound fast-path (P92, fully blueprinted, GO-with-conditions + measure-first NO-GO gate), store-and-forward transcript-binding + replay-window (P93), in-memory `ScopeMask` bitmask (P94, wire form untouched) |
| **Performance tiers** | **P75–P91** | P75 CI bench-gate re-architecture (owns the baseline schema everything else benches into) · algorithmic fixes (P77/P78/P79) · large mechanical bench-coverage expansion (P80/P81/P82/P83) · physics/GPU bets (P86–P89, several operator/P38-gated) · crypto process+spec integrity (P85 NTT red-line remediation, P91 KEM ring correction) · P90 contention-bench registration. P84 reserved (operator-gated, unproposed) |
| **Product levers** | **P95**, **P96** | Living-memory BM25 persistence + incremental update (P95, verdict HOLD/NO-GO absent a real repeated caller — latent hazard only) · wire live Kalman/EMA courier speed into ETA (P96, small/isolated/non-red-line, byte-for-byte static fallback) |
| **Q-series governance** | **Q1–Q4** | Cross-cutting Q-namespace (deliberately not P-numbered — governance, not feature) ensuring each P-item's own stated DoD is actually met as it builds: Q1 claim-verification (`DONE-VERIFIED` status), Q2 feature telemetry (extends the closed `LogEvent` enum), Q3 review gate (mostly already exists), Q4 interface verification (reuses P38 render-floor + Playwright) |

Twelve scans **closed with no code** — five *validated the existing design under adversarial
scrutiny* (money/payment tri-state, crypto trust-boundary, core consolidation, batch posture,
corpus tokenization) and five/seven found *no target for a real technique exists here, often by
standing policy* (BitNet, QKD, fraud scoring, bit-slicing, energy-currency; each with a named
reopening trigger). These honest negatives are load-bearing deliverables — see ledger §2 for the
two-kinds-of-closed distinction (do not re-litigate a NO-TARGET item as "never investigated").

#### 19.2 Dependency-ordered execution sequence (high level — full table in ledger §3)

Two largely **independent lanes** that parallelize fully: the **dowiz kernel/engine perf lane**
(P75, P77, P79–P81, P83, P87–P89, P90-merge, P91) never touches bebop2, and the **bebop mesh +
perf lane** (P85, P76, P78, P82, M1, P93, P92, P94, D-9) never touches the dowiz kernel. Only two
soft cross-edges exist (P75's bench schema is cited by P82; P82's `HybridGate::check` lane is the
natural substrate for P92's measure-first D-BENCH gate) — neither is a hard block.

- **bebop lane gate-0 (freeze-breaker):** **P85** (NTT `--no-verify` red-line remediation) **+ C3
  ungated-keygen resolution** must precede the *entire* bebop lane — until both close, the hooks
  freeze **all** hook-respecting bebop commits, M1/P93/P92/P94 included. This is a real shared
  gate the mesh plan alone did not surface.
- **mesh cluster order:** **M1** (exporter fix, own reviewed commit + mandatory independent
  adversarial review) → **P93** (transcript+replay on the shared `signed_frame.rs` surface) →
  **P92** (opt-in fast-path; run D-BENCH — NO-GO if presence volume doesn't clear the threshold) →
  **P94** (its value is *created* by the fast-path, so it sequences after).
- **dowiz lane priority:** **P75** first — the perf gate everything else writes baselines into —
  then the small algorithmic fixes and the large bench-coverage expansion; P86/P87 wait on the
  operator-owned P38 §4.2 GPU decision; P89 is the falsifiable spectral-vs-DCT bet.

Highest-leverage single items: **P75** (dowiz) and **P85+C3** (bebop), then **M1**. P92/P94/
P86/P87 are correctly late — opt-in, gated, or awaiting an operator ruling not yet taken.

#### 19.3 Current real state — all LOCAL/UNPUSHED; three pre-existing code artifacts

Everything in this wave is **local, unpushed planning**. Push-state re-verified 2026-07-19 01:00:
dowiz `origin/main` sits at `4b30c9b4c` with the **entire local main line above it unpushed**
(the P57–P74 merge wave). The three code artifacts below were produced in the surrounding session
(not by this planning wave) and are *registered*, not re-implemented, by the ledger:

| ID | Artifact | Commit / location | Status |
|---|---|---|---|
| **I1 / NTT** | bebop2 ML-KEM-768 incomplete-NTT (exhaustively proven, 0/65,536 mismatches; **NOT wired**) | `986646a`, unpushed on bebop local `perf/bus-contention-2026-07-18` | **DONE-LOCAL-UNPUSHED-CODE · PROCESS-RED** — committed `--no-verify` past 5 gates incl. the mandatory 3-model review; a *blocked* item, not a completed one, until **P85** closes |
| **I2 / Arena** | thunderdome → `kernel/src/slot_arena.rs` behind off-default `slot-arena` feature | `a857cd71a`, unpushed (in the local dowiz main line above `4b30c9b4c`) | **DONE-LOCAL-UNPUSHED-CODE** — operator override of the research "no adoption" verdict, logged in the divergence ledger; zero default-build cost; push decision open (OD-4) |
| **I3 / Contention** | contended benches + budget CAS + `token_bucket` clock hoist (637 kernel tests green on branch) | `8c865805b` + `8256dbffb`, unpushed on dowiz local `perf/contention-bench-2026-07-18` | **DONE-LOCAL-UNPUSHED-CODE** — registered by **P90**; merge/push + GCRA-swap decisions open (OD-1/OD-2) |

A separate documentation-integrity note (ledger §0): 15 of this session's research/design docs
were briefly disk-lost to worktree/merge churn while untracked and recovered verbatim from
subagent transcripts — 4 design docs restored to this directory, 11 `docs/research/` docs staged
in scratchpad pending an operator/lead restore (OD-15). The **critical path is operator rulings,
not more design**: ledger §5 lists all 15 (gate-0 C3, P85 closure path, the push decisions, the
P38 §4.2 GPU call, the two P93 privacy/broadcast forks, the P92 D-BENCH proceed gate).

# Part III — Quality-Bar Standard (verbatim, 2026-07-17)

> Full content of the former `CORE-ROADMAP-STANDARD-2026-07-17.md`, inlined here 2026-07-20; the
> standalone file has been deleted. This is standing doctrine, not a status snapshot — it does not
> go stale the way the ground-truth/status docs do.

## CORE ROADMAP STANDARD (2026-07-17) — the planning ideas, saved before execution

> **Status: this document is the operator's standing quality bar for ALL future planning in this
> repo, not a one-off deliverable.** Per operator directive (2026-07-17, verbatim intent preserved
> in `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/00-SOURCE-PROMPT.md` and this session's
> transcript): "This should be not a guidance - but level of quality constant invoked for any
> planning... zero divergencies from it." Every future blueprint in this repo is measured against
> §2 below until the operator says otherwise.
>
> **Sequencing, per operator instruction:** this document is Step 1 — "save the planning ideas
> first." Step 2 — "then save the plans after finishing them" — is the phase-by-phase execution
> this document orchestrates, starting immediately after this lands (§5).

---

### 0. Ground-truth inventory (verified this session, not assumed)

Before designing anything new, the existing planning corpus was enumerated live (not from memory
or an older doc's claim):

**Pre-existing master roadmaps (dowiz, duplicated verbatim into the `dowiz-agentic-mesh` and
`dowiz-spectral-evolution` worktrees by shared history):**
- `MASTER-ROADMAP-MVP-2026-07-12.md` (repo root) — earliest, MVP-scoped.
- `docs/design/MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md`
- `docs/design/MASTER-INTEGRATION-PLAN-2026-07-14.md`
- `docs/design/MASTER-ROADMAP-10-PHASES-2026-07-14.md`
- `docs/design/MASTER-EXECUTION-PLAN-2026-07-13.md` — **added 2026-07-17**: omitted from this
  inventory as originally written; the P-I Wave-1 audit (§0/§2.5) found it (the SOVEREIGN doc's own
  header already named it) — so the superseded set is **5 older docs, not 4**, and the banner pass
  covered all 5.
- `docs/design/MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md` — **the newest, most-actively-
  referenced one**, with 19 phase blueprints already written (`docs/design/sovereign-roadmap-
  2026-07-16/BLUEPRINT-P01..P19-*.md`) and a live P06-blocks-three-arcs finding already tracked in
  memory (`sovereign-architecture-19-phase-roadmap-2026-07-17.md`).

**Consolidated arc summaries (each a self-contained sub-roadmap for one initiative):**
- `dowiz-agentic-mesh/docs/design/agentic-mesh-protocol-2026-07-17/AGENTIC-MESH-PROTOCOL-CONSOLIDATED.md`
- `dowiz-spectral-evolution/docs/design/spectral-energy-flow-evolution-2026-07-16/SPECTRAL-EVOLUTION-CONSOLIDATED.md`
- `docs/design/living-interface-2026-07-16/LIVING-INTERFACE-ROADMAP.md`

**Today's mesh-masterwork corpus (this session):**
- `docs/design/bebop2-mesh-tensor-hermetic-2026-07-17/` — source prompt, 10 batch/correction docs,
  the v1 synthesis, and (in flight) the v2 synthesis.
- 13 additional standalone research blueprints landed yesterday/today (latency, eigenvector,
  cache-tensor-arena, event-driven orchestrator, fault isolation, Linux-engineering-adoption,
  memory-optimization, native-telemetry, wave-scheduling, delivery-flows audits, web3 synthesis).

**Decision on consolidation (per "not revisit twice"):** `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-
2026-07-16.md` becomes the **single canonical entry point**. It already has the newest phase
structure and is already the one this session has been updating (§8.12/Phase 30 added earlier
today). The four older MASTER-* docs are **not deleted** (they're historical record of earlier
planning rounds, real audit trail) but get an explicit **SUPERSEDED-BY banner** pointing here, so
a future reader never re-derives from a stale one. This document (`CORE-ROADMAP-STANDARD`) is the
**quality contract** the canonical roadmap and every phase blueprint under it must satisfy — it is
deliberately a *different document* from the roadmap itself, because a standard shouldn't be
rewritten every time a phase changes.

---

### 1. Scope boundary (operator-stated, binding)

- **All local repos**: `dowiz` (this repo), `bebop-repo` (`bebop2/` crates — active), `openbebop`
  (live push remote for bebop-repo), `dowiz-agentic-mesh`, `dowiz-spectral-evolution`,
  `hermes-agent-kernel-rewrite` where cited.
- **Excluded**: the original `bebop` repo (git `origin` remote of `bebop-repo`,
  `git@github.com:SyniakSviatoslav/bebop.git`) — legacy, archived, ideas may be raided from it but
  it is not maintained or planned against.
- **Execution substrate**: kernel/Rust/WASM only, per the standing execution-model rule
  (`bebop2-mesh-masterwork-2026-07-17.md`) — Node/TS/JS/Python are adapters/bridges at most, being
  actively eliminated (`tools/eqc-rs` port, `apps/web`+`packages/*` decommission already landed
  this session).
- **Deployment target**: decentralized local nodes (courier devices, owner-operated hub servers,
  client devices) — Fly/Supabase decommissioned this session, not the planning target going
  forward.

---

### 2. THE STANDARD — what every phase blueprint under the canonical roadmap must contain

This is the reusable contract (operator: "not a guidance — a quality constant"). A blueprint that
skips any of these is incomplete, not merely light:

1. **Ground truth section** — every claim about existing code carries a `file:line` cite verified
   *this pass*, not inherited from an older doc's claim. "Ground truth is non-discussible."
2. **DoD (Definition of Done)** — falsifiable, machine-checkable where possible (a test that goes
   RED before the change and GREEN after, not a prose checkbox).
3. **Spec-driven + event-driven TDD plan** — the spec (types/schemas/invariants) precedes the test,
   the test precedes the code; state transitions are modeled as events (matches the kernel's own
   `decide`/fold law), tests assert on event sequences, not just end-state.
4. **Predefined types & constants** — every new domain concept gets a named Rust type/const before
   implementation starts (no stringly-typed or magic-number placeholders in a blueprint).
5. **Adversarial/chaos test cases, including intentionally-failing ones** — at least one test per
   blueprint designed to break the invariant under test (operator: "test cases... designed to
   literally break everything"), not only happy-path coverage.
6. **AI/system-hazard safety section grounded in math/engineering** — reachability of an unsafe
   state must be argued from type-system/invariant structure (per the Monocoque doctrine already
   established: `docs/design/hermetic-architecture-2026-07-16/HERMETIC-ARCHITECTURE-PRINCIPLES.md`
   + `bebop2-mesh-tensor-hermetic-2026-07-17/19-SYSTEM-COHERENCE-AND-AUTHORITY-BOUNDARY-REDO.md`'s
   "finite anchored authority, not zero" finding) — never a policy/prose assurance.
7. **Links to docs & memory** — every blueprint cross-references the memory files and design docs
   it depends on or supersedes, by name, so the index (§6) stays navigable.
8. **Schemas designed for scaling** — data shapes must state their scaling axis (nodes/tiles/
   events/sec) and the point at which they'd need to change, not be presented as timeless.
9. **Linux-OS-development-style engineering discipline** — per
   `docs/design/BLUEPRINT-LINUX-ENGINEERING-PRINCIPLES-ADOPTION-2026-07-17.md`'s verdict framework
   (ALREADY-EQUIVALENT / REINFORCES / EXTENDS / GAP / DOES-NOT-TRANSFER) — reused, not re-derived.
10. **Benchmarks + telemetry** — every hot-path change ships with a measured before/after number
    (this session's own build-test-first passes are the model: real `cargo bench`/microbenchmark
    output, not an estimate) and a telemetry hook so regressions show up automatically, not only at
    review time.
11. **Microservice-style isolation / bulkhead** — a blueprint touching a shared resource must name
    the isolation boundary that keeps its failure from propagating (per idea #141 in the mesh
    dialogue's 185-item ledger, and the already-built `bounded_drainer.rs`/`budget.rs`
    degrade-closed patterns).
12. **Mesh-networking awareness** — where relevant, state whether the feature is node-local,
    gossip-propagated, or requires the transport layer (`iroh_transport.rs`/`discovery.rs`), and
    cite the real payload-size/frequency budget it needs.
13. **Rollback/fallback + self-healing/self-terminating, stated as math, not metaphor** — per the
    operator's own three-way synthesis (idea #185): Self-Healing = redundant/error-correcting math
    property; Self-Termination = a hard invariant boundary (unrepresentable-state, not a
    supervisor's decision); Snapshot Re-entry = cheap regenerative recovery from the last valid
    epoch. A blueprint claiming any of these three must show which one and why, not use the words
    loosely.
14. **Error-propagation isolation + "smart index" for catching mistakes** — cite the specific gate
    (type system, drift-gate, CI check) that would turn the bug class this blueprint introduces
    into a compile-time or CI-time failure, not a runtime surprise.
15. **Living-memory awareness (time/topology/data-flow)** — cross-reference
    `internal-retrieval-living-memory-arc-2026-07-14` where the blueprint's data has a temporal or
    topological access pattern, rather than treating storage as flat.
16. **Tensor/spectral representation where applicable** — reuse the hybrid/spectral tensor-graph
    machinery already built (`kernel/src/spectral.rs`, `spectral_cache.rs`, the Phase-28 arena) and
    the `tools/eqc-rs` equation-compiler for any closed-form math, storing generated equations as
    data (RGB-seed/procedural-encoding pattern, idea #130, ADOPTED per
    `20-BUILD-TEST-FIRST-REEXAMINATION.md`'s CORDIC proof) where a deterministic portable
    implementation exists — never a per-platform-libm form.
17. **Regression tracking** — every blueprint that fixes or changes behavior gets a named regression
    test that stays in the suite permanently, referenced in `docs/regressions/REGRESSION-LEDGER.md`.
18. **Clear instructions for other agentic workers** — a blueprint must be executable by an agent
    with zero prior session context: explicit file targets, explicit acceptance criteria, no
    "you'll know it when you see it" language.
19. **Reuse-first, upgrade-if-needed, unbounded token/time budget** — a blueprint proposing new
    machinery must first show the existing pattern it could extend and why extension doesn't work;
    "it would take too long" or "it's simpler to skip" are not valid reasons to avoid a needed
    refactor (operator: "Refactoring or major changes must not be avoided to avoid responsibility").
20. **Hermetic principles honored explicitly** — cite which of the seven Hermetic principles
    (`HERMETIC-ARCHITECTURE-PRINCIPLES.md`) the blueprint's design choice reflects or tests against.

---

### 3. Phase structure — lowest (core) to highest, absorbing existing work rather than re-deriving

> **NAMING RULING (2026-07-17, Wave 3 — P-I audit §4):** the "P-A..P-I" letters below are ratified
> as **`Layer A..I`** — an orthogonal **altitude axis** grouping clusters of numeric phases, never a
> renumbering of the canonical execution numbering **P01–P30** (P01–P19 as numbered blueprint files
> in `sovereign-roadmap-2026-07-16/`; P20–P30 as standalone blueprints indexed from SOVEREIGN
> §8.1–§8.12 — this section's original "P01-P19" reconcile scope is stale by 11 phases). The "P-"
> prefix is retired from prose to kill the P-D/P04 lexical collision; on-disk filenames keep their
> provenance names. Crosswalk table: `CORE-ROADMAP-INDEX.md`.

Ordering rule (operator, restated across this whole session): smallest kernel-level abstraction
first, highest-level product/UI last. Each phase below states what it absorbs from the existing
252-document corpus rather than starting blank.

| Phase | Scope | Absorbs / supersedes | New this pass |
|---|---|---|---|
| **P-A. Core kernel primitives** | Equations-not-primitives (`eqc-rs`), tensor/sparse/branchless memory layout, HugePages/tiling | Mesh-masterwork Batch 1, Batch 8, `BLUEPRINT-EIGENVECTOR-REFACTOR-PLAN`, `BLUEPRINT-CACHE-REFERENCE-GRAPH-TENSOR-ARENA` | Wire `eqc-rs` into `geo.rs:39`/`domain.rs:95` (already identified, unstarted) |
| **P-B. State/consistency + living memory** | Event log, content-hashing, snapshots/epochs, CRDT boundary | Batch 2, System-Coherence doc 19 (tile→normalize→hash→snapshot chain + the 2 real bugs found) | The normalize-before-hash fix; drift-gated arena snapshot |
| **P-C. Safety / self-healing / self-terminating** | Circuit breakers, invariants, the watchdog/authority boundary | Batch 3, doc 19 Part 2 (finite-anchored-authority finding) | Hysteresis fix (`hydra.rs`), restart-intensity as a launch-path predicate (T-6) |
| **P-D. Consensus / trust / capability** | Sybil-resistance, DecisionUnit gossip, PoQ | Batch 4, 6, 7, `BLUEPRINT-LATENCY-ELIMINATION` §2 (Decision Compiler) | `RootDelegationPolicy` closure (`node_id.rs:156-184`) — open operator decision |
| **P-E. Network / hardware / crypto-in-core** | Mesh transport, hardware attestation, crypto-verification speedup | Batch 5→14v2 (target-corrected), the SIMD-batched-verify + core/cache-domain-NUMA redirect (in flight) | Pending the current Opus redo |
| **P-F. Local AI / MoE mesh** | DecisionUnit compilation, MoE-as-mesh-mirror, STARK-in-core | Batch 21 (distributed-inference rejection), pending MoE-specific redo (in flight) | Pending |
| **P-G. Product/UI on kernel** | WASM bridge wiring, physics-UI, RLS-safe migration | Batch 9 (bridge already exists, wiring gap), `BLUEPRINT-P16-product-ui-rebuild`, `LIVING-INTERFACE-ROADMAP` | Money dual-authority flip (explicitly gated, not Wave 1) |
| **P-H. Ops / telemetry / benchmarks / regression** | Native telemetry, chaos testing, regression ledger | `BLUEPRINT-NATIVE-TELEMETRY-LATENCY-EXPLAINABLE-EVENTS`, `BLUEPRINT-WAVE-SCHEDULING-CONCURRENT-EXECUTION`, `REGRESSION-LEDGER.md` | Chaos-injection test harness (idea #143), unconditional-fail test suite |
| **P-I. Cross-repo consolidation** | Update the 4 older MASTER-* docs with SUPERSEDED banners, reconcile P01-P19 against this structure | All 5 pre-existing master roadmaps | The consolidation pass itself |

P06 (ML-DSA `key_V` split-identity verifier) remains the cross-cutting blocker already identified
(memory: `sovereign-architecture-19-phase-roadmap-2026-07-17.md`) — it gates P-C's independent-
verification leg and P-G's product-safety story. Highest-leverage single build item across those
phases, unchanged finding.

**Correction (2026-07-17, P-D audit + BLUEPRINT-P-D, both independently verified):** P-D's
capability issuance (`RootDelegationPolicy` in `bebop-repo/bebop2/proto-cap/src/node_id.rs:156-184`)
is **NOT gated by P06**. P06 is a dev-time CI merge fence over code diffs; P-D's issuance is
runtime courier onboarding. They share substrate (`load_genesis`/`verify_chain`) and the open C4b
hardening item, but neither functionally blocks the other — P-D's mint/admission path is
Ed25519-only today, with zero `key_V` dependency in the code (grep-confirmed). See
`docs/design/CORE-ROADMAP-2026-07-17/P-D-audit-root-delegation-policy.md` §3 for the full
Descartes-square reasoning behind this correction.

---

### 4. Orchestration plan (Step 2, starting immediately after this saves)

Per operator: "orchestrate the planning phase smartly assigning each agent or small team of agents
the corresponding phase." Model assignment restated: **Opus for research/audit** (grounding each
phase in live code), **Fable for reasoning/planning** (writing the actual blueprint against §2's
contract). Waves are collision-free (different files/phases, no shared mutable state):

- **Wave 1** (parallel, Opus): ground-truth audits for P-D (RootDelegationPolicy), P-G (WASM-bridge
  wiring detail), P-H (existing telemetry/regression tooling inventory), P-I (read all 5 old
  MASTER-* docs + all 19 P0x blueprints in full, produce a diff-against-this-standard).
- **Wave 2** (parallel, Fable, after Wave 1 + the in-flight mesh resynthesis both land): write the
  actual phase blueprints P-A through P-I against §2's 20-point contract, each citing its Wave-1/
  mesh-masterwork grounding.
- **Wave 3** (single Fable pass): the canonical roadmap update — supersede-banner the 4 old
  MASTER-* docs, fold P-A..P-I into `MASTER-ROADMAP-SOVEREIGN-ARCHITECTURE-2026-07-16.md`, build
  the master index (§6 below, promoted to a real `docs/design/CORE-ROADMAP-INDEX.md`).

Not dispatched yet — this document is the Step-1 save. Wave 1 dispatches next.

---

### 5. What this document deliberately does NOT do

Per the standard it sets (§2 item 19: reuse-first) this document does not re-derive P06, the
mesh-masterwork verdicts, or the Hermetic principles — it points at them. It does not yet contain
the actual phase blueprints (that's Wave 2's output, saved separately and indexed here once real).

# Part IV — Ground-Truth Re-Baseline, 2026-07-19 (verbatim, historical)

> Full content of the former `GROUND-TRUTH-2026-07-19-FINAL.md`, inlined here 2026-07-20; the
> standalone file has been deleted. **Superseded for LIVE status by Part I §0 above** — kept here
> verbatim for its landing-wave detail and the specific conflict-resolution/branch-audit record,
> which remain historically accurate for that day even though the numbers are no longer current.

## GROUND-TRUTH — dowiz local `main` final state (2026-07-19)

> Authoritative re-baseline after the autopilot landing wave. Supersedes
> GROUND-TRUTH-2026-07-17 (that doc was stale — main had moved through P57–P74
> by the time it was written). Read THIS for current truth; pasted "pending" todos
> from compacted sessions are hypotheses, not facts.

### What is on local `main` (verified this session)

#### Harness A3/A4 (the real remaining harness work)
- `feat/harness-a3a4-fix` → main `5ef8fbb78`.
- A4: dead concurrency cap → `WorkerSlots` counting semaphore (try_acquire → typed
  `DispatchError::Busy`, degrade-closed refusal; slot guard released on thread completion).
- A3: unbounded `MemStore` cache → `BoundedStore<S>` LRU (default cap 1024), needed
  `BlockStore::remove` (additive default-noop + real impls for MemStore/FileBlockStore).
- VERIFIED: llm-adapters 21 pass; kernel 866→894 pass; `cargo tree -p dowiz-kernel` = NO
  http-client (kernel stayed HTTP-free).

#### Product/CI branches landed this wave (all `--no-ff`, all gated green by the
pre-commit hook's `cargo test` + gitleaks + firewall)
- harness A3/A4, p34, p71, p79, p80, p81, p83, p88, p89, p96, p01, p75, p77,
  p72-v3, contention-bench, p91 (ML-KEM ring), p47 (payment rail), reconcile-redline,
  p06 (took main's CLOSED HybridSigner — see below).

#### Final test evidence (fresh, this session)
- kernel: **894 passed / 0 failed** (3 ignored)
- engine: **121 passed / 0 failed**
- ci-truth (tools/ci-truth): builds clean; HybridSigner = main's CLOSED variant
  (commit `58987d79d`, e2e GREEN).
- RED-line grep-gates still green: `payment_capability::red_line_no_real_provider_references`,
  `wallet::no_card_data_in_wallet`, kernel firewall (no http-client).

### Conflicts resolved this wave (root-cause, not assumptions)
- `kernel/Cargo.toml` `[[bench]]` block: p80's 8-entry expansion + contention's
  `[[bench]] name="contention"` — both additive, merged (kept p80's full block + appended
  contention entry).
- `kernel/benches/criterion.rs`: p77's `bench_spool_drain`/`bench_spine_build` + p89's
  `bench_field_eigen` — both registered in `criterion_group!` (no drop). Repaired a
  merge-induced dropped `spine` import.
- `kernel/src/lib.rs` (p72-v3): kept HEAD's real `pub mod` decls (wallet/hub_provisioning/
  span_metrics/hub_supervisor/landing) that p72-v3 predated; dropped the empty branch side.
- `kernel/src/ports/customer.rs`: removed a merge-duplicated `use crate::vendor::VendorId;`,
  restored `use crate::rng::Rng;` (used 12× in file).
- `.gitignore` (reconcile-branch): kept both sides' additions (`.worktrees/` + ci-truth
  v1-sigverify telemetry jsonl).

### p06 decision (explicit)
`feat/p06-v1-real-signer` was NOT merged wholesale: it carried 81 conflict hunks against
main's already-CLOSED `HybridSigner` (commit `58987d79d`, independent 3-model-verified
GREEN). Its only genuine delta vs main was native signing telemetry (SIG_TAG_K/V,
`record_telemetry`, JSONL sink). To avoid destabilizing the verified-green signing gate,
the merge took main's CLOSED signer (HEAD) for `ci-truth/src/{main,v1}.rs`. **p06's
telemetry delta is DEFERRED** — re-port it as an additive module once desired, don't
re-litigate the signer.

### Unmerged-branch final audit (2026-07-19, definitive)
Re-checked every unmerged branch with `git log main..<branch>` (NOT `main...branch`, which
falsely inflates via main's later files):
- **22 branches fully contained in main** (main..branch empty) → redundant junk. Safe to delete:
  all `recover/*` (except the 2 stash-* below), `*-snapshot-*`, `pq-crypto-tier1`, `kalman-organ`,
  `markov-attractor-signal`, `agent-capability-boost`, `decentralized-pq-protocol`,
  `remove-legacy-thin-layer`, `rw-02/03`.
- **`docs-research-2026-07-19`** (9 commits) → PROVEN REDUNDANT: merge brought only 3 conflicted
  files (README.md + Q-SERIES + SYNTHESIS blueprints); all 3 were the branch's STALE versions that
  main already superseded via the P75–P96 waves. `git diff HEAD` after resolving = empty → no unique
  content. Aborted (no empty merge).
- **`recover/stash-1-2994e6c8`** (77 commits) + **`recover/stash-2-93919edd`** (40 commits) → unique
  commits exist, BUT they are `git stash` recovery branches from the SUPERSEDED `feat/sovereign-core-
  phase-zero` arc (dated 2026-07-06/07). Operator rule: NEVER auto-merge `recover/*`. Left unmerged;
  operator decision.

**Conclusion: zero actionable product/CI branches remain unmerged. Core roadmap is COMPLETE on
`origin/main` (d8004a3c7). The only not-done items are operator-EXTERNAL: GitHub public-flip (P18),
secrets-scrub, bebop frozen-lane (C3/P85) in the bebop repo, and cleanup of the 24 redundant local
branches (deletion is operator hygiene, not required for roadmap completion).**

### Dashboard
- Local `main` HEAD after wave: `5a97e1f6f` (p06 merge).
- Kernel 894 / engine 121 / ci-truth green. 0 failures.
- Push to `origin/main`: authorized, executed per operator word.

# Part V — Space-Grade Kernel Execution Roadmap, 78 Items (verbatim, 2026-07-19, reconciled 2026-07-20)

> Full content of the former `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md`, inlined here
> 2026-07-20; the standalone file has been deleted. The ~50 individual `BLUEPRINT-ITEM-*.md` files
> this doc links to per-item remain separate — too many individual files to also inline, and they
> are genuine detail documents, not competing roadmap narratives. **Read the reconciliation table
> immediately below FIRST** — it corrects ~40 items whose status changed on 2026-07-20, after this
> doc's own prose was last touched.

> **2026-07-20 status reconciliation (added by the roadmap merge, per operator request to prune
> staleness rather than just inline text as-is).** The item-by-item prose below carries ✅ markers
> current only through 2026-07-19 — the autopilot execution wave that followed on 2026-07-20 (see
> `SPACE-GRADE-VERIFIED-STATUS-LEDGER-2026-07-20.md`, kept standalone as the evidentiary ledger)
> closed roughly 40 more items that the prose text below does not yet reflect inline. Rather than
> hand-edit ~40 deep-buried paragraphs (risking transcription drift in dense technical text), this
> table is the single accurate status source for every item as of 2026-07-20 — read it FIRST, then
> the prose below for design rationale. Items not listed here (1,3,4,13,15–19,25,29,30,73–78) are
> unaffected by this reconciliation: their own inline markers (or D11's spec-only status for
> 73–78) already reflect current truth.

| Item | Doc's own inline marker (as written below) | True status, 2026-07-20 | Evidence |
|---|---|---|---|
| 2 | (verification note, no ✅) | DOC/CI-ONLY — closed | `cargo test -p dowiz-kernel file_store` green |
| 5 | ✅ DONE 2026-07-19 | unchanged, still current | — |
| 6 | ✅ DONE 2026-07-19 | unchanged, still current | — |
| 7 | ✅ EXECUTED 2026-07-19 | unchanged, still current (hardening-gate + kani-gate live) | — |
| 8 | (no marker below) | **DONE-VERIFIED** | Kani `proof_gcra_transition_contract`+`proof_gcra_two_step_interleaving`, 0 failures; commit `f84049c61` |
| 9 | (no marker below) | **DONE-VERIFIED** | `breaker::`, 34 pass; exhaustive trip/close/half-open |
| 10 | (spec-only in prose) | DOC/CI-ONLY, correctly design-only | TLA+ FSM — `.tla` artifacts + tlc-gate job shape, no kernel code by design |
| 11 | (spec-only in prose) | DOC/CI-ONLY, correctly design-only | ARINC653 scheduler — TLC model is the formal artifact |
| 12 | (spec-only in prose) | DOC/CI-ONLY pilot, gated on item 9 (now closed) | Temporal TMR fault-injection test designed, not yet landed |
| 14 | ✅ DONE 2026-07-19 | unchanged, still current | — |
| 20 | (no marker below) | **DONE-VERIFIED** | Option A std-only persistence; 6 new property tests; commit `f16d603d7`; kernel lib 1052 pass |
| 21 | (no marker below) | **DONE-VERIFIED** | `autonomic::`, 7 pass; 9-LawTable exhaustive |
| 22 | (no marker below) | **DONE-VERIFIED** | `mesh::` (pq), 8 pass; ML-DSA-65 signed hash chain |
| 23 | (no marker below) | **DONE-VERIFIED** | `mesh::` (pq), 8 pass; gossip/import adversarial, 6 tests green |
| 24 | (no marker below) | **DONE-VERIFIED** | `mesh::` (pq), 8 pass; KAT-gated over `pq::dsa` |
| 26 | ✅ superseded note added this merge | measurement CLOSED 07-19, **batching itself DONE-VERIFIED 07-20** | `hydra::`, 25 pass; 53× durability throughput, opt-in; commit `85022e49d` |
| 27 | classifier-input half ✅ DONE 2026-07-19 | classifier-input half unchanged; **the NEW-BUILD sibling half also DONE-VERIFIED** | commit `07057e2ee`; independent re-check 1 passed |
| 28 | (spec/ruling only in prose) | **DONE-VERIFIED** | Phase A doc + `optical.rs` behind `optical` feature; re-check 3 passed; zero-dep gate holds |
| 31 | (audit-only in prose) | **DONE-VERIFIED** | cargo-deny wired `ci.yml:245`; zero-dep-gate job exists |
| 32 | (no marker below) | **DONE-VERIFIED** | eqc IR extension, item-18 precedent; scalar `Expr` confirmed |
| 33 | (no marker below) | **DONE-VERIFIED** | `ITEM-33-RECONCILIATION.md`, real `cargo bench` evidence; 0/5 prior claims confirmed (refuted as noise/unsourced) |
| 34–44 | (spec-only in prose) | **IN-PROGRESS** — 35+36+38 DONE (38's SIGSEGV root-caused+fixed, full suite 1069 green); 34/37/39–44 re-dispatched | worktree `exec/toy-pilot-arc` |
| 45 | (spec, this merge's own correction below applies) | DOC/CI-ONLY — compile-time law recorded; **CI job not yet wired**, per this merge's own §-note below | unchanged from this session's earlier finding |
| 46 | (spec-only in prose) | **DONE-VERIFIED** | Determinism goldens already at HEAD; inventory doc commit `0a3dfa05e`; re-check 13 passed |
| 47 | (spec-only in prose) | **IN-PROGRESS**, after 35/42 | worktree `exec/toy-pilot-arc` |
| 48 | (no marker below) | **DONE-VERIFIED** | `fdr::`, 26 pass; kill9/panic/hang children recovered |
| 50 | (no marker below) | **DONE-VERIFIED** | `fdr::schema`, 6 pass; Refuted/Undecidable pinned |
| 51 | (spec-only in prose) | **DONE-VERIFIED** | commit `be1b985c1`; re-check 5 passed |
| 53 | (CI-job spec in prose) | DOC/CI-ONLY — closed, promotable to required | lint-gate config-only |
| 54 | (no marker below) | **DONE-VERIFIED** | `ports::agent::sentinel`, 9 pass; RevocationSet deny-closed |
| 55 | (no marker below) | **DONE-VERIFIED** | `spectral::`, 33 pass; K3 verdict class retrofit |
| 56 | (no marker below) | **DONE-VERIFIED** | `spectral::`, 33 pass; `DriftBasis` recorded, grep-proven off decision path |
| 57–58 | (spec-only in prose) | **DONE-VERIFIED** | commits `8765757ee`+`912e13af1`; telemetry re-check 1 passed |
| 59 | (spec-only in prose) | **DONE-VERIFIED** | `agent::loop`, 52 pass; Instant-based, wasm-safe |
| 60 | (spec-only in prose) | **DONE-VERIFIED** | already at HEAD (`cb00706b1`); engine re-check 122 passed; `FRAME_BUDGET_US` pinned |
| 61 | (spec-only in prose) | **CORRECTED 2026-07-20 (2nd pass): NOT-BUILT, mismarked** | No dedicated runtime-counter-closure code exists beyond items 48/50/54 (already independently landed). The `fdr::` acceptance filter (min=26) is module-wide and passes only because those *other* items already push the module's test count past 26 — the filter never exercised anything specific to item 61's own claim. Ledger row and this table both need re-flagging as NEW-BUILD/GATED, not DONE-VERIFIED. |
| 62 | (spec-only in prose) | **CORRECTED 2026-07-20 (2nd pass): NOT-BUILT, mismarked** | `parent_span_id` has **zero** grep hits anywhere in `kernel/src` — the claimed relational-linkage field does not exist in code. Same module-wide-filter root cause as item 61 (`fdr::` passes on unrelated pre-existing tests). |
| 63 | (spec-only in prose) | **DONE-VERIFIED** | `agent::`, 52 pass; core-never-depends-on-AI firewall test green |
| 64 | (spec-only in prose) | **DONE-VERIFIED** | commit `7f8c23b2a5`; re-check 5 passed |
| 65 | (spec-only in prose) | **DONE-VERIFIED** | `ports::agent::cap`, 8 pass; zero direct kernel dependency |
| 66 | (spec-only in prose) | **CORRECTED 2026-07-20 (2nd pass): NOT-BUILT, mismarked** | Zero occurrence of "scrub"/bitrot/at-rest-reverification anywhere in `kernel/src` or the repo — no durable-log-scrub feature exists. The commit that flipped this to DONE-VERIFIED (`981b24378`) touches **only** `docs/audits/hardening/HOT-PATHS.tsv` and the ledger doc itself — zero kernel code. Same module-wide-filter root cause (`event_log::` passes on 13 pre-existing, unrelated tests). |
| 67–69 | (spec-only in prose) | **DONE-VERIFIED** | commits `ca7c00fe8`/`b11b42a24`/`42523e508`; cost_oracle 6 / footprint 5 passed |
| 70–71 | (spec-only in prose) | **DONE-VERIFIED** | commit `ce1a74ada`; digital_twin 8 passed |
| 72 | (spec-only in prose) | **DONE-VERIFIED** (folds under the 70–72 digital-twin close) | same worktree, `exec/cost-twin-arc` |
| 73–74 | this merge's own correction below applies | scripts real, **not CI-wired** | unchanged, this session's earlier finding stands |

**2nd-pass correction (2026-07-20, same day, independent re-verification):** items **61, 62, and
66 above were themselves wrongly marked DONE-VERIFIED** by the first reconciliation pass, because
that pass trusted `SPACE-GRADE-VERIFIED-STATUS-LEDGER-2026-07-20.md`'s own rows without
re-deriving them from code. All three share one root cause: their acceptance filters (`fdr::`,
`event_log::`) are **module-wide** rather than scoped to a test that actually exercises the new
claim, so pre-existing test counts from *other*, genuinely-landed items (48/50/54 for `fdr::`)
silently satisfy the stated minimum. `docs/design/SPACE-GRADE-VERIFIED-STATUS-LEDGER-2026-07-20.md`
carries the same error at its own item 61/62/66 rows and has NOT yet been corrected there (it is
kept standalone as a historical evidentiary record — see the correction note appended to that
file instead of a silent rewrite). The `HOT-PATHS.tsv` rows these three items registered should be
removed or re-scoped to a real per-claim test — not yet done, flagged as an open follow-up, not
actioned in this documentation-only pass.

**Reading rule going forward:** when this table and the prose below ever disagree again, the table
in the ledger it cites (or a newer dated reconciliation appended above this one) wins — that is
this whole document's own D8-style "newest wins" precedence rule, applied to itself. This 2nd-pass
correction block is itself now the newest word on items 61/62/66 — it wins over both the table
rows above it and the ledger file.

---
## Execution Roadmap — Space-Grade Kernel Synthesis, Items 1–32, Dependency-Ordered

**Source:** `docs/design/SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` (commit `10164bd74`).
**Sorting rule:** actual technical dependency, lowest first — not the document's topic order. The
two dependencies the source states explicitly — **item 21 strictly after item 9** (§16(c)) and
**item 23 after item 22** (§17(b) addendum) — are preserved verbatim and never resequenced. Every
other ordering choice below that is not explicit in the source is flagged **[new ordering choice]**
with its reason.

---

### 0. Operator rulings — recorded 2026-07-19, same day as the synthesis

All five open decision gates were presented and ruled on the same day:

| Gate | Source | Ruling |
|---|---|---|
| GCRA lock-free TokenBucket swap | §1.3 / item 8 | **ADOPT** — gated behind the differential oracle + Kani interleaving proof already scoped in item 8; built and tested before it ships. |
| Mesh integration approach | §17(d) / item 22 | **REIMPLEMENT IN DOWIZ, ZERO-DEP** — bebop's proven mesh-node/proto-wire/proto-cap serves as design reference/parity oracle only, not a linked dependency. The same ruling covers `agent-governance-wasm`'s `bebop2-core` path-dep and `mesh-adapter`'s sibling paths per §25's table. |
| Optical/pixel context compression | §20(c) / item 28 | **PURSUE** — model-weight dependencies are ruled outside §0's compiled-Rust-crate scope; archival/display-plane content only, never the P0/P1 determinism planes (§10/P6). |
| ARINC-653-style scheduler | item 11 | **PURSUE, design-only** — Phase 0 (design doc + TLC model), no code until the breaker (item 9) exists, per the source's own restriction. |
| SIHFT triple-vote pilot | item 12 | **PURSUE, design-only for now** — needs the breaker + FDR to exist first regardless of the ruling; design/scoping work can start. **Premise retro-corrected 2026-07-19 (consistency audit §§1.1–1.2, same treatment as item 54's §J correction):** the synthesis §6 valuation behind the original "optional" grading ("ECC-RAM Hetzner hosts ⇒ residual value modest") was the identical rejected cloud-ECC reasoning the operator reversed for Sentinel/item 54 — the actual target is local, offline-first, consumer-grade hardware typically WITHOUT ECC, so the compute-time SEU class is material. The ruling itself stands (design-only remains correct — the pilot needs items 9 + FDR); the design must be sized under the non-ECC premise and lands as **temporal TMR** per the item-12 re-scope in §E (audit-A finding + the OS-patterns temporal-TMR research MERGED into item 12 — one item, no new number). |
| eqc indexed-summation IR extension | item 32 | **PURSUE** — extend eqc's `Expr` language to support the Laplacian's neighbor-sum operator, not just scalar control laws. |

The Laplacian reimplement-vs-vendor fork (§14(d)) is **not a live gate** — §26(d)'s correction found
`laplacian_spmv` already exists in-kernel at `csr.rs:552`; the surviving work is the parity pin
(item 18, Tier 0 below) and reconciling bebop's `step_wave` as a third representation, not a
build-vs-vendor choice. The §27 frequency/wave-domain communication idea remains parked — no
research has been done, and the source document itself requires a research pass before its own
gate applies.

---

### A. Tier 0 — zero prerequisites, read-only or self-contained on already-tested surfaces. READY NOW.

Nothing here depends on any other item; each is pure investigation or a small change under existing
test coverage.

- **Item 2** — `FileEventStore` wiring verification. Dual check: (a) is the durable store constructed
  anywhere, (b) has the `Result`-typed `insert` fix landed since 07-16 (§10/P4 — "a wired store that
  swallows IO is arguably worse than an unwired one"). Highest consequence-per-cost in the roadmap.
  **✅ RESOLVED-AS-DEFECT-FILED 2026-07-19** (re-verified adversarially against live `HEAD`): **(b) PASSES**
  (typed `StoreError` propagation confirmed, fix `4dec04218`, regression test `hydra.rs:1188-1218`);
  **(a) FAILS** — no production composition root constructs the durable store (all 6 `FileEventStore::open`
  sites are test-only; no binary builds a durable `Hydra`/`EventLog`). Defect filed:
  [`BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md`](BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md)
  (verification: [`BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md`](BLUEPRINT-ITEM-02-file-event-store-verification-2026-07-19.md)).
  Fix scoped there as a follow-up Tier-1 build item (§B territory) — NOT built (Tier-0 read-only audit).
- **Item 3** — `order_machine` const-adjacency + `idx_of` dedup. Golden signature and 1e-12 oracle
  already cover 2 of the 3 stated proof clauses (verified live 2026-07-20: one `idx_of` definition,
  golden-signature + 1e-12-oracle tests both green). **The third clause — "zero heap allocations
  under a counting allocator test" — was never actually built** (zero `count-allocs`/
  `counting_alloc` references in `order_machine.rs`, confirmed by grep); real blueprint now exists:
  [`BLUEPRINT-ITEM-03-order-machine-zero-alloc-proof-2026-07-20.md`](BLUEPRINT-ITEM-03-order-machine-zero-alloc-proof-2026-07-20.md).
- **Item 30** — state-machine proliferation audit (`capability_cert.rs`, `hub_provisioning.rs`,
  `hub_supervisor.rs`, `hydra.rs`). Read-only table. **✅ CLOSED 2026-07-19** —
  [`AUDIT-ITEM-30-state-machine-final-2026-07-19.md`](AUDIT-ITEM-30-state-machine-final-2026-07-19.md)
  (+ [`BLUEPRINT-ITEM-30-state-machine-audit-2026-07-19.md`](BLUEPRINT-ITEM-30-state-machine-audit-2026-07-19.md),
  previously unlinked from this citation — found + fixed 2026-07-20): all 4 modules INDEPENDENT (0
  shared with the FSM proof kit), 4 PARITY-PIN tickets (I30-T1..T4, 0 collapses forced). **1
  confirmed silent defect** (I30-D1, `resume()` owner-zeroing) fixed with a red→green guard on
  `exec/space-grade-tier0-2026-07-19` (`707848dfd`, independently re-verified 2026-07-20 as a real
  ancestor of `main` matching this claim); the in-session "2 confirmed silent defects" phrase
  confirmed UNSOURCED.
- **Item 15** — eigen-surface entry-point + parity-scope verification. Read-only; defect filed only
  if found. **✅ AUDITED 2026-07-19** — single eigen-surface HOLDS (`spectral.rs:225 eigenvalues` →
  `householder::eigenvalues_contig`, no `lowrank.rs`); gap = R3 parity is values + dominant-residual
  only (`spectral.rs:1254 let _ = dvecs;`). Ticket **I15-T1** (vector-scope cross-solver pin) filed
  in [`AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md`](AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md);
  not built (new scope).
- **Item 16** — `GraphSpectrum` single-spectrum audit. Read-only unless a P2 defect forces collapse.
  **✅ RESOLVED-BY-REFACTOR 2026-07-19** — P2 CONFIRMED (`graph_spectrum` computed the adjacency
  spectrum 3×, `graph_energy_report` 4×, both claiming "single pass"). Collapse LANDED, option (b)
  (internal, zero public-signature change): `classify_drift_with_rho` + shared `drift_guards_ok`/
  `drift_band`; `graph_spectrum` now = exactly 2 passes (adj + Laplacian), `graph_energy_report`
  4→2. Proof = thread-local `EIGEN_CALLS` exactly-2 counter + field-consistency test. Kernel suite
  **902 / 0 / 3** (was 899). Committed `e125f0c97`, pushed to `exec/space-grade-tier0-2026-07-19`.
  Resolution note: `AUDIT-ITEMS-15-17-19-followup-tickets-2026-07-19.md` §0.
- **Item 17** — `engine` thick/thin classification table (RC-4's three mirrored items as first
  entries). **✅ AUDITED 2026-07-19** — RC-4 triple: `DriftClass` + `dt` CLOSED (pinned post-H2);
  **L-operator OPEN** (`engine/src/field_frame.rs:10-40` engine-side 5-point Neumann Laplacian
  unpinned to kernel `csr.rs:552 laplacian_spmv`). Ticket **I17-T1** (engine-boundary Laplacian pin;
  cross-references item 18's intra-kernel pin, does NOT duplicate it) filed in the tickets doc; not
  built (new scope).
- **Item 19** — retrieval spectral-routing audit (`diffusion.rs`/`ppr.rs`). Read-only.
  **✅ AUDITED 2026-07-19** — independent-by-design (zero `spectral`/`GraphSpectrum` refs;
  `ppr.rs:6-7` "No eigendecomposition"), correctly so — NOT the second GraphSpectrum consumer. New
  smell: `ppr.rs:3-5` is a comment-bound unpinned mirror of `markov.rs:162-170`'s inner loop, no
  test pin. Ticket **I19-T1** = parity-pin (NOT collapse — `retrieval/mod.rs:14` red-lines touching
  `markov.rs`) filed in the tickets doc; not built (new scope).
- **Item 22 (verification half only)** — read `mesh.rs`, classify real-port vs stub. The ruling is
  now recorded (§0 above: reimplement), so this verification informs HOW MUCH of `mesh.rs` is
  reusable scaffolding versus needs building from scratch, not whether to proceed.
  **✅ VERIFICATION COMPLETE 2026-07-19** — proof filed
  [`AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`](AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md)
  (classification table, one row per public symbol, file:line + caller-or-NONE + verdict; blueprint
  independently re-verified and CONFIRMED). Finding: `mesh.rs` (387 lines, `#[cfg(feature="pq")]`,
  `pq` NOT default) is a real, tested ML-DSA-65 signed-log primitive with **ZERO production kernel
  callers** — `MeshLog`/`MlDsaSigner`/`Signer` bench-only, `SignedEntry`/`MeshError`/`HubTransport`
  uncalled, protocol layer absent; even `mesh-adapter` bypasses it (bebop path-deps, no `pq`).
  **Scoping handoff to item 23 (gated strictly after — NOT started here):** it is **"mostly stub
  above the log layer"** — reuse `SignedEntry`/`MeshLog`/`MlDsaSigner` + the `HubTransport` seam
  as-is (keep, don't rewrite), but sync/consensus/capability/gossip start near-scratch; gossip
  admission must extend `decision/import_unit()`, never fork a parallel importer (synthesis §17(b)).
- **Item 18 (narrowed)** — the Laplacian parity pin: dense `laplacian()` ↔ `csr.rs:552
  laplacian_spmv`, plus a `step_wave` reconciliation note. **[new ordering choice]** — promoted from
  mid-roadmap to Tier 0 because §26(d) shrank it to one parity test against an oracle already in-tree.
- **Item 31 (investigative half)** — `rusqlite` usage read + reclassification; pin the
  `cosmic-text = "*"` wildcard; verify `sha2`-vs-kernel-keccak on the body digest.
  **✅ INVESTIGATIVE HALF COMPLETE 2026-07-19** — findings filed
  [`AUDIT-ITEM-31-dependency-findings-2026-07-19.md`](AUDIT-ITEM-31-dependency-findings-2026-07-19.md)
  (blueprint independently re-verified). **(a) rusqlite** → KEEP-and-contain (cat-2 foreign format,
  Hermes `state.db` only, no default build path) — docs-only ruling. **(b) cosmic-text `*`** →
  DEFECT CONFIRMED + FIXED: pinned to already-resolved `0.19.0` (`engine/Cargo.toml:30`), lockfile
  unchanged, `cargo check --features text` green, committed `c2d0f306a` on
  `exec/space-grade-tier0-2026-07-19`. **(c) sha2 vs keccak** → NOT a defect, KEEP `sha2`; blueprint
  CORRECTED (`pub mod pq` is `#[cfg(feature="pq")]`-gated, not "already linked" — the swap would
  pull `aes-gcm`+`curve25519-dalek`, a net dep increase). Bonus flag confirmed: **dual in-kernel
  Keccak-f[1600]** (`event_log.rs:67` vs `pq/keccak.rs:156`) — dedup ticket owed to item 25, filed
  not fixed. Enactment half (Tier 2) allowlists rusqlite+sha2.

### B. Tier 1 — foundational builds. ✅ COMPLETE (2026-07-19) — all items DONE; the kernel's default no-dev build has ZERO external crates.

- **Items 1 + 13 combined** — the CI zero-dep gate, born deterministic:
  `cargo tree -e no-dev --locked --offline` + lockfile-hash assertion, 3-crate allowlist shrinking
  monotonically. **[new ordering choice — bundling]**: item 13 hardens item 1's own mechanism;
  building it nondeterministic first is two passes over one CI job.
  See [`BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md`](BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md)
  (previously cited only as an internal "§G.7" cross-reference, never linked — found + fixed
  2026-07-20).
  **✅ DONE (2026-07-19)** — `kernel/ZERO-DEP-ALLOWLIST.txt` + `scripts/zero-dep-gate.sh` (3 gates:
  tree⊆allowlist, monotonic-shrink, `Cargo.lock` sha256) + `zero-dep-gate` CI job under `unshare -n`;
  all §G.7 clauses red-proven; `01acd673e` on `exec/space-grade-tier0-2026-07-19`. See §G.7 for detail.
- **Item 14** — `rust-toolchain.toml` pin + structural compiler-bump trigger. Independent, parallel.
  See [`BLUEPRINT-ITEM-14-toolchain-pin-2026-07-19.md`](BLUEPRINT-ITEM-14-toolchain-pin-2026-07-19.md)
  (previously unlinked from this bullet, though `rust-toolchain.toml`'s own header comment already
  cites it — found + fixed 2026-07-20).
  **✅ DONE 2026-07-19** (commit `bb1e9e8dc`, `exec/space-grade-tier0-2026-07-19`) — root
  `rust-toolchain.toml` pins `channel="1.96.1"` (exact, verified = dev-box toolchain; no pin existed
  pre-change, CI floated on runner stable); `toolchain-bump-gate` job added to `ci.yml` (always-runs,
  required-check safe, enforcement fires only on a `channel`-value change and then requires
  `docs/audits/toolchain/spot-check-<new>.md` w/ both mandated headings in the same diff — pin's own
  intro = `<absent>→1.96.1`, so it carries the baseline `spot-check-1.96.1.md`). Baseline artifact is
  HONEST: real source-level constant-time audit of all 6 pq surfaces (flags the pre-existing,
  compiler-independent variable-time `!=` FO tag-compares in `kem.rs`/`hybrid.rs`, owed to P91.2),
  assembly audit PARTIAL with the full per-branch taint proof DEFERRED to Tier 2 item 7 (Kani) — no
  fabricated clean claim. Proofs: kernel `cargo test` 902/0/3, engine 117/0, gate logic 6/6 +
  end-to-end `git show BASE:$FILE` extraction test (maps 1:1 onto §G.8). Owed (G5): flip the gate to
  a required status check in branch protection (server-side).
- **Item 25 (procedure doc first) — ✅ DONE (2026-07-19).** The slot-arena/qrng standing procedure
  is codified and independently re-verified in
  [`PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md`](PROCEDURE-DEPENDENCY-REPLACEMENT-STANDING-2026-07-19.md)
  — all citations checked against live HEAD, both precedents (slot-arena override, QRNG
  never-replace) confirmed. **This doc is now BINDING**: Items 4+29 (`tracing`/`tracing-subscriber`
  logger/FDR rewrite, incl. the `telemetry` `SpanMetricsLayer` consumer) and Item 5 (`regex`
  retirement) MUST run its numbered 10-step ruling per crate before cutover (§18(a)'s "under this
  exact procedure, not a bespoke one"). **Re-verification finding (new scope, owed ticket, not an
  item-25 fix):** the `qrng` feature is undeclared in `kernel/Cargo.toml`, so the QRNG provider
  (incl. its sanctioned `master_seed()`) is dead code that never compiles — a
  standing-rule-vs-reality inconsistency filed in the procedure doc §3.
- **Items 4 + 29 combined, with the §1.2 `JsonWriter` absorbed in the same change — ✅ DONE
  (2026-07-19).** The hand-rolled logger/FDR tier-(b) buffer with the energy/hardware field set
  first-class in the schema from day one. Both bundlings are the source document's own explicit
  mandate (§21, §10/P2). The largest Tier-1 item; the keystone of the tier. Landed as three isolated
  commits on `exec/space-grade-tier0-2026-07-19` (`f04142f89` build → `4f4872a54` flip →
  `eb350464e` remove): `kernel/src/fdr/` (json/schema/ring/macros/mod) coexisted, then the 13 call
  sites + `SpanMetricsLayer`→`SpanMetricsObserver` (a kernel `fdr::SpanObserver`) flipped, then
  `tracing`/`tracing-subscriber` removed. Proofs discharged: `cargo tree -e no-dev` 25→6 crates (**19
  dropped**, exceeds ≥13); `metric.jsonl` + markov CLI JSON byte-identical before/after (golden-pinned);
  kill-9→restart→recover test (real child SIGKILLed, 300/300 events recovered + PostMortem emitted);
  `hw` first-class with `joules_uj` reporting `unavailable:no_rapl_interface` (named absence) on this
  RAPL-less host; duplicate `mldsa_verify` wrapper deduped; wasm32 cdylib green (`Instant` gated off
  wasm); full kernel suite 938 passed / 0 failed; `scripts/zero-dep-gate.sh` GREEN (5 external crates,
  allowlist shrunk by 19). Ruling recorded in `fdr/mod.rs` doc + `kernel/Cargo.toml` + the blueprint
  ([`BLUEPRINT-ITEMS-04-29-logger-fdr-rewrite-2026-07-19.md`](BLUEPRINT-ITEMS-04-29-logger-fdr-rewrite-2026-07-19.md)).
- **Item 5 — retire `regex`, after the logger exists. ✅ DONE (2026-07-19) — CLOSES ALL OF TIER 1.**
  The kernel's last external crate. Its entire production surface was one function
  (`TrigramIndex::query_regex`) with **zero production callers** (re-verified by full-workspace
  grep across `kernel/ engine/ apps/ tools/ agent-loop/ agent-adapters/`); the only pattern ever
  compiled anywhere was `note-.*-recall`. Ruling per item 25's procedure = terminal state (a)
  removed outright, replaced by a kernel-owned restricted matcher for the used subset
  ({literal, `.`, `.*`}, unanchored contains-match, greedy leftmost segment placement — no
  backtracking exists ⇒ no pathological blowup), with typed rejection (`PatternError::UnsupportedMeta`)
  of every other metacharacter (degrade-closed). Landed as three isolated commits on
  `exec/space-grade-tier0-2026-07-19` (`18152ef84` build → `c6b5d2176` flip → `6605166cd` remove):
  `kernel/src/retrieval/pattern.rs` + `query_pattern` coexisted, then the seam flipped, then
  `regex = "1"` was removed. Proofs discharged: parity proven BEFORE cutover — differential vs the
  live `regex` crate over the 20-doc FIXTURE + 2000-doc synthetic corpus + a proptest sweep (random
  subset patterns × ASCII docs), all bit-identical; a permanent independent naive recursive
  reference matcher + a frozen golden (`query_pattern("note-.*-recall") == vec![7]`) carry the
  guarantee post-removal; rejection tests assert typed errors with byte positions.
  `cargo tree --manifest-path kernel/Cargo.toml -e no-dev --locked --offline` = **`dowiz-kernel`
  root ONLY, ZERO external crates** (regex's whole subtree — regex, regex-automata, regex-syntax,
  aho-corasick, memchr — dropped; regex survives only as a `criterion` dev-dep transitive in
  `Cargo.lock`, outside the no-dev proof surface). `ZERO-DEP-ALLOWLIST.txt` shrunk 5 → 0;
  `scripts/zero-dep-gate.sh` GREEN "0 external crates" (also fixed a latent gate abort at the true
  zero-dep end state — its filter greps returned exit 1 when they filtered every line out, aborting
  under `set -euo pipefail`; now `|| true`-guarded, gate A/B/C semantics unchanged). Full kernel
  suite green (925 lib unit tests / 0 failed / 3 ignored, +22 integration). Ruling recorded in
  `pattern.rs` module doc + `kernel/Cargo.toml` tombstone + allowlist header + `fdr/mod.rs` +
  `lib.rs`/`retrieval/mod.rs`, and the blueprint
  ([`BLUEPRINT-ITEM-05-regex-retirement-2026-07-19.md`](BLUEPRINT-ITEM-05-regex-retirement-2026-07-19.md)).
  **With this, every §B Tier-1 item (1+13, 14, 25, 4+29, 5) is DONE: the kernel's default build has
  genuinely zero external dependencies.**

### C. Tier 2 — process/verification layer. Parallelizable.

- **Item 6** — §4 hardening checklist codified + CI enforcement, with §10/P7's correction built in:
  CI must re-execute oracles and dudect self-tests, never presence-check artifacts.
  See [`BLUEPRINT-ITEM-06-hardening-checklist-ci-2026-07-19.md`](BLUEPRINT-ITEM-06-hardening-checklist-ci-2026-07-19.md)
  (previously unlinked from this bullet — found + fixed 2026-07-20).
  **✅ DONE 2026-07-19** (`ae4964e61`, branch `exec/space-grade-tier0-2026-07-19`). Real CI config +
  a new dudect harness landed. Three deliverables: `docs/audits/hardening/CHECKLIST.md` (standing
  law), `docs/audits/hardening/HOT-PATHS.tsv` (machine-read manifest — 14 rows seeded from the real
  surfaces: pq/dsa+kat, pq/keccak, event_log Keccak-copy-B, pq/x25519, pq/kem, pq/hybrid,
  order_machine FSM, householder+spectral eigen, token_bucket, retrieval/pattern, fdr/json, ct_gate),
  and the `hardening-gate` CI job (`scripts/hardening-gate.sh`). The gate **re-executes, never
  presence-checks** (§10/P7): every verdict is a live `cargo test` exit code + the PARSED `N passed`
  count asserted `>= min_tests`; a filter matching **zero** tests is RED (anti-forgery core). **RED/
  RED/GREEN proven with real output:** (a) a diff touching a hot ZONE with no manifest row → exit 1;
  (b) a manifest row whose filter matches zero tests → exit 1; (c) my own commit's diff (touching 3
  registered rows) → exit 0. **Independent-verification CORRECTION to the blueprint's premise:** the
  cited pq KATs (ACVP/Keccak/x25519/KEM/hybrid) do **NOT** re-execute in the default `cargo-test` job
  — `pq` is not a default feature, so `cargo test --offline` never compiles them; they were **dark in
  CI**. The gate's unconditional oracle floor now runs them with `--features pq` every build, closing
  that gap. **dudect (honest gap — built):** `kernel/src/ct_gate.rs`, a zero-dep Welch-t harness + a
  reusable `ct_eq` constant-time primitive + a **planted-leak self-test** (variable-time `naive_eq`
  detected at |t|≈300+, `ct_eq` |t|<1.3, separation >290×) run in release in the gate step. **item 3
  (debug_assert differential):** wired for `order_machine::assert_transition` (slice-vs-`FSM_ADJ`
  dual-representation) and `householder::eig2x2` (Vieta trace/det) as the pattern; corpus-oracle rows
  carry `N/A(corpus-oracle)`. **Scoped vs deferred (ledgered in the manifest's own `gap` column):**
  dudect crypto-surface coverage → items 7/8; `kem.rs`/`hybrid.rs` variable-time tag compares are
  `KNOWN-RED(P91.2)` (NOT fixed here — the CT fix is the gate's first customer); `token_bucket` GCRA
  differential oracle → item 8; item-4 exhaustive assembly → item 7 (Kani). Full kernel suite
  **955/0/8** at the commit. Docs (this roadmap + CORE-ROADMAP-INDEX) pushed to `origin/main`.
- **Item 7** — verification wiring for Keccak, FSM graph algorithms, NTT arithmetic, GCRA
  transition (now applies to the adopted GCRA, §0 above). **RESCOPED 2026-07-19 (blueprint v2,
  authority: `RESEARCH-NATIVE-KANI-REPLACEMENT-FEASIBILITY-2026-07-19.md` under item 25's binding
  procedure):** 16/22 harnesses land as **native exhaustive `#[test]`s** in the existing
  `csr.rs`/`order_machine.rs` idiom (identical all-inputs guarantee, zero new tooling, riding item
  6's `hardening-gate` rows); **Kani narrows to 4 harnesses now** (`montgomery_reduce`, `ntt`,
  `invntt`, Keccak cross-copy equivalence — the last dissolving entirely if the owed dual-Keccak
  dedup ticket lands first) **+ 2 GCRA harnesses deferred to item 8**; 0/22 need a hand-rolled SAT
  solver — Kani is CI-time tooling (item-25 terminal state (c), never linked, `cargo tree -e
  no-dev` unaffected), so "replace Kani natively" was the wrong question and target-rescoping was
  the right move. Item 7 no longer gates on Kani toolchain bootstrap: the 16 native targets land
  whole even if `cargo kani setup` fails in CI. See
  [`BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md`](BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md).
  **✅ EXECUTED 2026-07-19 (mostly done; two honest ledgered limits) — real code + CI landed** on
  `exec/space-grade-tier0-2026-07-19` (`df92f0c16` kernel proofs+native tests → `23f583b3e`
  kani-gate CI). **Kani toolchain bootstrap SUCCEEDED in the exec environment** (`cargo-kani 0.67.0`,
  its own `nightly-2025-11-21` rustc + CBMC/CaDiCaL; CI-time only, zero-dep gate mechanically
  unaffected — all harnesses `#[cfg(kani)]`, nothing added to `Cargo.toml`/`Cargo.lock`,
  `cargo tree -e no-dev` still 0). **7 Kani harnesses verified SUCCESSFUL via real `cargo kani`
  runs:** `proof_rotl_contract`, `proof_keccak_f_total`, `proof_reduce32_contract`,
  `proof_montgomery_reduce_contract` (overflow-free + range `[−Q,Q]` — Kani caught my first
  assertion's open-interval error, the boundary is inclusive), `proof_ntt_butterfly_lemma`,
  `proof_invntt_butterfly_lemma`, and the `proof_selftest_planted_overflow` planted-fault self-test
  (SUCCESSFUL only because the seeded i32 overflow IS caught; RED-path demo verified — removing
  `should_panic` → VERIFICATION FAILED). **15 native exhaustive `#[test]`s** (FSM ×4, dsa ×4, kem
  ×5, keccak ×2) all pass; full kernel suite `--features pq` **1131/0/8** (was 1116). CI: separate
  `kani-gate` job + `scripts/kani-gate.sh` + `HOT-PATHS.tsv` `mode=kani` rows + `hardening-gate.sh`
  skip-with-notice. **TWO honest limits ledgered (NOT silently dropped):** (a) the STRONG full-state
  Keccak cross-copy equivalence (2^1600) exceeded the 25-min CI budget (measured) — shipped the
  §3.1 fallback rung: a native machine-checked index-map equivalence (ρ/π tables + round constants +
  π destinations — the ONLY divergence) + `proof_rotl_contract`; the strong form is preserved in
  `kani_proofs_strong`, runnable nightly. (b) The Montgomery congruence `r·2^32 ≡ a (mod Q)` is NOT
  machine-checked (symbolic modulo over ±1.8e16 timed out >7 min in both i128 and i64 forms) — stays
  covered by ACVP KATs; the harness proves the overflow/panic fault class the synthesis §7 names.
  GCRA (2 harnesses) correctly deferred to item 8 — see item 8's inherited design requirements below.*
- **Item 8** — GCRA decision package. **Ruling: ADOPT (§0 above).** Differential oracle + Kani
  interleaving check now execute toward a real swap, not just an evidence package.
  See [`BLUEPRINT-ITEM-08-gcra-swap-2026-07-19.md`](BLUEPRINT-ITEM-08-gcra-swap-2026-07-19.md)
  (previously unlinked from this bullet — found + fixed 2026-07-20).
  **TWO DESIGN REQUIREMENTS INHERITED FROM ITEM 7 (executed 2026-07-19; authority:
  `BLUEPRINT-ITEM-07-kani-wiring-2026-07-19.md` §5, enforced via the `token_bucket.rs proof_gcra`
  `mode=kani` row in `HOT-PATHS.tsv`, `min=0` placeholder until this item lands the harness):**
  1. **The GCRA transition MUST be a pure function** —
     `fn gcra_decide(now_ns: u64, tat_ns: u64, cost_ns: u64, burst_ns: u64) -> Option<u64>`
     (returns the new TAT on grant, `None` on deny). A pure fn is Kani-provable AND
     differential-oracle-testable; the CAS-retry shell stays a thin loop around it. (This is why
     the bench-local `GcraBucket`'s inlined f64 decision does not qualify as-is.)
  2. **Integer nanoseconds, NOT f64, inside the transition** — the bench version computes
     `limit = now as f64 + burst_nanos` and compares `new_tat as f64 > limit`; f64 in the decision
     path is BOTH a CBMC cost-cliff AND a rounding-determinism hazard at large `now`. Any f64→u64
     conversion happens ONCE at construction, never in the hot decision.
  Item 7 also pre-specified the two harnesses item 8 must land (blueprint §5):
  `proof_gcra_transition_contract` (single-step no-over-grant: `new_tat = max(tat,now)+cost` on
  grant, `deny ⇔ max(tat,now)+cost > now+burst`, no overflow under the headroom assumes) and
  `proof_gcra_two_step_interleaving` (two sequential applications conserve `cost₁+cost₂`, TAT
  monotone — the strongest interleaving statement Kani can honestly make; the full concurrency
  argument is item 8's differential oracle + the `compare_exchange` semantics, NOT Kani). When
  item 8 adds these harnesses, bump the `proof_gcra` row's `min` from 0 to 2.
- **Item 31 (enactment half)** — per-crate allowlist CI gate + shared kernel-side JSON-parse
  primitive for the serde carriers + manifest-recorded rulings. Depends on items 1 and 25.
  See [`BLUEPRINT-ITEM-31-enactment-per-crate-gate-2026-07-19.md`](BLUEPRINT-ITEM-31-enactment-per-crate-gate-2026-07-19.md)
  (its sibling investigative half correctly links `AUDIT-ITEM-31-dependency-findings...`; this half
  was unlinked — found + fixed 2026-07-20).
  **✅ DONE 2026-07-19 — real CI config + kernel module landed** on `exec/space-grade-tier0-2026-07-19`
  (`ae2da4a9d` gate → `dd6876a73` json+oracle → `c64ca923b` cutover). **Four blueprint claims
  independently re-verified, TWO corrected:**
  - **Workspace = 26 crates** (not the synthesis's 20; the six `tools/telemetry/*` were missed) —
    confirmed. **12 already zero-external-dep** by default.
  - **Gate**: `scripts/zero-dep-gate.sh` parametrized `[<crate-dir>]` (no-arg = kernel,
    backward-compatible); path-dep filter generalized to `grep -v ' (/'` (verified against real
    `cargo tree --prefix none` — root + every path dep render with an abs path in parens). Added
    `scripts/zero-dep-crates.txt` (24-crate roster) + `<crate>/ZERO-DEP-ALLOWLIST.txt` × 25 (12 empty
    floors, 13 frozen closures with item-25 ruling headers). CI `zero-dep-gate` job loops the roster
    under one `unshare -n`; **mesh-adapter** gate rides its existing dual-checkout job (relative bebop
    path); **agent-governance-wasm EXCLUDED** (absolute-path `/root/bebop-repo` dep — CI-unresolvable,
    filed as its own portability defect). **Proof**: full roster GREEN 24/24 (5×); Gate A RED on an
    injected unlisted dep (`cfg-if`→`tools/eqc-rs`), GREEN on revert; Gate C lockfile-hash stable.
    (Also regenerated 10 downstream `Cargo.lock`, removals-only — pruning the regex/tracing closure the
    kernel dropped in items 4/5/29 so `--locked` resolves.) A subtle CI-poison bug was root-caused +
    fixed: any FAILING `git origin/main:<untracked-path>` access corrupts cargo's next `rustc -` target
    probe in a shared `.git`; Gate B now probes with a no-pathspec `git ls-tree`.
  - **Serde carriers = NINE** (not seven: + `rust-spool`, + `topics`) — confirmed.
  - **JSON primitive — HONEST SCOPE-DOWN**: built `kernel::json` (always-compiled, pure-std, bounded
    recursive-descent RFC 8259 parser + serializer, degrade-closed), SEPARATE from `fdr::json`
    (serialize-only). `serde_json` kept as a **dev-dep differential oracle** (outside the `-e no-dev`
    surface → kernel allowlist stays empty). Oracle: 50-item real-carrier corpus (all 50 agree, 31
    accept / 19 reject, 31 round-trip) + a 2000-case proptest fuzz over the carriers' real number/
    string/nesting distribution. **Phase-A cutover of the carriers that BOTH shrink the tree AND are
    a sound cutover: `agent-facade` (11→0 ext deps) + `skillspector-rs` (15→5).** Serde carriers
    **9 → 7** (a real decrease). **Correction to the blueprint's projected 3rd (wasm)**: verified NOT
    Phase-A — its `a11y_build_mirror` site (de)serializes the SHARED `dowiz-engine` `SemanticScene`/
    `A11yTree` through engine's `serde` feature; cutting it would couple `kernel::json` to engine's
    schema. **wasm deferred to Phase-B**, reopening trigger: engine exposes a serde-free codec.
    `native-spa-server`/`llm-adapters`/`async-spool` deferred — **verified via `cargo tree -i
    serde_json` that removing the direct dep shrinks NOTHING** (axum's default `json` + ureq's `json`
    feature retain `serde_json`); reopen only if those framework json features go optional.
  - **`rust-spool` deletion — DEFERRED (corrects the blueprint's "referenced by nothing")**:
    independent grep found `tools/telemetry/lib.sh:37` hardcodes + `tg_spool_ensure` LAUNCHES
    `rust-spool/target/release/telemetry-spool` as the LIVE Telegram telemetry drainer. Deleting it
    would break the live pipeline. Retire only after `async-spool` is deployed + `lib.sh` cut over.
  - **Dedup ticket owed — `kernel::json` vs `fdr::json` (filed 2026-07-19, consistency audit §3.2;
    same format as the dual-Keccak ticket in §A item 31):** the honest scope-down above left the
    kernel carrying TWO JSON-write/string-escaping surfaces (`kernel::json` parser+serializer,
    `fdr::json` serialize-only) — the exact §10/P2 "second escaping primitive" failure shape the
    synthesis itself named. BP-31E acknowledged it only parenthetically and added a round-trip
    test, but unlike the dual-Keccak case no dedup-or-parity ticket was recorded. Ticket: either
    consolidate `fdr::json`'s writer under `kernel::json::write`, or record a permanent escaper
    parity pin + a one-escaper-implementation rule; verify escaper sharing in the exec-branch code
    when it merges. Owed to item 25's ledger; filed, not fixed.
- **Item 26** — batching research pass. Zero prerequisites; scheduled low-priority, measurement-only.
  **✅ DONE 2026-07-19** — real measurements landed:
  [`AUDIT-ITEM-26-batching-measurements-2026-07-19.md`](AUDIT-ITEM-26-batching-measurements-2026-07-19.md).
  Inventory re-verified (all §1 citations accurate). M1 event-log commit: p50 637 µs / p99 1343 µs /
  1,513 ev/s, **exactly 1 fsync+open+close per event** (strace) — group-commit worth **~53×** at
  batch-64 but changes the crash contract ⇒ *operator-gated opt-in, not a default*. M2 FDR ring:
  normal 2.56 µs vs alarm-fsync 571 µs (~148×); 1 MiB→4 MiB cap buys only ~11% ⇒ **KEEP AS-IS**
  (design already amortizes fsync over a segment; baseline now on record). M3 `import_unit`:
  0.87 µs p50 / ~0.6 ns-per-case marginal ⇒ **measured DON'T-BATCH**. M4 skipped per its own gate
  (allocation is noise). **PMU unavailable** (`perf_event_paranoid=4`, no `perf`) — wall-clock +
  `strace -c` fallback, no fabricated counters. No batching code landed (scope law held).
  Scaffolding (bench + `#[ignore]` probe) on `exec/space-grade-tier0-2026-07-19`.
  **UPDATE 2026-07-20 — M1 opt-in group-commit code now LANDED** (`85022e49d`, `main`):
  `FileEventStore::with_batch_size(n)` / `flush_pending()` / `DurabilityCounters::pending_unsynced`.
  Default `batch_size = 1` is byte-for-byte the pre-existing per-event `sync_all` cadence — the
  ~53× win only applies when a caller explicitly opts in, and `n > 1` is a documented
  acknowledged-before-durable tradeoff (up to n−1 events lost on crash before their batch syncs),
  never silent (panics if set after a write is already pending). Also folds in the fd-reuse half
  this doc separately called "contract-neutral" — the handle is now cached lazily across inserts
  instead of reopened per event. 4 new tests + the existing 21 hydra tests green (25/25); 1046/1046
  kernel lib. No caller has opted in yet (still `batch_size = 1` everywhere in this repo) — this is
  the mechanism landing, not a default-behavior change.
- **Item 27 (classifier-input half)** — ✅ **DONE** (`03887462a`, branch
  `exec/space-grade-tier0-2026-07-19`). See
  [`BLUEPRINT-ITEM-27-pmu-classifier-input-2026-07-19.md`](BLUEPRINT-ITEM-27-pmu-classifier-input-2026-07-19.md)
  (the response half correctly links its own file; this half was unlinked — found + fixed
  2026-07-20). PMU counters now ride alongside every `Verdict`/`DriftClass`
  emission as an FDR companion, WITHOUT touching either classifier. New `kernel/src/fdr/pmu.rs`:
  `PmuStamp` (all `Reading<u64>`), a sibling of `HwStamp` on the same `Reading<T>`/`Absence`
  machinery. **Tier A** (`rdtsc` + `/proc/self/stat` minflt/majflt/nswap + `/proc/self/status`
  ctxt-switches) reads real data with zero permissions. **Tier B** (instructions/cycles/
  cache-misses/branch-misses) via a hand-rolled zero-dep `perf_event_open(2)` raw syscall (`asm!`),
  every failure mode degrading to a named `Absence` (new `NoPmuInterface` variant; EPERM/EACCES →
  `PermissionDenied`) — never a fabricated 0, never a panic. Wired via `PmuStation::bracket`: the
  `markov_attractor` bin window-brackets `analyze_detailed` and logs ONE `markov_verdict` `FdrEvent`
  carrying `verdict_str()` + the PMU delta on the SAME record (optional `pmu` field, absent
  elsewhere so all other FDR records stay byte-identical). `analyze_detailed`/`classify_drift` stay
  pure (P6 preserved). Diagnostic-grade; NO CI gate keyed to any PMU value. 6 `fdr::pmu` unit tests
  + 1 end-to-end integration test (spawns the real bin, recovers the real FDR ring) green; full
  kernel suite 955 passed / 0 failed.
  - **Independent-verification correction to §C/line 212's "PMU unavailable" premise:** the
    self-management agent process runs as **root with `CAP_PERFMON`/`CAP_SYS_ADMIN`**, which
    **bypasses `perf_event_paranoid=4` entirely** — so Tier B `perf_event_open` actually SUCCEEDS in
    that context and returns real hardware counters (measured live: IPC ≈ 3.7, instructions/cycles/
    cache-miss/branch-miss all real, hardware-plausible). A genuinely *unprivileged* process on this
    host would still see `permission_denied`; that named-absence path is proven deterministically
    (errno-table + forced-absence serialization tests) rather than relying on the live privilege
    level. **Operator note (informational, non-blocking):** for the unprivileged production path,
    `sysctl kernel.perf_event_paranoid=2` OR granting `CAP_PERFMON` (kernel ≥5.8) to the kernel's
    process would unlock Tier B's real IPC/cache-miss data there too — a host-level knob, flagged
    here for awareness, no decision required for this half to stand.
  - The autonomic-**response** half stays routed to Tier 4 (below) per the source's own requirement —
    gated on item 9 (breaker) + item 21 (gain-scheduling); untouched here.

### D. Tier 3 — THE PIVOT.

- **Item 9** — build `kernel/src/breaker/` from Blueprint A under the §1.5/§10-P4 standard (typed
  `Result<Permit, Tripped>`, unconstructible tripped-but-permitting state, `CommitError` alarms
  routed in). **The pivot point of the entire roadmap** — items 11, 12, 21, 27(response), and 32
  (control-law half) all sit behind it. Best entered after item 2's finding and Tier 1's FDR. See
  [`BLUEPRINT-ITEM-09-breaker-2026-07-19.md`](BLUEPRINT-ITEM-09-breaker-2026-07-19.md).
- **Item 10** — TLA+ spec of decision-import + order FSM. No structural dependency on the breaker;
  same-tier verification of the same state-machine family, runs in parallel with item 9. See
  [`BLUEPRINT-ITEM-10-tlaplus-decision-fsm-2026-07-19.md`](BLUEPRINT-ITEM-10-tlaplus-decision-fsm-2026-07-19.md).

### E. Tier 4 — gated on the breaker.

- **Item 21** — autonomic gain-scheduling module. Explicit stated dependency: strictly after item 9. See
  [`BLUEPRINT-ITEM-21-autonomic-gain-scheduling-2026-07-19.md`](BLUEPRINT-ITEM-21-autonomic-gain-scheduling-2026-07-19.md).
- **Item 11** — ARINC-653 scheduler Phase 0 (design doc + TLC model only). **Ruling: PURSUE,
  design-only (§0 above)** — can start now as a design artifact; the model itself doesn't need the
  breaker to exist, only the eventual code does ("code comes only after the breaker exists"). See
  [`BLUEPRINT-ITEM-11-arinc653-scheduler-phase0-2026-07-19.md`](BLUEPRINT-ITEM-11-arinc653-scheduler-phase0-2026-07-19.md).
- **Item 12** — SIHFT pilot, **re-scoped 2026-07-19 as TEMPORAL TMR** (merged re-scope: the
  consistency audit's premise correction §§1.1–1.2 + the OS-patterns research §3 name the same
  underlying redundancy concept — one refined item, no new number). **Ruling: PURSUE, design-only
  for now (§0 above, premise retro-corrected there)** — the pilot itself needs breaker + FDR;
  scoping/design work can start immediately, sized under the **non-ECC local-hardware premise**.
  Refined scope: **temporal** triple-run (2–3× sequential re-execution on one core over the same
  inputs + a trivial-equality vote — spatial TMR is unavailable to a single-process kernel and
  shared-silicon-correlated anyway, synthesis §6 caveat kept) over the 2–3 most critical µs-scale
  pure functions only (money gate, event-id hash, FSM transition candidates); vote-mismatch →
  item 9 breaker trip + FDR `Alarm`, never an SEU-immunity claim; honestly PARTIAL (permanent
  faults and software bugs corrupt all runs identically; the voter is kept a trivial equality to
  minimize its own exposure). Genuinely ADDITIVE over item 54: Sentinel guards struct bytes
  at-rest/at-transition; temporal TMR guards the *evaluation itself* against compute-time
  transient flips — complementary halves, named as such in both designs. Per the Kleene audit
  (finding 6), the FDR entry carries `VoteOutcome::{Unanimous, SingleDissent(replica-id),
  NoMajority}` — both non-unanimous classes trip identically (behavioral collapse kept, distinct
  typed cause recorded; item-50 shape) — bake this into the design doc now at zero code cost. See
  [`BLUEPRINT-ITEM-12-temporal-tmr-2026-07-19.md`](BLUEPRINT-ITEM-12-temporal-tmr-2026-07-19.md).
- **Item 27 (response half)** — after item 21. See
  [`BLUEPRINT-ITEM-27-response-half-2026-07-19.md`](BLUEPRINT-ITEM-27-response-half-2026-07-19.md).
- **Item 32 (split)** — Laplacian half already lands with item 18 (Tier 0). **Ruling: PURSUE the IR
  extension (§0 above)** — this can start as its own eqc-rs capability work, independent of the
  breaker; only the §16 pilot-control-law half needs items 9 + 21. See
  [`BLUEPRINT-ITEM-32-eqc-ir-extension-2026-07-19.md`](BLUEPRINT-ITEM-32-eqc-ir-extension-2026-07-19.md).

### F. Parallel lanes

- **Spectral/physics lane:** item 18 (Tier 0, narrowed) → item 32's Laplacian half (also Tier 0/now).
  eqc IR extension (item 32, ruled PURSUE) runs alongside, independent.
- **Living-memory lane:** item 19 (audit) → **item 20** (P95 persistence — genuinely open,
  externally ungated, READY now; see
  [`BLUEPRINT-ITEM-20-living-memory-persistence-2026-07-19.md`](BLUEPRINT-ITEM-20-living-memory-persistence-2026-07-19.md))
  → **item 28** (optical compression — **ruled PURSUE**, pilot scoped
  to the archival plane only, sequenced after item 20 since it consumes the same durability
  machinery; see
  [`BLUEPRINT-ITEM-28-optical-compression-2026-07-19.md`](BLUEPRINT-ITEM-28-optical-compression-2026-07-19.md)).
- **Mesh/gossip lane:** **item 22** (verification, READY) → reimplementation work (per the §0 ruling,
  not a vendor integration) → **item 23** (explicit stated dependency: after item 22 — preserved
  exactly; extends `import_unit()`, no parallel importer; see
  [`BLUEPRINT-ITEM-23-gossip-import-extensions-2026-07-19.md`](BLUEPRINT-ITEM-23-gossip-import-extensions-2026-07-19.md))
  → **item 24** (crypto surfaces under §4 —
  depends on item 6's re-executing CI machinery and item 14's trigger; see
  [`BLUEPRINT-ITEM-24-mesh-crypto-hardening-2026-07-19.md`](BLUEPRINT-ITEM-24-mesh-crypto-hardening-2026-07-19.md)).

---

### G. Garden of Eden — Recommended First Execution Batch, hand to Opus now, in this order

1. **Item 2** — Proof: a cited line constructing the durable store in production, or a filed defect;
   plus the §10/P4 check on `Result`-typed `insert`. **✅ DONE 2026-07-19 — defect filed** (no production
   construction site exists; (b) `Result`-typed insert confirmed): `BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP-2026-07-19.md`.
2. **Item 30** — Proof: a table, one row per module, citing file:line for shared-vs-independent
   state-machine logic; every independent one gets a collapse-or-parity-pin ticket.
   **✅ DONE** — proof table + 4 parity-pin tickets in `AUDIT-ITEM-30-state-machine-final-2026-07-19.md`;
   1 confirmed defect (`resume()` owner-zeroing) fixed (`707848dfd`).
3. **Items 15, 16, 19, 17** (read-only audits, any order) — Proofs verbatim from the source: single
   backend + named parity test cited by file:line or P2 defect filed; one eigenvalue computation
   feeding all functionals; shared backend cited or defect filed; every public `engine` item
   classified with RC-4 as first three entries.
4. **Item 22 (verification half)** — Proof: the classification cited by file:line — typed boundary
   plus real kernel caller, or no-caller finding filed. Now feeds directly into reimplementation
   scoping (§0 ruling), not a decision package. **✅ DONE 2026-07-19** —
   [`AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md`](AUDIT-ITEM-22-mesh-classification-final-2026-07-19.md):
   no-caller finding filed (zero production callers; `MeshLog`/`MlDsaSigner`/`Signer` bench-only,
   `SignedEntry`/`MeshError`/`HubTransport` uncalled). "Mostly stub above the log layer" — see §A.
5. **Item 3** — Proof: zero heap allocations under a counting allocator test; one `idx_of`
   definition; golden signature and 1e-12 oracle both green.
6. **Item 18 (narrowed)** — Proof: a parity test computing Lu via dense `laplacian()` and via
   `laplacian_spmv` — exhaustive over small graphs plus a large randomized corpus — green to float
   epsilon; `cargo tree` unchanged.
7. **Items 1+13** — Proof: CI fails on any new dependency, allowlist shrinks monotonically; gate
   verdict identical with networking disabled, lockfile hash unchanged.
   **✅ DONE 2026-07-19** — baseline re-verified (`cargo tree -e no-dev --locked --offline` = exactly
   24 external crates, matches the blueprint). Landed `kernel/ZERO-DEP-ALLOWLIST.txt` (24 names),
   `scripts/zero-dep-gate.sh` (Gate A tree⊆allowlist / Gate B `comm -13` monotonic-shrink vs
   `origin/main` / Gate C `Cargo.lock` sha256 stable), and the `zero-dep-gate` CI job running under
   `unshare -n`. All four §G.7 clauses red-proven: Gate A RED on a throwaway `libc` dep, Gate B RED on
   a grown allowlist + GREEN on a shrink, `unshare -r -n`/`unshare -n` identical 24-crate verdict.
   Committed `01acd673e`, pushed to `exec/space-grade-tier0-2026-07-19`. Blueprint:
   `BLUEPRINT-ITEMS-01-13-ci-zero-dep-gate-2026-07-19.md`. Scope held to `dowiz-kernel` (item 31 = Tier 2).
8. **Item 14** — Proof: a toolchain-bump diff without the spot-check artifact fails CI; a non-bump
   diff never triggers the job. **✅ DONE 2026-07-19** (`bb1e9e8dc`) — proof discharged: gate logic
   unit-tested 6/6 (non-bump → vacuous-green exit 0; bump-without-artifact → RED exit 1;
   bump-with-artifact → GREEN; malformed-artifact → RED; `<absent>→1.96.1` with/without baseline)
   plus an end-to-end `git show BASE:$FILE` extraction test against the real committed pin. Live
   GH-Actions run of the introduction PR is the `<absent>→1.96.1` end-to-end green; G5 (required-check
   registration) still owed server-side.
9. **Item 25 (procedure doc)** — then **Items 4+29 (+JsonWriter)** — **✅ DONE 2026-07-19**
   (`f04142f89`, `4f4872a54`, `eb350464e`). All proofs discharged: `cargo tree -e no-dev` 25→6
   (19 dropped ≥13); `metric.jsonl` + markov CLI JSON byte-compatible; post-mortem readback test
   (kill -9, restart, recover — 300/300 events + PostMortem); event schema `hw` first-class,
   RAPL-less host shows `unavailable:no_rapl_interface` (named absence, not silent omission);
   `zero-dep-gate.sh` GREEN; wasm32 green; 938 tests pass.
10. **Item 5** — **✅ DONE 2026-07-19** (`18152ef84`, `c6b5d2176`, `6605166cd`). Proofs discharged:
    `cargo tree --manifest-path kernel/Cargo.toml -e no-dev --locked --offline` = `dowiz-kernel`
    root ONLY (**0 external crates**); pre-cutover parity of the kernel-owned {literal, `.`, `.*`}
    matcher vs the live `regex` crate (20-doc + 2000-doc + proptest, bit-identical) + permanent
    independent naive-reference differential + frozen golden; `zero-dep-gate.sh` GREEN "0 external
    crates" (empty allowlist; latent zero-state gate abort fixed); existing parsing tests green
    (925 lib unit / 0 failed). **This closes ALL of Tier 1.**

Everything in this batch is now unblocked — no operator ruling stands between it and execution.

---

### H. Items 33–44 — Deterministic AI Inference Arc (appended 2026-07-19, second wave)

**Source:** `DETERMINISTIC-AI-INFERENCE-SYNTHESIS-2026-07-19.md` (grounding + five resolved
decisions) and `RAW-PROMPT-4-deterministic-ai-inference-self-verifying-code-2026-07-19.md`
(verbatim source dialogue). **Governing ruling, recorded:** *"безпека і передбачуваність понад
швидкість"* (safety and predictability over speed) — applied in the synthesis §2 to resolve all
five of the dialogue's open questions: **own-kernel zero-dep engine** (not TVM/Burn),
**inference-only** (training stays edge/build-time), **embedded weights** (generated committed
Rust static, `#[repr(align(64))]`, SHA3 init self-check, codesign; objcopy/link-section deferred
with named trigger), **i8-symmetric per-tensor quantization** (integer domain end-to-end,
`div_half_up` requantization), and **fold_transitions pinning moot pending re-measurement**
(item 33; separate-core rejected on the `core_pinning.rs` DECART precedent). Same sorting rule
as items 1–32: actual technical dependency, lowest first. **Standing law for the whole arc:**
zero new external crates (the live empty `ZERO-DEP-ALLOWLIST.txt` gate makes violation a CI
failure); every hot path ships under the §4 hardening checklist (item 6's machinery) — no
parallel checklist; dependency questions, if any arise, follow item 25's BINDING procedure.
**Build-plane AI-optional law (item 45, recorded here per item-45 blueprint §1/§5 step 1):** when the
inference subsystem (items 33–44) lands, it MUST ride a **non-default cargo feature** named
`inference` in `kernel/Cargo.toml` — the exact `pq`/`slot-arena`/`gpu` surface-control pattern
(lines 65–92), with a header comment stating what it pulls and the `cargo tree -p dowiz-kernel -e
no-dev` verification that the DEFAULT graph stays AI-free. No `inference` feature is added now (item
45 adds nothing to gate yet — over-design guard); the invariant is enforced today by the
`ai-optional-gate` CI job (scripts/ai-optional-gate.sh): the default-features kernel suite is
re-executed green (AI absent) and a dependency-direction grep forbids the seven core decision
modules (`order_machine`, `decision/`, `hydra`, `event_log`, `markov`, `spectral`, `fdr`) from
naming the reserved `crate::inference` path outside `#[cfg(feature = "inference")]`. When the
feature lands, that grep is additionally backed by name-resolution failure (the AI module simply
does not compile absent the feature). The deterministic-math organs `attention`/`micrograd`/`online`
are explicitly OUT of the forbidden set (non-AI per attention.rs:17–20).
Planning only — no item below starts before the operator dispatches it.

- **Item 33 — bench ground-truth re-measurement (Tier-0-class, zero prerequisites, NOT
  gated on item 34).** The raw prompt's telemetry numbers (+30% wire, 3.02x ML-DSA @N=64,
  +16.6% `fold_transitions`, +14.3% `empirical_identify`, "123 passed" engine, MISSING
  `fundamental_matrix_16`) match **no committed artifact in their claimed context** in either repo
  (synthesis §1.2) — names real, numbers unverified. (Lone near-match, corrected on re-verify: the
  figure "123 passed" *is* a real committed count, but for `bebop-proto-cap`
  (`WAVE-CLOSEOUT-P57-P74-2026-07-19.md:36`, P65), NOT `engine` — engine is 112/116/117/121 across
  committed docs, never 123; a cross-wired attribution, which strengthens the "real numbers from a
  different session" reading rather than weakening it.) Run the full tracked bench set (all baseline.json keys, both
  repos' perf branches reconciled) against committed baselines; confirm or refute each claimed
  regression; close the `_cur.json` partial-run gap so MISSING→RED cannot be produced by an
  incomplete run. **Proof:** a dated results doc with per-bench delta vs `baseline.json`; each
  raw-prompt number explicitly CONFIRMED (with the reproducing command) or REFUTED; a full-key
  run recorded with zero MISSING rows; any confirmed regression gets its own follow-up ticket
  (static-data-layout-first per the Q2 resolution — item 3's const-adjacency is the named fix
  shape; separate-core stays rejected). See
  [`BLUEPRINT-ITEM-33-bench-remeasurement-2026-07-19.md`](BLUEPRINT-ITEM-33-bench-remeasurement-2026-07-19.md).
- **Item 34 — pilot workload selection + scope ruling (`RESOLVED 2026-07-19` — operator ruled;
  gates items 35–44).** No model exists in-repo, so the arc must not start as an engine in search
  of a workload. Candidate real-product surfaces were presented (synthesis §3: retrieval reranker
  head, `Verdict`/`DriftClass` anomaly scorer, ETA-adjacent regressor — each KB–MB-scale,
  bounded-domain). **Operator's ruling (recorded, CLOSED — not an open gate): SYNTHETIC/TOY PILOT
  FIRST.** A small hand-built synthetic classifier — a toy MNIST-style or hand-authored pattern
  classifier, weights hand-written or fit offline at KB scale, **zero product data, zero PII, zero
  product risk** — that exercises the WHOLE determinism pipeline end-to-end (quantization → arena →
  SIMD kernels → reference oracle → golden checksum → embedded weights) and proves it works BEFORE
  any real product workload is attempted. Explicitly **NOT** a real-product classifier (the three
  §3 surfaces are DEFERRED to a follow-on second pilot, itself gated on this toy pilot landing
  green); **NOT** design-only/deferred (the toy pilot is *built*, it is the concrete vehicle for
  items 35–44); and — restating the arc-wide non-goals — not an LLM, not training, not GPU.
  **Scope consequence threaded to the downstream items:** the toy pilot's input plane is
  **public/synthetic by construction** (no capability/crypto/secret-adjacent inputs, no
  product/PII data anywhere in items 35–44), so item 43's constant-time gate takes its
  cheap-but-optional branch for THIS pilot — the mandatory dudect branch and any PII/secret-plane
  handling activate only for the deferred real-product pilots (item 43's named reopening trigger).
  **Proof (ruling half — DONE):** this ruling recorded here and in synthesis §3. **Proof (spec
  half — owed on dispatch):** a one-page spec fixing the toy classifier's bounded input domain D
  (synthetic, enumerable or tightly bounded) and the output-tolerance guarantee the engine must
  prove — the pure-function `f(x)=y` contract of the source dialogue's part 3. See
  [`BLUEPRINT-ITEM-34-toy-pilot-spec-2026-07-19.md`](BLUEPRINT-ITEM-34-toy-pilot-spec-2026-07-19.md).
- **Item 35 — fixed-point number-format + rounding-law spec (after 34).** The Q5 ruling made
  concrete: i8-symmetric weights, per-tensor scale (power-of-two shift preferred), i32
  accumulators with per-layer proven no-overflow bounds, `div_half_up` requantization,
  saturating-clamp semantics, refuse-never-fall-back on any unprovable bound. **Proof:** a spec
  doc with every law as a checkable equation; the i8×i8 multiply-accumulate law exhaustively
  proven over all 65 536 pairs (the house 65536-pair standard, literally); overflow-bound lemma
  stated falsifiably per layer shape. See
  [`BLUEPRINT-ITEM-35-fixed-point-rounding-spec-2026-07-19.md`](BLUEPRINT-ITEM-35-fixed-point-rounding-spec-2026-07-19.md).
- **Item 36 — eqc-rs indexed-summation IR extension, quantized-dot target (after 35; extends
  the already-ruled item 32 IR work — one extension, two consumers, never two IRs).** Grow
  `Expr` with the Σ-over-index construct needed by BOTH the Laplacian neighbor-sum (item 32)
  and the quantized dot-product inner law; `emit_fixed_rust` learns the i32-accumulator Q-format
  path. **Proof:** `emit_proof_program` harness green on an emitted quantized dot (compiled with
  real rustc, self-asserted against the tree-walking evaluator); the fixed emitter demonstrably
  refuses an inexpressible node; item 32's Laplacian consumer still green — one IR serves both. See
  [`BLUEPRINT-ITEM-36-eqc-indexed-summation-ir-2026-07-19.md`](BLUEPRINT-ITEM-36-eqc-indexed-summation-ir-2026-07-19.md).
- **Item 37 — reference oracle implementation (after 35; parallel with 36).** The "Schoolbook"
  of this arc: scalar, obviously-correct integer-domain matmul + activation set (i64/i128
  shadow accumulation — std-only, no dependency), retained forever as the test-only
  differential target, per the §4 checklist's oracle clause and the NTT schoolbook precedent.
  **Proof:** exhaustive small-dimension cases + large randomized corpus, oracle vs
  wide-accumulator shadow, zero divergence; the oracle module documented as permanent (never
  deleted on optimization). See
  [`BLUEPRINT-ITEM-37-reference-oracle-2026-07-19.md`](BLUEPRINT-ITEM-37-reference-oracle-2026-07-19.md).
- **Item 38 — static tensor workspace on the Arena (after 34; parallel with 35–37).** The
  dialogue's part-5 shape on the existing `BumpArena` precedent: one preallocated workspace,
  tensors as fixed offsets computed at build time from the pilot graph, zero mid-inference
  allocation, zero-copy layer-to-layer reads. **Proof:** a counting-allocator test (item 3's
  own proof machinery reused) shows zero heap allocations across a full inference; offsets are
  `const`; a deliberately-overlapping layout fails to construct (illegal state unrepresentable,
  §1.5 house standard). See
  [`BLUEPRINT-ITEM-38-tensor-arena-workspace-2026-07-19.md`](BLUEPRINT-ITEM-38-tensor-arena-workspace-2026-07-19.md).
- **Item 39 — SIMD quantized kernels via `core::arch` (after 36+37+38).** AVX2
  `_mm256_maddubs_epi16`/`_mm256_madd_epi16`-class integer paths, runtime-detected with the
  scalar oracle as fallback — `simd.rs`/`householder.rs` house pattern. Named dividend of Q5:
  integer arithmetic is associative, so within-row vectorization is *legal* here (unlike the
  f64 lanes' across-rows-only rule) — but the chosen lane order is still fixed and documented,
  and debug builds carry `debug_assert_eq!` against item 37's oracle (the `ring_mul` standard).
  **Proof:** differential corpus vs oracle bit-exact on both paths; the §4 checklist artifacts
  present and CI-re-executed; bench added to `baseline.json` so the bench-gate guards it. See
  [`BLUEPRINT-ITEM-39-40-simd-kernels-golden-checksum-2026-07-19.md`](BLUEPRINT-ITEM-39-40-simd-kernels-golden-checksum-2026-07-19.md).
- **Item 40 — per-layer golden-checksum oracle + hard-fail (after 39).** Build-time golden
  CRC32 per layer over pinned test vectors (reusing `fdr`'s hand-rolled CRC32 — P2, no second
  CRC), runtime spot-check, hard-fail to safe state on mismatch — a checksum mismatch is
  hardware/memory fault evidence, not a model error. Until item 9's breaker exists the fail is
  a typed trap + FDR entry; when the breaker lands, it routes through `Result<Permit, Tripped>`
  (composition named in synthesis §3 — design does NOT gate on item 9). **Proof:** a planted
  single-bit corruption (weights AND activation, separately) demonstrably trips the fail path
  and writes the FDR entry; an uncorrupted run is checksum-silent; CI re-executes the planted
  fault (P7 — the verifier proves it can reject). See
  [`BLUEPRINT-ITEM-39-40-simd-kernels-golden-checksum-2026-07-19.md`](BLUEPRINT-ITEM-39-40-simd-kernels-golden-checksum-2026-07-19.md).
- **Item 41 — embedded weight pipeline (after 35; parallel with 39–40).** The Q4 ruling made
  real: generator emits committed `static WEIGHTS: [i8; N]` Rust source (eqc_gen precedent),
  `#[repr(align(64))]` wrapper (first in-repo use — flagged as new surface), SHA3-256 golden-
  hash self-check at init (reusing `event_log`'s Keccak), ML-DSA codesign via `pq/codesign.rs`
  for update-blob shipping. The objcopy/`link_section` alternative is parked with its named
  reopening trigger (weights > ~1–2 MB committed-source practicality, or measured build-time
  regression) per item 25's procedure. **Proof:** init self-check demonstrably fails on a
  tampered byte (red→green); alignment asserted by test; the parked alternative + trigger
  recorded in the module doc (slot_arena format); zero-dep gate untouched. See
  [`BLUEPRINT-ITEM-41-embedded-weight-pipeline-2026-07-19.md`](BLUEPRINT-ITEM-41-embedded-weight-pipeline-2026-07-19.md).
- **Item 42 — fixed-sequence scheduler (after 38+39+41).** The engine's spine: a `const`
  function-pointer array / straight-line layer sequence, cyclomatic complexity 1, no dynamic
  graph traversal, no hash-map dispatch — the whole model as one compiled call sequence.
  **Proof:** a source-structure test asserts the sequence is `const` and branch-free at the
  dispatch level; an assembly spot-check of the dispatch path filed under item 14's toolchain-
  keyed audit format; end-to-end inference reproduces bit-identical outputs and (via item 40)
  identical per-layer checksums across repeated runs and across native/wasm32. See
  [`BLUEPRINT-ITEM-42-fixed-sequence-scheduler-2026-07-19.md`](BLUEPRINT-ITEM-42-fixed-sequence-scheduler-2026-07-19.md).
- **Item 43 — constant-time inference gate (after 42; scope decided by input-plane
  classification first).** Classify the pilot's input plane per §10/P6 plane-ranking: if inputs
  are secret-adjacent (anything fed from capability/crypto surfaces), the full dudect-style
  gate with planted-leak self-test (the `ntt_ct_gate` template) is mandatory and ReLU-class
  branches become mask/cmov per the dialogue's part 4; if provably public-plane, record that
  ruling and ship the gate as cheap-but-optional. **For the item-34 synthetic/toy pilot the
  classification is already settled — inputs are public/synthetic by construction, so this pilot
  takes the cheap-but-optional branch; the mandatory dudect branch activates only for the deferred
  real-product pilots (item 34's reopening trigger).** **Proof:** the plane classification recorded
  with its reasoning; if gated — Welch |t| < 4.5 across input classes AND the planted leak
  demonstrably caught; if not gated — the recorded ruling names the reopening trigger (any new
  secret-adjacent consumer). See
  [`BLUEPRINT-ITEM-43-constant-time-inference-gate-2026-07-19.md`](BLUEPRINT-ITEM-43-constant-time-inference-gate-2026-07-19.md).
- **Item 44 — arc-wide CI integration + retroactive checklist pass (after 40+42; final).**
  The inference hot paths join item 6's designated-hot-path list; the §4 CI job re-executes
  (never presence-checks) the oracle corpus, the planted-fault checksum test, and (if gated)
  the dudect self-test; benches join the bench-gate baseline; the FDR carries per-inference
  cycles + (where RAPL exists) joules per item 29's schema — a token-count-only cost report
  fails review per §21. **Proof:** a deliberately artifact-less test diff touching an inference
  hot path fails CI; the full suite green; `cargo tree -e no-dev` still resolves to the kernel
  root alone — the arc lands with the allowlist still empty. See
  [`BLUEPRINT-ITEM-44-arc-ci-integration-2026-07-19.md`](BLUEPRINT-ITEM-44-arc-ci-integration-2026-07-19.md).

**Dependency graph, one line:** 33 ∥ 34 → 35 → {36 ∥ 37 ∥ 38} → 39 → 40 → 42 → 43 → 44, with
41 branching off 35 and merging before 42; item 9 (breaker) composes with 40's fail path when
it exists but gates nothing here.

---

### I. Items 45–49 — Whole-System Determinism & AI-Optional Arc (appended 2026-07-19, third wave)

**Source:** `CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-SYNTHESIS-2026-07-19.md` (Fable
synthesis) over `RESEARCH-CRASH-CONSISTENCY-FORMAL-VERIFICATION-GUARDIAN-2026-07-19.md` (Opus
grounding, 11 findings) and
`RAW-PROMPT-5-crash-consistency-formal-verification-fail-fast-guardian-2026-07-19.md` (verbatim
dialogue). **Governing directive, recorded:** *"вона має 100% передбачуваною, математично
детермінованою із запобіжниками. Окрім цього уся система повинна здатна працювати без AI"* —
(a) whole-system determinism + safeguards, broader than the items-33–44 AI subsystem; (b)
AI-optional as a preserved architectural INVARIANT (GROUNDED already-true today: `attention.rs`
"the kernel stays non-AI"; `order_machine`/`decision`/`hydra` import zero AI modules). Ground
truth honored throughout: the kill-9 mechanism IS a Sequential Append-only Log (not pointer-swap,
not hybrid); Kani/TLA+ remain planned-only (items 7/10/11, unchanged); Coq/Lean-class full
formal verification is OUT OF SCOPE per the synthesis §5 proportionality ruling (BITE/runtime-
verification primary — where the source dialogue's own self-correction landed). Same standing
laws as §H: zero new external crates, §4 hardening checklist via item 6's machinery, item-25
procedure for any dependency question. Planning only — no item starts before the operator
dispatches it.

- **Item 45 — `ai-optional-gate`: AI-optional as an enforced compile-time invariant (Tier-0/1-
  class, zero prerequisites, READY NOW — asserts today's truth, gains teeth when items 33–44
  land).** Structural law amended into the §H arc: the inference subsystem lands behind a
  **non-default cargo feature** (e.g. `inference`) — the exact `pq`/`slot-arena` surface-control
  pattern already in the kernel. New CI job (zero-dep-gate/toolchain-bump-gate precedent shape):
  (a) default-features build (AI absent) must compile AND pass the FULL kernel test suite; (b) a
  dependency-direction check — no core decision module (`order_machine`, `decision/`, `hydra`,
  `event_log`, `markov`, `spectral`, `fdr`) may reference the AI module paths outside the feature
  gate (AI depends on core, never core on AI). Explicitly NOT built: runtime kill-switch service,
  dual-binary pipeline, AI-health monitor (over-design guard; the runtime half is item 47's
  `None` path). **Proof:** a planted core→AI import (or a planted default-features AI reference)
  demonstrably turns the gate RED before the gate counts as landed (P7); the default-features
  full suite runs green inside the job; the feature-gate law is recorded in §H's header and the
  AI module's own doc when it lands. See
  [`BLUEPRINT-ITEM-45-ai-optional-gate-2026-07-19.md`](BLUEPRINT-ITEM-45-ai-optional-gate-2026-07-19.md).
  **Status correction (2026-07-20, `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md`):** `scripts/ai-optional-gate.sh`
  exists on `main` (169 lines, real logic, landed via `cb00706b1`) but is **not referenced in any
  `.github/workflows/*.yml` job** — the "New CI job" described above is written but not yet live.
  Script-exists ≠ gate-live; do not read this item as CI-enforced until the workflow wiring lands.
- **Item 46 — float-determinism containment, evidence-scoped (READY NOW; composes with item 14's
  closed bump gate).** NOT a kernel-wide f64→fixed rewrite — rejected as disproportionate
  (synthesis §2.3: the one real float-nondeterminism bug ever shipped was libm `sin`/`cos` ULP
  drift, fixed by the Q30 CORDIC, `REGRESSION-LEDGER.md` row 25; basic IEEE-754 arithmetic is
  bit-deterministic for a fixed binary on the pinned 1.96.1 toolchain). Scope: (i) inventory
  every libm-transcendental call site (`sin`/`cos`/`exp`/`ln`/`powf`; `sqrt` exempt —
  correctly-rounded) in the deterministic kernel plane (`spectral.rs`, `markov.rs`,
  `token_bucket.rs`, `attention.rs`), disposition each as migrate-to-CORDIC-class or
  pin-under-golden; (ii) every value feeding a cross-version/cross-host comparison surface
  (golden signatures, oracle pins, `wire_code()`s, `DRIFT_BAND`-class constants) must be either
  integer-domain or covered by a golden test. **Re-execution mechanism (verified precise): the
  toolchain-bump gate itself only requires a `spot-check-<new>.md` artifact on a `channel` bump;
  the golden tests are actually re-run under the new compiler by the always-on full-suite
  `cargo test` job (pinned via `rust-toolchain.toml`) plus item 6's `hardening-gate` unconditional
  oracle re-run** — so a compiler-induced float divergence turns the bump PR RED, never a silent
  ship (once this item adds the missing golden coverage); (iii) the full fixed-point
  conversion is parked as an explicitly-flagged-LARGE item with named reopening triggers: a
  reproduced cross-version golden divergence in basic float arithmetic, or a multi-ISA deployment
  requirement. **Framing amendment (2026-07-19, consistency audit §1.5):** the local-first mesh
  target means heterogeneous peer hardware whose peers replay each other's DecisionUnits
  (`import_unit`'s replay-before-persist), so the multi-ISA reopening trigger must be evaluated
  against *fleet heterogeneity* (incl. aarch64 consumer devices), not a single-host assumption —
  scope (ii)'s cross-host comparison surfaces are the first line either way. **Proof:** the
  inventory doc with per-site disposition and zero unclassified
  transcendental sites; the new golden float surfaces sit in the always-on full-suite /
  `hardening-gate` oracle set (a deliberately perturbed golden value turns CI RED under the pinned
  toolchain — red-proven), and a `channel` bump is additionally gated on the `spot-check-<new>.md`
  `## Full-suite re-run` artifact; the parked rewrite + triggers recorded in the doc and the
  relevant module docs. See
  [`BLUEPRINT-ITEM-46-float-determinism-containment-2026-07-19.md`](BLUEPRINT-ITEM-46-float-determinism-containment-2026-07-19.md).
- **Item 47 — Guardian: semantic advice gate + deterministic-primary path (spec after item 35;
  full wiring after item 42; EXTENDS item 9, cross-references item 40 — no competing breaker, no
  fold-in).** The kernel's decision seam takes `Option<Proposal>` — advice is DATA; `None`
  (AI absent/crashed/rejected) is a first-class tested input, and the deterministic path is the
  total function (the "fallback" IS the system — AI-optional expressed in the type system).
  Admission is parse-don't-validate: `admit(Proposal, &Invariants) -> Result<ValidatedProposal,
  Rejection>` with `ValidatedProposal` constructible only through `admit`
  (illegal-state-unrepresentable, the item-9 `Result<Permit, Tripped>` standard); invariants
  written as checkable equations (the `Result.velocity < MAX_SAFE_SPEED` class). Static
  procedures are NAMED pure functions, statically dispatched, `match`-based (the `order_machine`
  style), every loop statically bounded (`0..MAX_N`, item-42-style source-structure assertion;
  WCET tooling explicitly out of scope). Distinct from item 40 by plane: 40 rejects corrupted
  BITS (hardware-fault evidence), 47 rejects well-formed-but-unsafe MEANING; both hard-fail
  observable. Every `Rejection` emits an FDR event; when item 9 lands, repeated rejections route
  through the breaker (same composition clause as item 40 — design does NOT gate on item 9).
  Named precedent to extend, never fork: `decision/import.rs::import_unit`'s
  verify-before-persist replay gate — the same shape at import granularity. **Proof:** the
  invariant spec doc with every law as a checkable equation; planted-invalid-advice red→green
  (the gate demonstrably rejects — P7); the `None`-path test proving bit-identical output vs the
  deterministic baseline; exhaustive enumeration where the advice domain is enumerable +
  oracle/differential corpus otherwise + a proptest sweep (the item-5 regex-parity testing
  stack, reused not reinvented); the source-structure bounded-loop assertion green. See
  [`BLUEPRINT-ITEM-47-guardian-semantic-advice-gate-2026-07-19.md`](BLUEPRINT-ITEM-47-guardian-semantic-advice-gate-2026-07-19.md).
- **Item 48 — FDR blind-spot closure: panic forensics + liveness heartbeat (after items 4+29 —
  satisfied; READY once the FDR branch merges).** The kill-9 test proves recovery AFTER process
  death; it is structurally blind to (a) a panicking process that writes nothing before dying
  and (b) a HUNG process that never dies (no PostMortem is ever emitted — the one failure class
  FDR cannot see; the k3 span-metrics self-deadlock, root-caused+fixed `67851b2f3`, is the
  in-repo precedent). Two narrow closures, both BITE-shaped: **(a)** `std::panic::set_hook`
  emitting ONE fsynced `Alarm` FDR record (message + location; `Alarm` already fsyncs) — a panic
  hook, NOT a `#[panic_handler]` (`std` kernel; the bare-metal construct does not apply);
  register/stack core-dumps explicitly not pursued. **(b)** a periodic `Heartbeat` `Kind`
  variant (closed-enum growth) carrying seq + progress counters; liveness JUDGMENT and restart
  authority stay OUTSIDE the kernel (systemd `WatchdogSec` / deployment layer;
  `hub_supervisor`'s crash-loop detection is the deploy-granularity precedent) — a missed
  heartbeat converts a hang into the kill-9 crash class the system already provably survives.
  The kernel carries NO self-kill/self-restart logic (`Kernel_Init`-over-`Kernel_Recover`,
  KISS). **Proof:** a test child that panics yields a recovered `Alarm` record carrying the
  panic site (red→green: without the hook, nothing is recovered); a test child that deliberately
  hangs (loop + no heartbeat) is flagged by the external liveness check WHILE producing no
  PostMortem — demonstrating exactly the gap closed; all other FDR records byte-identical
  (optional-field discipline, item-27 precedent); clean-shutdown emits a final heartbeat and no
  false alarm. See
  [`BLUEPRINT-ITEM-48-fdr-blind-spot-closure-2026-07-19.md`](BLUEPRINT-ITEM-48-fdr-blind-spot-closure-2026-07-19.md).
- **Item 49 — event-log replay-bound measurement + Hybrid/LSM park (after item 2's wiring fix
  lands — currently gated: no production composition root constructs the durable store).** The
  raw prompt's Hybrid (WAL + periodic snapshot) recommendation, dispositioned per surface: for
  the FDR ring it is REJECTED permanently (replay bounded by construction at 2×1 MiB segments);
  for the durable `EventLog` (genuinely unbounded hash-chain replay; `hub_supervisor`'s
  `StateSnapshot` is an update-rollback epoch pointer, NOT replay-speedup) it is PARKED behind
  measurement — measuring an unwired store would optimize an unreachable path. Once wired:
  measure startup replay time vs event count (item-26 measurement-only discipline: real numbers,
  no code landed), state a replay budget, and record the parked snapshot design with its named
  reopening trigger (measured replay exceeding budget at realistic event volume). Carried-forward
  correctness note if ever built: data-file fsync strictly BEFORE pointer swap (the dialogue's
  caveat, endorsed; consistent with `ring.rs`'s kill-9-vs-power-loss separation). **Proof:** a
  dated measurement doc (replay µs at N ∈ {1e3, 1e4, 1e5} events, methodology stated); the
  budget + trigger recorded; zero snapshot code landed (scope law, item-26 precedent); the FDR
  permanent-rejection rationale recorded in `fdr/ring.rs`'s module doc when next touched. See
  [`BLUEPRINT-ITEM-49-event-log-replay-bound-measurement-2026-07-19.md`](BLUEPRINT-ITEM-49-event-log-replay-bound-measurement-2026-07-19.md).

**Dependency graph, one line:** 45 ∥ 46 ∥ 48 ready now (48 pending the FDR branch merge);
47 spec after 35, full wiring after 42, composes with item 9's breaker when it exists;
49 strictly after item 2's wiring-gap fix. No item here gates any §H item; item 45's feature-gate
law binds §H's build items when they land.

### J. Items 50–54 — Validity (K3 Admission), Live-Struct Sentinel & Proportionate Open-Source Hardening Arc (appended 2026-07-19, fourth wave)

**Source:** `KLEENE-TRUTHFULNESS-VALIDITY-SYNTHESIS-2026-07-19.md` (Fable synthesis) over
`RESEARCH-KLEENE-TRUTHFULNESS-OPENSOURCE-HARDENING-2026-07-19.md` (Opus grounding) and
`RAW-PROMPT-6-…-kleene-unknown` + `RAW-PROMPT-7-…-sentinel-shadow-mode` (one combined verbatim
dialogue). **Terminology RULING, binding from here on (synthesis §1):** "Truthfulness" =
byte-reproducibility, exclusively the swarm-safety arc's property, NOT a term of this roadmap;
the RAW-PROMPT-6 content-based concept is renamed **"Validity" (derivational validity)** — a
proposal is valid iff its supplied reasoning/evidence path checks against the stated
axioms/invariants; incomplete evidence downgrades to Undecidable, never to assumed-valid.
**Dispositions recorded here (synthesis §§2.3/2.5, Part 3):** the Sentinel read-time integrity
check for critical LIVE in-memory structs is **ADOPTED as item 54**, proportionately scoped —
an earlier draft of this pass rejected it on a "commodity ECC cloud hardware" argument that the
operator **reversed on 2026-07-19** on two grounds: (i) genuine space-grade engineering quality
is the standard for this arc regardless of substrate, and (ii) the deployment premise was
factually wrong — the target is **local, offline-first, consumer-grade hardware, which typically
LACKS ECC**, so the in-memory bit-flip fault class is *higher* not negligible, strengthening the
mechanism's justification. Item 54 reuses the in-kernel CRC32 (zero new primitive), checks at
transition points (not per-field-read), and is scoped to the live mutable authority structs item
40's read-only weight checksum structurally does NOT cover (item 47's `Invariants` table, item 21
gain-schedule, live inference config) — genuine overlap with item 40 is a boundary, not a reason
to skip. Kani-for-K3 is item-7 target-list growth, not a new item. proptest stays strictly
dev-only (zero-dep-gate law). **Operator-facing repository-state flag (Part 4):
items 1–49's actual CODE (all Tier 1, item 6's gate + `ct_gate.rs`, the FDR module, fixes from
items 16/30/31) still lives ONLY on the unmerged `exec/space-grade-tier0-2026-07-19` branch —
`main` has documents only.** Items below that touch FDR or item 47 inherit that merge as a
prerequisite. Same standing laws as §§H–I: zero new external crates, item-6 hardening machinery,
item-25 procedure for any dependency question. Planning only — no item starts before the
operator dispatches it.

- **Item 50 — K3 admission-verdict extension + Validity terminology binding (spec-level
  amendment to item 47 — same gating: spec after 35, wiring after 42; EXTENDS item 47's
  `admit`, never a parallel type).** The public seam stays exactly item 47's
  `admit(Proposal, &Invariants) -> Result<ValidatedProposal, Rejection>` — Kleene-False and
  Kleene-Unknown MUST be behaviorally identical at the seam (advice unused, deterministic path
  taken), so no third control-flow arm exists for "Unknown" to be handled leniently through.
  `Rejection` gains a two-class cause, `RejectionClass::{Refuted, Undecidable}`: Refuted = a
  named invariant/inference rule demonstrably violated (K3 False); Undecidable = evidence
  chain incomplete/absent/over-budget (K3 Unknown — RAW-6's Evidence-based Unknown adopted
  verbatim: model confidence/logits are NEVER an input to `admit`). The literal
  `#[repr(u8)] enum TruthState { False=0, True=1, Unknown=2 }` lands as an INTERNAL combinator
  type of the admission module: each sub-check returns `TruthState`; the strong-Kleene fold
  governs (any False short-circuits to Refuted — `False & Unknown = False`; else any Unknown
  folds to Undecidable — `True & Unknown = Unknown`; all True admits). `None` ≠ `Unknown`:
  the seam's `Option<Proposal>` None (advice absent, items 45/47) and Undecidable (advice
  present but unevaluable) both take the deterministic path but log as distinct facts. The
  class rides item 47's existing per-`Rejection` FDR event so item 9's breaker and item 51 can
  weight Refuted vs Undecidable differently. **Proof:** exhaustive truth-table tests — all 9
  cases per binary operator + 3 for NOT, the full state space enumerated literally (RAW-7's
  own exhaustive-beats-random point; NO new proptest use); planted incomplete-evidence
  proposal demonstrably lands `Undecidable` and planted rule-violation lands `Refuted`
  (red→green, P7); the item-47 `None`-path bit-identity test still green with the extension in
  place; the K3 fold joins item 7's Kani target list (recorded there, executed under item 7). See
  [`BLUEPRINT-ITEM-50-k3-admission-validity-2026-07-19.md`](BLUEPRINT-ITEM-50-k3-admission-validity-2026-07-19.md).
- **Item 51 — shadow-mode divergence telemetry at the decision seam (after item 47's wiring +
  item 50; FDR branch merge prerequisite — genuinely NEW pattern, full design in synthesis
  §2.4).** No second execution lane: item 47's deterministic decision D is already total and
  always computed, so on `Some(proposal)` the comparison is nearly free. New FDR
  `Kind::ShadowDivergence` variant (closed-enum growth — item-48 `Heartbeat` precedent)
  carrying decision-site id, Admitted/`RejectionClass`, agreement bit, and short DIGESTS of D
  and the proposed action (never full payloads; records without the surface stay
  byte-identical — item-27 optional-field discipline). **Digest primitive, named in-spec
  (2026-07-19, consistency audit §3.3 — max-nativeness):** digest = the in-kernel CRC32
  (hardware-fault plane, matching items 40/54) or truncated in-kernel SHA3-256 — never a new
  algorithm, no third ad-hoc hash under deadline. Emission policy: every disagreement and
  every Admitted-but-differs logged; Undecidable-while-D-decides at a bounded rate (the
  "model adds nothing on this domain" signal); agreement SAMPLED at a low fixed rate for the
  base-rate denominator — bounded emission preserves the FDR ring's replay-bounded-by-
  construction property (item 49's rationale). Advisory by definition AND by test: no build
  fails, no decision changes, no breaker trips on a shadow event alone (aggregated
  Refuted-class counts still reach item 9 via item 47's own rejection events — shadow mode
  adds observation, never authority). Distinct from every existing differential in-tree: those
  all fail/reject on disagreement (`decision/import.rs` ReplayDisagreement rejects;
  pq/spool/spine differentials are tests); nearest advisory kin is `metrics.rs`'s
  merge-plane anomaly flag — different plane, cited not extended. **Proof:** deterministic
  output bit-identical with shadow logging on vs off (item-47 `None`-path test pattern
  reused); a planted disagreeing proposal yields exactly one recovered `ShadowDivergence`
  record with correct class + digests (red→green through the real FDR ring); emission-rate
  bound asserted under a flood of planted disagreements; all non-shadow FDR records
  byte-identical before/after. See
  [`BLUEPRINT-ITEM-51-shadow-mode-divergence-telemetry-2026-07-19.md`](BLUEPRINT-ITEM-51-shadow-mode-divergence-telemetry-2026-07-19.md).
- **Item 52 — `miri-gate`: targeted UB detection over the real unsafe surface (independent —
  zero prerequisites on items 47/50/51; dispatchable now).** GROUNDED baseline: Miri runs
  nowhere (aspirational doc-comments only; `ROADMAP-LIVE-STATUS-2026-07-18.md:24` "component
  absent this toolchain"). **Inventory corrected by independent re-verification 2026-07-19 (the
  research/RAW figure was wrong):** the real unsafe surface is **19 blocks in only 4 modules** —
  `arena.rs` (6), `simd.rs` (5), `fdr/pmu.rs` (5 — `_rdtsc`/raw-`syscall5` FFI, exec-branch only,
  joins post-FDR-merge), `householder.rs` (3). `messenger.rs`/`slot_arena.rs`/`chaos.rs`/
  `bounded_drainer.rs` contain **ZERO real unsafe** — every `unsafe` token in them is a *comment*
  (`slot_arena.rs`'s doc-comment literally says "No `unsafe` in this wrapper"); the old "21 blocks /
  7 modules" list counted those comment mentions and omitted `fdr/pmu.rs`. `pq/` has ZERO unsafe
  (the raw prompt's crypto guess was wrong). Scope: ONE CI job running `cargo miri test` filtered to
  the genuinely unsafe-bearing modules — `arena.rs`'s bump-allocator raw-pointer logic (where UB
  actually hides) plus the scalar paths of `simd.rs`/`householder.rs`; NOT the four unsafe-free
  wrappers (filtering them matches zero unsafe — theater), NOT miri-everything. Honest limitation,
  recorded in the gate's own doc: `core::arch` AVX2 intrinsic bodies AND `fdr/pmu.rs`'s
  `_rdtsc`/syscall FFI are largely unsupported under Miri; the house runtime-detection +
  scalar-fallback pattern means the interpreted run exercises the scalar paths of
  `simd.rs`/`householder.rs`, and intrinsic/syscall-body coverage stays with the items-37/39
  differential oracles + item 7 — a green `miri-gate` is never read as "SIMD/PMU is Miri-clean"
  (exact intrinsic support confirmed empirically on first run, not asserted). Toolchain: Miri
  needs a nightly component; the BUILD pin (item 14, 1.96.1) is untouched — the job pins its
  own analysis nightly, recorded in the workflow + `docs/audits/toolchain/`, bumps recorded
  not floating. **Proof:** a planted UB self-test (out-of-bounds / use-after-free behind a
  test-only cfg) demonstrably turns the gate RED before it counts as landed (P7); clean run
  green; a filter matching zero tests is RED (item-6 anti-forgery clause reused); build
  toolchain pin byte-unchanged. See
  [`BLUEPRINT-ITEM-52-miri-gate-2026-07-19.md`](BLUEPRINT-ITEM-52-miri-gate-2026-07-19.md).
- **Item 53 — `lint-gate`: clippy + fmt (+ miri-required promotion) contribution gates (LOW
  priority, LAST in this arc, blocks nothing — sequenced behind 50–52 by explicit RULING).**
  GROUNDED: none of the triad exists in CI (zero clippy/fmt/miri workflow hits; real gates
  today = cargo-test, dco-check `ci.yml:210-226`, decart-dep-lint, v5c-reexec, gitleaks,
  supply-chain, bench-regression); AND open-sourcing is NOT imminent — ADR-0020 Accepted but
  public-flip + EUTM are operator-gated and unauthorized — so the raw prompt's "any PR is an
  attack vector" urgency presumes a contribution surface that is not authorized to exist yet.
  Scope when dispatched: one cheap job — `cargo clippy --deny warnings` + `cargo fmt --check`
  (both components ALREADY pinned by item 14's `rust-toolchain.toml`
  `components=[rustfmt,clippy]`); miri-required = promoting item 52's job to a required check,
  no new machinery. Inherits item 14's owed G5 caveat: advisory until marked required in
  branch protection (server-side). **Named escalation trigger:** operator authorization of
  public-flip preparation (ADR-0020's gate) promotes this item to a pre-flip BLOCKER alongside
  the ADR-recommended all-origin-refs gitleaks sweep; until then it stays last. **Proof:** a
  planted clippy warning and a planted fmt divergence each turn the job RED (P7); clean tree
  green; the escalation trigger recorded here and in the job's comment header. See
  [`BLUEPRINT-ITEM-53-lint-gate-2026-07-19.md`](BLUEPRINT-ITEM-53-lint-gate-2026-07-19.md).
- **Item 54 — Sentinel: read-time integrity check for critical LIVE in-memory structs (after
  {item 47 wiring (post-42) + item 50} + the FDR branch merge; registry enumeration startable
  now; full design in synthesis §2.3 — operator-reversed 2026-07-19 from an earlier draft
  rejection).** Deployment premise, corrected and load-bearing: the target is **local,
  offline-first, consumer-grade hardware that typically LACKS ECC**, so a single-/multi-bit
  in-memory flip is a real fault class — NOT a cloud/ECC context, and the "space-grade" standard
  binds regardless of substrate. GROUNDED baseline: the live-struct read-time pattern is genuinely
  absent (all existing integrity machinery is AT-REST — `backup` CAS, `event_log` chain-walk, FDR
  ring CRC32). Proportionate on three axes: **(scope)** only structs that are long-lived AND a
  money/safety/decision authority input AND lack at-rest backing qualify — the enumerable registry
  is item 47's `Invariants` table (a flipped bound silently mis-certifies *every* `admit`), item 21
  gain-schedule/decision-config, and the live inference config (distinct from item 40's read-only
  weights); transient scratch and already-at-rest-verified state are excluded. **(primitive)**
  REUSES the in-kernel CRC32 already built for the FDR module (P2 — no second CRC, no new
  algorithm, no external crate; CRC32 not crypto — the threat is a hardware fault, not an in-memory
  adversary). **(frequency)** checked at defined transition points (once per authority-use, e.g.
  per `admit` over the `Invariants`; recompute-and-store on the rare centralized mutation) — NOT
  per-field-read, so the hot-path tax and the missed-re-hash false-trip surface are both bounded;
  an immutable-after-init struct is a pure read-time check with zero re-hash burden. On mismatch:
  ONE fsynced FDR `Alarm` (hardware-fault evidence, item-40 semantics) + fail-closed deterministic
  path (a corrupted `Invariants` table REFUSES admission), composing with item 47's `Rejection`
  seam and item 9's `Result<Permit, Tripped>` when it lands (does NOT gate on item 9). Distinct
  from item 40 by plane: 40 guards read-only static WEIGHTS, 54 guards live MUTABLE authority
  structs — complementary surfaces, one shared CRC. **Proof:** a planted single-bit corruption of a
  registered struct (behind a test-only cfg raw-pointer flip, mirroring item 40's planted-fault
  test) demonstrably trips the Safe-State path and writes the `Alarm` (red→green, P7); an
  uncorrupted run is checksum-silent; mutate-then-read passes (re-hash correctness); CI re-executes
  the planted fault; the critical-struct registry is enumerated with per-struct justification (why
  critical, why no at-rest backing); `cargo tree -e no-dev` byte-unchanged (existing CRC32 reused,
  zero new dependency and zero new algorithm — max-nativeness law). See
  [`BLUEPRINT-ITEM-54-sentinel-live-struct-integrity-2026-07-19.md`](BLUEPRINT-ITEM-54-sentinel-live-struct-integrity-2026-07-19.md).

**Dependency graph, one line:** 50 rides item 47's gates (spec after 35, wiring after 42);
51 after {47-wiring + 50} + the FDR/exec branch merge; 52 independent (on-`main` targets
`arena`/`simd`/`householder` dispatchable now, `fdr/pmu` folds in post-FDR-merge); 53 last by
ruling, trigger-promoted on public-flip authorization; 54 parallel with 51 (same {47-wiring + 50}
+ FDR-merge prerequisite; registry enumeration startable now). Nothing here gates any §H/§I item;
items 50 and 54 amend/extend item 47's surface in place (one admission gate, one shared integrity
primitive, never a fork).

### K. Items 55–72 — Consistency Retrofit, Pervasive-Telemetry & Digital-Twin Arc (appended 2026-07-19, fifth wave — master-synthesis pass)

**Sources (six, merged by `MASTER-SYNTHESIS-CONSISTENCY-TELEMETRY-DIGITAL-TWIN-2026-07-19.md`):**
`AUDIT-SPACE-GRADE-CONSISTENCY-DEPLOYMENT-NATIVENESS-2026-07-19.md` (corrections applied above:
§0/§E item 12, item 7 annotation, item 31 `kernel::json` ticket, items 46/51 amendments, SYNTH
§6/§9/§11 and BP-27 §5 retro-corrections), `AUDIT-BINARY-VS-KLEENE-LOGIC-2026-07-19.md` (8
SHOULD-BE-3-VALUED findings / 27 keep-binary / 11 already-correct),
`AUDIT-TELEMETRY-EVERYWHERE-AI-OPTIONAL-OS-2026-07-19.md` (13 gaps G1–G13 + the work-normalized
cost ledger + the AI-optional P1–P5 proposals),
`RESEARCH-OS-ARCHITECTURE-PATTERNS-ADOPTION-2026-07-19.md` (3 adoptions + 1 small gap; category
mismatches ruled out), `RESEARCH-NATIVE-KANI-REPLACEMENT-FEASIBILITY-2026-07-19.md` (already
enacted as item 7's v2 rescope — no new item here, consistency confirmed above), and
`RESEARCH-RESOURCE-FOOTPRINT-ZERO-BLINDSPOT-RELATIONAL-TELEMETRY-2026-07-19.md` (threads 1–5:
derived footprint views, zero-UN-NAMED-blind-spots, FDR relational linkage, the 10-step
completeness procedure, the predictive-oracle principle + digital-twin split).

**Standing laws, same as §§H–J:** zero new external crates (empty-allowlist gate), item-6
hardening machinery (no parallel checklist), item-25 procedure for any dependency question,
item-27 P3-plane law for every telemetry value (excluded from all hash/gate/replay surfaces).
**New binding procedure for this arc:**
[`PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md`](PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md)
(item 57 ratifies it — the item-25 pattern). **Merged, not duplicated:** the temporal-TMR
adoption is item 12's §E re-scope (above), NOT a new number; item 7's rescope is enacted in §C.
**Named out-of-scope flags (recorded, not itemized):** audit-3 G10 (bebop-repo NTT wire-in has
zero perf telemetry — a `cycles-per-op` decision with no data; belongs to the bebop repo's own
lane) and G13 (`apps/api` Node latency telemetry — legacy surface, outside the kernel arc).
**Planning only — no item below starts before the operator dispatches it.**

- **Item 55 — K3 verdict-class retrofit across roadmap verdict surfaces (spec-level amendments
  in the item-50 shape; zero prerequisites, READY NOW — each amendment's code cost rides its
  host item's own build).** Applies the Kleene audit's remaining spec findings (1, 3, 4, 7, 8;
  finding 6 already applied in item 12's §E re-scope; findings 2/5 are item 56). The invariant
  shape for every one: **behavioral collapse to the safe pole KEPT, distinct typed cause ADDED
  to the record** — no third control-flow arm anywhere. (a) **Item 33:** per-number verdict
  becomes `{Confirmed(cmd), Refuted(cmd), Unresolvable(cause)}` — a claimed delta smaller than
  the bench's measured CI (the documented ±40% `fold_transitions` noise-bound vs +16.6% claim)
  is `Unresolvable`, recorded with measured CI + claimed delta side-by-side and a
  bench-stabilization ticket, never a manufactured CONFIRMED/REFUTED; MISSING→RED tracker
  semantics unchanged. (b) **Items 7/10/11:** Kani/TLC result artifacts carry per-target
  `{Proved, Refuted(cex), Undecidable(cause: bound/timeout/resource)}`; CI collapses
  Refuted|Undecidable → RED identically, but the class rides the job artifact — an exhausted
  bound needs a bound bump, a counterexample needs a code fix; conflating them mis-routes the
  response. (c) **Item 9 (+21 inherits):** `Tripped` carries
  `TripCause::{Exceeded(named-threshold), Unevaluable(Absence)}`, and the previously-unstated
  input policy becomes law: a trip predicate evaluating a `Reading::Unavailable` input takes the
  CONSERVATIVE pole (trip-eligible, never silently healthy), logged distinctly — the seam stays
  two-armed. (d) **Item 43:** the classification law gains its unstated third case —
  `Unclassifiable ⇒ treated as secret-adjacent` (mandatory dudect branch), recorded as its own
  classification value so the fail-closed default is visible. (e) **Items 6/43 dudect:** the
  recorded verdict becomes `{LeakFound, NoLeakAtSamples(n), Inconclusive(underpowered)}` with
  sample/class counts recorded — a green run is citable as "no leak detected at power N," never
  "CT proven"; Inconclusive ⇒ RED; the planted-leak positive control stays. (f) **Item 35
  (consistency note, no new state):** emitter refusal carries `{BoundViolated, BoundUnprovable}`.
  **Proof:** each host item's entry/blueprint text amended with the class enum + policy sentence
  (this item is DONE when the amendments are recorded and each host item's own proof section
  names the planted-class red→green obligation — e.g. item 33's results doc must contain at
  least the capability to record an `Unresolvable` row; item 9's blueprint must state the
  Unavailable-input policy before build). See
  [`BLUEPRINT-ITEM-55-k3-verdict-class-retrofit-2026-07-19.md`](BLUEPRINT-ITEM-55-k3-verdict-class-retrofit-2026-07-19.md).
- **Item 56 — kernel classifier epistemic-basis retrofit: `markov::Verdict` fail-open record +
  `spectral::DriftClass` conflated record (code; Kleene audit findings 2 + 5 — the only
  fail-open-to-lenient instance found, and its fail-closed sibling).** Behavior and wire
  contracts are KEPT in both cases; only the record gains a basis. **(a) markov (the headline):**
  `analyze_detailed` maps window-too-short ⇒ `Healthy` (`markov.rs:110`) and
  `markov_attractor.rs:36` maps analyzer-error ⇒ `"HEALTHY"` — Unknown emitted at the MOST
  lenient pole, and item 27's FDR record carries only `verdict_str()` so "couldn't analyze" is
  byte-identical to "measured healthy" in telemetry. Fail-open stays (advisory hook — no
  evidence ⇒ no intervention is the right behavior); ADD a typed basis
  (`Basis::{Measured, WindowTooShort, AnalyzerError}`) on `Report` — NOT a fourth `Verdict`
  variant (CLI JSON is golden-pinned byte-identical) — and an optional basis field on
  `emit_verdict_pmu`'s FDR record (item-27 optional-field discipline). Downstream law: items
  9/21 must never count an unevaluated-Healthy window as health evidence. **(b) spectral:**
  `classify_drift` collapses three cannot-evaluate causes (non-finite entries, ragged matrix,
  checked-constructor Err) into `Unstable` — the fail-closed collapse is correct and stays, and
  the pinned `wire_code` 0/1/2 makes a fourth variant wrong; ADD out-of-band provenance
  (`DriftBasis::{Measured, IllFormedInput(cause)}` via the `classify_drift_with_rho` report path
  / item-27-style optional FDR companion) so forensics can separate a genuinely diverging loop
  from NaN-poisoned input. **Prereqs:** none for the pure-kernel halves (`markov.rs`/
  `spectral.rs` live on main); the FDR-field halves join after the exec-branch FDR merge.
  **Proof:** markov CLI JSON goldens byte-identical before/after (the pinned contract is the
  regression test); a forced short-window run and a forced analyzer-error run each yield
  `Healthy` + the correct distinct basis in the FDR record (red→green: today they are
  byte-identical to measured-healthy); spectral: a NaN-poisoned matrix and a genuinely-divergent
  matrix both classify `Unstable` with distinct recorded bases; `wire_code` round-trip test
  untouched and green. See
  [`BLUEPRINT-ITEM-56-classifier-epistemic-basis-retrofit-2026-07-19.md`](BLUEPRINT-ITEM-56-classifier-epistemic-basis-retrofit-2026-07-19.md).
- **Item 57 — telemetry-completeness standing procedure RATIFIED + HOT-PATHS accounting columns
  (the enforcement spine of this arc; zero prerequisites — the procedure doc exists as of this
  pass).** The item-25 pattern replayed: the 10-step (+3 cost-oracle steps) procedure in
  `PROCEDURE-TELEMETRY-COMPLETENESS-STANDING-2026-07-19.md` becomes BINDING for every future
  blueprint in this arc once the operator ratifies it. Mechanical half: extend
  `docs/audits/hardening/HOT-PATHS.tsv` with an `eff` column — every hot-zone row must either
  name its workload-kind/span or carry a ledgered `gap:` reason (the item-6 gate mechanism,
  extended not replaced), and every function in a hot zone is classified
  `INSTRUMENTED | CHEAP(SamplingDisabled) | EXCLUDED(reason)`. This is the honest form of the
  operator's "enforced everywhere": **zero UN-NAMED blind spots** — 100% coverage of the
  *accounting*, with the impossibility triangle (100% stamps ∧ zero cost ∧ deterministic replay)
  stated rather than violated. Also rules on audit G9: the cheap-path FDR envelope (one relaxed
  atomic load when disabled) is the always-compiled floor; heavy stamps stay feature-gated —
  recorded as the standing posture. **Proof:** procedure doc cross-linked from
  `docs/audits/hardening/CHECKLIST.md`; the extended gate goes RED on a hot-zone row carrying
  neither an `eff` value nor a `gap:` reason (planted-row red→green, anti-forgery clause
  reused); the G9 ruling recorded in the procedure doc + `fdr/mod.rs` when next touched. See
  [`BLUEPRINT-ITEMS-57-58-telemetry-completeness-cost-ledger-2026-07-19.md`](BLUEPRINT-ITEMS-57-58-telemetry-completeness-cost-ledger-2026-07-19.md).
- **Item 58 — work-normalized cost ledger (after item 57 + the exec-branch FDR merge; audit-3
  §1.3 design adopted).** On `SpanClose`-class FDR records for a named workload: emit
  `(work: {kind, Δcount}, cost: HwStamp-delta ⊕ PmuStamp-delta)` — **pairs of raw u64, never
  ratios** (the landed losslessness law; ratios are a consumer concern). Closed workload-kind
  enum seeded from work units that already exist: `DecisionUnitsImported`, `FdrRecordsAppended`,
  `TransitionsFolded`, `TokensGenerated`, `FramesRendered`, `EigensolvesCompleted`,
  `SignaturesVerified`. Degradation ladder self-describing per field via `Reading<T>`: Tier E
  (per-joule, RAPL hosts), Tier C (per-cycle/instruction, PMU hosts), Tier T (per-tick/wall —
  the tier this dev host actually runs at, honest not aspirational); a cross-tier efficiency
  comparison is structurally UNCOMPUTABLE (absent counters are absent), and on hosts where C and
  T are both live, work/cycles vs work/ticks must agree within a stated band — a free self-test
  of the counters. **Proof:** schema tests + named-absence serialization proof (the literal
  `unavailable` reason greppable on this RAPL-less/paranoid host — procedure step 10's
  red→green); the pair-not-ratio law asserted structurally (no ratio field exists in the
  schema); the cross-tier consistency band test green where both tiers are live; first consumer
  deployments = items 59–61. See
  [`BLUEPRINT-ITEMS-57-58-telemetry-completeness-cost-ledger-2026-07-19.md`](BLUEPRINT-ITEMS-57-58-telemetry-completeness-cost-ledger-2026-07-19.md).
- **Item 59 — agent-turn timing closure (gaps G1+G2+G12 — the highest-leverage single gap:
  tokens are already counted, wall-clock is one `Instant` pair away; after item 58).** (a) The
  kernel LLM port (`ports/llm.rs`) `ChatResponse` gains a duration/TTFT surface (additive typed
  field or timing companion — the port contract can currently not transport latency even where
  adapters measure it); (b) `agent-loop`'s host binary times each turn (it bypasses the ONE
  timed path, `Dispatcher`'s `ms`, by driving `OllamaAdapter` directly) and folds per-turn
  Δwall + Δticks alongside the existing token counts into `track_record.jsonl`; (c) the kernel
  agent executor (`kernel/src/agent/loop.rs`) records per-iteration timing at span granularity.
  Workload-kind: `TokensGenerated`. **Proof:** a live loop run yields track-record entries
  carrying both tokens and duration for the direct-adapter path (parity with the Dispatcher
  path's existing `ms`); tokens/sec derivable consumer-side from one record's raw pair; an
  LLM-absent turn records a named absence, never a fabricated 0; existing golden/track-record
  consumers unbroken (additive-field discipline). See
  [`BLUEPRINT-ITEM-59-agent-turn-timing-closure-2026-07-19.md`](BLUEPRINT-ITEM-59-agent-turn-timing-closure-2026-07-19.md).
- **Item 60 — engine frame-loop + voice instrumentation (gaps G3+G11; after item 58; engine
  currently has ZERO `Instant::now` — grep-verified).** (a) `EngineLoop::frame()` measures
  frame time against a NAMED frame-budget constant (one authority site + pin test — P3 rate
  discipline); `FrameProfiler` gains time alongside its call counts; workload-kind
  `FramesRendered`. (b) `voice.rs`: `WakeWordSpotter`/`AsrModel::feed` latency measured — the
  module carries an explicit "battery lever" efficiency claim with zero measurement, and
  `InferError::Timeout` exists with no timer feeding it; wire the timer. (c) All engine timing
  must state its wasm leg per procedure step 9 (native `Instant` / wasm `performance.now`
  import or named absence — coordinates with item 62's wasm clause, one design not two).
  **Proof:** frame-time p50/p99 emitted under the telemetry feature with a budget-breach test
  (planted slow frame flagged); `InferError::Timeout` demonstrably reachable from the real
  timer (red→green — today it is dead); the budget constant pinned; wasm cdylib stays green. See
  [`BLUEPRINT-ITEM-60-engine-frame-voice-instrumentation-2026-07-19.md`](BLUEPRINT-ITEM-60-engine-frame-voice-instrumentation-2026-07-19.md).
- **Item 61 — kernel runtime-counter closure: durability, subprocess, eigensolver, crypto spans
  (gaps G5+G6+G7+G8; after item 58).** (a) `EventLog::append`/`FileEventStore::insert` gain
  continuous counters (events + Δticks + fsync count) — item 26 measured 637 µs p50 once at
  bench time, but the operator-gated 53× group-commit decision has NO ongoing data feed;
  workload-kind `FdrRecordsAppended`/events. (b) `living_knowledge.rs` subprocess spawns record
  duration + exit rusage (`wait4`) + an FDR record — a hung/expensive child is currently
  invisible to FDR (adjacent to item 48's liveness class, composes with it). (c)
  `spectral.rs`/`householder.rs` join the span roster — HOT-PATHS zones with no runtime spans;
  workload-kind `EigensolvesCompleted` (cycles/eigensolve is the cleanest Tier-C efficiency
  metric in the kernel). (d) Fix the `mldsa_verify` span double-gating (`telemetry` AND `pq`):
  a `pq`-only production build currently has zero crypto latency telemetry — either the span
  compiles under `pq` alone or the gap is ledgered in HOT-PATHS as an explicit `gap:` row (no
  silent dark zone); workload-kind `SignaturesVerified`. **Proof:** counters recoverable from
  the FDR ring after N appends in a test; child-process record carries real rusage (planted
  slow child observable); eigensolver spans emit under load with HOT-PATHS `eff` rows filled;
  the pq-only build either emits crypto spans or carries the ledgered gap row (gate-checked). See
  [`BLUEPRINT-ITEM-61-kernel-runtime-counter-closure-2026-07-19.md`](BLUEPRINT-ITEM-61-kernel-runtime-counter-closure-2026-07-19.md).
- **Item 62 — FDR relational linkage: `span_id` + `parent_span_id: Reading<u64>` + the wasm
  clock leg (gaps: doc-6 thread 3's decisive finding + G4; after the FDR merge; parallel with
  item 58).** The FDR schema is FLAT/UNLINKED today — grep over `schema.rs` for
  parent/trace/span/caller = zero hits; `seq` conveys temporal succession, never causal
  parentage. Extend (never replace) the envelope on the P3 plane: `span_id: u64` (per-process
  counter) + `parent_span_id: Reading<u64>` with `Unavailable(NoParent)` at a root — the
  named-absence doctrine covering "this is a root," no magic 0, no missing key. Cross-process
  edges (subprocess spawns, agent↔LLM boundary) seed the parent id across the boundary — OTel
  propagation reduced to passing one u64. Cost honest: ~16 bytes + a counter increment, P3 so
  it never touches determinism. The wasm leg (G4): `FdrEvent::stamp` is cfg'd off wasm because
  `Instant` panics there — this item states the wasm-safe clock (`performance.now()` import) or
  the named `Absence` reason for the 24 wasm pub fns; the FDR plan may no longer structurally
  EXCLUDE the wasm surface silently. **Proof:** nested spans reconstruct a correct call tree
  from a recovered ring (test walks parent links); root records carry the literal `NoParent`
  reason (greppable); records on surfaces without linkage stay byte-identical (optional-field
  discipline); the P3 grep proof (no span id feeds any hash/gate/replay surface) green; wasm
  cdylib green with the stated clock or named absence. See
  [`BLUEPRINT-ITEM-62-fdr-relational-linkage-2026-07-19.md`](BLUEPRINT-ITEM-62-fdr-relational-linkage-2026-07-19.md).
- **Item 63 — item-45 spec extension: AI-boundary disposition table + build-provenance record +
  feature-matrix legs (audit-3 §2.3 P2/P4/P5 adopted; P3's reject-list endorsed as correct, not
  deferral; spec-level now, teeth when item 45 lands; audit-3 P1 — "dispatch item 45 now, it is
  READY-NOW and converts safe-by-convention into safe-by-gate before items 33–44 create real
  risk" — is recorded here as an operator-dispatch recommendation).** (a) Disposition table over
  the pre-existing surfaces item 45's spec is silent on: `{micrograd, online, attention, evals,
  ports/llm, ports/agent, agent/, engine/voice.rs}` → each classified CORE-DETERMINISTIC
  (`attention` — it is math, no learned weights), AI-EDGE (moves behind `inference` when it
  lands — `micrograd`/`online` are the candidates; undefined = grandfathered leak), or
  SANCTIONED-SEAM (trait-only always-compiled ports — the syscall-interface shape, named as
  legal so the gate's grep can distinguish a seam from a violation); the gate's scope clause
  extends to the engine's `voice`/`inference` firewall (currently outside it entirely). (b) One
  startup `Kind::Event` FDR record naming the compiled feature set (`inference` on/off, `pq`,
  `telemetry`, …) — forensics can tell an AI-absent binary from an AI-present one from the
  flight recorder alone; pairs with item 48's heartbeat. (c) Feature-matrix CI legs: `default`
  AND `default+inference` compile + full suite on every PR once the flag exists — the absent
  leg stays green forever, not only at gate-landing. **Proof:** the table recorded in item 45's
  spec + the named modules' docs; a planted core→AI-EDGE reference RED under the extended gate
  (P7); the provenance record recovered from a real ring in a test; both matrix legs green in
  CI when the flag exists. See
  [`BLUEPRINT-ITEM-63-ai-boundary-disposition-2026-07-19.md`](BLUEPRINT-ITEM-63-ai-boundary-disposition-2026-07-19.md).
- **Item 64 — capability-secure declarative composition root (the strongest OS-pattern
  adoption — the only one backed by a PROVEN defect: item 2's finding that NO production
  composition root constructs the durable store; SUBSUMES the
  `BLUEPRINT-P-FILE-EVENT-STORE-WIRING-GAP` Tier-1 fix; Tier-1-class build, dispatchable
  now).** A declarative, dependency-ordered init for the host binaries replacing today's flat
  ad-hoc `main()` wiring: (i) explicit init order derived from a declared module-dependency DAG,
  validated by the EXISTING `order_machine` proof kit (`has_cycle`/`topological_order` reused
  over module-init nodes — a cyclic init dependency is a caught startup error, not a runtime
  surprise); (ii) each module declares the ports/capabilities it requires and FAILS CLOSED if
  one is absent (generalizing `isolation/microvm.rs`'s refuse-the-adapter pattern from
  deployment gating to module init); (iii) the root constructs the durable
  `FileEventStore`/`EventLog` (closing item 2's defect at last), performs the FDR
  recover-readback before normal operation begins (item 48's declared place to live), and is
  the SOLE MINTER of item 65's in-process capability tokens (seL4's "init task holds all
  capabilities and delegates," sized to one process). **Proof:** a cited line in a production
  binary constructing the durable store — item 2's original proof condition, finally
  dischargeable; a planted cyclic init declaration fails at startup with a typed error
  (red→green); a module with an absent declared capability refuses init fail-closed (test);
  a permuted declaration order yields the identical derived init sequence (order comes from the
  DAG, not source order); kill-9 recovery test still green through the new root. See
  [`BLUEPRINT-ITEM-64-composition-root-2026-07-19.md`](BLUEPRINT-ITEM-64-composition-root-2026-07-19.md).
- **Item 65 — typed in-process AI/agent capability boundary (extends item 45; tokens minted
  ONLY by item 64's root; after items 64 + 45; the proportionate seL4 slice — ~70% was already
  scoped by item 45 + the Wasmtime-fuel pattern, this is the new ~30%).** A zero-sized
  unforgeable capability type (constructible only by the composition root) that the AI/agent
  subsystem must present BY SIGNATURE to call a kernel port — `cap: &CoreWriteCapability` makes
  authority-to-touch-the-deterministic-core illegal-state-unrepresentable at the call site;
  strictly additive over item 45 (45 stops cross-references at compile time; this also stops
  runtime authority a compiled-in-but-untrusted path might exercise). Reuses the existing
  `capability_cert.rs` attenuation/scoping machinery internally — no new crypto, no new
  dependency, no memory-capability system invented. Includes the OTP-slice companion: a uniform
  per-port fail-closed containment property test (one failing/panicking adapter cannot escalate
  past its own port boundary — asserted across every port, not left per-port convention;
  composes with item 9's breaker as the containment receiver). **Proof:** a compile-fail test
  proves a capability-requiring port method is uncallable from code never handed the token; the
  token's only constructor site is the composition root (visibility + grep proof); the per-port
  containment property test green across all `ports/` seams; `cargo tree -e no-dev`
  byte-unchanged. See
  [`BLUEPRINT-ITEM-65-typed-capability-boundary-2026-07-19.md`](BLUEPRINT-ITEM-65-typed-capability-boundary-2026-07-19.md).
- **Item 66 — periodic durable-log scrub (the one small journaling-FS gap; gated on item 64 —
  scrubbing an unwired store is pointless; composes with item 54's integrity-alarm seam).**
  ZFS-scrub slice only: an idle-cadence pass walking the durable EventLog + closed FDR
  segments, re-verifying the EXISTING CRC32/SHA3 checksums to catch latent at-rest bit-rot
  before a read needs the data (on non-ECC local storage, proportionate defense-in-depth); any
  mismatch emits one FDR `Alarm` (hardware-fault evidence, item-40 semantics). No new
  primitive, no new dependency; the scrub cadence is a NAMED constant with one authority site
  (P3 rate discipline). **Proof:** a planted at-rest corruption in a closed segment is detected
  by the next scrub pass and writes the `Alarm` (red→green, P7); an uncorrupted store scrubs
  silent; cadence constant pinned; `cargo tree` unchanged (grep: existing CRC32/SHA3 only). See
  [`BLUEPRINT-ITEM-66-durable-log-scrub-2026-07-19.md`](BLUEPRINT-ITEM-66-durable-log-scrub-2026-07-19.md).
- **Item 67 — cost-oracle classification backfill: COVERAGE-COMPLETE, PRECISION-HONEST (after
  item 57; the named principle from doc 6 §5.2 made mechanical).** Literal "100% correct cost
  prediction for any code" is undecidable (WCET reduces to halting); the honest achievable form
  is 100% *classification* coverage: EVERY `HOT-PATHS.tsv` row (and every future row,
  gate-enforced) carries a bucket — `ORACLE-EXACT` (input domain enumerated or cost provably
  input-independent; evidence = the enumeration/CT proof), `ORACLE-BOUNDED` (fixed operation
  schedule; evidence = the analytic `[min,max]` derivation), or `MEASURED-ONLY` (genuinely
  dynamic/I/O/probabilistic; evidence = p50/p99/CI + methodology) — with a traceable evidence
  pointer per row; *unclassified* is the one forbidden state. Seeded from doc 6 §5.5's
  grounded sample (FSM 144-transition table → EXACT; `ct_eq` inherits EXACT from its dudect
  proof — the CT property IS the cost-constancy property, free; `ntt`/`invntt`/`householder` →
  BOUNDED via fixed schedules; `eigh` iterative QR + event-log fsync + subprocess/agent/AI →
  MEASURED-ONLY, item 26's 637 µs distribution as the exemplar). Reuses the Kani-feasibility
  B/C split as ready-made evidence (Bucket B → EXACT, Bucket C → BOUNDED); the kernel's hot
  core is dominated by EXACT/BOUNDED with MEASURED-ONLY confined to I/O+subprocess+AI — the
  backfill is tractable, not boil-the-ocean. **Proof:** zero unclassified rows in the extended
  TSV; the gate goes RED on a new hot-zone row without a bucket (planted-row red→green); every
  evidence pointer resolves to a real test name / derivation section / measurement doc
  (spot-check re-executed, never presence-checked — P7). See
  [`BLUEPRINT-ITEM-67-68-69-cost-oracle-2026-07-19.md`](BLUEPRINT-ITEM-67-68-69-cost-oracle-2026-07-19.md).
- **Item 68 — ORACLE-EXACT/BOUNDED cost capture as a correctness-proof byproduct (after item
  67 + item 7's native exhaustive sweeps; doc 6 §5.3's decisive reuse).** The same structural
  property that makes correctness exhaustively provable makes cost exactly knowable — so
  capture it in the SAME pass, never a separate harness: (a) add Tier-A `rdtsc` cycle capture
  (reusing `fdr/pmu.rs`'s reader) to item 7's Bucket-B exhaustive `#[test]` sweeps, folding to
  a single constant/tight interval where control flow is input-independent (all the
  straight-line crypto reductions) and to a complete per-input cost table otherwise; (b) derive
  analytic `[min,max]` intervals for the Bucket-C fixed-schedule functions (8-layer/1024-
  butterfly, 24 Keccak rounds — the WCET-decidable straight-line subclass, the butterfly-lemma
  induction reused for cost); (c) MEASURED-ONLY surfaces report p50/p99/CI, never a fabricated
  point estimate. **Honest caveat carried verbatim:** even ORACLE-EXACT yields measured cycles
  with host noise — the claim is "input-dependence of cost fully characterized," absolute
  cycles remain a per-host interval; precision-honest at the exact end too. **Proof:** a
  generated cost table/constant per classified function with its stated noise interval,
  recorded as evidence behind item 67's rows; an input-independence assertion for EXACT
  functions (cost class identical across the swept domain); the P3 grep proof that no captured
  cost value feeds any decision/gate surface. See
  [`BLUEPRINT-ITEM-67-68-69-cost-oracle-2026-07-19.md`](BLUEPRINT-ITEM-67-68-69-cost-oracle-2026-07-19.md).
- **Item 69 — water/carbon as derived, constant-multiplied views of joules (small standalone;
  after item 58; doc 6 thread 1 — the honest form of "atoms/molecules/water/air").** The
  kernel needs NO new *measured* footprint field beyond `joules_uj` — "atoms/molecules
  consumption" honestly IS silicon power draw, i.e. joules, and item 27's RAPL/PMU work already
  is that mechanism. Build the consumer-side conversion table keyed on operator-supplied
  `(region, deployment-class)` constants: `co2e = joules × grid-carbon-intensity` (gCO₂e/kWh),
  `off-site water = joules × WUE-source` (L/kWh) — each a `Reading<T>` degrading to a named
  absence when joules is absent OR the regional constant is unsupplied; **on-site water is a
  PERMANENT named absence** on a local device (a facility cooling property software cannot
  observe — fabricating litres is a standard violation, procedure step 4); adding raw
  `water_ml`/`co2e` fields to `HwStamp` is likewise a violation. Lights up automatically on a
  RAPL-capable deploy with zero schema change. **Proof:** derivation golden tests against
  hand-computed values; on this RAPL-less host every derived view serializes the literal
  `unavailable` reason (greppable — procedure step 10's red→green); the on-site-water absence
  is unconditional by construction (no code path can produce a value); the SCI-rate
  (ISO/IEC 21031) pairing note recorded for ratio consumers. See
  [`BLUEPRINT-ITEM-67-68-69-cost-oracle-2026-07-19.md`](BLUEPRINT-ITEM-67-68-69-cost-oracle-2026-07-19.md).
- **Item 70 — state-mirroring digital twin, half (A) — REAL, NEAR-TERM (after items 67 + 68;
  call matrix fed by item 62; doc 6 §5.7(A)).** NOT a new subsystem: the twin is the
  COMPOSITION of three already-real/already-scoped pieces — (i) the per-function cost oracle
  (item 67's buckets + item 68's tables/intervals/distributions); (ii) the aggregate call-graph
  layer reusing `spectral.rs`/`markov.rs`/`csr.rs` AS-IS: ρ(A) of the frequency-weighted call
  matrix decides whether total propagated cost converges (`c = (I−A)⁻¹·c_self` — the existing
  `classify_drift` `Damped/Resonant/Unstable` enum applied to the call matrix, zero new
  machinery), Laplacian diffusion for where cost concentrates (bottlenecks), `markov::analyze`
  over discretized cost-tier tokens for resource-regime drift; (iii) the `eqc-rs` precedent
  (equation → proven-faithful Rust mirror) as the template that "real behavior mirrored by real
  math" already works here. **Forced-metaphor guard, binding (Anu/Ananke — carried exactly):**
  the spectral machinery answers GRAPH-level questions only (convergence, flow, bottleneck,
  drift); per-leaf cost comes from enumeration/interval ONLY — the twin must never present a
  spectral quantity as an individual function's cycle count. Deliverable: given (action,
  inputs) → its bucket + value/interval/distribution + evidence pointer, and (via ρ(A)) the
  propagated aggregate answer. **Proof:** coverage-complete over every HOT-PATHS action (an
  unclassifiable query returns the forbidden-state error, never a guess); a differential check
  on ORACLE-EXACT functions (twin's stated cost class matches a fresh measurement within the
  stated noise interval); ρ(A) verdict validated on a synthetic recursive call graph with known
  divergence (red→green both directions); the forced-metaphor guard asserted structurally (no
  per-leaf API derives from spectral values — reviewed + doc-ruled, grep-checkable naming). See
  [`BLUEPRINT-ITEM-70-71-72-digital-twin-2026-07-19.md`](BLUEPRINT-ITEM-70-71-72-digital-twin-2026-07-19.md).
- **Item 71 — cost-aware eqc-rs rewrite-extraction (half (B′) — the ONE honestly-scoped
  near-term step toward (B); independent of items 67–70; operator-gated whether to build at
  all — offered as the smallest grounded step, not a commitment).** Give eqc-rs codegen a
  cost-aware extraction over a SMALL, HAND-CURATED, FINITE set of provably-equivalent algebraic
  rewrites — strength reduction (`a*2 → a+a`), factoring (`a*b + a*c → a*(b+c)`), constant
  folding — choosing the cheaper form by lower op-count at codegen time, and REUSING the
  existing `emit_proof_program` to prove the chosen form still equals the `Expr::eval`
  reference. Equality-saturation's "extraction picks the cheapest equivalent" idea at toy scale:
  **no e-graph, no SMT, no SAT, zero new dependency** — honestly "constant folding plus
  strength reduction with a proof," NOT a superoptimizer, and it must never be described as
  one. **Proof:** per rule, an emitted case where the cheaper form is demonstrably chosen with
  its proof program green (compiled by real rustc, self-asserting); a no-rule-applies case
  emits unchanged output byte-identical to today's; the op-count cost metric documented in the
  eqc-rs README; the full eqc-rs suite green; `cargo tree` unchanged. See
  [`BLUEPRINT-ITEM-70-71-72-digital-twin-2026-07-19.md`](BLUEPRINT-ITEM-70-71-72-digital-twin-2026-07-19.md).
- **Item 72 — auto-optimizing digital twin, half (B) — LONG-TERM ASPIRATION, EXPLICITLY NOT
  PROMISED (named so the direction is on the roadmap without over-promising; doc 6 §5.7(B)).**
  "Always finds a shorter/faster version of any action" is automated superoptimization — a
  real, hard, active research field (STOKE stochastic search, Souper SMT synthesis, egg/egglog
  equality saturation with cost-model extraction), and its machinery (exponential search
  spaces, e-graph/SMT engines) is antithetical TODAY to a zero-dep deterministic kernel. This
  item carries **no proof conditions and no schedule** — deliberately. Instead it records its
  ENTRY CRITERIA, all three required before any work: (i) item 71 landed with measured wins
  demonstrating extraction value on real kernel math; (ii) an explicit operator ruling
  accepting the tooling/determinism cost for a bounded target domain; (iii) a fresh research
  pass (this item is a pointer, not a plan). Until then: named direction, zero commitment —
  the honest opposite of a fabricated roadmap promise. See
  [`BLUEPRINT-ITEM-70-71-72-digital-twin-2026-07-19.md`](BLUEPRINT-ITEM-70-71-72-digital-twin-2026-07-19.md).

**Dependency graph, one line:** 55 ∥ 56 ∥ 57 ∥ 63 ∥ 64 ready now (56's and 58–62's FDR-field
halves inherit the exec-branch FDR merge, same as §J's flag); 58 after 57; {59 ∥ 60 ∥ 61} after
58; 62 parallel with 58 (both extend the envelope, coordinated in one schema change); 65 after
{64 + 45}; 66 after 64; 67 after 57; 68 after {67 + item 7's native sweeps}; 69 after 58; 70
after {67 + 68} with 62 feeding its call matrix; 71 independent + operator-gated; 72
aspirational, entry-gated on {70, 71, operator ruling}. Temporal TMR = item 12 (§E, merged —
no number here); item 7's rescope = §C (enacted, no number here). Nothing in §K gates any
§§H–J item.

### L. Items 73–78 — Governed Self-Evolution Arc: AI-Proposed Change Governance (self-healing + self-upgrading under an untouchable human gate) — appended 2026-07-19, sixth wave

**Source:** operator directive (verbatim, three parts, recorded 2026-07-19): *"allowing & making
possible for the ai to build & change internal os, but without touching the core & red lines/gates
— but only with manual approvals/prompts/injections from the human operator with the same amount
level of predictability/space quality/transparency/control/telemetry"*; *"track own health too, to
make fixes, self-healing allowed, this should be a regenerative software based on safe
conditions/gates/invariants/human operator decisions"*; *"so both self-healing and self-upgrading —
only with the human operator decisions/approval and inability to change this or bypass the human
gate & other red lines gates."* This is a PRODUCT capability of the dowiz kernel/OS itself, not a
statement about this development session. It generalizes item 47's `Proposal → admit() →
ValidatedProposal` pattern (runtime ADVICE plane) to the CODE/architecture plane, and composes
with — never duplicates — items 45 (AI-optional gate), 50 (K3 admission), 64/65 (composition root
+ typed capability boundary), 62 (relational lineage), 67/68/70 (cost oracle), 75←eqc-rs
(pre-verified generation precedent). **Same standing laws as §§H–K. Planning only — no item
starts before the operator dispatches it.**

**The one governing invariant (everything in §L is subordinate to item 73):** the human-approval
gate, the red-line registry, and the verification machinery are themselves CORE — permanently
outside the reach of the very pipeline they govern. No sequence of individually-plausible approved
proposals may ever reach them. This is the corrigibility / tamper-proof-oversight problem from the
AI-safety literature, addressed structurally, not by policy.

- **Item 73 — the Gate-Root Invariant: the governance mechanism is structurally unmodifiable by
  what it governs (FIRST and foundational; spec-level now, structural enforcement lands with
  items 64/65 + 74; everything else in §L is subordinate to this holding).** Three structural
  clauses, none of them "a rule someone follows": **(i) Root placement.** The red-line registry
  (item 74) + the gate-enforcement code live at/behind item 64's composition root — the sole
  capability minter. **No capability type granting write access to the root, the registry, or the
  gate code EXISTS in the type system** — not "is never handed out": is unconstructible
  (illegal-state-unrepresentable, the §1.5 house standard applied to the meta level). By
  construction, nothing downstream of the root — including any AI-generated proposal however it
  entered — can hold authority to touch the root itself (doc-4's seL4 "init task holds all
  capabilities" pattern, completed: the root delegates everything EXCEPT authority over itself).
  **(ii) Dependency-direction check, item-45 style.** A CI check (same planted-reference
  red-proof discipline) asserts no module reachable from the proposal pipeline references any
  gate/registry mutation surface — trivially checkable because no such mutation surface is
  `pub`: the registry is static data + code with NO runtime mutation API at all. **(iii)
  Out-of-band-only modification.** Changing the gate, the registry, or the verification logic
  happens ONLY through the normal human engineering process (a human directly editing code,
  review, the full CI gate suite) — NEVER through the proposal pipeline, even nominally, even
  "with extra approvals": a proposal whose diff touches a red-line path is REFUSED at
  classification time (step zero — before verification, before any human is asked; it is a
  category error, not a reviewable request). No "beneficial change to the gate" exception
  exists — that exception IS the corrigibility trap (a system able to modify its own safety
  constraints can eventually remove them through individually-plausible steps). **Proof:** a
  planted proposal targeting the gate/registry/red-line paths is refused with a typed red-line
  rejection + FDR record, red→green (P7); a compile-fail test proves no gate-mutation
  capability constructor exists; the dependency-direction CI check goes RED on a planted
  pipeline→gate reference; the out-of-band-only law recorded in the registry's module doc +
  `docs/audits/hardening/CHECKLIST.md`; item 74's registry carries this section's own machinery
  as rows (the recursion recorded, not implied). See
  [`BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`](BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md).
  **Status correction (2026-07-20, `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md`):** `scripts/gate-root-invariant.sh`
  exists on `main` (real logic, landed via `cb00706b1`) but is **not wired into any CI workflow** —
  clause (ii)'s "CI check" is written but not yet live.
- **Item 74 — the red-line/core registry: grounded machine-readable enumeration of what AI
  proposals may NEVER touch (after item 73's spec; the safety-critical deliverable of this arc —
  every row cited, zero hand-waving).** Format: a machine-read manifest in the `HOT-PATHS.tsv`
  idiom (path-prefix zones → deterministic diff classification), each row carrying WHY + the
  real source. Seed rows, grounded against this project's actual standing red-lines and this
  roadmap's own proof machinery: **(1) Product red-lines** (standing rules, memory
  `test-integrity-rules-2026-06-27` + the SELF-DEVELOPMENT charter's "red-lines
  (money/auth/RLS/migrations/bulk) preserved even under full autonomy"): `money.rs`
  (integer-only checked arithmetic), auth/capability surfaces, pgrust migrations, bulk-op
  paths. **(2) Proven-FSM core:** `order_machine.rs` + its `GOLDEN_SIGNATURE` drift gate and
  const-proof kit (`FSM_SPECTRAL_RADIUS`) — the kernel's exemplar proof surface. **(3) The
  verification seam:** `decision/import.rs` `import_unit` replay-before-persist gate; item 47's
  `admit`/`Invariants` + item 50's K3 fold when they land. **(4) The forensic truth surface:**
  `event_log.rs` (SHA3 hash chain) + `kernel/src/fdr/` (ring, schema, CRC) — the evidence trail
  must never be modifiable by the subject of the evidence (item 76 depends on this being
  trustworthy). **(5) Crypto:** `pq/` (KAT-gated ML-DSA/ML-KEM/Keccak/x25519), `ct_gate.rs`,
  `capability_cert.rs`. **(6) The proof machinery itself:** `scripts/zero-dep-gate.sh` + every
  `ZERO-DEP-ALLOWLIST.txt`, `scripts/hardening-gate.sh` + `CHECKLIST.md`/`HOT-PATHS.tsv`,
  `rust-toolchain.toml` + the bump gate, the miri/lint/ai-optional gates, the `ci.yml` gate
  jobs. **(7) Safety machinery as it lands:** item 9's breaker, item 54's Sentinel, items
  64/65's composition root + capability types. **(8) §L's own gate code + this registry**
  (item 73's recursion, recorded as rows). Registry changes are themselves out-of-band-only
  (item 73(iii)); row REMOVAL additionally requires an explicit operator ruling recorded in the
  row's history (shrink is the dangerous direction — the allowlist monotonicity insight,
  inverted). **Proof:** the registry exists with every row justified + cited; the diff
  classifier maps a change-set → touched-red-line verdict deterministically (path-prefix
  zones); one planted diff per row CLASS demonstrably refused (P7); the registry file is
  itself in the registry (self-row test). See
  [`BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`](BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md).
  **Status correction (2026-07-20, `ROADMAP-BLUEPRINT-GAP-AUDIT-2026-07-20.md`):** `scripts/red-line-classifier.sh`,
  `scripts/red-line-monotonicity.sh`, and `scripts/verify-item-74.sh` all exist on `main` (real
  logic, landed via `cb00706b1`) but **none are wired into any CI workflow** — the diff-classifier
  proof above is written but not yet a live gate.
- **Item 75 — the change-proposal pipeline: typed `ChangeProposal` → kernel-gate verification →
  explicit human approval → recorded application (after items 73+74; extends item 47's grammar
  to the code plane — same shape, different plane, no parallel admission concept).** An
  AI-generated change (from items 33–44's inference subsystem or the agent surface) is DATA: a
  typed `ChangeProposal` carrying the diff/artifact + its trigger + its claimed intent. Step
  zero: item 74's classifier — a red-line touch is refused before anything else runs (73(iii)).
  Verification: the kernel's OWN existing gates re-executed on the proposed state — zero-dep
  gate, hardening-gate rows, full suite, miri where applicable, the item-44 CI discipline —
  never a parallel/lighter checklist; a proposal failing any gate NEVER reaches a human
  (machines filter, humans decide). Pre-verified generation is the preferred arrival shape:
  the `eqc-rs` precedent (equation → generated Rust + `emit_proof_program` self-assertion)
  means a proposal can arrive as a PROVEN artifact rather than raw untrusted code — reuse it,
  don't invent a second generator discipline. Then the hard gate: **an explicit human "apply"
  action is required for every application — no autonomous apply path exists** (structurally:
  the apply function requires a human-approval token only the operator's out-of-band action
  mints — the item-65 capability shape reused at the approval seam); absence of approval is a
  permanent pending state that expires, silence is never consent. Admission grammar = items
  47/50 verbatim: `admit(ChangeProposal, …) -> Result<VerifiedChangeProposal, Rejection>` with
  `RejectionClass::{Refuted, Undecidable}` (+ the named red-line cause riding `Refuted`);
  Kleene-Unknown collapses to the safe pole (not-applied), logged distinctly. **Proof:** a
  planted valid proposal passes all gates and STOPS at pending-approval — a red-proof
  demonstrates no code path applies it without the human token (unconstructible, compile-fail
  test); a planted gate-failing proposal never surfaces for approval; a planted red-line
  proposal is refused at step zero with the typed cause; approval/refusal/expiry each write
  FDR records; the whole flow re-executed in CI, never presence-checked (P7). See
  [`BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`](BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md).
- **Item 76 — proposal lineage + cost-classified impact at the approval seam (after item 75;
  consumes items 62 + 67/68; "the same amount of predictability/transparency/telemetry" made
  mechanical).** Every proposal carries a full FDR-logged causal trail, linked by item 62's
  `span_id`/`parent_span_id`: trigger (health verdict / operator prompt / upgrade candidate) →
  generation → per-gate verification verdicts (item-55 class discipline: Proved/Refuted/
  Undecidable recorded per gate) → human approval or rejection (operator identity + timestamp,
  an FDR event) → application record. A proposal is a reconstructible causal TREE in the
  flight recorder, end to end. AND the approval screen is never blind: the proposal's predicted
  cost/impact goes through item 67's classification — `ORACLE-EXACT / ORACLE-BOUNDED /
  MEASURED-ONLY` with evidence pointer (item 68's tables; item 70's aggregate propagation where
  the change touches the call graph) — presented to the human BEFORE approval; an
  unclassifiable impact is presented AS the forbidden/unclassified state (precision-honest),
  never a fabricated estimate. **Proof:** an end-to-end test recovers the complete lineage tree
  from a real recovered ring; the approval record demonstrably carries the impact class +
  resolving evidence pointer; a proposal with unclassifiable impact shows the honest marker
  (red→green vs a fabricated number); P3 law holds (no lineage value feeds any decision
  surface — grep proof). See
  [`BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`](BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md).
- **Item 77 — self-healing specialization: health-classifier-triggered fix proposals (SAME
  pipeline, one trigger class — explicitly NOT a lighter gate; after item 75; consumes item
  56's basis retrofit).** **Grounded baseline — health tracking already exists; what's missing
  is the consumer:** `markov::Verdict` (Healthy/LimitCycle/StrangeAttractor) is real and live
  but OBSERVATIONAL-ONLY today — grep-verified 2026-07-19: no in-kernel consumer acts on it
  (its only non-test consumer is its own `markov_attractor` bin emitting CLI JSON + the item-27
  FDR companion record; the autonomic-response half is explicitly Tier-4-deferred behind items
  9+21). `spectral::DriftClass` ALREADY has one real fail-closed consumer:
  `RetainedBase::admit` (`spectral_cache.rs:267`) rejects `Unstable` input. So "track own
  health" is mostly-already-done; this item adds the missing trigger-consumer, not a new
  classifier. **The two kinds of self-healing, kept apart:** **(A) Process-level
  restart-recovery stays AUTOMATIC** — crash → PostMortem → restart from last-good state is
  already built and proven (items 45–49; the kill-9 test's 300/300 recovery with zero human
  intervention): pure recovery, modifies no code/logic, already at space-grade rigor — no
  approval gate applies, none is needed. **(B) Code/logic-level fix is the NEW capability and
  takes the FULL item-75 pipeline:** a RECURRING adverse classification (e.g. `LimitCycle`/
  `StrangeAttractor` on the same subsystem across ≥N consecutive windows — N a named constant,
  P3 discipline) generates a `ChangeProposal` with the health evidence attached; it is then a
  proposal like any other — verification gates, human approval, FDR lineage — *never*
  autonomous, "it's just fixing a detected problem" is not an exception (operator's directive
  is explicit). Trigger-evidence law inherits item 56: only `Basis::Measured` verdicts count —
  an unevaluated-Healthy (or unevaluated-anything) window is never trigger evidence in either
  direction. **Prior-art grounding:** this is the autonomic-computing MAPE-K loop (Kephart &
  Chess, *The Vision of Autonomic Computing*, IEEE Computer 2003) — Monitor (FDR/HwStamp/PMU)
  → Analyze (markov/spectral classifiers) → Plan (the proposal) → Execute over shared Knowledge
  (FDR ring + HOT-PATHS + cost oracle + red-line registry) — with ONE deliberate, stated
  deviation: **Execute is never autonomous for code-level change; the human operator IS the
  Execute gate.** "Regenerative software" = this loop under those constraints, not a new
  mechanism. **Proof:** a synthetic recurring-adverse verdict stream yields exactly ONE
  proposal carrying the full health trail, which STOPS at pending-approval (the item-75
  red-proof reused); a single adverse window does NOT trigger (threshold pinned); planted
  unevaluated-basis windows are provably excluded from trigger evidence (red→green against
  today's byte-identical records); recovery class (A) remains automatic and green (kill-9 test
  unchanged). See
  [`BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`](BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md).
- **Item 78 — self-upgrading specialization: improvement proposals beyond fixes (SAME pipeline,
  broader trigger class; after item 75; enriched by items 70/71 when they exist).** Trigger =
  not a detected problem but a proposed improvement: cost-oracle-informed candidates (item 70's
  twin identifying bottlenecks/regressions worth attacking), pre-proven rewrite candidates
  (item 71's eqc-rs extraction arriving with its proof program), or operator-prompted upgrade
  requests routed through the same typed shape. Upgrade proposals additionally carry a
  before/after predicted-cost DELTA from the oracle (items 67/68; aggregate via 70) on the
  approval screen. The gate is IDENTICAL — same verification, same human approval, same
  lineage; and the item-73 law binds hardest here: an upgrade proposal touching a red-line path
  is refused identically at step zero, with **no beneficial-change exception** — the
  corrigibility trap is precisely a sequence of individually-beneficial-looking upgrades
  reaching the gate. **Proof:** an eqc-rs-generated, pre-proven rewrite flows end-to-end to
  pending-approval with its cost delta + proof-program result attached; a planted "beneficial"
  proposal touching gate/registry paths is refused at step zero (item 73's red-proof
  re-executed at this level); refused + approved + expired upgrade proposals all leave complete
  FDR lineage. See
  [`BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md`](BLUEPRINT-ITEMS-73-78-governed-self-evolution-2026-07-19.md).

**Dependency graph, one line:** 73 (spec) first and governing; 74 after 73; 75 after {73 + 74}
(structural halves of 73 land with 64/65); 76 after {75 + 62 + 67}; 77 ∥ 78 after 75 (77 also
consumes 56; 78 enriched by 70/71 but not gated on them). §L consumes §K's machinery (56, 62,
64/65, 67/68, 70/71) and item 47/50's grammar; it gates nothing outside itself. The AI that
proposes remains behind item 45's `inference` gate and item 65's capability boundary at all
times — §L grants a governed PROPOSAL channel, never authority.

### M. Cross-mesh data replication — MESH-07 parity (landed 2026-07-20, out-of-band of §A–L's
numbering; tracked here so it is not lost)

Not one of the original 78 items — this was raised by
[`DOWIZ-STRATEGIC-REGRET-MINIMIZATION-SYNTHESIS-2026-07-20.md`](DOWIZ-STRATEGIC-REGRET-MINIMIZATION-SYNTHESIS-2026-07-20.md)
§5 ("decide the durability spine... replication reserved") and §3.G ("full cross-mesh backup —
a single-node pilot with an off-node encrypted snapshot is an acceptable interim"). The operator
overrode the synthesis's own suggested deferral: **"Build real replication now"** — explicitly
rejecting the interim single-node option (which is what
[`BLUEPRINT-P68-hub-supervisor-update-backup.md`](CORE-ROADMAP-2026-07-17/BLUEPRINT-P68-hub-supervisor-update-backup.md)
already specs: one hub, one client-side-encrypted blob, one offsite bucket — explicitly
node-local, never over mesh transport).

**✅ DONE 2026-07-20** (`307c3ead5`, `main`) — `kernel/src/mesh_replication.rs`: native, zero-dep
reimplementation of bebop2's MESH-07 (`proto-wire/src/sync_pull.rs` — design reference only, per
§0's zero-dep mesh ruling, not a linked dependency). `MerkleLog` (sorted-leaf pair-hash digest),
`PullRequest`/`pull`/`ingest` (per-actor-watermark anti-entropy pull, G-Set CvRDT merge over
content-addressed ids), `reconcile()` (one full pull+ingest round). `EventStore` gained `ids()`
(default empty — degrades closed), overridden for `MemEventStore` and `hydra::FileEventStore`.
11 tests prove the MESH-07 RED-test criterion verbatim — two nodes diverge offline, reconnect,
pull, land on an identical folded event set — for both the in-memory store and disk-backed
`FileEventStore`, independent of which side initiates first. 1057/1057 kernel lib tests green.

**What this is not (deliberately):** transport (how bytes actually move node-to-node) and
signature verification are explicitly out of scope, matching `mesh-adapter/src/lib.rs`'s own
anti-scope ("no transport, no storage") and `event_log::EventLog`'s own doc ("the network layer
never re-runs decide — it only verifies signatures"). This is the pure, synchronous,
`std`-only reconciliation ALGORITHM — proven correct against any two `EventStore`s, in-process
here, over a real socket/QUIC transport later (a separate port, consistent with this crate's
existing ports/adapters split; async I/O has no place in the kernel's deterministic core per
MANIFESTO C2). Wiring a live transport, and layering `crate::mesh`'s ML-DSA-65 signing on top
of ingested events before they reach `ingest()`, remain open follow-on work — not claimed done
here.
