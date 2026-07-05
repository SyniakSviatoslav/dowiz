# Design Proposal — Tooling Integration Evaluation

- Slug: `tooling-integration-eval`
- Status: **Proposed — RESOLVE round 2 applied 2026-06-29 (HARD-EXIT: 0 unresolved CRITICAL/HIGH)**
  (design-time only — no production code). Breaker round-1 (C1/H1/H2/H3/H4/M1/M2/M3/L1/L2) and **RE-ATTACK
  round (RA-1…RA-8)** dispositioned; Counsel advisories + RE-EXAMINE liveness addressed; see
  `docs/design/tooling-integration-eval/resolution.md` (§ "RESOLVE round 2"). 0 ETHICAL-STOP. The injection
  corpus is now **structurally non-reachable** (repo-root `tests/injection-corpus/`, outside every Docker
  `COPY` / `build-apps.ts` `cpSync` path), not merely scan-checked.
- Author: System Architect (DeliveryOS)
- Date: 2026-06-29
- ADR: `docs/adr/ADR-tooling-integration-eval.md`
- Relates: ADR-0011 (`docs/adr/0011-menu-parse-eval-and-grounding.md`), ADR-0012
  (`docs/adr/0012-agent-token-economy-dev-tooling.md`), `scripts/compliance-gate.ts`,
  `tools/eslint-plugin-local/src/index.js`, `apps/api/src/lib/ai-ocr-parser.ts`.

> This proposal turns a pre-triaged research shortlist into an attackable design with explicit,
> deterministic gates. It does **not** re-survey the tools — the ADOPT/PILOT/DEFER/REJECT triage is
> taken as input. The architect's job here is: name the integration topology, prove the safety
> boundary, attach a red→green guardrail to each adopted item, and size the cost.

---

## 1. Problem + non-goals

### Problem
The AI menu-parser (`apps/api/src/lib/ai-ocr-parser.ts`, `menu-region.ts`, `pii-redactor.ts`,
`menu-grounding.ts`) ingests **untrusted** scraped/OCR content and concatenates it into a Claude prompt
(`ai-ocr-parser.ts:508-542`, sink at `:544`). ADR-0011 already established the prompt-injection threat
("OCR text is UNTRUSTED data, not instructions") and was honest that **there is no hard automated
backstop** — human review of the draft is the floor. Three gaps remain:

1. **No adversarial corpus** to continuously prove the parser ignores injected directives.
2. **No quality eval** beyond the field-level B1 scorer — no faithfulness/hallucination grading.
3. **The failing tail of scrape ingestion** (URLs the Playwright + brand-extractor pipeline cannot read)
   has no measured recovery path.

A research pass produced a candidate tool list. The risk is that adopting them naively imports
**license contamination** (AGPL into the dowiz tree), **PII egress** (eval/scrape tools phoning home or
to a vendor cloud), and **new attack surface on a 🔴 red-line** (auth/PII). This proposal integrates only
the safe subset, each behind a boundary that is mechanically enforced.

### Non-goals
- Re-evaluating or re-surveying the tool candidates (triage is input).
- Shipping any of these to the **production runtime** of `apps/api`. Everything here is CI-only,
  local-only, or an out-of-band sidecar. Nothing enters the request path.
- Replacing human review of menu-parse drafts (ADR-0011 floor stands).
- Building a multi-step / resumable agent runtime (LangGraph-JS is **DEFER**).
- Any auth change (WorkOS / auth.md is **DEFER** — explicitly do not host an `auth.md`).
- Persisting any tool output to a tenant DB table. No schema, no migration (see §5).

---

## 2. Back-of-envelope

All figures assume **Claude Sonnet-class judge** pricing of **$3 / Mtok input, $15 / Mtok output**
(the existing Anthropic key) unless a local/cheap LLM is named. Token counts are deliberately
conservative (high side). These size the *gate's* recurring cost, not a product feature.

### 2.1 Injection-resistance corpus (Item 1)
- Fixtures: **30** synthetic, paraphrased injection strings (5 categories × ~6 each).
- The assertion needs a **real parser call per fixture** (prove the directive is ignored, prices
  unchanged). Per parser call ≈ 8k tok in + 2k tok out = `8000·$3/1e6 + 2000·$15/1e6` = **$0.024 + $0.030
  = $0.054**.
- Full run: `30 × $0.054 ≈ **$1.6**`. Sampled smoke (5 fixtures/PR) ≈ **$0.27**.
- **Topology decision (cost-driven):** the *reachability* half of the gate (corpus dir can never be
  globbed into a prompt path) is a **static lint/AST check costing $0** and runs on every PR. The
  *behavioural* half (LLM ignores the directive) runs **nightly / on parser-path changes only**, not on
  every PR. Rationale: the static guard is the load-bearing safety property; the behavioural test is
  corroboration and need not pay $1.6 per push.

### 2.2 DeepEval CI eval (Item 2)
- Fixed fixture corpus: **30** menu-parse fixtures (B1 today has 15; this grows it).
- Metrics per fixture: **3** LLM-judged (faithfulness, hallucination, G-Eval-correctness). Per judge call
  ≈ 3k in + 0.5k out = `3000·$3/1e6 + 500·$15/1e6` = **$0.0165**.
- Judge cost: `30 fixtures × 3 metrics × $0.0165 ≈ **$1.49**`.
- If the eval **re-runs the parser** per fixture (not just scores a cached output): `+ 30 × $0.054 ≈
  $1.6` → **full run ≈ $3.1**.
- Cadence: parser-path changes + nightly. Assume **~25 qualifying runs/month** → **~$40–80/month**
  worst case. Acceptable; bounded by a per-run fixture cap. (Compare: a single careless prod incident on
  the parser costs far more.)

### 2.3 Skyvern failing-tail pilot (Item 4 — out-of-band)
- Failing tail: **30–50** restaurant URLs the current pipeline cannot read.
- Vision agent steps per URL: ~5 (navigate, screenshot, locate menu, extract, paginate). Per step
  ≈ 2k tok + 1 screenshot (~1.5k image tok). Per URL ≈ `5 × 3.5k = 17.5k` in + 3k out.
  - On Anthropic vision: `17500·$3/1e6 + 3000·$15/1e6 = $0.0525 + $0.045 = **$0.098/URL**` →
    **50 URLs ≈ $4.9 one-time pilot**.
  - On the **named local/cheap LLM** (the spec's requirement): compute cost ≈ **$0** (operator GPU/time
    only). This is the chosen pilot config; the Anthropic figure is the ceiling if a local model is
    unavailable.
