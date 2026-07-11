# G06 — Sovereign Core MVP: close the exit gate (or honestly redefine it)

> Gap blueprint, 2026-07-11. Read-only research session; nothing in the tree was modified.
> Grounds: audit `docs/research/2026-07-11-full-project-audit-dowiz-bebop.md` (§4.2, §5.3, §6.2,
> §7.2), the MVP corpus `docs/design/sovereign-core-mvp/` (GRAND-PLAN, PROGRESS,
> HANDOFF-2026-07-07-SESSION, IMPLEMENTATION-ROADMAP, PHASE-2-2-CART-TOKEN-SPEC, DECISIONS),
> memory (`sovereign-core-mvp-handoff-2026-07-06`, `red-proof-0b5-completed-2026-07-07`,
> `session-2026-07-07-*`, `sovereign-core-phase-zero-2026-07-05`), and direct code/live-endpoint
> verification performed for this blueprint (every fresh claim below is marked VERIFIED /
> CONTRADICTED with a file:line or probe).

---

## 1. Gap & evidence

**The gap.** The Sovereign Core MVP was declared "SHIPPING-READY" (`project-state-2026-07-08.md`)
while the program's own exit gate (GRAND-PLAN §"MVP exit gate") was never closed. The last honest
cursor (memory `sovereign-core-mvp-handoff-2026-07-06`, 0b-3 entry) defined **12 remaining steps to
the exit gate: 0b-4 / 0b-5 / 0b-6 + Phase-1 1.1–1.5 + Phase-2 2.1–2.4** (5 red-line: 0b-5, 1.1,
1.2, 2.2, 2.3). As of the 2026-07-07 late sessions, **7 of those 12 are built** (0b-4, 0b-5, 1.1,
1.2, 1.5, 2.2, 2.3 — all on staging v266, branch lineage `feat/sovereign-core-phase-zero` →
contained in `feat/paleo-dinosaur-digs`), and **5 are open**: 1.3, 1.4, 2.1, 2.4, 0b-6. On top of
that sits **verification debt on the "built" 7** — the HANDOFF's "Immediate Next Steps" (staging
validation checklist, `hub_checkout` flag verify, replay-parity verify, Playwright vs staging) were
never recorded done, and this research found several of them are not merely un-run but
**un-runnable or false-green as written** (§2.3). Finally the gate's last clause — sovereign CI
check required on `main`, then main merge + prod — is blocked behind the history bifurcation
(G01/G02 territory).

**Exit-gate text being closed against** (GRAND-PLAN.md §"MVP exit gate", verbatim): *an owner can
(1) register channels and print QR/links, (2) receive a real direct order end-to-end at 0%
commission through the sealed core, (3) see it attributed in their dashboard, (4) own and erase the
customer record — with the full money battery green, `/reliability-gate` GO, replay-parity green,
NOBYPASSRLS suites green, and the sovereign CI check required on `main`. Everything not
launch-ready stays flagged OFF.*

**Evidence of the claim/reality split** (audit §7.2, re-verified here):
- "MVP is SHIPPING-READY" vs the same week's memory "MVP is only 5 of 12 phases complete …
  prod merge deferred" (`session-2026-07-07-late-summary.md`).
- `hub_checkout` "wired, default OFF, *verify in next session*" — the verify never happened, and
  §2.3(a) below shows the flag currently gates **nothing** (telemetry-only).
- Two 07-07 memories both titled reliability-gate: one records "GO FOR PRODUCTION" (parallel
  **code-audit** agents + 4 real fixes), the other records "prod deferred, MVP incomplete"; the
  HANDOFF still lists "run /reliability-gate on staging — manual trace" as an **open next step**
  for the new checkout path.

---

## 2. Research findings

### 2.1 The 12-phase definition is fully specified — 1.3/1.4/2.1/2.4 have real specs

All four open phases have scope / files / DoD / deterministic gate / RED proof / effort already
written in `GRAND-PLAN.md` (no spec work is needed, only build):

