# ROADMAP + GROUND TRUTH — dowiz (product) & bebop (protocol) — 2026-07-11

> Canonical roadmap. Written and PUSHED to remote before any code runs (operator rule 2026-07-11:
> memory-first + push-plans-first + ground-truth-outranks-plans). The brief this derives from spans
> BOTH repos: bebop/bebop2 = protocol, dowiz = product. This doc is the single source of truth for
> what to consult before writing code. It separates DONE (verified on local disk) from PLANNED (desired
> state from the brief), because the brief repeated several claims that the live repo contradicts.

## 0. GROUND TRUTH — what is actually on disk right now (re-verified 2026-07-11)

### 0.1 Missing research/design reports (CRITICAL gap)
The brief cites ~20 research/design reports dated 2026-07-11. **None of them exist on local disk in
either repo.** If they were produced by agents, they were never written to a file / never committed.
This is the headline risk: a large body of "findings" is currently only in chat context and will be
lost on session close.

Cited-but-MISSING (dowiz side, expected under docs/research/ or docs/design/):
- HUB-ARCHITECTURE-REVIEW, MAX-EV-SYNTHESIS (+4 lens reports), DESIGN-LIBRARIES-RESEARCH,
  PARTICLE-CLOUD-INTERACTION-ANALYSIS, PARTICLE-CLOUD REVIEW+PLAN, HONEST-ASSESSMENT-BEBOP-DOWIZ,
  RELAY-HETZNER-TAILSCALE-MESH, LAUNCH-WITHOUT-LAWYER-ALBANIA, FIELD-SIM SYNTHESIS.
- LOCAL-FIRST-HUB SYNTHESIS A/B/C/D, 03-ANONYMITY-ARCHITECTURE, 04-ANONYMITY-MESH-MESSENGER-REVISION,
  TIDE-OVER-BEDROCK (INTERFACE-DIRECTION), PROTOCOL-TECH-COMPLETION-BLUEPRINT,
  DESIGN-COMPLETION-BLUEPRINT, UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT, MASTER-BUILD-SEQUENCE.

Cited-but-MISSING (bebop side, expected under bebop-repo/docs/design/):
- The local-first + field-sim + anonymity reports above are NOT in bebop-repo either (grep of doc
  titles returned 0 hits). bebop-repo DOES have its own 2026-07-11 set: bebop-fable-research,
  bebop-math-physics-fable-research, bebop2-deep-research, UNIFIED-DELIVERY-PROTOCOL-BLUEPRINT-v3,
  multi-channel-field-integration-plan, CONSOLIDATED-AUDIT-EXTRACT, plan-audit-bebop,
  bebop-memory-optimisation-fable-research, fable-protocol-2026-07-11/.

**ACTION (operator gate):** the missing dowiz reports must be re-authored to disk and committed, OR
the brief's findings must be treated as UNVERIFIED until then. Do NOT build code against claims in
missing reports.

### 0.2 Surviving plan-collapse docs (committed to origin/main, POST-PUSH revision)
These ARE on disk and already correct the brief's stale claims:
- dowiz `docs/design/MASTER-BUILD-SEQUENCE-UPDATED-2026-07-11.md` — Tier 0–5 + PARKED, with a
  "CORRECTIONS" table that strikes /claim-404 and GDPR-trio-from-T1 as DONE.
- dowiz `docs/design/PARALLEL-EXECUTION-PLAN-2026-07-11.md` — Batches A/B/C/D (parallel) vs sequential gates.
- dowiz `docs/design/DRIFT-ANALYSIS-2026-07-11.md` — R1–R12 drift between bebop-protocol and dowiz-product.