- This is a **one-shot measurement**, not a recurring cost. Output = a recovery-rate-vs-cost number that
  decides whether Skyvern ever becomes a fallback (it does **not** in this change).

### 2.4 oh-my-mermaid pilot (Item 3)
- Subtree: `apps/api/src/routes` ≈ 40–60 files.
- If the tool is LLM-backed: one summarization pass ≈ 30–50k tok in, ~5k out =
  `50000·$3/1e6 + 5000·$15/1e6 ≈ **$0.23**` one-time. If AST-deterministic: **$0**.
- Run via pinned `npx`, **once**, compared against the repowise overview. One-time, sub-dollar.

### 2.5 Roll-up
| Item | When | Per-run | Recurring/month |
|---|---|---|---|
| Injection static guard | every PR | $0 | $0 |
| Injection behavioural | nightly / parser PRs | ~$1.6 | ~$25 |
| DeepEval | nightly / parser PRs | ~$3.1 | ~$40–80 |
| Skyvern pilot | one-shot | ~$0 (local LLM) / $4.9 ceiling | $0 |
| oh-my-mermaid | one-shot | <$0.25 | $0 |

**Conclusion:** the only meaningful recurring spend is the eval/injection nightly (≤ ~$105/mo ceiling),
and it is capped by a fixture-count limit and a parser-path trigger. No item touches per-request runtime
cost. Back-of-envelope clears comfortably.

### 2.6 Maintenance surface (the ops/attention tax — Counsel (c))
> Token cost is not the real cost. "Cheap to add" ≠ "cheap to own." This table prices *attention*, and it
> re-confirms the phasing in §4.