| Phase | What it is (GRAND-PLAN) | Gate + RED proof (as spec'd) | Effort |
|---|---|---|---|
| **1.3 sync PORT** | ONE shell trait (`SyncPort`: `append`/`read_since`); impl #1 = Postgres (wraps 1.2), impl #2 = in-memory (tests only). Explicitly NOT async-trait forests/retry/storage abstraction. The D2 "libp2p later = swap not rewrite" seam. | Shared contract suite green on both impls; `/reliability-gate` still green. RED: in-memory impl drops last event → suite red on that impl only. | S/M |
| **1.4 signed-event envelope** | `content_hash` = SHA-256 over **codec canonical event bytes** computed in Rust at append; `signature` stays NULL for the whole MVP (slot only; signing = Phase 3). | Independent non-mirror oracle: pgcrypto `digest(payload,'sha256')` over STORED bytes == stored Rust hash for all rows. RED: mutate one stored payload byte in a rolled-back staging txn → mismatch. | S |
| **2.1 distribution artifacts** | Per-channel attribution link `/s/:slug?ch=<token>` + client-side QR; storefront maps `?ch=` → `x-channel` header; channel CRUD UI completes 1.5; flag `hub_channels` default OFF. Malformed `ch` degrades to `other`, never blocks an order. | Playwright vs staging: order via `?ch=qr` → DB `metadata.channel='qr'` + dashboard count increments; forged `?ch` asserts `other` AND a successful order. RED: break param→header mapping → attribution test red while order-success stays green. | M |
| **2.4 aggregator stub** | Dashboard tab "All orders" behind flag `aggregator_view` (default OFF), ONE trait `AggregatorSource::fetch_orders_readonly`, zero impls, empty-state only. Pins the D1 seam. | CI config-assert prod flag OFF; Playwright flag ON/OFF. RED: flip prod config in a PR → config-assert red. | S |

Current tree state of each (VERIFIED this session): **1.3** — `rebuild/crates/api/src/ports/` does
not exist; not started. **1.4** — *partially landed via 1.2*: the `order_events` migration
(`packages/db/migrations/1780350000001_order-events-log.ts`) already carries `content_hash text NOT
NULL` **and** the `signature bytea` NULLable column (exactly as GRAND-PLAN planned: "columns for
1.4/D2 land in this ONE migration"); what's missing is the canonical-bytes discipline and the
pgcrypto oracle gate (§2.3d). **2.1** — zero artifacts: no `hub_channels` string anywhere in
rebuild/apps/packages, no `?ch=` storefront plumbing (grep-verified); the ingredient pieces exist
(`modules/channel_attribution/` normalization, `x-channel` header reader in `checkout.rs:144`).
**2.4** — zero artifacts: `AggregatorSource`/`aggregator_view` grep = 0 hits.

### 2.2 The "done" phases are real code in the current tree (spot-checks all pass)

All 13 load-bearing commits exist in `feat/paleo-dinosaur-digs` history (VERIFIED `git log -1` each):
0b-1 `c10814ab`, 0b-2 `e3e30ac1`, 0b-3 `31520e8a`, 0b-5 flip `92cc239b`, 0b-5 RED proof
`69293616`, 1.1 `9a113ce8`, 1.2 `3649cb84`, 1.5 `888f6202`, 2.3 `03a7031a`+`162ef1ec`, 2.2+2.3
`56f1f872`, tests `a6ea2001`+`2dd72a99`. Files present: `kernel::decide` at
`rebuild/crates/domain/src/kernel.rs:306` (composes machine → actor-gate → cc1 → pricing corridors,
PlaceOrder prices via `price_cart`); `rebuild/crates/api/src/routes/orders/checkout.rs` (266 lines)
**wired as the live handler** — `mod.rs:675` `.route("/api/orders", post(checkout::create_order))`;
`rebuild/crates/api/src/modules/customer_management/{mod.rs,pg.rs,module.toml}`;
`rebuild/crates/api/src/routes/owner/{channels/,customers.rs}`; migrations
`1780350000000/1/2` are **formally placed** in `packages/db/migrations/` (not drafts);
`rebuild/scripts/sovereign-gate.sh`, `rebuild/deny.toml`, `scripts/replay-parity-check.sh`,
`rebuild/crates/api/tests/{phase_2_2_adversarial_money_suite,sovereign_core_e2e}.rs`,
`apps/web/tests/sovereign-core-mvp-e2e.spec.ts` all exist. The 0b-5 inject→deploy(v265)→observe
CorridorBreach→revert(v266) RED proof is recorded with timestamps and remains the strongest
deployed-reality proof in the project. Staging is live (probe 2026-07-11 16:41Z: `/livez` 200
0.25s; `/health` degraded only on `fallback`).

### 2.3 The sharp findings — verification debt is worse than "not yet run"

**(a) `hub_checkout` does not gate anything — spec vs code CONTRADICTED.**
`PHASE-2-2-CART-TOKEN-SPEC.md:154` mandates: *OFF ⇒ `x-dowiz-cutover: true` is ignored, old Node
path used; ON ⇒ kernel path live.* The implementation (`checkout.rs:68-70`) reads
`std::env::var("HUB_CHECKOUT")` and uses it **only inside a `tracing::debug!` line**
(`checkout.rs:125-132` — "the flag governs the wider rollout telemetry"). The cutover checkout path
runs unconditionally whenever the Rust shell serves `POST /api/orders`. The flag is also **not** in
`packages/config/src/index.ts` EnvSchema (grep-verified — audit §6.2's "wired, default OFF" is
accurate but flattering). Mitigations that hold today: the forbidden-price-field guard is
unconditional (money safety is flag-independent), and prod has 0% Rust cutover, so exposure is
staging-only. But the "verify hub_checkout" debt is actually a **design-and-build** item: decide
where the launch gate lives (Rust handler refuse/fall-through vs the Node front-door matcher, which
already owns per-surface routing for S5 — `apps/api/src/lib/cutover/route-templates.generated.ts:340`
maps `POST /api/orders` → S5) and implement it with a falsifiable ON/OFF proof.

**(b) The replay-parity job is a placeholder — Phase 1.2's load-bearing gate is not real.**
`scripts/replay-parity-check.sh` says so itself: *"For now, this is a placeholder assertion: just
verify the event log is not empty. Full replay logic … staged for Phase 1.2.1."* It can only go red
on an orphaned event log; it can never catch a status/totals divergence. GRAND-PLAN's own
adversarial critique predicted exactly this failure: *"if replay-parity ever goes advisory … the
event log decays into decoration … or 1.2 was theater."* By the repo's VbM rule this gate currently
validates nothing.

**(c) `cause_hash` is the literal string `"placeholder"`.**
`rebuild/crates/api/src/routes/orders/pg.rs:865` hardcodes it. The Envelope's causality seam (D2 —
dedupe/ordering) is not wired into the persistent log; the real `request_hash` exists one module
over.

**(d) `content_hash` bypasses the codec.** The dual-write hashes `serde_json::to_vec(event)`
(`pg.rs:855-861`), not `domain::codec::canonical_bytes` (which exists, `codec.rs:24`, pure +
wasm-clean). 1.4's DoD (core-owned canonical bytes + pgcrypto independent oracle) is unmet; the gap
is small because the schema slot and the hash column are already live.

**(e) The Playwright "staging validation suite" is vacuous as written — it cannot fail.**
`apps/web/tests/sovereign-core-mvp-e2e.spec.ts` (10 tests, never recorded as executed):
hardcoded `STAGING_URL`; sends `location_id` while the Rust DTO requires `locationId`
(`dto.rs:137` explicit rename — every POST will 400); sends `x-sales-channel` while the code reads
`x-channel` (`checkout.rs:144`); uses fake ids (`'demo-location-id'`, `'sushi-roll-id'`); and its
assertion structure is `if (404||422) return; if (201) { assert }` — a 400 falls through **with
zero assertions executed** and the test passes. This is precisely the false-positive-metric class
VbM bans. The suite must be repaired (real fixtures, hard assertions, RED case) before "run
Playwright vs staging" means anything.

**(f) The "18/18 tests PASS (4 adversarial + 14 e2e)" claim overstates.** Both Rust suites are
in-process, no DB/no HTTP (grep: zero `PgPool`/`DATABASE_URL` hits in either file);
`sovereign_core_e2e.rs` tests are kernel-behavior tests, several substantially comment-bodied.
They are fine as unit gates; they are not end-to-end and do not discharge the staging checklist.

**(g) The L0–L11 "PASS/GO" was a code-audit, not a live trace of the new path.** The two 07-07
gate memories describe 5 parallel *auditor agents* over the codebase (which found and fixed 4 real
bugs — migrations 085/086, courier channel, RLS context — genuinely valuable), but the HANDOFF
still lists the manual staging trace, explicitly including the new **L2 "order create via Phase
2.2 checkout" and "idempotent double-POST"**, as undone next steps. No memory or ops doc records a
live order traced through the kernel checkout on staging.

**(h) Staging's cutover state is unknown — a precondition for everything above.** The h_t frame
(`docs/ops/rebuild-cutover-h_t.json`, 2026-07-05, branch `fix/audit-remediation`) records S5=rust
on staging, but staging has since been redeployed (v265/v266) from the sovereign lineage; whether
`POST /api/orders` on staging **today** reaches the Rust `checkout::create_order` (vs the Node
handler, which ignores `hub_checkout` entirely) is unverified. Probe of `/s/demo` today shows no
`x-dowiz-cutover` response header (S1 human page = Node, consistent with the frame). Flags live in
the `cutover_flags` DB table (migrations role writes; readable via staging DB access — memory
`staging-db-access-2026-06-30`).

### 2.4 0b-6 CI gate — staged, proven locally, needs an operator `cp` + branch-protection flag

`proposed-sovereign-core-ci/APPLY.md` contains the complete job (wasm32 purity gate + clippy
disallowed-methods via `rebuild/scripts/sovereign-gate.sh`, core unit tests, `module-integrity.mjs`)
with a local red→green proof recorded (SystemTime::now injection) and a real catch on first run
(uuid `v4` entropy feature). Today `.github/workflows/ci.yml` contains **zero** Rust/cargo/sovereign
steps (grep-verified) — the Rust core runs in no CI. To activate: (1) extend the job per GRAND-PLAN
0b-6 with `cargo deny check bans` (`rebuild/deny.toml` exists) and **scope deny to the core
ban-list only** — a workspace-advisories run reds forever on the pre-existing RUSTSEC-2023-0071
(rsa) + yanked num-bigint in the api web-push/jwt chain (documented non-blockers); (2) operator
applies to `.github/workflows/ci.yml` (protect-path — agents cannot); (3) re-prove RED **in CI**
on a throwaway branch (chrono dep + SystemTime inject — "runs locally ≠ wired remotely" is the D5
failure class the plan itself names); (4) operator marks `sovereign-core` a required status check
on the `main` protection rule. Note the interaction with G01: required-check-on-main only bites
once the bifurcated history can actually raise PRs against main.

### 2.5 The staging-validation checklist, reconstructed precisely (HANDOFF-2026-07-07 §Next + §Pending)

1. **/reliability-gate manual staging trace L0–L11** with the two new L2 items (Phase-2.2 checkout
   create; idempotent double-POST) — gate doc `docs/ops/reliability-gate-cutover-2026-07-05.md`.
2. **Playwright suite vs staging** (`VITE_BASE_URL=… pnpm exec playwright test
   apps/web/tests/sovereign-core-mvp-e2e.spec.ts --reporter=list`) — blocked on §2.3(e) repair.
3. **`hub_checkout` verify** — default OFF, toggleable ON, actually gates — blocked on §2.3(a) build.
4. **Replay-parity verify** — events logged, replay matches, job green — blocked on §2.3(b) build.
5. **Red-line gate review** (operator): 0b-5 flip, 1.2 log, 2.2 money invariant, 2.3 PII/RLS.
6. **Full-lifecycle e2e** (from `session-2026-07-07-late-summary` "What's Left"): owner data-hub
   flow ❓, customer tracking + real-time status ❓, courier assignment + delivery signals ❓.
7. **Prod-merge readiness**: all phases validated on staging, flag OFF default, migrations
   staging-first, explicit operator approval.

Known blockers to executing it: the **staging checkout-flow break** (`checkout-phone` testid,
flagged 2026-07-04, never closed — audit §6.4) sits directly on items 2/6; the **staging
rate-limiter vs E2E matrix** (100 req/min/IP false-fails; bypass-token vs serialized-runs never
decided) sits on any full Playwright run; fixtures need real demo location/product UUIDs (memory
`test-owner-fixture-sushi-demo`).

**Bonus finding — 1.5 is endpoint-complete, exit-gate-incomplete.** PROGRESS marks 1.5 ✅ but its
own entry says "**NEXT:** UI tab + i18n + Playwright". Grep confirms: no channels UI in
`apps/web/src`, nothing in `rebuild/web`, no channel i18n strings. The exit-gate clause "(3) see it
attributed in their dashboard" is therefore **not yet satisfiable by any UI** — only by API
(`GET …/channels/with-attribution`). The dashboard tab lands naturally with 2.1 (GRAND-PLAN:
"Channel CRUD UI completes 1.5").

### 2.6 Phase 1.4 vs bebop2 crypto — documented seam, deliberately NOT a dependency

`docs/design/dowiz-agent-cli/CORE.md` documents the bridge explicitly: *"Where the Grand Plan says
dowiz-core, read Bebop kernel; where it says libp2p mesh, read Bebop SyncPort transport"* — bebop's
kernel copies the same `decide/fold/replay` + `Envelope{seq,at,cause}` shape, and its crypto layer
(hybrid ML-DSA-65 + Ed25519, sign/verify over canonical bytes → sha256) is the realized form of
what GRAND-PLAN's Phase-3 roadmap row reserves for the dormant `signature` column. So the design
intent to share primitives **exists on paper** — but it is Phase-3 machinery, gated on a security
council (D6), and the audit's §7.10 warning applies in full: bebop2's hand-rolled, KAT-green-but-
unaudited primitives must not guard money/identity. **For the MVP exit gate, 1.4 = content-hash
oracle only; `signature` stays NULL; zero coupling to bebop2.** Any future signing decision is an
explicitly separate operator arc.

### 2.7 Prod-merge path — owned elsewhere, dependency noted only

The 07-05 secrets scrub bifurcated history; a straight merge produced 500+ add/add conflicts
(HANDOFF); the merge must be rewrite-aware (tree-based, curated), and the remote scrub force-push
(EXPANSION-PLAN Layer 0.1) is a blocking operator gate. **G01 (history bifurcation / merge design)
and G02 blueprints, being written in parallel, own this.** This blueprint's Phase 4 consumes their
output; it does not re-design the merge. One local fact that helps them: the three sovereign
migrations are already formally numbered in `packages/db/migrations/` and were applied
staging-first, so the MVP slice adds no migration-draft debt to the merge.

---

## 3. Options & tradeoffs

**Option A — Close the full 12-step gate as defined.**
Build 1.3 + 1.4 + 2.1 + 2.4, clear all verification debt, activate 0b-6, merge+prod.
*Pros:* the recorded definition is honored; no scope debate. *Cons:* 1.3 and 2.4 deliver zero
owner-visible value (both are seams: D2 transport, D1 aggregator); by D1's own MVP definition they
are insurance, not product; ~2 extra lane-days spent before the market test the audit (§7.8) calls
the project's largest risk. Effort ≈ 11–16 lane-days total.

**Option B — Redefine the exit gate to "D1-complete" (RECOMMENDED).**
The gate's four owner capabilities require: 2.2 ✅(verify), 2.3 ✅(verify), 1.5 **UI** (missing),
**2.1** (missing — the only open phase D1 actually needs: register channels, print QR/links).
Keep: verification-debt closure (Phase 1 below), 1.4's content-hash oracle (S effort, protects the
log's integrity claim, already half-landed), 0b-6 required check (S effort, the machine definition
of "sovereign" — the gate text names it). **PARK with dated markers:** 1.3 (revisit at the first
real second transport — libp2p/mesh demand, per GRAND-PLAN Phase-3 table; also unblocks A3 orders-
split whenever resumed) and 2.4 (revisit when aggregator ingestion is scheduled; the doctrine +
trait contract doc can be written in the PARKED note for ~0 cost). *Pros:* every remaining hour
maps to the D1 thesis (0%-commission escape) or to falsifiable proof; honest about what "MVP" means.
*Cons:* amends a recorded plan — requires an explicit operator sign-off line in PROGRESS.md +
memory, or it becomes another silent goalpost move (the §7.2 disease this blueprint exists to cure).
Effort ≈ 8–11 lane-days.

