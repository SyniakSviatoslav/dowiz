# G04 — Rebuild-Cutover Re-baseline: Resume or Mothball (Execution Blueprint)

- **Date:** 2026-07-11 · **Author:** read-only research session (zero code/config touched)
- **Gap source:** 2026-07-11 full-project audit §9 rec 7 — "Re-baseline or formally mothball the
  cutover… a half-flipped staging with an unknown state is the worst of both."
- **Status:** RESEARCH COMPLETE (incl. live staging probe). Execution NOT started — every mutation
  below is a proposal awaiting the operator decisions in §6.
- **Legend:** ⛔ = operator-only / red-line gate · VbM = Verified-by-Math falsifiable proof
  (per docs/operating-model/verified-by-math.md) · "session" = one focused agent session.

---

## 1. Gap & evidence

**The gap (G04):** the rebuild's build-half is 100% (10/10 Rust surfaces built dark, councils,
byte-frozen domain) but the cutover-half is ~15% and has had **zero commits since 2026-07-05**
(audit §1, §5.1). The last known-good state is a 6-day-old frame
(`/root/dowiz/docs/ops/rebuild-cutover-h_t.json`, 2026-07-05T08:45Z, branch `fix/audit-remediation`
@ `45b7c9bf`) while staging was since redeployed repeatedly from sovereign-core-lineage branches —
so whether flags/upstream still reflected reality was **unknown** until this session's probe (§2.2).

**Evidence trail (all verified in-tree / live):**
- Frozen frame: `docs/ops/rebuild-cutover-h_t.json` — 9/10 flags rust (S2+S8 held on Node),
  L0–L7+L11 lifecycle green via Rust, 2,384 ms live rollback, degrade-storm incident 14:02Z.
- Gate record: `docs/ops/reliability-gate-cutover-2026-07-05.md` — GO for flipped surfaces'
  lifecycle; NOT-YET-GO for prod (Nuance-A courier-deliver race → council; S8 parity; tail; S5
  deferred preflight/track-token; per-surface operator gos).
- Harness code: `apps/api/src/lib/cutover/{front-door,flags,matcher,node-keep.generated,ws-router,
  route-templates.generated}.ts`, registered at `apps/api/src/server.ts:438` (HTTP) and `:881` (WS);
  inert unless `CUTOVER_RUST_UPSTREAM` set (`packages/config/src/index.ts:63`).