### 0.3 Verified code facts (grep/git, 2026-07-11)
| Claim | Verdict | Evidence |
|---|---|---|
| `/claim` 404 on prod | CONTRADICTED — DONE | f0bd9966 pushed; `fix/prod-blockers-P2` adds `/claim` to SPA_ROUTES (server.ts:858). Brief's "404 everywhere / server.ts:858" is stale. |
| `/courier-invite` missing from SPA_ROUTES | CONTRADICTED | Registered at `apps/web/src/main.tsx:51` already. |
| Money tween real (breaks trust-cue) | CONFIRMED | `ClientLayout.tsx:154` `AnimatedNumber` on cart total; `AnalyticsPage.tsx:262` `AnimatedNumber`; `courier/EarningsPage.tsx:47` `CountUpPrice`. 3 sites, not the brief's "ClientLayout:245 / Dashboard:451 / Analytics:265" line numbers (those are wrong — real tween is at 154). |
| bebop skin dormant (admin/courier/404 on paperSkinAttr) | CONFIRMED | `AdminRoutes.tsx:121` paperSkinAttr; `CourierRoutes.tsx:45,57` paperSkinAttr (bebopSkinAttr imported but unused). |
| kernel::decide bypass (Command::PlaceOrder never constructed) | bebop-repo, NOT dowiz | Part of bebop PARKED protocol; dowiz checkout is Node. Not a dowiz prod blocker. |
| In-flight parallel batches | CONFIRMED | Branches exist: feat/design-p1-tokens, feat/reliability-push, feat/ci-security-gates, feat/gtm-channel. |
| Degrade-storm ratchet (Batch B) | IN PROGRESS on this branch | `apps/api/src/lib/reliability/ratchet.ts` (new) + `apps/api/tests/boot-ratchet.test.ts` + server.ts wiring, uncommitted on feat/ci-security-gates. |
| Sync script `sync-memory-to-hermes.mjs` | MISSING | Referenced by living-memory skill but absent; `HERMES.md` was never generated. Skill is partially stale. |
| Corpus state | dowiz corpus present & updated (MEMORY.md.bak shows sync ran 2026-07-11 20:05); bebop corpus dir exists but no MEMORY.md index found at expected path. |

### 0.4 Repo topology
| Repo | Role | Key branch (current) | Tests to re-run |
|---|---|---|---|
| /root/dowiz | Product (TS app) | feat/ci-security-gates | `git status`; pnpm lint/typecheck/build; node:test via `npx tsx --test` |
| /root/bebop-repo | Protocol (Rust/WASM) | main (bebop2) | `cargo test --workspace` (expect green), `cargo fmt --check` |

### 0.5 STACK (canonical, operator 2026-07-11 — FINAL, overrides brief's TS/Node framing)
- **Kernel:** Rust → WASM (deterministic core: decide/fold Law over order-status machine, integer money,
  CRDT-safe menu/presence, single-writer sequencer for money/orders). bebop2 Rust is the protocol twin.
- **Frontend:** Astro (content/landing/owner storefront shell) + Svelte 5 (islands: owner dash, courier,
  tracking, customer track). NOT React/TS.
- **Rendering:** WebGL2 (hand-rolled particle-cloud, ≤7kB gz chunk gate) + canvas2D fallback; CSS/SVG only for
  storefront polish. WebGPU optional later (fallback required). This is the "Tide over Bedrock" surface.
- **Node.js & TypeScript are DROPPED** from the build target.
- **Consequence:** the current /root/dowiz TS/Node production app is the LEGACY ORACLE — keep it running as the
  reference + live demo (strangler-fig pattern), but new code targets Rust/WASM + Astro/Svelte. Swap-at-end.
- **Wave-1 TS agents (design/reliability/gtm) dispatched 2026-07-11 are REFERENCE-ONLY** — their findings map to
  the Svelte/Astro port, not merge targets. The money-tween/skin findings → Svelte token layer; sw.js push →
  courier Svelte PWA service worker; OG/qr/channel-reader → Rust API + Astro owner card. Do not merge TS agents to
  a stack being dropped.

---