**Option C — Park the whole exit gate pending the arbiter doc.**
Audit rec #2: four programs (rebuild cutover, MVP, OSS flip, bebop) compete with no ranking; and
prod carries the GDPR trio gap (audit rec #1) which outranks any MVP work on harm. *Pros:* cheapest;
respects real priority. *Cons:* the MVP is ~85% built and its verification debt rots fastest
(staging redeploys erase the deployed-reality claims — the 0b-5 proof is already only as good as
staging's current flag state); parking un-verified "done" claims is how CLAIMED-UNVERIFIED becomes
CONTRADICTED. If C is chosen, Phase 1 below (verification debt only, ~3 lane-days) should still run
first so the park is on a *known* state.

**Recommendation: B, with Phase 1 executed first regardless of the A/B/C choice** — and with the
explicit note that even a closed exit gate does not discharge audit §7.8: the gate ends at "an
owner *can*", not "an owner *did*". The first real venue order is a separate, higher-value arc.

---

## 4. Recommended execution blueprint (Option B, phased)

Conventions: **RED-LINE** = money/checkout/schema/RLS surface (operator gate mandatory before
staging deploy; council per CLAUDE.md red-line globs). **OP** = operator-only action (protect-path
or judgment). Effort: S ≤ ½ day · M = 1–2 days · L = 3–5 days (lane-days, Haiku-doer where
routable). Every step's VbM proof must ship its RED case.

### Phase 0 — Ground truth (do first, ~½ day, no code)

| # | Action | Gate marker | VbM falsifiable proof | Effort |
|---|---|---|---|---|
| 0.1 | **Probe staging's actual cutover state**: read `cutover_flags` table via staging DB (read-only; creds per `staging-db-access-2026-06-30`) + assert `x-dowiz-cutover` response header presence/absence on one route per surface; record a dated frame next to `rebuild-cutover-h_t.json` | none (read-only) | The frame lists per-surface `rust|node` with the SQL output pasted; RED: header assert on a known-Node route must come back absent — if it comes back present the probe method is broken | S |
| 0.2 | **Record the exit-gate ledger** as of today (this doc §1/§2) into PROGRESS.md + memory, incl. the Option A/B/C decision request | OP (decision) | PROGRESS.md diff shows the 7-done/5-open ledger + chosen option signed with date | S |

### Phase 1 — Close the verification debt on the already-built phases

| # | Action | Gate marker | VbM falsifiable proof | Effort |
|---|---|---|---|---|
| 1.1 | **Build the real `hub_checkout` gate** per spec §Feature Flag: recommended placement = the Rust handler (flag OFF + `x-dowiz-cutover` request ⇒ 403/`FEATURE_DISABLED` envelope or documented fall-through-to-legacy semantics; flag ON ⇒ kernel path), flag added to config surface (EnvSchema entry or documented Rust env contract), default OFF everywhere | **RED-LINE** (checkout/money route) — operator sign-off on placement semantics BEFORE code (the spec's OFF-behavior "old Node path used" is a front-door concern; the operator must pick handler-refuse vs front-door-route, see §2.3a) | On staging: flag OFF → cutover POST provably NOT served by the kernel path (assert the refusal envelope or Node marker); flag ON → 201 with `x-dowiz-cutover` asserted + DB totals re-read. RED: flip the flag and assert the opposite outcome fails the suite. Unit RED already half-exists (`hub_checkout_defaults_off`) | M |
| 1.2 | **Make replay-parity real**: replace the placeholder loop in `scripts/replay-parity-check.sh` with actual replay — decode `order_events.payload` → `domain::replay`/`fold` → compare `{status, totals, binding}` to the live row (a small Rust bin or `cargo run` helper is legitimate; the core's `replay_envelopes` exists) — and wire the real `request_hash` into `cause_hash` (kill the `"placeholder"` literal, `pg.rs:865`) | **RED-LINE** (order-of-truth, 1.2's plan-mandated council class) | Run vs staging DB: all touched orders parity-green; RED (GRAND-PLAN's own): flip one staging order's status in a rolled-back txn → job exits 1 → rollback | M |
| 1.3 | **Repair the Playwright suite**: `VITE_BASE_URL` param not hardcoded URL; `locationId` (dto.rs:137) not `location_id`; `x-channel` not `x-sales-channel`; real staging fixture UUIDs (sushi-demo fixture memory); delete the early-return vacuity — every test must end in a hard assertion; add the explicit RED spec (client-injected `total` → asserted 400) | none (test code; touches no product path) | The suite FAILS against staging before fixes land (proving it can red), passes after; paste `--reporter=list` output. RED case ships in-suite | M |
| 1.4 | **Execute the staging validation checklist** (§2.5 items 1, 2, 6): live L0–L11 trace incl. the two new L2 checkout items; owner data-hub flow; customer tracking e2e; courier flow; full repaired Playwright run. Precondition: triage the `checkout-phone` staging break (§6.4 audit) — fix if small, else record it as the checklist's named blocker; decide rate-limiter strategy (test-token bypass vs serialized run) | rate-limiter strategy + `checkout-phone` disposition = **OP**; the trace itself is read/exercise on staging | A dated `docs/ops/reliability-gate-sovereign-YYYY-MM-DD.md` with per-item PASS/FAIL + pasted evidence (order ids, DB re-reads, Playwright output) + memory entry. RED: the suite includes at least one intentionally-broken probe (e.g. wrong-location read → 0 rows) demonstrating the harness can report red | L |
| 1.5 | **Re-verify the 4 red-line gates for the operator packet** (0b-5 / 1.2 / 2.2 / 2.3): one page per gate = invariant, its RED proof pointer, current green run | OP (review/sign-off) | Operator signs the packet (recorded in memory); unsigned = Phase 2 does not start | S |

### Phase 2 — Build the remaining D1 scope (minimum-honest versions)

| # | Action | Gate marker | VbM falsifiable proof | Effort |
|---|---|---|---|---|
| 2.1 | **Phase 2.1 distribution artifacts** exactly per GRAND-PLAN (attribution link `/s/:slug?ch=<token>` + client-side QR + param→header plumbing + channel CRUD UI which also **completes 1.5's dashboard tab + i18n al/en**), flag `hub_channels` default OFF; module lands in `modules/` per A5 with manifest | flag default-OFF; no red-line (write-only attribution doctrine — the registry never feeds pricing/authz; module-integrity enforces) | GRAND-PLAN's own gate: Playwright vs staging — `?ch=qr` order ⇒ `metadata.channel='qr'` DB re-read + dashboard count increments THROUGH the UI; forged `?ch=<junk>` ⇒ `other` + order still succeeds. RED: break the param→header map on a proof branch → attribution red while order-success green | M–L (M spec'd; +UI/i18n from 1.5 completion) |
| 2.2 | **Phase 1.4 minimum**: switch the dual-write hash input to `domain::codec::canonical_bytes` (or land a pinned equality test if serde_json bytes are canonical already), then the pgcrypto oracle | none per plan (no red-line) — but it touches `pg.rs` money-adjacent code; keep the diff surgical | pgcrypto `digest(payload,'sha256') == content_hash` for ALL staging rows (two independent implementations agreeing on real data). RED: mutate one stored payload byte in a rolled-back txn → mismatch detected | S |
| 2.3 | **PARK 1.3 + 2.4** with dated markers in PROGRESS.md + GRAND-PLAN annotation: 1.3 → "revisit at first second-transport demand OR at A3 resume (A3 is blocked on it)"; 2.4 → "revisit when aggregator ingestion is scheduled; single-money-surface invariant restated here" | OP (this IS the exit-gate redefinition — requires the operator's explicit line) | The PARKED entries exist, dated, operator-attributed; the exit-gate paragraph in GRAND-PLAN gains a dated amendment note (never silently edited) | S |

### Phase 3 — 0b-6 CI sovereign gate activation

| # | Action | Gate marker | VbM falsifiable proof | Effort |
|---|---|---|---|---|
| 3.1 | Extend `proposed-sovereign-core-ci/APPLY.md` job: add `cargo deny check bans` **scoped to the core ban-list** (NOT workspace advisories — pre-existing RUSTSEC-2023-0071/num-bigint would perma-red it) + keep tests by manifest-path (rename-proof) | none (doc/staged artifact) | Local run of the exact job commands green; deny RED: add `chrono` to core deps → `cargo deny` fails → revert | S |
| 3.2 | Operator applies the job to `.github/workflows/ci.yml` (`cp` per APPLY.md; protect-path) | **OP** | First CI run green on the branch (link pasted) | S (OP) |
| 3.3 | **Re-prove RED in CI** on a throwaway branch: (a) `chrono` in core deps → deny job fails; (b) `SystemTime::now()` inject → gate-2 fails. Then delete the branch | none (throwaway branch) | Two red CI run links + the clean green run after — "runs locally ≠ wired remotely" discharged | S |
| 3.4 | Operator marks `sovereign-core` a required status check on the `main` branch-protection rule | **OP** (GitHub settings) | GitHub API read-back of the protection rule shows the check required; RED: a PR missing the check cannot merge (observe the blocked merge box) | S (OP) |

### Phase 4 — Merge + prod (consumes G01/G02; do not start before their blueprints land)

| # | Action | Gate marker | VbM falsifiable proof | Effort |
|---|---|---|---|---|
| 4.1 | Adopt the G01 rewrite-aware merge design (tree-based curated integration of the staging-validated set onto `origin/main`); rehearse on a scratch branch; the GDPR trio (`5ded9f19`/`58caf4f4`/`d6b3473e`) rides the SAME or an EARLIER merge — never after (audit rec #1) | **OP** + G01 dependency; secrets remote force-push (Layer 0.1) is its own OP gate per G02 | Scratch-branch merge builds green, `pnpm verify:all --ci` green, migration set diff reviewed; RED: the rehearsal intentionally includes one known-conflict file to prove the conflict process reports | M (excl. G01 work) |
| 4.2 | Prod deploy on explicit operator approval; `hub_checkout` + `hub_channels` + `aggregator_view` all OFF in prod config; migrations prod-apply per runbook | **RED-LINE + OP** (prod, money) | Post-deploy: `/livez` 200, conservation SQL over prod orders green, flags read back OFF (config assert), one legacy order placed and traced; RED: the config assert is the 2.4-style CI check — flipping a flag in the PR reds it | M |
| 4.3 | Exit-gate closure record: one dated doc + memory entry mapping every gate clause to its proof artifact; PROGRESS.md cursor set to CLOSED | OP (sign) | The closure doc exists with per-clause links; any clause without a proof link = gate NOT closed (the doc's own lint) | S |

**Total effort (Option B): ~8–11 lane-days + 4 operator touchpoints** (1.1 placement, checklist
strategy calls, CI apply + required-check, merge/prod). Option A adds ~2 days (1.3 + 2.4).

---

## 5. Risks & rollback

- **Staging drift erodes proofs (highest, already happening).** The 0b-5 deployed-reality proof and
  the h_t flag frame are only as good as staging's *current* state; every redeploy since 07-07
  weakens them. Mitigation: Phase 0.1 probe FIRST; re-run the cheap header/flag probe after every
  staging deploy during this arc. Rollback: n/a (read-only).
- **False-green class (the arc's signature risk).** Two of the recorded gates (replay-parity,
  Playwright suite) were false-green-capable; assume others may be. Mitigation: VbM RED case
  mandatory per step (already in the tables); treat any gate that has never been seen red as
  unverified. Rollback: n/a.
- **1.1 (hub_checkout gate) touches the live money route.** A wrong refusal semantics could 403
  legitimate legacy creates. Mitigation: header-scoped (only `x-dowiz-cutover` requests), legacy
  path byte-untouched (the existing handler already isolates this), staging-only until Phase 4,
  RED-LINE operator gate before deploy. Rollback: `git revert` + `bash scripts/deploy-staging.sh`
  (the 0b-5 protocol proved a ~26s staging deploy cycle; prod is not in scope until 4.2).
- **Real replay-parity may find real divergence** (dual-write bugs between fold-columns and the
  log). That is the point — if it reds, STOP and record per PROGRESS GUARDRAILS; do not "fix the
  test". The event log is not yet an authority for reads, so a divergence is repairable without
  user impact (fold-columns stay authoritative).
- **0b-6 mis-scoped cargo-deny bricks CI** (pre-existing RUSTSEC advisories). Mitigation baked into
  3.1 (bans-only scope). Rollback: operator removes the required-check flag (one click) — the job
  itself stays informative.
- **Required check on main + bifurcated history deadlock.** If the required check is flipped on
  before the G01 merge design exists, no branch can merge (nothing shares history). Mitigation:
  3.4 sequenced after G01's merge rehearsal exists, or applied with the check initially
  non-required until 4.1 rehearses green.
- **Scope creep on the parked seams** (GRAND-PLAN's own warning: the port wants to be a storage
  abstraction, the stub wants ingestion, the signature slot wants signing/bebop2). Mitigation: the
  PARKED entries name the re-entry trigger; anything earlier loses in review by default.
- **Opportunity-cost risk (§7.8).** This whole gate can be closed perfectly and still validate
  nothing about the business. The blueprint deliberately keeps Option B lean so the operator can
  point the freed week at a real venue order.

---

## 6. Operator decision points

1. **Choose A / B / C (§3)** — B recommended. If B: sign the exit-gate amendment (Phase 2.3's
   PARKED markers). This is the single decision that de-inflates "shipping-ready" honestly.
2. **`hub_checkout` gate placement** (step 1.1): Rust-handler refuse vs Node front-door routing —
   money/checkout red-line, needs your semantics call before code.
3. **Staging E2E mechanics**: rate-limiter strategy (bypass token vs serialized runs) and the
   `checkout-phone` break disposition (fix-now vs named blocker) — both block checklist item 1.4.
4. **Red-line packet sign-off** (step 1.5): 0b-5 / 1.2 / 2.2 / 2.3 — the HANDOFF's "user/council
   review" line that was never executed.
5. **0b-6 applies** (steps 3.2 + 3.4): `cp` the CI job into `.github/workflows/ci.yml`; flip the
   required-status-check on main — sequenced against G01 (see risk above).
6. **Merge + prod go** (Phase 4): explicit approval, after G01/G02 land; decide whether the GDPR
   trio ships in the same train or an earlier hotfix train (recommended: earlier or same — never
   after).
7. **Standing arbiter question** (audit rec #2): where does closing this gate rank against rebuild
   cutover resume, OSS flip, bebop, and the first real venue order? This blueprint intentionally
   does not answer that — it only makes the MVP option honestly priced.

---

*Prepared 2026-07-11 by a read-only research session. The only file created is this blueprint.
Working tree, branches, and staging were left exactly as found; all probes were read-only GETs.*
