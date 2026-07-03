# Meta-Loop Gap Map — missing connections in the self-improving system (2026-07-02)

> Read-only audit of the harness *wiring*, not its parts. The parts are mostly built and mostly
> healthy (`meta-loop-audit-2026-07-02` closed the P0/P1 authority-layer holes). This map is about
> the **connections between the parts** — the hops where signal is produced but never consumed, or a
> script is built but never invoked. Every claim is grounded in a `file:line` or a count.
>
> Method mirrors `memory-corpus-meta-patterns-2026-07-02` #12: *a step that ends in prose with no
> artifact a machine inspects dies within a week; measurement is the precondition for the ratchet.*
> Fix pattern throughout: **advisory-first, deterministic-authority** (pattern #4) — light up the
> silent death with a SOFT check that reuses an existing actuator; never add a HARD gate to an
> advisory mirror.

---

## Scorecard

| # | Gap | Severity | Disconnected hop | Wire tag |
|---|-----|----------|------------------|----------|
| 1 | Calibration ledger is write-only — nothing resolves predictions | **HIGH** | `predict` → **✗ resolve** → reflection → doubt | SAFE (soft check) |
| 4 | INBOX reflection: production gated, consumption ungated | **HIGH** | Stop-hook writes → **✗** → librarian drains | SAFE (soft check) + routine |
| 2 | `scout-feeds` + `asset-surface-scan` orphaned — never invoked | **MED-HIGH** | script built → **✗** → charter SCOUT / any cadence | SAFE |
| 3 | `security-redblue` loop never dispatched | MED | CERTIFIED card → **✗** → any firing | SAFE (blue arm read-only) |
| 5 | Guardrail PROPOSALS have no enactment path | MED | proposals doc → **✗** → librarian/council | needs-council to enact |
| 6 | Hot loops bypass `finalize`; no §5 telemetry | MED | demo-builder/acquisition run → **✗** → metrics.jsonl | SAFE |
| 7 | plane-guard now in CI ✅ but live/health sub-checks never fire | LOW | `agent-health-pass` / `--staging` → **✗** → automation | SAFE |

**Count by severity: HIGH 2 · MED-HIGH 1 · MED 3 · LOW 1.**

---

## GAP 1 — Calibration ledger is write-only; the "read the gap" half never runs (HIGH)

**What's disconnected.** `model-calibration.md:24` defines the whole method as *"record the prediction,
record the fact, read the gap."* Only the first third runs.

**Evidence.**
- `loops/runs/predictions.jsonl` = **1 row**, `"resolved":false` (the single prediction
  `e957e06090d8`, stuck-unresolved by M1 ordering friction — predicted after the run's first event).
- `resolve` appears **only in prose**: `docs/governance/plane-maintainer-agent.md:23` (SENSE step) and
  `docs/design/plane-telemetry-principles/proposal.md:76`. Grep for an executed `resolve --prediction`
  anywhere in logs/scratch: none.
- Cloud run **predicted 3, resolved 0** (`plane-telemetry-closed-loop-2026-07-02.md:47-48`, self-named
  "charter compliance gap — its own first miss-class datum").
- `plane-guard.mjs:142` `telemetry-liveness` checks event *freshness* only — it does **not** look at
  `resolved:false` rows. So an unresolved ledger is invisible.

**Consequence.** The chain `model-calibration.md §4` (`miss/partial → WHY-reflection →
result-vs-expectation doubt trigger → librarian → guardrail`) is **dead at hop 1** — no gap is ever
read, so no reflection, doubt, or guardrail ever originates from calibration. The prediction ledger is
a diary no one re-reads.

**Wiring (SAFE — soft, advisory).** Add a SOFT `plane-guard.mjs` check
`prediction-resolution-liveness`: count `resolved:false` rows whose `run_id` is older than ≥2 firings
(or ts_predicted > N days). `>0 stale-unresolved` → SOFT warn in the digest ("N predictions written,
never resolved — the mirror has a prediction half and no fact half"). **Must stay soft** — the ledger
is "a mirror, never a stick" (`model-calibration.md:104`); a HARD gate here violates the standing
constraint and the advisory-forever plane-guard self-scan (`plane-guard.mjs:206`). This just makes the
omission legible so the cloud SENSE step (which already prescribes resolve) can't silently skip it.
Mirrors the existing `telemetry-liveness` H3 pattern exactly.

---

## GAP 4 — INBOX: production is deterministically gated, consumption is not (HIGH — the core asymmetry)

**What's disconnected.** The Stop hook now **forces** a reflection into INBOX on every qualified
change, but **nothing** forces the librarian to drain it. Production is a hard gate; consumption is
manual prose.

**Evidence.**
- Production side is HARD: `.claude/hooks/require-classification.sh:51-73` — a qualified change (≥3
  code files or a red-line surface) with no `*.reflection.md` in INBOX <8h → `_block`. Registered on
  the `Stop` event (`.claude/settings.json:107`).
- Consumption side has **no trigger**: grep `librarian` across `.claude/hooks/*` and
  `.claude/settings*.json` = **empty**. The only drains are manual `/librarian` or the weekly cloud
  routine `harness-librarian-health` (prose, per `meta-loop-audit-2026-07-02.md:20`).
- Result is monotonic growth: INBOX = **6 files** (`ls docs/reflections/INBOX` — 5 reflections + the
  proposals doc). ARCHIVE = 6, LESSONS = 7.
- This is exactly the failure the 07-02 revival was meant to end (`advisory-arm-revival` reflection:
  "obligations with no artifact die in ~a week"), now recreated in reverse — the intake is armed, the
  outflow is prose.

**Wiring (SAFE — soft check + existing routine as actuator).** Add a SOFT `plane-guard.mjs`
`inbox-drain-liveness` check: `INBOX *.reflection.md count > K` **or** oldest reflection age > N days →
SOFT warn ("librarian backlog: N reflections un-curated since <date>"). Curation itself needs a model,
so it can't be a hard gate — the SOFT check is the smoke alarm; `harness-librarian-health` is the
actuator. This is the single sibling of the `telemetry-liveness` mechanism and reuses the in-CI
plane-guard runner.

---

## GAP 2 — `scout-feeds.mjs` + `asset-surface-scan.mjs` are orphaned (MED-HIGH)

**What's disconnected.** Both scouts were built (2026-07-02) with test suites, but nothing on any
cadence invokes them — including the plane-maintainer SCOUT step that logically owns them.

**Evidence.**
- The charter SCOUT step (`plane-maintainer-agent.md:34-42`) names only `new-dep-scan.mjs` + generic
  "new OSS / upstream releases / research". Grep of the charter for `scout-feeds|asset-surface` =
  **empty**.
- Both scripts are referenced **only** by the `security-redblue` loop artifacts (`security-redblue.yaml`,
  its report/memory, `docs/security/*`, `tools/security-redblue/dry-run.mjs`) — never by `plane-guard`,
  `verify:all`, CI, or the daily routine.
- They have **never run**: `loops/runs/` has `dep-baseline.json` + `inbox-cursor.json` but **no**
  `scout-cursor.json` or `asset-surface-baseline.json` — the state files these scouts write on first
  run don't exist. The CT-log subdomain scout and upstream-advisory scout produce zero signal in
  practice.

**Wiring (SAFE).** (a) Add both scripts explicitly to the charter SCOUT step — they *are* the
"upstream releases of adopted deps" + "new asset surface" the step describes in prose but never
invokes (`docs/governance/` edit, proposable, no `.claude` touch). (b) Add a SOFT `plane-guard`
`scout-liveness` check: newest `scout-cursor.json` / `asset-surface-baseline.json` mtime < N days,
else SOFT "scouts silent" — same H3 anti-silent-skip shape as `telemetry-liveness`.

---

## GAP 3 — `security-redblue` is CERTIFIED but never dispatched (MED)

**What's disconnected.** The loop's telemetry+ledger wiring exists *in the card*
(`security-redblue.yaml:109-113` — plane-telemetry emit + finalize + ledger rows), so the connection
is authored. The gap is **dispatch**: trigger is `/security-redblue` (manual) only, on no cadence.

**Evidence.**
- No `security-redblue-*.json` in `loops/runs/` → the loop has never completed a run.
- Its blue-arm scouts have never executed (see GAP 2) → the "continuous self-security posture" in the
  intent (`security-redblue.yaml:11`) is aspirational.

**Wiring (SAFE — blue arm is `scope_class: A` read-only).** Fold the blue-arm SAFE subset
(`scout-feeds` + `asset-surface-scan` + `verify:{rls,secrets,privacy}` replay) into the daily
plane-maintainer SENSE/SCOUT step, or a weekly cloud routine. The RED arm stays human-gated/Kali-only
(unchanged). Overlaps GAP 2's wiring — do them together.

---

## GAP 5 — Guardrail PROPOSALS pile up with no enactment path (MED)

**What's disconnected.** `docs/reflections/INBOX/2026-07-02-cross-pattern-guardrail-proposals.md`
holds three concrete red→green candidate checks (P-A `gate-release-hygiene`, P-B
`remote-ref-integrity`, P-C reify-lesson) and self-declares (line 3, line 104): *"PROPOSALS ONLY …
Enact via librarian promotion (red→green + ledger row) or Council retro only."* No trigger routes
them; they inherit GAP 4's dead outflow.

**Evidence.** The doc has sat in INBOX since authoring; P-A and P-B are directly implementable
(P-A generalizes the ledger #47/#48 point-fixes into an umbrella assertion; P-B addresses the
"GOTCHA that bit twice" remote-ref class). Nothing consumes them.

**Wiring (needs-council to enact).** Draining is GAP 4's soft check. Enacting P-A/P-B means **adding
checks to `plane-guard.mjs`** — a governance-surface change → librarian promotion (red→green + ledger
row) or Council retro. Highest-value concrete action: implement P-A and P-B (they are the missing
generalizations of already-enacted point-fixes). Tag: **needs-council** for enactment.

---

## GAP 6 — Hot loops bypass `finalize`; §5 telemetry is unrepresentative (MED)

**What's disconnected.** `registry.md:9` mandates *"Жодної петлі поза харнесом … ЗАВЖДИ емітить §5 …
через finalize"* — the two most-used loops violate it.

**Evidence.**
- `loops/runs/metrics.jsonl` = **6 rows**, from only **2 loop types** (autoupgrade ×5 all zero-cost
  degenerate; test-hardening ×1) out of **20 registered loops** (`registry.md`).
- `demo-builder` (~14+ per-run files visible in `loops/runs/`, ~60 per memory) and
  `acquisition-bulk-provision` (7 files) write **bespoke** per-run JSON directly, bypassing `finalize`
  → never land in `metrics.jsonl`.
- `finalize`/metrics-emit lives in `tools/loop-harness/src/{collect,storage,cli,governor}.ts` — the
  `.mjs` runners never call it.

**Consequence.** No cost/eco/iteration telemetry for the loops actually running → the health-pass and
pattern-critic have no data (measurement-is-precondition-for-ratchet, #12, fails for loops).

**Wiring (SAFE).** Have `demo-builder.mjs` + `acquisition-bulk-provision.mjs` append a `metrics.jsonl`
row at run end (call the collector). Deterministic check: extend `loops-registry-sync` style assertion
— each CERTIFIED loop with a recent run-file also has a metrics row. Non-protected script edits.

---

## GAP 7 — plane-guard IS in CI (closed) but its live/health sub-checks never fire (LOW)

**What's disconnected — mostly RESOLVED.** The meta-loop-audit P0 finding "`verify:all --ci` wired
into NO workflow" is **fixed**: `ci.yml:41` runs `pnpm verify:all --ci`, and plane-guard is in
`verify-all.ts:48`. So plane-guard now gates CI. ✅

**Residual.**
- `plane-guard.mjs:93` P8 live-staging-drift is SOFT + **skipped** without a DB url (none in CI);
  `telemetry-liveness:142` is SOFT; `--staging` is never run automatically.
- `agent-health-pass.mjs` (counsel Duty B) is wired into **no** automation — grep across
  `.github/workflows/`, `verify-all.ts`, `.claude/hooks/` = empty. It runs only inside the cloud
  routine (prose). Its output (`docs/governance/`) has no freshness check.

**Wiring (SAFE).** Make the cloud routine run `agent-health-pass` as an artifact-producing step (it
already writes `docs/governance/`) + a SOFT plane-guard `health-pass-freshness` check.

---

## Prioritized wire-the-gaps plan

**Tier 0 — one edit, lights up four gaps (do first).**
A unified **`*-liveness` soft-check family in `plane-guard.mjs`**, cloned from the proven
`telemetry-liveness` (H3) shape — `prediction-resolution-liveness` (GAP 1), `inbox-drain-liveness`
(GAP 4), `scout-liveness` (GAP 2), `health-pass-freshness` (GAP 7). All SOFT, all advisory, **zero new
authority**, and plane-guard is *already in CI* so the actuator exists. This is the highest-leverage
move: it reuses one in-place mechanism to make every silent death VISIBLE — which is the precondition
(#12) for the cloud routine to actually resolve / drain / scout. SAFE-to-wire.

**Tier 1 — reconnect the producers (SAFE).**
- GAP 2/3: add `scout-feeds` + `asset-surface-scan` to the charter SCOUT step and fold the blue-arm
  SAFE subset into the daily routine.
- GAP 6: `demo-builder.mjs` + `acquisition-bulk-provision.mjs` → call `finalize` → `metrics.jsonl`.
- GAP 1: charter SENSE step is fine as prose *once* the soft check makes skips visible — no code.

**Tier 2 — enact the banked proposals (needs-council).**
- GAP 5: implement P-A `gate-release-hygiene` + P-B `remote-ref-integrity` via librarian promotion
  (red→green + ledger row) or Council retro. They are the missing generalizations of ledger #47/#48.

**Do NOT touch without a gate:** any HARD gate on the prediction ledger or INBOX (violates
"mirror never a stick" / advisory-forever); `.claude/**`, `.github/**` (protect-paths — propose, don't
apply); adding plane-guard checks to CI-gating status = governance surface → council.

---

## The five missing connections (one line each)

1. `predict` → **✗** `resolve` → reflection → doubt  (calibration's fact-half never fires — GAP 1)
2. Stop-hook writes reflection → **✗** → librarian drains INBOX  (intake armed, outflow is prose — GAP 4)
3. `scout-feeds`/`asset-surface-scan` built → **✗** → charter SCOUT / any cadence  (never run — GAP 2)
4. loop run (demo-builder/acquisition) → **✗** `finalize` → `metrics.jsonl`  (6 rows / 20 loops — GAP 6)
5. guardrail-proposals doc → **✗** → librarian/council enactment  (P-A/P-B banked — GAP 5)

**Single highest-leverage wiring:** the **Tier-0 `*-liveness` soft-check family in `plane-guard.mjs`**.
It is one small SAFE edit that reuses the already-in-CI plane-guard runner and the proven
`telemetry-liveness` pattern to surface four silent deaths at once (unresolved predictions,
un-drained INBOX, dead scouts, stale health-pass) — turning invisible rot into a visible digest line,
which is the precondition every other reconnection depends on.