## 1. DONE (verified) — do not re-do
- `/claim` 404 fix → prod (f0bd9966).
- P2 checkout enum/receiver.
- P3 GDPR storage-DI into AnonymizerService.
- bebop protocol W/A/H P0-6 = SCAFFOLDED skeletons (not production): `bebop-proto-wire`/`bebop-proto-cap`/`bebop-proto-crypto` at /root/bebop-repo/bebop2/ (untracked, `agent/protocol-wah` branch; `cargo check` green; NO-COURIER-SCORING guard in). Production logic deferred to Tier-4 (gated G11).
- **KERNEL SLICE (2026-07-11, canonical stack):** `kernel/` Rust→WASM crate at /root/dowiz — `decide/fold` Law (order_machine.rs, 1:1 port of packages/domain/order-machine.ts) + integer money (money.rs, 1:1 port of apps/api/src/lib/money.ts). 17/17 RED+GREEN VbM tests pass; wasm32-unknown-unknown build GREEN. Branch `kernel/rust-wasm-core` (pushed). This is the FIRST canonical-stack code; TS app is legacy oracle.
- **WAVE-3 (2026-07-12, canonical stack, DONE+verified, `kernel/rust-wasm-core` bd1da1a0):** kernel is now web-callable + both web surfaces exist.
  - `kernel/src/wasm.rs`: wasm-bindgen JSON API (place_order_js, apply_event_js, channel_ledger_js, reduce_anomalies_js); serde+serde_json added. `kernel/pkg/` glue (~78 KiB gz wasm) generated.
  - `web/src/components/CourierTrack.svelte` + `web/src/lib/push.js`: live tracking island (fires particle-cloud VOCAB per status, reduced-motion, no money-tween) + courier push-subscribe wiring `public/sw.js` (closes locked-screen dispatch gap).
  - `web/src/components/OwnerDashboard.svelte` + `web/src/lib/channel.js`: orders-by-channel + funnel + anomalies reader (closes open attribution loop on the UI).
  - Unified `web/src/pages/index.astro` mounts Storefront + OwnerDashboard + CourierTrack.
  - Kernel: **37/37 RED+GREEN VbM tests; wasm32 build green; fmt clean.** NO-COURIER-SCORING enforced (0 rating fields). Agent-escape + index.astro merge reconciled via `wave3/integrate`.
- **WAVE-4 (2026-07-12, canonical stack, DONE+verified, `kernel/rust-wasm-core` fa637399):** full G11 working backend pulled forward (Tier-4 substrate ahead of G11 per operator "drop legacy" redirect).
  - `server/` NEW Rust crate (axum+tokio+rusqlite bundled): `POST /api/orders` (kernel place_order, server-authoritative integer money), `POST /api/orders/:id/event` (apply_event, **409 on illegal transition** = RED), `POST /api/courier/push/subscribe`, `GET /api/orders/channel` (ChannelLedger), serves `web/dist`. SQLite store = local memory. **5/5 lib + 5/5 integration RED+GREEN tests.**
  - `web/src/lib/kernel/`: wasm-bindgen glue (web+node) + `kernel.js` loader. Storefront `placeOrder` now calls REAL kernel (node test: `real kernel order id = ord_0 status = PENDING`, no money-tween).
  - `bebop2/proto-wire` WssTransport (tokio-tungstenite) impl + signed-frame round-trip (**6/6 tests**) — **UNCOMMITTED in bebop-repo** (3-model review gate honored; awaits operator sign-off).
  - NO-COURIER-SCORING enforced (0 rating fields across server/web/bebop).
  - All 3 agents escaped worktrees (wrote to main trees) — reconciled + independently verified.
- NO-COURIER-SCORING final (DRIFT R2).
- **WAVE-2 (2026-07-11, canonical stack, DONE+verified, on `kernel/rust-wasm-core` 5c082b66):** Node/TS/Supabase/Fly DROPPED for new slices; pure Rust+Astro/Svelte+WebGL.
  - `kernel/src/domain.rs`: Order aggregate + compute_order_total + Decider (place_order/apply_event) via decide/fold Law.
  - `kernel/src/analytics.rs`: ChannelLedger (orders_by_channel + funnel) + reduce_anomalies via fold_transitions — closes the open attribution measurement loop.
  - `web/`: greenfield Astro+Svelte5 storefront, kernel-compatible order payload, no money-tween (build green).
  - `webgl/particle-cloud`: WebGL2 event-visual vocab (4165 B gz, ≤7 kB), reduced-motion.
  - `public/sw.js`: courier out-of-app push handler (closes locked-screen dispatch gap).
  - Kernel: **32/32 RED+GREEN VbM tests pass; wasm32-unknown-unknown build GREEN.** NO-COURIER-SCORING enforced (0 rating fields).
  - NOTE: 2 agents (web-astro, webgl) wrote into the main tree instead of their worktrees — reconciled + committed on `kernel/rust-wasm-core`. Kernel agents (domain, analytics) stayed in worktrees; merged via `wave2/integrate`.