| Item | New toolchain / runtime | Standing CVE/update surface | Owner of updates |
|---|---|---|---|
| 1 — corpus + exit-1 guards | **none** (Node scripts in-repo) | low — repo-native, no new runtime | CI maintainer / Parser owner |
| 3 — oh-my-mermaid | one-shot pinned `npx`, no install | **none** (one-shot, nothing persists) | pilot runner |
| 2 — DeepEval | **a Python venv in a TS shop** | **a whole new language CVE stream** + DeepEval bumps (OTel #2497 is the live exhibit) | CI maintainer |
| 4 — Skyvern | Docker sidecar, AGPL, vision-LLM | heaviest — container + AGPL + egress surface, time-boxed by expiry | Operator |

The tax rises monotonically down the list → adopt in that order (§4).

---

## 3. Options (≥2, each a named concept)

### Option A — "Quarantine-and-CI-gate everything"
Adopt all three ADOPT items + run the Skyvern pilot, each isolated in its own boundary (quarantined test
dir, isolated Python venv in CI, pinned npx, separate sidecar), each with a deterministic guardrail in
`tools/eslint-plugin-local` and/or `scripts/compliance-gate.ts`.
- **+** Maximal coverage: adversarial corpus + quality eval + living docs, all proven safe before "done".
- **+** Each boundary is mechanically enforced, so the safety property does not rely on discipline.
- **−** Most moving parts now: a Python venv in CI, a sidecar to stand up, new guard rules to maintain.
- **Concept:** *defence-in-depth via isolation + a guard per boundary.*

### Option B — "Minimal: synthetic corpus only, defer the rest"
Ship only Item 1 (the synthetic injection corpus + reachability guard). Park DeepEval, oh-my-mermaid,
and Skyvern until the corpus proves value.
- **+** Smallest surface; the highest-value safety artifact (adversarial corpus on the 🔴 PII/injection
  red-line) ships first and alone.
- **+** Zero new language runtime (no Python venv), zero sidecar, near-zero recurring cost.
- **−** Leaves the quality-eval gap (hallucination/faithfulness) and the scrape failing-tail unmeasured.
- **−** Defers the cheapest wins (oh-my-mermaid is one-shot, sub-dollar) for no real risk reduction.
- **Concept:** *YAGNI / minimum viable — one artifact, the load-bearing one.*

### Option C — "Sidecar-service mesh"
Run DeepEval and Skyvern as long-lived self-hosted services that the parser pipeline can call over HTTP
as live fallbacks/quality-gates.
- **+** Reusable beyond CI; Skyvern could become a real production scrape fallback.
- **−** Directly violates the spec's "do NOT wire Skyvern as a production fallback until justified" and
  pulls AGPL into the live request path's dependency graph (reach risk). Adds always-on attack surface to
  a pipeline that handles untrusted content. **Rejected on principle** — failure-first says do not stand
  up an always-on service for an unproven need.
- **Concept:** *premature productionization — the anti-pattern this canon forbids.*

### Decision: **Option A, scoped by the cadence trims from §2** — i.e. defence-in-depth via isolation,
but with the *static* guards on every PR and the *paid* behavioural/eval jobs on a parser-path/nightly
trigger. We take A over B because the extra ADOPT items (DeepEval, oh-my-mermaid) are cheap and isolated,
and the eval gap is real on a red-line surface; we take A over C because C productionizes unproven tools
into the request path. Skyvern stays a **one-shot out-of-band measurement** inside A, never a wired
fallback (that boundary is the same one C would have crossed).

---

## 4. Decision + rationale (ADR-format; mirrored to `docs/adr/ADR-tooling-integration-eval.md`)

Adopt the three ADOPT items and run the one PILOT, each behind an explicit deterministic gate:

> **Phasing (RESOLVE / Counsel (b)) — adopt in this order, do NOT fan out four toolchains at once:**
> **Item 1 (corpus + exit-1 guards — zero new runtime, load-bearing safety) → Item 3 (oh-my-mermaid —
> one-shot, sub-dollar) → Item 2 (DeepEval — first Python venv; learn the egress-jail discipline isolated)
> → Item 4 (Skyvern — AGPL + sidecar, the heaviest).** Each later item inherits the boundary discipline
> proven by the earlier one; the §2.6 ops-tax table confirms this risk×ownership ordering.

1. **Synthetic injection corpus** under `apps/api/tests/fixtures/injection-corpus/` (inert `.txt`),
   categories seeded by a **neutral taxonomy — OWASP LLM Top-10 LLM01 (Prompt Injection) + academic
   injection categories as the primary reference** (a persona-branded jailbreak repo, if cited at all, is a
   secondary footnote only — Counsel (a) reputational de-risk), **authored in-house, zero verbatim
   vendoring**. Gate = the **exit-1 reachability guard** (corpus dir / sentinel can never reach prompt
   assembly, dist, or the Docker image — G1) **plus** a behavioural test (parser returns only menu JSON,
   ignores directives, sentinel absent, prices grounded).
2. **DeepEval** as a **CI-only, isolated-venv Python job** scoring the menu-parser, telemetry hard-off,
   all known egress sinks blocked, OTel-hijack defect neutralized before any non-synthetic input.
3. **oh-my-mermaid** as a **one-shot local `npx` pilot** (pinned version, no login/cloud) producing
   `.mmd` docs for `apps/api/src/routes`, compared to the repowise overview.
4. **Skyvern** as a **one-shot out-of-band sidecar measurement** of failing-tail scrape recovery;
   AGPL kept as a separate HTTP service, telemetry off, local LLM; **not** wired as a fallback.

**Rationale:** every adopted item lives outside the production runtime; the only thing that touches the
🔴 red-line surface (the parser's untrusted-input path) is the *defensive* corpus, whose primary safety
property is a $0 static guard, not an LLM behaviour. License risk is contained by the
"no AGPL in the dowiz tree / no verbatim vendoring" boundary, which is itself made mechanical (§8). This
honours "boring & proven > novelty" and "schema rich, runtime minimal" — we add seams (test dirs, CI
jobs, a sidecar) without turning on any runtime path.

---

## 5. Data / migrations

**N/A for every item — stated explicitly per the mandate:**

- **Item 1 (injection corpus):** test-only inert `.txt` files at the **repo-root test-only dir
  `tests/injection-corpus/`** (RESOLVE-2 RA-3/4/8 — relocated out of `apps/api/tests/` so it is outside
  every Docker `COPY`/`build-apps.ts` `cpSync` path → structurally absent from the build context). **No
  schema, no migration, no DB table.**
- **Item 2 (DeepEval):** CI job reading fixed fixtures, writing a CI artifact (score report). **No DB,
  no migration.** Fixtures and `expected/*.json` are version-controlled, not stored in Postgres.
- **Item 3 (oh-my-mermaid):** generates `.mmd` files in `docs/`. **No DB, no migration.**
- **Item 4 (Skyvern):** the sidecar has its **own** ephemeral storage (its own Postgres/SQLite inside the
  container), **never** the dowiz DB. Pilot output is a CSV/JSON measurement artifact under
  `docs/research/`. **No dowiz schema, no migration, no RLS surface** — and that isolation is the point:
  Skyvern must not be granted a dowiz `DATABASE_URL`.

Because nothing writes to a tenant table, **RLS FORCE / integer-money / forward-only-migration concerns
do not apply** — there is no tenant data path to protect here. The relevant invariant is the inverse:
prove these tools **never gain** a dowiz DB credential (enforced in §8).

---

## 6. Consistency + idempotency

- **Eval/corpus determinism is the consistency property.** LLM judges are non-deterministic, so the gate
  must not flap. Mitigations:
  - **Fixtures are frozen** (committed `.txt` / `.pdf` / `expected/*.json`); a run scores the *same*
    inputs every time.
  - **Corpus-growth ritual (RESOLVE / H3 + Counsel open question) — synthetic by construction, layered
    PII control.** A real-world near-miss is **never** committed as raw scraped bytes (those
    carry incidental third-party PII — the exact reason `menu-region.ts` + `pii-redactor.ts` exist). The
    path is: prod parser-incident / human-review injection catch → **PII-scrub** (`pii-redactor` + drop the
    `menu-region` NON_MENU sections) → **human review** confirms no residual third-party PII → author a
    **synthetic paraphrase** reproducing the *attack shape*, not the original text → commit the paraphrase.
    The raw page never enters the repo. This keeps the feedback edge alive (kills the Goodhart "museum of
    known attacks") while preserving "no customer PII by construction." **Named owner of the growth edge:
    the Parser owner**, triggered on "any parser incident or human-review injection catch → a paraphrased
    fixture before the incident ticket closes." (This resolves the §8.3-vs-§6 contradiction the Breaker
    flagged: §8.3's "synthetic only" wins, and the growth rule is redefined to produce *synthetic
    paraphrases*, never raw misses.)
    - **PII control — honest layering (RESOLVE-2 RA-5).** The **load-bearing** PII floor is the *authoring
      ritual + mandatory human review* above (synthetic paraphrases, never raw bytes, reviewed before
      commit). The **fixture-content regex gate is defense-in-depth, not the proof**: it catches *structured*
      PII (emails, ≥8-digit runs → card/phone, query-URLs, IBANs, role-triggered names) but **cannot**
      reliably catch a **bare personal name** — `pii-redactor.ts:19`'s name pattern is anchored to a role
      trigger, so "ask for Maria", "Hoxha family recipe", a byline "— Arben" pass it. "Synthetic by
      construction" is therefore *enforced by the gate for structured PII* and *held by the ritual + human
      review for bare names* (the honest floor). Residual bare-name leakage to the judge is **accepted**
      (owner: Parser owner) — bounded because synthetic-only-on-disk means any surviving name is an *invented*
      one in a paraphrase, not a real customer record (same class as ADR-0011 R1).
    - **Liveness heartbeat (RESOLVE-2 / Counsel RE-EXAMINE — R11).** The event trigger above is reactive
      only; a quiet/vacant Parser-owner role would let the corpus silently freeze. Complement it with a
      **recurring** check riding existing machinery (no new ceremony): the Council retro / `librarian`
      curation cadence (fires on stage-close) + the periodic agent-health pass ask each cycle — *(i) has every
      parser incident since last review produced its fixture?* and *(ii) is the corpus still representative vs
      the current OWASP LLM01 taxonomy?* Optional advisory nudge: a `tests/injection-corpus/LAST-REVIEWED`
      date a guard **warns** on (not exit 1) past N days. The Parser owner holds the pen; the cadence
      confirms the pen is used.
  - **Thresholds, not exact scores.** The gate asserts `faithfulness ≥ T`, `hallucination ≤ T`, recall ≥
    0.95 (inherit ADR-0011), price = **exact integer-minor-unit match (zero tolerance)** — the one place
    determinism is absolute, because money. A threshold tolerates judge jitter while still catching a
    real regression. **L1 caveat:** because the parser grounds against `redactedText` and the redactor
    eats 13–19 / 8+ digit runs (`pii-redactor.ts:34-42`), injection fixtures and price-grounding fixtures
    are **separate sets** — the exact-price assertion runs only on the dedicated price set (small,
    well-spaced prices, no adjacent long numeric string), and the sentinel token is purely alphabetic so
    it never trips the redactor.
  - **Seeded / pinned judge model.** Pin the judge model id and `temperature=0` where the SDK allows, so
    re-runs are as stable as the provider permits. The behavioural injection test asserts a **structural**
    invariant ("output parses as menu JSON; injected sentinel string absent from output; prices match
    grounded tokens"), which is deterministic regardless of judge jitter.
- **Idempotency:** every job is **re-runnable with no side effects** — it reads fixtures and emits a
  report artifact. No enqueue, no DB write, no external mutation. Re-running yields the same verdict
  (modulo threshold-respecting judge jitter). The Skyvern pilot is a one-shot whose URLs and outputs are
  recorded so the measurement is reproducible.

---

## 7. Failures + degradation

Failure-first. Each external/new call gets a timeout + fallback; **no cascade into the product runtime**
is structurally impossible here because none of these run in the request path. The failure modes that
matter are *containment* failures:

| Failure | Detection | Degradation / response |
|---|---|---|
| **A corpus `.txt` reaches a prompt path** | **Exit-1 reachability guard** `scripts/guardrail-corpus-reachability.mjs` in `verify:all`+CI+pre-commit (whole-tree content+sentinel scan, dist/image-aware) — `process.exit(1)`, NOT a warn-level lint | **Hard CI fail — merge blocked.** Caught *statically* (no LLM) and as authority, not advisory (C1/H1 fix). It cannot ship dark. |
| **DeepEval telemetry can't be disabled / arbitrary egress** | Pre-flight (G2) asserts `DEEPEVAL_TELEMETRY_OPT_OUT=1` AND a network-egress **allowlist** (only the Anthropic judge endpoint reachable) is active — probe an **arbitrary non-allowlisted host → must fail**; the job **exits non-zero before any fixture loads** if any check fails | Job fails closed. An allowlist proves an arbitrary `OTEL_EXPORTER_OTLP_ENDPOINT`/analytics host is unreachable; a 4-host denylist (M1) cannot. **CI is the authority (RESOLVE-2 RA-6):** the egress allowlist is a CI network-layer construct the raw `deepeval` CLI cannot escape. The dowiz wrapper's opt-out + PII-gate + `--synthetic-only` is **local best-effort** (a dev can call the raw binary); local exposure is bounded to **synthetic-only-on-disk** so worst-case a raw local run leaks synthetic data, not customer PII. |
| **DeepEval OTel global-TracerProvider hijack (issue #2497)** | The CI job pins a DeepEval version known-patched or applies the documented neutralization; the egress **allowlist** (above) makes any OTEL endpoint unreachable regardless | If neutralization can't be confirmed → job fails closed; **only synthetic fixtures are ever fed** until confirmed. |
| **DeepEval/judge LLM down or rate-limited** | Non-200 / timeout from Anthropic → **no metric computed** | **Infra-vs-quality discriminator (L2):** transport error → status `NEEDS-RERUN`, **exit 0 advisory** (sticky — a parser-path PR that *only* ever produced `NEEDS-RERUN` is flagged for a human re-run, never auto-passed); a *computed* metric below threshold → **exit 1 blocking**. A provider outage doesn't block all merges, and a real regression can't hide behind one. |
| **Skyvern sidecar leaks page/PII data** | **Load-bearing = a network-level egress allowlist (RESOLVE-2 RA-7)** permitting only {explicit target URLs, local LLM endpoint}, dropping everything else at the network layer (a reverse-proxy to Anthropic is blocked — the destination is not allowlisted); **+ no-`DATABASE_URL` asserted + human review of a sample** for incidental third-party PII (G4/H4). **Best-effort/secondary:** the `SKYVERN_LLM` host-string check (a label, *not* proof of egress) and a menu-only URL-path heuristic (unenforceable on JS-heavy SPAs). | Network-jailed, no dowiz credential, one-shot/out-of-band. The human PII review catches an incidental third-party name before any number is trusted. **Residual (accepted):** an operator running a malicious proxy *at the allowlisted local-LLM endpoint* defeats the jail — bounded by one-shot scope, sampled human read, expiry. Kill-switch = `docker stop`; **named expiry date + owner** (Operator). |
| **oh-my-mermaid attempts cloud push (`omm login`)** | Run with no credentials; the command is invoked as `npx <pinned> generate`, never `login`/`push` | Without creds the cloud path can't authenticate; it fails locally with no egress. |
| **Eval cost runs away** | Per-run fixture cap + parser-path trigger (not every-PR) | Job aborts past the cap; cadence trim (§2) bounds monthly spend. |

The single non-negotiable: **the exit-1 reachability guard (`scripts/guardrail-corpus-reachability.mjs`)
is the floor** — a standalone `process.exit(1)` script in `verify:all`+CI+pre-commit, *not* a warn-level
ESLint rule (C1). Everything else can degrade to "didn't run"; that one must be green or the merge is
blocked.

---

## 8. Security + tenant isolation (the core concern)

Four threats, each made **mechanical** so it doesn't rely on reviewer discipline:

### 8.1 Prompt-injection reachability (corpus → prompt path)
The corpus is adversarial strings. If any of them is ever globbed/read into prompt assembly, we have
*built our own injection payload delivery*. The prompt sink is the template literal at
`ai-ocr-parser.ts:508-542` consuming `redactedText`, called at `:544`.
- **Guard — exit-1 script, NOT an ESLint rule (RESOLVE C1/H1/H4/M2):** the AST-lint approach the first
  draft proposed is **rejected** — ESLint sees only static literals, but the injection delivery is
  `redactedText` = file **bytes read at runtime** (`ai-ocr-parser.ts:426` → sink `:542,:544`); a dynamic
  path (`process.env.CORPUS_DIR`, `['injection','corpus'].join('-')`), a loader in
  `scripts/`/`apps/worker`/`packages/*`, a test-helper imported into `src`, or a rename/symlink/copy all
  evade it (and `no-mock-in-prod` matches *variable names*, not paths). The guard is instead
  `scripts/guardrail-corpus-reachability.mjs` (`process.exit(1)`, in `verify:all`+CI+pre-commit).
  **RESOLVE-2 (RA-3/4/8) — structural non-reachability over scan-checking:** the corpus is **relocated** to
  the repo-root test-only dir **`tests/injection-corpus/`**, which is *not* under `apps/`/`packages/`/
  `scripts/` (the only source trees `Dockerfile:11-13` `COPY`s) and is *not* an `apps/api/public` or
  `apps/web/dist` `cpSync` source (`build-apps.ts:95-104`). It is therefore **structurally absent from the
  build context** — no `COPY`, no `cpSync`, no `.dockerignore` edge case reaches it (RA-4's nested-ignore and
  RA-3's image/bundler-inspection questions are *moot*, not patched). The guard's role is (1) a **structural
  assertion** — exit 1 if the corpus dir is ever found under any `COPY`/`cpSync`-reachable path (inspecting
  the *real* `build-apps.ts` copy-sources, not Dockerfile text — RA-3); (2) a **content/sentinel scan over
  the whole monorepo** keyed on a **rename-proof alphabetic sentinel** (now *defense-in-depth*, secondary to
  the structural property); (3) an import-edge pass for test-helper-laundering. The scan does a single
  **deterministic two-constant self-skip** (its own `fileURLToPath(import.meta.url)` + the one canonical
  corpus dir) — **no growable self-exclusion list** that could become a parking spot (RA-8). Full spec in the
  Appendix (G1).
- **Defence-in-depth:** the corpus dir carries a `README` marking it quarantined (and noting, per Counsel
  §3.4, that if dowiz becomes public this is a *deliberately published curated injection corpus*); the
  behavioural test is the **load-bearing proof** — each fixture fed *as OCR input* leaves prices grounded
  and the output free of the injected sentinel, run in CI on the parser path.

### 8.2 AGPL reach (license contamination)
- **Skyvern (AGPL-3.0)** and any AGPL eval wrapper must **never** be imported into the dowiz tree. Boundary
  = a separate process reached over **HTTP only**; no Python/JS `import`, no shared module, no vendored
  source. Skyvern lives in its own repo/container, not under `/root/dowiz/apps` or `/packages`.
- **Guard — REAL license logic (RESOLVE H2/M3):** the cited `scripts/compliance-gate.ts` has **no** license
  check today (its checks A–D are PII/subprocessor/DPIA; `walk()` collects only `.ts/.tsx`, `:36`) — "AGPL
  guarded" was vaporware. Fix: a license arm (in compliance-gate or sibling `scripts/guardrail-license.mjs`,
  `process.exit(1)`) that parses `pnpm-lock.yaml`'s resolved **production dependency closure** (falling back
  to `node_modules/<pkg>/package.json` `license`) and fails on any `AGPL-*`/`GPL-*`/`UNLICENSED`/
  missing-license package; AGPL is allowed **only** for a fenced sidecar absent from the closure and never
  `import`ed (Skyvern). Plus a **forbidden-dependency list** (`langgraph`,`@langchain/langgraph`,
  `@workos-inc/*`,`skyvern`,`recall*`,`moneyprinter*`,`glossopetrae*`) so every ADR DEFER/REJECT is a
  mechanical fence, not reviewer memory (M3). The dowiz↔Skyvern link appears only as an HTTP base URL env
  var, documented as a subprocessor-equivalent.
- **DeepEval (Apache-2.0)** is license-clean, but it stays in an **isolated Python venv in CI**, never in
  `apps/api` runtime deps — so it can't drift into the shipped image.

### 8.3 PII egress (DeepEval / Skyvern / Confident-AI / vendor clouds)
- **DeepEval (RESOLVE M1/H3; RA-6):** `DEEPEVAL_TELEMETRY_OPT_OUT=1`, Confident-AI cloud login **not**
  performed, and the CI job runs behind an **egress allowlist** (network policy) permitting **only** the
  Anthropic judge endpoint — every other host (any OTEL endpoint, any analytics host, a transitively-added
  sink) is unreachable by default. (A 4-host *denylist* is fail-open and cannot prove an arbitrary endpoint
  unreachable — M1.) **CI is the enforceable authority (RA-6):** the allowlist is a CI network-layer
  construct the raw `deepeval` CLI cannot escape. The dowiz wrapper (opt-out + fixture-PII gate +
  `--synthetic-only`) is **local best-effort, not the boundary** — a dev can invoke the raw venv binary
  directly. Local exposure is bounded because **only synthetic fixtures ever exist on disk** (authoring
  ritual + the structured-PII gate), so a raw local run leaks synthetic data, not customer PII. Judge calls
  go only to the existing Anthropic endpoint (a registered subprocessor).
- **Skyvern (RESOLVE H4 — it targets the PII-dense pages the parser deliberately drops):** the failing tail
  is disproportionately JS-heavy contact/about pages — exactly the `menu-region.ts:19-20` NON_MENU drop-list
  (About/Team/Chef/Owner/Contact/Reviews) where owner names/phones live; the parser *refuses* those, the
  AGPL sidecar has none of that protection. **Load-bearing control (RESOLVE-2 RA-7) = a network-level egress
  allowlist** permitting only {explicit target URLs, the local LLM endpoint} and dropping everything else at
  the network layer — a `localhost` reverse-proxy/tunnel to Anthropic is blocked because the *destination*
  (Anthropic) is not allowlisted, regardless of the host *label*. Plus: container env asserted to hold **no**
  `DATABASE_URL*`/RLS-bearing secret; **human review of a sample** of sent/received content for incidental
  **third-party PII** (G4). **Demoted to best-effort/secondary (honest, RA-7):** the `SKYVERN_LLM`
  host-*string* check (a label, not proof of egress) and the menu-only URL-path heuristic (unenforceable on
  JS-heavy SPAs — the failing tail is exactly the pages you cannot pre-classify). Never wired as a fallback;
  one-shot/out-of-band; named expiry + owner. **Residual (accepted):** a malicious proxy at the allowlisted
  local-LLM endpoint defeats the jail — bounded by one-shot scope + the sampled human read + expiry.
- **Compliance-gate alignment (RESOLVE H2 + RA-1 — a *classification* gate, not a pattern detector):** check
  B (`scripts/compliance-gate.ts:80-89`) iterates a **closed `SERVICE_ENV` enum** — a new `SKYVERN_BASE_URL`/
  `CONFIDENT_AI_API_KEY`/`DEEPEVAL_API_KEY` is not in the array, so the loop never examines it. A naive
  "flag every `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/` identifier absent from `subprocessors.md`" instead
  **storms** — 30 such identifiers exist in `packages/config/src/index.ts`, ~18 of them **internal** secrets
  (`JWT_PRIVATE_KEY`, `BACKUP_ENCRYPTION_KEY`, `COURIER_PII_ENCRYPTION_KEY`, `DEV_AUTH_SECRET`,
  `VAPID_PRIVATE_KEY`…) that will *never* be in `subprocessors.md`; the only green is a re-grown allowlist
  (the relocated H2 bypass — RA-1). There is no pattern-level signal separating internal from external. **Fix
  — an explicit classification manifest `compliance/env-classification.md`** (G5(c)): every matched
  identifier must be classified `internal` | `external-subprocessor` (with a subprocessor name); the gate
  **fails closed** on any *unclassified* matched env (a new `SKYVERN_BASE_URL` blocks until a human
  classifies it), and every `external-subprocessor` entry **must** appear in `compliance/subprocessors.md`.
  Seeded **once** with the existing 30 envs (a reviewed one-time classification). The residual is a
  *misclassification* (declaring an external env "internal") — but that is now an explicit, reviewable,
  semantically-loaded claim in a compliance file, not an invisible array line.

### 8.4 Tenant isolation
Not applicable in the usual sense — none of these touch tenant tables (§5). The inverse invariant holds:
**no tool may obtain a dowiz `DATABASE_URL`/RLS-bearing credential.** Enforced by 8.2's no-AGPL/no-import
guard plus the operational rule that Skyvern's container env contains only its own DB string.

---

## 9. Operability

| Item | Where it runs | Owner / kill-switch |
|---|---|---|
| Injection reachability guard | `scripts/guardrail-corpus-reachability.mjs` — exit-1 in `verify:all`+CI (every PR) + pre-commit | CI maintainer; disabling it is a visible diff to `verify:all`/`package.json`/the script (and would itself fail review). NOT a warn-level lint (C1). |
| Injection behavioural test | CI nightly + parser-path PRs | Test owner; skip = visible (and banned by the no-`.only`/skip integrity rules). |
| DeepEval eval job | **CI only**, isolated venv | CI maintainer; kill-switch = remove the CI workflow step. Never in `apps/api` runtime. |
| oh-my-mermaid | **Local only**, one-shot `npx` | Whoever runs the pilot; nothing persists to CI/runtime. |
| Skyvern pilot | **Out-of-band sidecar**, operator host | Operator; kill-switch = `docker stop`; **named expiry date** — after expiry the container is stopped and the measurement frozen (M3/R9). Network egress-allowlisted (load-bearing), no dowiz credential, human PII-review of a sample (H4); host-string + menu-only are secondary/best-effort (RA-7). Not in CI, not in prod, not wired to the app. |

- **Health: degraded vs down.** The eval job distinguishes *infra-failure* (judge LLM 5xx/timeout →
  advisory, non-blocking) from *quality-failure* (threshold breach → blocking). The static guard is
  binary: green = safe, red = merge blocked.
- **Observability < 1 min.** Each job emits a single-line verdict + a report artifact; the static guard
  surfaces in the standard lint output. No new dashboard needed.
- **Rollback.** Every item is additive and isolated: rolling back = delete the CI step / test dir / stop
  the container. No migration to reverse, no runtime flag to flip in prod (nothing is in prod).
- **Flag / scaling-gate.** None of these is a launchable feature, so no feature flag is required; the
  "gate" is the guardrail itself. Skyvern promotion to a real fallback would be a **separate** ADR with
  its own flag (default-off) — explicitly out of scope here.

---

## 10. Open / accepted risks

| # | Risk | Disposition | Owner |
|---|---|---|---|
| R1 | **No hard automated backstop for prompt-injection** (inherited from ADR-0011). The corpus raises confidence but human review of the draft remains the floor. | **Accept** — explicitly carried forward from ADR-0011; the corpus is corroboration, not a guarantee. | Parser owner |
| R2 | **Judge-LLM non-determinism may flap the eval near a threshold.** | **Mitigated** (threshold not exact-score, pinned model, temp=0, frozen fixtures) + **accept residual** — infra-failure is advisory, not blocking. | CI maintainer |
| R3 | **DeepEval OTel hijack (#2497) could exfiltrate via a transitive update.** | **Mitigated** — pin a known-patched version + an egress **allowlist** (only the Anthropic judge endpoint reachable; any OTEL endpoint unreachable by construction — M1 fix) + synthetic-only-on-disk; **CI is the enforceable authority, local raw-CLI is best-effort** (RA-6). Re-review on any DeepEval bump; job fails closed if neutralization unconfirmed. | CI maintainer |
| R4 | **Skyvern (AGPL) license/reach.** | **Accept under boundary** — separate HTTP sidecar, no import, no vendoring, never wired as fallback. Guarded by the **shippable** license check (third-party-only closure + proper SPDX parse — LGPL≠GPL, dual-OR handled; RA-2) + forbidden-dep list (H2/M3), no longer vaporware nor all-red. Promotion = separate ADR. | Operator |
| R5 | **Taxonomy-seeded corpus could be read as "derived from AGPL/unlicensed source" / reputational snag.** | **Mitigated** — primary reference is the **neutral** taxonomy (OWASP LLM01 + academic categories); persona-branded repo is a footnote at most (Counsel (a)); categories only are seeded, all strings authored in-house, paraphrased, zero verbatim copy; the corpus README records provenance + the public-repo note (Counsel §3.4). | Parser owner |
| R6 | **Eval cost drift** if the corpus grows unbounded or cadence widens. | **Mitigated** — per-run fixture cap + parser-path/nightly trigger; monthly ceiling ≤ ~$105 (§2.5). Revisit if corpus > ~100 fixtures. | CI maintainer |
| R7 | **A future contributor wires Skyvern as a live fallback / pulls a DEFER/REJECT dep back** (Option C drift). | **Mitigated** — the **real** license check + **forbidden-dependency list** (`langgraph`/`@workos-inc`/`skyvern`/`recall`/`moneyprinter`/`glossopetrae`) + no-DB-credential machine-check make this fail CI (H2/M3 fix; was prose-only). Intentional promotion requires a new ADR. | Architect |
| R8 | **Corpus-growth from real misses egresses real third-party PII / decays into a Goodhart museum** (Breaker H3 + Counsel open question). | **Mitigated** — growth ritual = scrub→human-review→**synthetic paraphrase** (raw page never committed); the load-bearing PII floor is the ritual + human review, the fixture-content gate is defense-in-depth for *structured* PII only (RA-5); named owner = Parser owner, triggered per incident. | Parser owner |
| R11 | **Corpus liveness — the event trigger is reactive-only; a quiet/vacant owner lets the corpus silently freeze** (Counsel RE-EXAMINE). | **Mitigated** — recurring freshness check folded into the Council retro / `librarian` cadence + agent-health pass (did every incident produce its fixture? still representative vs OWASP LLM01?) + an advisory `LAST-REVIEWED` warn. | Parser owner |
| R12 | **Env *misclassification* residual (RA-1)** — an external env declared `internal` in `env-classification.md` dodges `subprocessors.md`. | **Accept** — converted from a silent false-negative into an explicit, reviewable, semantically-loaded compliance-file claim; fail-closed on the *unclassified* case is the mechanical bound. | CI maintainer + reviewer |
| R13 | **Skyvern local-LLM proxy residual (RA-7)** — a malicious proxy at the allowlisted local-LLM endpoint forwards full pages to a cloud model. | **Accept** — bounded by one-shot/out-of-band scope, the network egress allowlist (the load-bearing control), the sampled human PII read, and a named expiry; host-string check is honestly secondary. | Operator |
| R14 | **Bare-name leakage to the judge (RA-5)** — `pii-redactor.ts:19` is role-trigger-anchored; a bare personal name in a paraphrase passes the gate. | **Accept** — the authoring ritual + human review is the floor; bounded because synthetic-only-on-disk means any surviving name is *invented*, not a real record (same class as ADR-0011 R1). | Parser owner |
| R15 | **DeepEval raw-CLI local bypass (RA-6)** — the venv binary skips the wrapper's opt-out/PII-gate/`--synthetic-only`. | **Accept** — CI (network allowlist) is the enforceable authority; local is best-effort, bounded to synthetic-only-on-disk so a raw run leaks synthetic data, not customer PII. | CI maintainer |
| R9 | **Skyvern sidecar never `docker stop`'d → persistent always-on AGPL+egress surface** (M3). | **Mitigated** — §9 carries a named expiry date + owner; container stopped + measurement frozen after expiry. | Operator |
| R10 | **Egress allowlist depends on CI network-policy support.** | **Accept** — if CI lacks network-policy support the job runs `--synthetic-only` fail-closed (no non-synthetic data, judge calls only); residual is bounded. | CI maintainer |

---

## Appendix — Explicit gates (the deterministic "done" proof per item)

> A guardrail is the authority; nothing below is "done" until its gate is **red→green** proven and rowed
> in `docs/regressions/REGRESSION-LEDGER.md`.

> **RESOLVE update (2026-06-29):** gates rewritten to be genuinely enforceable after the Breaker round
> (C1/H1/H2/H4/M1/M2/M3/L1/L2) — the floor is now an **exit-1 standalone script over the whole tree**, not
> a warn-level lint, and G5 carries real license logic. See `resolution.md`. Phasing: **1 → 3 → 2 → 4**.

- **G1 — Injection corpus (reachability) — exit-1 script, NOT warn-lint [fixes C1/H1/M2/L1; RESOLVE-2
  RA-3/4/5/8]:** the floor is `scripts/guardrail-corpus-reachability.mjs`, a standalone Node script that
  **`process.exit(1)`** on any violation and is wired into **`verify:all`** + CI + pre-commit via a new
  `"guardrail:corpus-reachability"` `package.json` script — independent of ESLint severity and of the staged
  set. **Structural non-reachability is primary (RA-3/4):** the corpus canonical location is the **repo-root
  test-only dir `tests/injection-corpus/`** — outside every `Dockerfile` `COPY` (`apps`/`packages`/`scripts`,
  lines 11-13) and every `build-apps.ts` `cpSync` source (`apps/api/public`, `apps/web/dist`, `:95-104`), so
  it is structurally absent from the build context (and `.dockerignore:7`'s bare `tests` correctly excludes a
  root-level `tests/`). The script then:
  1. **Structural assertion (RA-3):** exit 1 if the corpus dir is found under any `COPY`/`cpSync`-reachable
     path — it reads the *real* `build-apps.ts` copy-sources + `Dockerfile` `COPY` targets, not just
     Dockerfile text. No built `dist/` need exist for this to have teeth.
  2. **Content/sentinel scan (defense-in-depth, secondary):** scan `apps/**`,`packages/**`,`scripts/**`
     (minus `**/*.test.*`,`**/*.spec.*`,`**/tests/**`,`**/__fixtures__/**`,`dist`,`node_modules`) for the
     **rename-proof alphabetic sentinel** (`DOWIZ-INJECTION-CORPUS-SENTINEL` — no digit run, never trips the
     redactor, L1) and the dir-name substring `injection-corpus`. The `**/dist/**` sentinel scan stays as
     corroboration (now secondary to the structural property — RA-3's "vacuous on empty dist" is acceptable
     for a defense-in-depth check). **Self-skip (RA-8):** the scanner skips exactly two constants — its own
     `fileURLToPath(import.meta.url)` and the one canonical corpus dir — **no growable exclusion list**.
  3. an `import`-edge pass: any `import` resolving under the corpus dir, or a test-helper that reads it being
     imported into a `src` file (closes H1's test-helper-laundering / cross-scope bypass).
  4. a **fixture-content PII gate — defense-in-depth, NOT the proof (RA-5):** exit 1 if any fixture matches
     the `pii-redactor.ts` *structured* patterns (email / ≥8-digit card·phone / iban / url-with-query /
     role-triggered name). It is **honest about its hole**: a **bare** personal name (no role trigger,
     `pii-redactor.ts:19`) is NOT caught — that is held by the authoring ritual + mandatory human review (the
     load-bearing PII floor, §6), with residual bare-name leakage accepted (owner: Parser owner).
  An ESLint `no-corpus-in-source` rule MAY exist as fast feedback but only at **`'error'`**, and it is
  **not** the floor. **Load-bearing proof:** the behavioural test `apps/api/tests/<...>injection-corpus.test.ts`
  (in `test:unit`, on parser-path PRs) resolves the corpus from the test context
  (`path.resolve(repoRoot, 'tests/injection-corpus')`) and asserts each fixture fed as OCR input → output
  parses as menu JSON, the per-fixture sentinel is **absent** from the output, prices match grounded tokens;
  each *new* paraphrase must first red a deliberately-vulnerable parser (proves it still carries the
  injection). Red→green: corpus moved under a `COPY`/`cpSync` path → exit 1; canonical location → exit 0; a
  planted reaching reference / structured-PII fixture → exit 1; rowed in `docs/regressions/REGRESSION-LEDGER.md`.
- **G2 — DeepEval [fixes M1/L2]:** the CI job runs behind an **egress allowlist** (network policy) that
  permits only the Anthropic judge endpoint — proof is "probe an **arbitrary non-allowlisted** host →
  fails" (a denylist of 4 hosts is fail-open and cannot prove an arbitrary `OTEL_EXPORTER_OTLP_ENDPOINT`
  unreachable). A **pre-flight** step asserts `DEEPEVAL_TELEMETRY_OPT_OUT=1`, Confident-AI login absent,
  the allowlist active, and #2497 neutralized; it **exits non-zero before loading any fixture** if any
  fails. **CI is the authority; local is best-effort (RESOLVE-2 RA-6):** the egress allowlist is a CI
  network-layer construct the raw `deepeval` venv binary cannot escape regardless of invocation. The dowiz
  wrapper (telemetry-opt-out + fixture-PII gate + `--synthetic-only`) is documented as **local convenience,
  not the security boundary** — a dev may call the raw CLI; local exposure is bounded to synthetic-only-on-disk
  (no customer PII ever exists in the fixture dirs), so the worst local case leaks synthetic data.
  **Infra-vs-quality discriminator (L2):** a transport/provider error (non-200/timeout — no metric
  computed) → status `NEEDS-RERUN`, **exit 0 advisory** (sticky: a parser-path PR that only ever produced
  `NEEDS-RERUN` is flagged for human re-run, never auto-passed); a computed metric below threshold → **exit
  1 blocking**. The venv must **not** appear in `apps/api` runtime deps (asserted by G5). Proof = a CI log
  showing the arbitrary-host probe failing + a fixture score report.
- **G3 — oh-my-mermaid (sequenced 2nd):** a committed `.mmd` artifact under `docs/` for `apps/api/src/routes`
  + a one-line note comparing coverage vs the repowise overview. Run via pinned `npx`, no `omm login`.
  Proof = the artifact + the command transcript (no network login).
- **G4 — Skyvern pilot (sequenced LAST) [fixes H4 + Counsel care gap; RESOLVE-2 RA-7]:** a measurement
  artifact under `docs/research/` recording recovery-rate and cost-per-URL over the failing URLs, **PLUS
  controls — honestly layered (RA-7):**
  **Load-bearing:** (d) a **network-level egress allowlist** permitting only {explicit target URLs, the
  local LLM endpoint} and dropping everything else at the network layer — a `localhost` reverse-proxy to
  Anthropic is blocked because the *destination* is not allowlisted, not because the host *label* looks
  local; (c) the runner asserts the container env holds **no** `DATABASE_URL*`/RLS-bearing secret; (e)
  **human review of a sample** of what Skyvern *sent and received*, explicitly for incidental **third-party
  PII**, recorded in the artifact; (f) a **named expiry date + owner** — after expiry the container is
  `docker stop`'d and the measurement frozen.
  **Best-effort / secondary (not proof — RA-7):** (b) a `SKYVERN_LLM` host-*string* sanity check (a label,
  honestly *not* proof of egress — the network allowlist in (d) is the actual control); (a) a menu-only
  URL-path heuristic to skip obvious NON_MENU pages, acknowledged **unenforceable on JS-heavy SPAs** (the
  failing tail is precisely the single-page sites you cannot pre-classify) — so the human PII review (e) is
  the real backstop, not the pre-filter. Pilot is **one-shot / out-of-band**, never wired as a fallback.
  **Residual (accepted, owner Operator):** a malicious proxy at the allowlisted local-LLM endpoint defeats
  the jail — bounded by one-shot scope, the sampled human read, and expiry. Proof = recorded numbers + the
  egress-allowlist + no-credential attestation + the human PII review note. Promotion (not done here) = a
  separate ADR.
- **G5 — License/PII boundary (cross-cutting) [fixes H2/M3; RESOLVE-2 RA-1/RA-2] — REAL, shippable logic:**
  `scripts/compliance-gate.ts` (or a sibling `scripts/guardrail-license.mjs`), **`process.exit(1)`** on:
  - **(a) license check — THIRD-PARTY deps only, proper SPDX (RA-2).** Scan the **third-party** production
    closure resolved from `pnpm-lock.yaml`, **excluding first-party workspace packages** (identified by the
    `workspace:`/`link:` protocol + `pnpm-workspace.yaml` globs) — our own `private:true` packages with no
    `license` field are exempt by construction, never carve-out (else the gate is all-red on the clean tree).
    Read each third-party license from `node_modules/<pkg>/package.json`. **Evaluate SPDX properly** (via
    `spdx-expression-parse` + `spdx-satisfies`, both MIT): `OR` passes if any disjunct is permissive
    (`(MIT OR GPL-2.0)` → MIT → pass), `AND` requires all, `WITH` honours the exception, and **`LGPL-*` is
    distinct from and NOT denied as `GPL-*`** (no substring matching). Deny the SPDX IDs `AGPL-1.0*`,
    `AGPL-3.0*`, `GPL-2.0*`, `GPL-3.0*` in any third-party closure ⇒ exit 1; AGPL allowed **only** for the
    fenced Skyvern sidecar (absent from the closure *and* never `import`ed). A **missing license on a
    third-party dep** → exit 1, clearable only by an explicit reviewed `compliance/license-exceptions.md` row
    (`pkg@version` + reason).
  - **(b) forbidden-dependency list (M3)** — `langgraph`,`@langchain/langgraph`,`@workos-inc/*`,`skyvern`,
    `recall*`,`moneyprinter*`,`glossopetrae*` in the closure or as a source `import` ⇒ exit 1 (every ADR
    DEFER/REJECT becomes a mechanical fence, not reviewer memory).
  - **(c) env CLASSIFICATION gate, not a pattern detector (RA-1).** A bare "flag any
    `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/` identifier absent from `subprocessors.md`" **storms** — 30 such
    identifiers exist in `packages/config/src/index.ts`, ~18 internal (`JWT_PRIVATE_KEY`,
    `BACKUP_ENCRYPTION_KEY`, `COURIER_PII_ENCRYPTION_KEY`, `DEV_AUTH_SECRET`, `VAPID_PRIVATE_KEY`,
    `DATABASE_URL_OPERATIONAL`…) that never belong in `subprocessors.md`; the only green is a re-grown
    allowlist (the H2 bypass relocated). Instead, maintain **one explicit manifest
    `compliance/env-classification.md`** classifying every matched identifier as `internal` |
    `external-subprocessor` (+ subprocessor name). The gate **fails closed** on any *unclassified* matched
    env (a new `SKYVERN_BASE_URL` blocks until a human classifies it), AND every `external-subprocessor`
    entry **must** appear in `compliance/subprocessors.md`. Seeded **once** with the existing 30 envs (a
    reviewed one-time classification). The residual is a *misclassification* — but that is an explicit,
    reviewable, semantically-loaded claim in a compliance file, not an invisible array element.
  Proof = red→green on: a planted AGPL third-party lockfile entry → exit 1; an `LGPL-3.0` / `(MIT OR
  GPL-2.0)` dep → exit 0; a first-party `private` pkg with no license → exit 0; a forbidden-dep → exit 1; a
  new matched env with no `env-classification.md` row → exit 1; classified `external-subprocessor` but absent
  from `subprocessors.md` → exit 1.