- Open items pinned by the frame: S2 mixed-account parity gap + ⛔ §3 signature; S8 webhook
  secret-parity ⛔ council; 085–089 formal placement ⛔ (085 watermark HARD date **2026-07-10 has
  PASSED** with no recorded disposition); ~61-route strangler keep-set; degrade-storm ratchet
  (task #15) open; Phase-C not started; Phase-D owner+date blank (REV-C10 HARD GATE).

---

## 2. Research findings

### 2.1 What the h_t frame froze (2026-07-05T08:45Z)

Per-surface flags on staging (`cutover_flags` table, staging DB, placed by draft migration 089):

| Surface | Frozen state | Note |
|---|---|---|
| S1 storefront-read | **rust** (APIs + bot JSON-LD); human `/s/:slug` = Node React SPA (`CUTOVER_ASTRO_UPSTREAM` unset — Astro is a 3/27-island Phase-A scaffold, re-flip gated on the 77-feature UX-parity matrix) |
| S2 auth | **node** (fail-safe re-flip ~15:5xZ) — LIVE parity gap: Rust login 401 `WRONG_AUTH_METHOD` for password+Google MIXED accounts while Node mints a token; Rust hash-SELECT reads a different source than Node `local.ts`; interlock covered password-only class (8/8) |
| S3 catalog | **rust** (products/categories 0-diff; 13 unmounted routes kept) |
| S4 media | **rust** (presign token-proxy; enum bug fixed 886c83d7) |
| S5 orders/money | **rust** (create 201 correct integer money, idempotent; deferred: `preflight` E27 + `customer_track_grants`) |
| S6 realtime WS | **rust** (6/6 router tests + live 101 splice) |
| S7 courier | **rust** (me/shift/assignments/earnings live-probed 200) |
| S8 jobs/webhook | **node** — webhook held on the secret-parity gap (Rust validates the HEADER secret only, ignores the URL-path secret Node 404s on; `TELEGRAM_BOT_SECRET` also missing on dowiz-rust-staging); Rust crons run DARK (advisory-lock/SKIP-LOCKED safe) |
| S9 GDPR | **rust** (erasure ran end-to-end to 'completed' via Rust engine) |
| S10 platform-admin | **rust** (plane-gate armed 401; fallback_health DEFINER discriminating-proven) |

Also frozen: the degrade-storm incident (a 14:02Z Node restart auto-degraded ALL non-money
surfaces S1–S4,S6,S10 to Node silently while Rust was healthy; flags restored ~15:5xZ via the
migrations-role `cutover_flags` UPDATE; ratchet task #15 opened: boot-grace + restart-regression
test + alert-on-degrade — **discovery was accidental**), and the strangler-tail census (§2.5).

### 2.2 How state can be probed read-only — and what staging actually says TODAY

**There is no status endpoint.** `/health` (apps/api/src/routes/health.ts) exposes 11 checks —
none is cutover; `/livez` is event-loop-only. The **only external oracle** is the response header
the front-door stamps on every forwarded request: `x-dowiz-cutover: rust:<surface>` (or
`astro:S1`) — set exclusively in `forwardToRust()` (front-door.ts:244) **after** the upstream
answered, so header presence proves *the Rust app served the bytes*, not just a flag. Absence ⇒
Node. Auth'd surfaces are probeable with a **garbage Bearer token**: the cheap 401 gate
(server.ts) only checks token *presence*, so the request reaches the front-door and the 401 comes
back from whichever stack owns the surface, header included.

**Live probe, 2026-07-11 (~17:3xZ, all read-only GETs vs `https://dowiz-staging.fly.dev`):**

| Probe | Result | Served by |
|---|---|---|
| `/livez`, `/health` | 200 / 200 (health "degraded"-class body) | node |
| `GET /public/locations/demo/menu` | 200 | **`rust:S1`** |
| `GET /api/public/theme/demo` | 200 | **`rust:S1`** |
| `GET /s/demo` (Googlebot UA) | 200 | **`rust:S1`** (bot JSON-LD path) |
| `GET /s/demo` (human UA) | 200, **no header** | node React SPA (astro unset — matches frame) |
| `GET /api/owner/menu/products` (garbage bearer) | 401 | **`rust:S3`** |
| `GET /api/orders/<zero-uuid>` (garbage bearer) | 401 | **`rust:S5`** |
| `GET /api/courier/me` (garbage bearer) | 401 | **`rust:S7`** |
| `GET /api/owner/locations/<zero-uuid>/gdpr-requests` | 401 | **`rust:S9`** |
| `GET /api/owner/onboarding/<zero-uuid>/state` | 401 | **`rust:S10`** |
| `GET /api/auth/telegram/poll` | 400 VALIDATION_FAILED, **no header** | node (S2 = node, matches frame) |
| `GET /api/owner/orders` (S5 **keep-route** control) | 401, **no header** | node — per-route keep gate working |
| PROD control `dowiz.fly.dev` (same S1+S3 probes) | 200/401, **no header** | prod harness inert, as designed |

**Headline finding: staging is NOT in an unknown state — it is still live on Rust, and every
HTTP-probeable surface matches the 07-05 frame exactly** (S1/S3/S5/S7/S9/S10 rust, S2 node, human
page node). The harness, the `CUTOVER_RUST_UPSTREAM` env, the `cutover_flags` rows, and the
`dowiz-rust-staging` app all survived every redeploy since 07-05. Not probeable via cheap HTTP:
**S4** (no GET routes — all presign/confirm POSTs), **S6** (WS upgrade; 101 carries no marker),
**S8 flag** (all its GETs are in the keep-set → Node regardless). Authoritative closure for those
three = one read-only `SELECT surface, target, readiness_ok FROM cutover_flags` on the staging DB
(migrations/owner role — ⛔ credentialed, listed as Phase-0 step P0-2).

**The nuance that keeps this "worst of both":** the fact that staging *still* serves money-surface
traffic (S5 create!) from a Rust build is not the same as *proven parity today* — see §2.3.

### 2.3 What happened to staging since 07-05 (reconstruction)

- **07-05 (late):** frame frozen; astro unset; flags restored post-degrade-storm. Branch
  `fix/audit-remediation` @ 45b7c9bf was the deployed lineage.
- **07-06:** OG-preview/demo-upgrade deploys (sovereign-core lineage; memory
  `og-preview-demo-upgrades-2026-07-06.md`).
- **07-07:** Sovereign Core work deployed to staging repeatedly — 0b-5 RED proof: inject
  CorridorBreach → deploy **v265** (15:15:20Z) → observe refusal → revert → **v266** (15:15:46Z)
  (memory `red-proof-0b5-completed-2026-07-07.md`); reliability-gate session + 2 bug-fix commits;
  formal migrations 085/086 (payment-outcome-prepaid `a7dd2609`, order-ratings `d6f94c1f`)
  committed 18:45–18:54Z and applied to the staging DB.
- **07-08 → 07-10:** attention left for bebop-repo; paleo DIGs committed 07-10 but **GH issue #19
  (cloud-sandbox 403 egress) blocks all staging deploys from agent sessions** — so staging most
  plausibly still runs the 07-07 lineage.
- **Do those deploys contain the harness?** Yes — proven two ways: (a) the current
  `feat/paleo-dinosaur-digs` tree (superset of the sovereign lineage) registers the front-door +
  WS router in server.ts and carries the full `lib/cutover/` module with the 61-route keep-set
  regenerated on 07-05 (`2924142e`); (b) **the live probe above** — a deployed image without the
  harness could not stamp `x-dowiz-cutover`.
- **What DID silently move:** the Rust upstream itself. `dowiz-rust-staging` was redeployed with
  sovereign-core code (kernel/`hub_checkout` dark, plus the v265 inject/revert cycle). **The
  parity oracle (179-spec E2E net, live-PG cargo suite) has not been re-run against the currently
  deployed pair since 07-05.** So: routing state = confirmed; *behavioral parity* state = stale by
  4 days of Rust-side commits. That is the honest remaining unknown, and it is exactly what a
  re-baseline run closes.
- **Degrade-storm exposure is still armed:** front-door.ts has no boot-grace (health probe starts
  immediately; 3 consecutive fails × 5 s interval ≈ 15 s of Rust unavailability trips a global
  degrade of all non-money surfaces, persisted in the DB, never auto-reverted, alert = log line
  only). Every future staging Node restart that races a Rust outage/redeploy window re-runs the
  07-05 storm. Task #15 remains open in code (verified: no grace/alert in the current tree).

### 2.4 Migrations 085–089: drafts, the passed HARD date, and a numbering collision nobody recorded

**The five drafts** (all "⛔ OPERATOR ACTION REQUIRED — place VERBATIM into
`packages/db/migrations/`"; applied to the **staging DB only** via the shim runner):

| Draft | Location | What it does |
|---|---|---|
| `1790000000085_settlements-catchup.ts` | docs/design/audit-fix-money/migration-drafts/ | MONEY M-2: settlement generation becomes catch-up + immutable-once-paid + idempotent; single-flight advisory lock; operator-gated historical backfill; **bakes the literal watermark `2026-07-10 00:00:00+00` at 3 sites (lines 66/133/148)** |
| `1790000000086_refund-due-trigger.ts` | audit-fix-money/migration-drafts/ | MONEY M-1: non-throwing AFTER-UPDATE trigger recording `refund_due` on any writer moving a paid order to CANCELLED/REJECTED |
| `1790000000087_refund-due-reconciler.ts` | audit-fix-money/migration-drafts/ | MONEY M-3: `app_reconcile_refund_due()` deterministic alarm-of-last-resort, called by the timeout-sweep worker each tick |
| `1790000000088_gdpr-erase-definer.ts` | audit-fix-rls-reliability/migration-drafts/ | N1.1 structural fix: SECURITY DEFINER `gdpr_erase_customer` so post-NOBYPASSRLS erasure works without a table-wide customers RLS arm |
| `1790000000089_cutover-flags.ts` | rebuild-cutover-harness/migration-drafts/ | ADR-0022 §2 `cutover_flags` store: dowiz_app SELECT-only; flips = ⛔ operator acts on the migrations role; constrained `cutover_auto_degrade` DEFINER (node-direction only, refuses S5/S7/S9) |

**The 085 watermark chain of custody:**
1. `OPERATOR-TODO-prod-cutover.md` §2 (07-05): HARD MONEY GATE — apply 085 to prod **on or before
   2026-07-10** with the literal untouched; if it slips, **bump all three literals first**. Erring
   late = SAFE (rows defer to the operator backfill); erring early = **DOUBLE-PAYS couriers**.
2. **S5-council downgrade note** (`docs/design/rebuild-orders-s5-council/`): Q7/REV-S5-7 resolved
   that S5 code is **independent of 085**; 086-draft lands before the S5 flip; 085 stays on its own
   schedule with the watermark surfaced as a standalone **operator timing gate + tracked pre-apply
   assert** ("the 2026-07-10 tracked pre-apply-assert gate", resolution.md:103; counsel: "lift the
   watermark out of the S5 footnote into a standalone forcing function", counsel-opinion.md:281).
   So 085 is *draft-status with a real guard*, not an S5 blocker.
3. **2026-07-10 passed with no disposition recorded anywhere** (verified: no memory/doc entry).
   Prod never received 085 → prod (migrations ≤084, origin/main tip `7e24f139`) still runs the OLD
   settlement fn with its known loss class. Mitigating honesty: prod has no evidence of real
   (non-demo) settlement volume, so today's *realized* cost is ~0 — but the fix is dead-on-arrival
   the day a real venue settles cash, and the literal is now in the UNSAFE direction (earlier than
   any possible apply date → applying 085 verbatim now would scan rows processed by the old fn =
   the exact double-pay hazard). **Disposition required — see Phase A2/B2.**
4. **NEW finding (this session): the draft numbers are no longer free.** On 07-07 the sovereign
   session formally placed *different* migrations at 085/086
   (`1790000000085_payment-outcome-prepaid.ts`, `1790000000086_order-ratings-customer-insert-policy.ts`
   — commits `a7dd2609`/`d6f94c1f`, sovereign lineage only, staging DB applied, NOT on prod).
   The five drafts therefore **cannot be placed under their own names**: any formal placement must
   renumber them to ≥ 087 (proposed: 087 settlements-catchup · 088 refund-due-trigger ·
   089 refund-due-reconciler · 090 gdpr-erase-definer · 091 cutover-flags), reconcile whatever
   names the staging shim runner recorded in `pgmigrations`, and re-run
   `ci-migration-preflight.mjs` + `ci-schema-drift.mjs` against BOTH DBs. Nobody has recorded this
   collision before this document.

**Post-07-10 disposition (recommended, executed in Phase A2 or B2):** (a) renumber per above;
(b) bump the three watermark literals to a named future date `T_apply + margin` chosen by the
operator at placement time; (c) add the pre-apply assert REV-S5-7 asked for — a deterministic
check that FAILS the apply if `now() > watermark` (VbM: run it against the stale literal → RED;
against the bumped literal → GREEN); (d) record the disposition in living memory + REGRESSION-LEDGER.

### 2.5 The strangler tail, enumerated precisely (current tree = deployed keep-set)

`node-keep.generated.ts` (regenerated 07-05, commit `2924142e`) = **61 mapped-but-unmounted routes**
that stay on Node even while their surface is flipped — counts per surface
`{S1:1, S2:2, S3:13, S5:23, S8:17, S10:5}` (the h_t "remaining_76 / 58 RED-LINE" text predates the
R2a port; the generated file is the ground truth):

- **S5 — 23 (red-line money/PII):** settlements ×8 (list/get/approve/pay/dispute/reopen/
  regenerate + refunds-list), refunds-sent ×1, owner order-actions ×5 (assign-courier / deliver /
  mark-no-show / pickup / reveal-customer-contact), order verify GET, dashboard snapshot,
  owner-orders GET, order messages ×3, customer rating POST, customer order-status GET,
  `POST /webhook/payments/plisio` (HMAC webhook — zero tests, byte-fidelity unproven; REBUILD-MAP §8).
- **S8 — 17 (red-line FORCE-RLS; need operator-placed SECURITY DEFINER fns):** notifications ×4
  (status/targets/target-PUT/test + telegram connect-init), push ×5 (state/subscribe×2/
  unsubscribe×2), signals ×4 (list/compute/acknowledge/dismiss), alerts ×3 (list/ack/ack-all).
- **S3 — 13 (deferred-by-design):** promotions ×6 (POTEMKIN — no redemption runtime), brand ×3
  (incl. AI generate), menu-import ×3, menu translate ×1 (AI; prior-council-deferred).
- **S10 — 5 (red-line provisioning):** activation status/pickup/publish, onboarding, onboarding/start.
- **S2 — 2 (red-line auth):** customer OTP send/verify (OTP_ENABLED dark anyway).
- **S1 — 1:** `GET /branding-preview/:slug` (SPA-fallback page, Rust has no shell by scope-cut).

Plus, outside the keep-set: **whole-surface holds** S2 (login/refresh/google/telegram/track —
mixed-account gap + ⛔ §3 signature) and S8 webhook (⛔ secret-parity council +
`TELEGRAM_BOT_SECRET`); **S1 HTML keeps** ×5 (`/s/:slug/{cart,checkout,order/:id,orders/:orderId}`
+ branding-preview — until Astro parity); **2 UNMAPPED analytics routes**
(`/api/owner/analytics`, `/api/owner/analytics/product-orders` — flagged in
route-templates.generated.ts as needing an explicit S5 sub-scope decision, currently permanent-Node
by fail-closed default); **S5 deferred** `preflight` (E27) + `customer_track_grants`; and
**S6 semantics** (established sockets stay on Node until natural disconnect — by design).

### 2.6 Resume vs mothball — the honest inputs

- **Resume costs:** ~12–18 focused sessions of red-line-grade work (S2 fix+interlock ≈ 1–2; S8
  parity ≈ 1; migrations disposition ≈ 1; degrade ratchet ≈ 1; tail ≈ 6–10 as per-surface council-style
  efforts — the 07-05 session itself recorded "the remaining tail is UNIFORMLY complex… that's where
  a PII/money bug slips a happy-path probe"; prod standup + flips + soak ≈ 2–4) **plus ≥ 8 distinct
  operator acts** (§6) **plus** two environmental blockers: GH #19 (sandbox egress blocks agent
  staging deploys — deploys must run from the operator's machine or the policy must change) and the
  secrets-history scrub (blocks any main-merge/prod push path, OPERATOR-TODO §1).
- **Mothball costs:** the 69K-LOC Rust twin + 1,041 tests begin rotting against a moving schema —
  every future formal migration applied to staging changes tables under sqlx code that isn't
  rebuilt, silently accumulating exactly the bind/decode class the cutover kept finding (ledger
  #77); the h_t frame and councils age; re-entry cost grows roughly per schema/contract change.
  Mothballing does NOT delete anything — the option stays alive at the cost of a small keep-alive
  ritual (§4 Path B).
- **What the operator's attention implies:** 100% of attention since 07-08 is on bebop-repo
  (audit §2.3); the dowiz memory corpus hasn't been updated since 07-10. A resume plan that
  needs ≥8 operator acts against that gradient will stall again unless the operator explicitly
  re-prioritizes. Conversely, the probe shows the half-flipped state is *stable but unowned*:
  staging money traffic (S5 create) is served by an unmaintained Rust build, staging is no longer
  a faithful prod mirror (prod = Node everywhere), and the degrade-storm trap is still armed.
  **Doing neither — the status quo — is the only indefensible option.**

---

## 3. Options & tradeoffs

| | **A. RESUME** (finish the cutover) | **B. MOTHBALL** (formal PARKED) |
|---|---|---|
| Cost | 12–18 sessions + ≥8 operator acts + GH#19/secrets unblocks | 1–2 sessions + 1 operator flag-act; keep-alive ≈ 0.2 session/month |
| Payoff | The rewrite's actual point: Rust serving prod, Node decommissioned (Phase D), 69K LOC becomes the product | Staging returns to prod-mirror truth; rot is contained + measured; option preserved with dated re-entry criteria |
| Risk | Red-line money/PII ports where "a bug slips a happy-path probe"; operator-gate starvation → stalls again mid-flight (the current state IS that outcome once already) | Rust rot compounds quietly; re-entry cost grows; the sunk 69K LOC may never pay out; sovereign-core work that lives in the Rust shell (kernel, hub_checkout) loses its staging host unless carved out |
| Reversibility | Every flip is one UPDATE, rollback proven at 2.4 s; per-surface | Fully reversible (nothing deleted); re-entry = re-run Phase 0 + re-ratify councils older than the re-entry criteria |
| Fit to current attention | Poor unless operator re-commits; requires sustained multi-session focus | Good — one decisive session, then bebop continues undisturbed |
| Recommendation weight | Choose ONLY with an explicit operator commitment (named next-session cadence + the §6 acts banked up-front) | **Default recommendation if no such commitment exists** — an honest PARKED beats a zombie cutover |

**Hybrid worth naming (B+):** mothball the *cutover program* but keep S1-read (and optionally
S3/S10 read paths) flipped on staging as a living canary — smallest non-money read surface keeps
the harness + Rust build exercised for ~0 extra cost. Rejected as the primary recommendation
because it keeps staging a non-mirror of prod; offered as an operator checkbox in §6-D3.

---

## 4. Recommended execution blueprint

### PHASE 0 — RE-BASELINE (common to both paths; do this regardless; ~1 session)

**P0-1. External probe + new dated frame.** *(action)* Run the read-only probe procedure below and
write `docs/ops/rebuild-cutover-h_t-2026-07-11.json` (new file, additive) containing per-surface
`{surface, expected, observed_header, http_status, probe_url, ts}` assertions.

```bash
# Read-only serving-origin probe (no auth mutation; garbage bearer only exercises the 401 path)
B=https://dowiz-staging.fly.dev; AU='Authorization: Bearer probe-invalid-token'
chk(){ s=$1; exp=$2; shift 2; got=$(curl -sS -o /dev/null -m15 -D- "$@" | tr -d '\r' \
  | awk -F': ' 'tolower($1)=="x-dowiz-cutover"{print $2}'); got=${got:-node};
  [ "$got" = "$exp" ] && echo "GREEN $s=$got" || { echo "RED $s expected=$exp got=$got"; FAIL=1; }; }
chk S1  rust:S1 "$B/public/locations/demo/menu"
chk S3  rust:S3 -H "$AU" "$B/api/owner/menu/products"
chk S5  rust:S5 -H "$AU" "$B/api/orders/00000000-0000-0000-0000-000000000000"
chk S7  rust:S7 -H "$AU" "$B/api/courier/me"
chk S9  rust:S9 -H "$AU" "$B/api/owner/locations/00000000-0000-0000-0000-000000000000/gdpr-requests"
chk S10 rust:S10 -H "$AU" "$B/api/owner/onboarding/00000000-0000-0000-0000-000000000000/state"
chk S2  node "$B/api/auth/telegram/poll"
chk S5-keep-CONTROL node -H "$AU" "$B/api/owner/orders"          # keep-route must be Node
chk S1-human-CONTROL node -A "Mozilla/5.0 Chrome/126" "$B/s/demo" # astro unset ⇒ Node SPA
exit ${FAIL:-0}
```

*(VbM)* Falsifiable both directions: a silently degraded surface reads `got=node` vs
`expected=rust:*` → RED; a silently *widened* flip trips the two CONTROL rows (keep-route or
human page suddenly carrying a header) → RED. 2026-07-11 execution: **9/9 GREEN** (§2.2).
*(gate)* None — read-only. *(effort)* minutes; wire it as `scripts/rebuild-cutover/probe-frame.mjs`
so re-runs are one command.

**P0-2. Authoritative flag census (closes S4/S6/S8).** *(action)* One read-only
`SELECT surface, target, readiness_ok FROM cutover_flags ORDER BY surface;` on the staging DB
(scratchpad `.dburl-OPERATIONAL` / migrations role) + `flyctl status -a dowiz-rust-staging` +
`flyctl releases -a dowiz-staging -a dowiz-rust-staging` (the memory Fly token is expired —
re-auth needed ⛔ operator-supplied credential). Append results to the new frame, incl. which
image/lineage each app actually runs. *(VbM)* DB rows must equal the HTTP-observed set on the 7
probeable surfaces — any mismatch is RED and means the header oracle or the store lies (a finding
in itself). *(gate)* ⛔ credential access only (no mutation). *(effort)* 0.2 session.

**P0-3. Record the frame + the 085 overdue flag in living memory.** *(action)* New memory entry
`rebuild-cutover-rebaseline-2026-07-11.md` pointing at the new frame; explicitly state "085
watermark date PASSED, disposition pending §6-D2". *(VbM)* n/a (documentation), but the frame it
indexes is the proof artifact. *(effort)* trivial, same session.

> **Phase 0 exit:** a dated, falsifiable, per-surface serving-origin frame exists; S4/S6/S8 closed
> via DB census; operator has the §6 decision sheet. Only then choose A or B.

---

### PATH A — RESUME (sequenced; ⛔ gates marked; ~12–18 sessions total)

**A0. Operator commitment gate.** ⛔ Operator banks, in one sitting: S2 §3 signature decision,
S8 canonical-secret decision, 085 watermark new date, Phase-D owner+date, prod-creds read
approval (re-confirm the 07-05 banked Y), S5/S9 prod-flip re-confirm, GH#19 deploy path
(operator-machine deploys vs policy change), secrets-scrub schedule. **If this sheet cannot be
filled, take Path B — do not start Path A partially.** *(VbM)* the sheet itself, committed to
docs/ops/, with no blank ⛔ fields — a blank field = RED, path not entered.

**A1. Degrade-storm ratchet (task #15) — first code change, before anything else touches flags.**
*(action)* In front-door.ts: (i) boot-grace — `UpstreamHealth` must not count failures for a
configurable warm-up (e.g. 30 s) after start; (ii) alert-on-degrade — `autoDegrade` success fires
the existing Telegram alerter path, not just a log line; (iii) restart-regression test — an
integration test that boots the harness against a stub upstream that is unreachable for the first
N seconds and asserts **zero** `cutover_auto_degrade` calls, then kills the stub post-grace and
asserts degrade + alert fire. *(VbM)* the test is the RED case by construction: revert the grace
→ test reproduces the 07-05 storm (degrade during boot) → RED; with fix → GREEN. Deploy to
staging, restart Node deliberately, re-run the P0-1 probe → all surfaces still rust. *(gate)*
none (non-money, reversible, additive). *(effort)* 1 session.

**A2. Migrations disposition (085–089 → 087–091).** *(action)* Renumber drafts per §2.4-4; bump
the three 085 watermark literals to the ⛔ A0 date; add the pre-apply assert; reconcile staging
`pgmigrations` names; run `ci-migration-preflight.mjs` (SOURCE=prod) + `ci-schema-drift.mjs`;
⛔ operator places files into `packages/db/migrations/` (protect-path). *(VbM)* pre-apply assert
RED on the stale literal / GREEN on the bumped one; schema-drift zero-diff after staging apply;
`SELECT count(*) FROM pgmigrations WHERE name LIKE '179000000008%'` census matches the recorded
mapping. *(gate)* ⛔ red-line migrations placement + apply. *(effort)* 1 session + operator act.

**A3. Parity re-run against the CURRENTLY deployed pair (closes §2.3's real unknown).** *(action)*
Re-run the reliability-gate L0–L11 trace + the `#[ignore]` live-PG cargo suite vs staging; wire
that suite into CI (`docs/design/ci-rust-live-pg/` — "THE TOP RATCHET" per the frame). *(VbM)*
live-PG suite ≥ 833 pass / 0 product-fail (07-05 baseline; the 3 test-infra fails stay triaged);
L0–L7+L11 green with `x-dowiz-cutover` asserted per stage. RED case: the suite is exactly the
class that fails on bind/decode drift. *(gate)* none (staging, read/trace + test orders).
*(effort)* 1 session (+CI wiring).

**A4. S2 closure.** *(action)* Fix the mixed-account (password+Google) login parity gap (align the
Rust hash-SELECT source with Node `local.ts`); extend the interlock to the mixed class; mechanize
AR-2 (leeway/body-kid round-trip flag-interlock the front-door reads); implement the C2
refresh-quiesce flip runbook; ⛔ operator signs amendment §3
(`rebuild-auth-s2-council/amendment-cutover-reratify-RESOLVE.md` — currently 🟡 NOT RATIFIED);
then re-flip S2 on staging. *(VbM)* mixed-account seeded fixture: Rust login red (401
WRONG_AUTH_METHOD) → green (200, token verifies on BOTH stacks both directions); interlock
10/10 incl. the two new classes; AR-2 interlock flag demonstrably blocks a flip while red (flip
attempt with red flag → refused = the RED case). *(gate)* ⛔ §3 signature; auth = red-line.
*(effort)* 1–2 sessions.

**A5. S8 closure.** *(action)* ⛔ council decides canonical secret (recommend: Rust adopts Node's
path-secret 404 AND keeps the header check — strict superset, no Node change); align Rust; ⛔ set
`TELEGRAM_BOT_SECRET` on dowiz-rust-staging(+prod later); flip S8 webhook on staging; design the
S8 DEFINER fns for the notifications/push/signals keep-routes (feeds A6). *(VbM)* four-cell truth
table probed live: {right,wrong} path-secret × {right,wrong} header → Rust responses byte-class
identical to Node (404/401/2xx), wrong-secret cells are the RED cases. *(gate)* ⛔ council +
secret set. *(effort)* 1 session.

**A6. Strangler tail — per-surface focused efforts (NOT route-per-deploy grinding; the 07-05
recommendation stands).** Order by risk-adjusted value: (1) S5 order-actions batch (aliases the
already-proven transition engine) + settlements batch — council-style, includes the
courier-deliver conflict-bool Nuance-A ⛔ council decision; (2) S8 notifications/push/signals
(needs A5's DEFINER fns ⛔ placed via A2 flow); (3) S10 provisioning/activation; (4) S5 plisio
webhook (build the HMAC byte-fidelity harness FIRST — zero tests today); (5) S3
brand/import/translate (5 council-deferred; promotions stays parked with the POTEMKIN re-scope);
(6) S2 OTP (dark flag anyway); (7) decide the 2 UNMAPPED analytics routes (S5 sub-scope or
permanent-Node carve-out — record it). *(VbM per batch)* keep-set line-count strictly decreases
(the file IS the ratchet: `NODE_KEEP_ROUTES.size` 61 → target ≤ 13 pre-Phase-D, S3-deferred +
POTEMKIN only); per-route live probes with node-parity bodies; live-PG suite stays green; E2E
slice per surface green. RED: any probed route returning a Rust body ≠ Node oracle diff. *(gate)*
⛔ per-batch council for money/PII/FORCE-RLS/OTP. *(effort)* 6–10 sessions.

**A7. Prod standup + flips (Phase B close).** *(action)* ⛔ secrets/scrub posture per A0; create
`dowiz-rust` (prod, flycast-only); ⛔ operator seeds prod secrets (or approves read-from-prod-Node);
apply migrations (A2 set) to prod DB via the gated flow; deploy front-door env
(`CUTOVER_RUST_UPSTREAM`) to prod **with A1's ratchet already in the image**; S1-first flip +
rollback drill; then S3/S4/S6/S7/S10 in that order; ⛔ S5 and S9 flips = explicit per-surface
operator Y (re-confirmed, not carried from 07-05); 48 h soak per surface batch; the P0-1 probe
(prod variant) runs as a scheduled check during the whole window. *(VbM)* per flip: probe frame
asserts the new origin; rollback drill ≤ 5 s measured (baseline 2,384 ms); after-flip L0–L11
trace green on prod-equivalent smoke; auto-degrade alert path test-fired once (RED case: alert
must arrive — silence = RED). *(gate)* ⛔ everything in this step. *(effort)* 2–4 sessions +
operator acts, spread over soak days.

**A8. Phase C — channel-hub heads (outline; post-S5-prod-stability only).** Feed heads
(JSON-LD/GBP/Meta) → MCP/`.well-known/ucp` read-only stub → TMA wrap → conversational heads, each
under the §6 REBUILD-MAP invariants (cart-token single-money-surface; thin heads
machine-enforced; per-head kill switch; channel count = ops budget). Owner: ______ ⛔ · target
window: ______ ⛔. *(VbM)* per head: dependency-cruiser boundary rule red→green + feed-freshness
monitor with a poisoned-fixture RED. *(effort)* not costed here; explicitly OUT of the resume
commitment — schedule after A7 stabilizes.

**A9. Phase D — decommission (REV-C10 HARD GATE).** Owner: ______ ⛔ · date: ______ ⛔. Node
removal only after: full 179-spec run (frozen skip-list) + 162-shot visual re-baseline
human-reviewed + 48 h soak + reliability-gate GO + RLS adversarial/B3 matrix + all 67
regression-ledger rows re-proven + map-coverage zero UNMAPPED/UNBUILT/ORPHAN. *(VbM)* the
map-coverage gate is the falsifier by construction (any orphan/unbuilt row = RED CI).

---

### PATH B — MOTHBALL (formal dated PARKED; ~1–2 sessions + monthly keep-alive)

**B1. Flag posture decision + execution.** ⛔ Operator chooses ONE:
- **B1-full-node (recommended for a true park):** one migrations-role UPDATE per flipped surface
  → `target='node'` (S1,S3,S4,S5,S6,S7,S9,S10), leaving `CUTOVER_RUST_UPSTREAM` set (harness
  live-but-idle) or unset (fully inert — also removes the degrade-storm trap without code
  change). Staging returns to a faithful prod mirror.
- **B1-canary (the §3 hybrid):** S1 (+S3/S10 optionally) stay rust as a living canary; money
  surfaces (S5/S7/S9) MUST come back to node — an unmaintained Rust build must not keep serving
  staging money paths.
*(VbM)* re-run P0-1 immediately after: full-node ⇒ **zero** headers anywhere (any header = RED);
canary ⇒ exactly the chosen set (any extra/missing = RED). *(gate)* ⛔ flag UPDATEs are
operator/migrations-role acts by design (089 authority model). *(effort)* 0.2 session + operator.

**B2. The PARKED document (the formal act).** *(action)* Write
`docs/ops/rebuild-cutover-PARKED-2026-07-11.md` (or the date of execution), operator-signed ⛔,
containing: (i) the Phase-0 frame as the state-at-park; (ii) what is frozen — `rebuild/crates/*`,
`lib/cutover/*`, the 5 renumber-pending migration drafts **with the §2.4 collision + stale
watermark recorded as the disposition** ("watermark date passed 2026-07-10; literals must be
re-dated at any future placement; numbers 085/086 taken — renumber to ≥087"); (iii) which
councils remain valid vs which expire (S2 amendment stays 🟡 unsigned; Nuance-A courier-race
council pending); (iv) the keep-alive ritual (B3); (v) re-entry criteria (B4); (vi) explicit
statement that dowiz-rust-staging is either scaled to zero ⛔ or retained for the canary. *(VbM)*
plane-guard-style doc lint: the PARKED doc must contain the frame hash + all six sections
(checkable), and MEMORY.md gains the PARKED pointer — absence = RED in the memory index check.
*(effort)* 0.5 session.

**B3. Minimal keep-alive (keeps the option alive, measures the rot instead of guessing).**
Monthly (or per formal migration applied, whichever first): `cargo check + cargo test -p domain
-p api` on the frozen crates + one live-PG suite run vs staging DB + the P0-1 probe (expected
all-node / canary-set). Log one line per run in the PARKED doc. *(VbM)* the live-PG suite IS the
rot detector — a schema-moving migration that breaks a bind/decode reads RED here first; a green
run is a dated "re-entry cost still ≈ constant" proof. *(effort)* ~0.2 session/month, delegable.

**B4. Re-entry criteria (pre-committed, so un-parking is a decision, not a drift).** Re-open via
Phase 0 + Path A when ANY of: (a) a real paying venue is claimed/onboarded (the audit's Tier-1
business trigger — Rust perf/cost story becomes material); (b) Node-stack monthly cost or a
Fastify/dep EOL forces the platform question; (c) the operator finishes/parks bebop and
re-commits the A0 sheet; (d) keep-alive goes RED twice consecutively (rot is now compounding —
decide: fix forward or decommission the rebuild formally via an ADR, which is Phase-D-for-the-
rebuild-itself and needs its own ⛔ sign-off). *(VbM)* each criterion is an observable event, not
a mood; (d) is mechanically derived from B3's log.

---

## 5. Risks & rollback

| Risk | Path | Mitigation / rollback |
|---|---|---|
| Degrade storm re-fires on any staging Node restart before A1/B1 lands | both (status quo) | A1 first-in-order; B1-full-node removes the trap entirely; rollback of A1 itself = revert commit (no flag/data change) |
| Rust-side rot breaks a *currently-serving* staging surface silently (bind/decode class vs new migrations) | status quo & B1-canary | A3/B3 live-PG suite is the detector; per-surface rollback = one `cutover_flags` UPDATE, proven 2.4 s; money surfaces refuse auto-degrade by design → truthful 503, never silent corruption |
| 085 applied with the stale (passed) watermark | A2/B2 | double-pay hazard — the pre-apply assert (new) makes it mechanically impossible; the draft's own comments make it operator-visible; erring late stays safe by design |
| Migration renumbering diverges staging vs prod history | A2 | preflight + schema-drift gates both DBs; the mapping table is committed in the same change; staging `pgmigrations` reconciled by name before prod ever sees the files |
| S2/S7 auth flip splits a token family (revoke mid-shift) | A4/A6/A7 | C2 quiesce + AR-2 interlock are hard preconditions; gate-iv unchanged on both stacks; per-flip cleanup runbook incl. in-shift courier re-auth (AR-4) |
| Prod flip regression | A7 | flag rollback ≤ 5 s per surface; S5/S9 windows accept that in-window money/erasure actions are real — hence explicit per-surface ⛔ Y and soak ordering (S1 first, money last) |
| Path A stalls again mid-flight (the G04 failure mode itself) | A | A0 makes partial entry impossible: no blank ⛔ fields, no start; if any A-step ages > 14 days untouched, auto-fallback rule: execute B1-full-node + B2 with the then-current frame (pre-agreed in §6-D1) |
| Mothball becomes silent abandonment | B | B2 is *dated and signed*, B3 is scheduled and logged, B4 (d) forces an explicit decommission ADR rather than infinite park |
| GH #19 blocks agent-driven staging deploys mid-plan | A | named in A0: operator-machine deploys or network-policy change — decided before entry, not discovered at step 6 |

---

## 6. Operator decision points (the whole blueprint routes through these)

- **D1 — PATH CHOICE (required, single question):** Resume (A) with the A0 sheet fully banked, or
  Mothball (B)? Includes pre-agreeing the A-stall auto-fallback (any step idle >14 days ⇒ B1+B2).
- **D2 — 085 disposition (required regardless of path):** acknowledge the passed 2026-07-10 HARD
  date; approve renumbering 085–089 → 087–091; set the new watermark date (A2) or record
  "re-date at placement" in the PARKED doc (B2). ⛔ red-line migrations.
- **D3 — staging flag posture (required regardless of path):** keep serving Rust on staging
  (only defensible inside Path A), OR B1-full-node, OR B1-canary (S1±S3/S10 only, money back to
  node). One migrations-role act.
- **D4 — degrade-storm ratchet:** approve A1 as a standalone fix even under Path B-canary
  (it is the only code change that is justified on every branch of D3 except full-inert).
- **D5 (Path A only):** S2 amendment §3 signature · S8 canonical-secret call +
  `TELEGRAM_BOT_SECRET` · Nuance-A courier-deliver race council · prod-creds read approval ·
  S5 + S9 prod-flip explicit Y (re-confirmed) · Phase-D owner+date (REV-C10) · Phase-C owner+window ·
  GH #19 deploy path · secrets-scrub schedule (blocks main-merge/prod-push lineage).
- **D6 (Path B only):** sign the PARKED doc; choose dowiz-rust-staging fate (scale-to-zero vs
  canary host); confirm the B3 keep-alive cadence and who runs it.

---

*Research provenance: h_t frame + reliability-gate doc + REBUILD-MAP + OPERATOR-TODO (all in-tree),
memory corpus (session-resume-2026-07-05, red-proof-0b5, cross-branch-todo-map, sovereign-core
handoffs), cutover source read in full, S5/S2 council docs, live read-only HTTP probes of
dowiz-staging.fly.dev and dowiz.fly.dev on 2026-07-11 (§2.2 table = raw results; prod controls
negative). No file outside this document was created or modified; no flag, deploy, or DB row was
touched.*