## 2. PLANNED (desired, from brief) — gate before build
Tier 0 remaining (parallel, zero-pivot, non-red-line):
- A: money-tween + bebop-skin (RESOLVED-in-canonical 2026-07-12): the canonical Svelte/Astro Storefront (`web/`, `kernel/rust-wasm-core`) calls REAL kernel `placeOrder` with NO money-tween (node test: `real order id=ord_0 status=PENDING`, no count-up). The 3 legacy tween sites (ClientLayout:154 `AnimatedNumber`, AnalyticsPage:262 `AnimatedNumber`, EarningsPage:47 `CountUpPrice`) + dormant `paperSkinAttr` live ONLY in the dropped Node/TS `apps/web` oracle (swap-at-G11). Operator decision 2026-07-12: skip legacy edits; canonical stack satisfies the trust cue (money-never-tweens). RED (count-up assertion) is satisfied by the canonical Storefront. Legacy oracle carries the smell but is not the live path.
- B: degrade-storm ratchet finish + sw.js push handler. RED = flags reset on restart; push renders.
- C: gitleaks CI hard-fail + sync-CRDT fence + P7 gate scaffolding. RED = canary fails scan; price in MenuDelta breaks CI.
- D: OG <300KB + channel attribution reader + QR+?ch=. RED = content-length<300KB; broken-QR detected.

Tier 1 (red-line, operator decision): P1/P7/P8 execution; prod OG/demo (blocked on lost PROVISION_OPS_SECRET); remote-history scrub + branch prune.
Tier 2 (quality bars): design 13-item "stable enough" + storefront zero-diff Playwright gate; GTM 8-point per-venue gate; courier out-of-app signal (N1/N2).
Tier 3 (validation hinge): G11 GREEN = one real order from non-operator customer on claimed venue.
Tier 4 (substrate, gated on G11): protocol R node = kernel::decide-bypass fix; X settlement/dispute; migration ladder; channel registry; sync-CRDT menu lane; reliability LD0–LD11.
Tier 5 (earn-it, each gated): money crypto audit ladder (hybrid-only until audit); messenger transport (G7 survey); Astro port; .onion tier; multi-venue mesh.
Tier X (PARKED): bebop-as-protocol (capture-protect ~1h); field-sim (sign-bug fix if revived); sovereign-core cutover mothball; B3 RLS flip last.

---

## 3. PARALLEL-SAFE vs SEQUENTIAL (structure before code)

### PARALLEL-SAFE — run in own branch/worktree, independent files, zero-pivot-risk, non-red-line
- Dowiz Batch A (design tokens) — branch feat/design-p1-tokens.
- Dowiz Batch B (reliability + sw.js push) — branch feat/reliability-push.
- Dowiz Batch C (CI/security gates) — branch feat/ci-security-gates (this one).
- Dowiz Batch D (GTM/channel attribution) — branch feat/gtm-channel.
- bebop capture-protect (commit/push/1 demo/memory) — small, can run anytime, doesn't touch product.
- Re-author the missing dowiz research/design reports to disk — parallel-writable, no code deps.

### SEQUENTIAL GATES — cannot parallelize; need decision or prior tier
- Tier 1 P1/P7/P8 execution ← operator red-line approval.
- Prod worker restart + secret rotation ← operator (no prod access; "stopped 07-03" UNVERIFIED).
- Tier 2 quality bars ← wait on Tier 0 done.
- Tier 3 G11 GREEN ← external, not code.
- Tier 4/5 substrate ← wait on G11 GREEN.
- DRIFT R2 — reputation.rs courier-scoring vs NO-COURIER-SCORING HARD FORK (architectural, red-line).

### Invariant (shared spine, both repos)
Build DOWN from the first real order, not UP from the protocol. Gates are falsifiable conditions, not
calendar dates. bebop protocol work is PARKED until dowiz carries it.

---

## 4. OPEN OPERATOR DECISIONS (red-line, not auto-executed)
1. Execute P1/P7/P8 (currently docs-only)?
2. Prod worker restart + secret rotation (secret claim unverified)?
3. DRIFT R2 courier-scoring fork — architectural.
4. Re-author / recover the ~20 missing 2026-07-11 research/design reports, OR accept them as UNVERIFIED?
5. PROVISION_OPS_SECRET lost — blocks prod demo provisioning.

*Generated 2026-07-11. Source of truth: live repo + corpus. Re-verify before trusting any "DONE" line.*
