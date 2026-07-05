# Resolution — `tooling-integration-eval`

> RESOLVE round answering the Breaker findings (`breaker-findings.md`) and Counsel advisories
> (`counsel-opinion.md`) against `proposal.md` + `docs/adr/ADR-tooling-integration-eval.md`.
> Counsel raised **0 ETHICAL-STOP**. Every finding below was re-verified against the cited source
> before disposition (file:line shown). Disposition ∈ {**FIX**, **ACCEPT-RISK**, **DEFER-FLAG (MISSING)**}.
> Proposal §§ and gates G1–G5 were rewritten to be genuinely enforceable; the ADR mirrors the result.

Verification basis (read this round, HEAD of `feat/mvp-sensor-seams`):
`.husky/pre-commit:4-9` · `eslint.config.js:14-59` · `package.json:18-31` (`lint`,`verify:all`) ·
`scripts/compliance-gate.ts:18-89` · `Dockerfile:12,35-39` ·
`tools/eslint-plugin-local/src/index.js:439-475` · `apps/api/src/lib/ai-ocr-parser.ts:426,542,544` ·
`apps/api/src/lib/pii-redactor.ts:34-42`.

---

## Summary table

| # | Sev | Finding (one line) | Disposition | Owner |
|---|-----|--------------------|-------------|-------|
| C1 | CRIT | "$0 static floor" never blocks a merge (warn-lint, staged-only) | **FIX** | CI maintainer |
| H1 | HIGH | reachability guard can't see the runtime `fs` sink / cross-pkg / rename | **FIX** | CI maintainer + Parser owner |
| H2 | HIGH | G5 license check is vaporware; check B is a closed allowlist | **FIX** | CI maintainer |
| H3 | HIGH | grow-from-real-misses contradicts synthetic-only (Goodhart + PII) | **FIX** | Parser owner (corpus-edge owner) |
| H4 | HIGH | Skyvern targets the PII-dense pages the parser drops; env-only locks | **FIX (controls) + phase-last** | Operator |
| M1 | MED | DeepEval egress = fail-open denylist; no local-run block | **FIX** | CI maintainer |
| M2 | MED | `COPY apps` puts the corpus in the build image; no guard | **FIX** | CI maintainer |
| M3 | MED | DEFER/REJECT items have no forbidden-dep fence; sidecar no expiry | **FIX** | Architect |
| L1 | LOW | redactor digit patterns can mangle price fixtures → flaky assert | **FIX** | Parser owner |
| L2 | LOW | advisory-vs-blocking has no defined mechanism | **FIX** | CI maintainer |

| Counsel advisory | Disposition |
|------------------|-------------|
| (a) provenance README → neutral taxonomy (OWASP LLM01) primary, branded repo footnote | **ADDRESS** |
| (b) phase the rollout Item 1 → 3 → 2 → 4 (don't fan out 4 toolchains) | **ADDRESS** |
| (c) price the ops/attention tax, not just tokens (Python venv = new CVE stream) | **ADDRESS** |
| (care gap) G4 review explicitly about incidental third-party PII | **ADDRESS** (folded into H4) |
| (epistemic) check B doesn't fire on new env → see H2 | **ADDRESS** (= H2) |
| (open question) who owns growing the corpus → see H3 | **ADDRESS** (= H3) |
| (§3.4) public-repo = publishing a curated injection corpus | **ADDRESS** (corpus README line) |

---

## CRITICAL

### C1 — FIX. The static floor must be exit-1 scripts in CI over the whole tree, not warn-lint.

**Grounded:** `.husky/pre-commit:4` is `npx eslint $STAGED` with no `--max-warnings 0` (ESLint exits 0 on
warnings); `package.json:18` `"lint": "eslint ."` likewise exits 0 on warn; every recent local rule lands
at `'warn'` (`eslint.config.js:31-44`). The Breaker is correct: a `no-corpus-in-source` rule modelled on
`no-mock-in-prod` (warn) would merge green, and pre-commit only lints `$STAGED` so an already-committed
reaching file is never re-checked. The §7 "merge blocked" floor does not exist.

**Fix (in proposal §7, §8.1, G1, §9):**
- The reachability floor is **no longer an ESLint rule**. It is a dedicated standalone script
  `scripts/guardrail-corpus-reachability.mjs` that scans the whole tree and `process.exit(1)` on any
  violation (spec in H1). It is wired into **`verify:all`** (`package.json:31`, already the CI aggregate)
  AND added as its own `package.json` script `"guardrail:corpus-reachability"` so it runs in CI
  independent of lint severity and independent of the staged set.
- The license/subprocessor floor extends `scripts/compliance-gate.ts` (already `process.exit(1)` on
  violation, `:132`) and is likewise in `verify:all` + CI.
- Pre-commit additionally calls both guardrails (not via the staged-only eslint path) so a local commit
  that plants a reaching file fails before push.
- An ESLint `no-corpus-in-source` rule MAY still exist as a fast-feedback convenience, but it is
  **registered at `'error'`** and is **not** counted as the floor — the floor is the exit-1 script.
- **Red→green proof:** plant a fixture under `scripts/__tests__/` (or `tools/.../tests/`) that calls the
  guardrail against a temp tree containing a reaching reference → expect exit 1; remove it → expect exit 0.
  Row in `docs/regressions/REGRESSION-LEDGER.md`.

**Why not accept-risk:** the whole safety argument (§4: "primary safety property is a $0 static guard")
collapses to discipline otherwise — and §8.1 explicitly warns that a reaching corpus = "we built our own
injection payload delivery." This is the load-bearing boundary; it must be authority, not advisory.

---

## HIGH

### H1 — FIX. Reachability becomes a content/path scan over the whole monorepo, sentinel-keyed, image-aware.

**Grounded:** the injection delivery is `redactedText` = file bytes read at **runtime**
(`ai-ocr-parser.ts:426` → prompt sink `:542`, call `:544`). ESLint sees only static literals;
`no-mock-in-prod` (`index.js:465`) inspects `VariableDeclarator` **names**, `no-prod-base-in-test`
(`index.js:450`) inspects `Literal` strings — neither resolves a dynamic path
(`readdirSync(resolve(__dirname,'../tests/fixtures', dir))`, `process.env.CORPUS_DIR`,
`['injection','corpus'].join('-')`), a loader in `scripts/`/`apps/worker`/`packages/*`, a test-helper
imported into `src`, or a rename/symlink/copy of the dir. The AST rule the proposal leaned on cannot see
the real sink. Confirmed.

**Fix — `scripts/guardrail-corpus-reachability.mjs` does four things (proposal §8.1, G1):**
1. **Sentinel, not dir-name.** Every fixture file MUST embed a fixed, rename-proof marker token
   (e.g. `DOWIZ-INJECTION-CORPUS-SENTINEL-DO-NOT-SHIP`). The guard greps **all non-test source across the
   whole monorepo** (`apps/**`, `packages/**`, `scripts/**` minus `**/*.test.*`,`**/*.spec.*`,
   `**/tests/**`,`**/__fixtures__/**`,`dist`,`node_modules`) for the sentinel **and** for the dir-name
   substring (`injection-corpus`). A rename/symlink/copy of the dir still carries the sentinel inside the
   files, so a loader that reads the renamed dir and concatenates bytes will surface the sentinel — and a
   reference to the path surfaces the name. Either ⇒ exit 1.
2. **Build-output scan (closes M2).** Assert the sentinel/dir does **not** appear in any built package
   output (`**/dist/**`) and is excluded by `.dockerignore` (see M2). The corpus dir must live under
   `apps/api/tests/fixtures/` (a `tests/` path, never inside a built `src/` or package root).
3. **Cross-scope coverage.** Because the scan is the whole tree (not `apps/api/src/**`), a loader in
   `scripts/`/`apps/worker`/`packages/*` is in scope; a test-helper that does the `readFile` is excluded
   only if it lives under a `tests/`/`__fixtures__/` path — and if such a helper is imported into a `src`
   file, the import edge is flagged by a second pass (any `import` whose resolved target path is under the
   corpus dir or a known test-helper that reads it).
4. **Behavioural test is the load-bearing proof, in CI on the parser path.** `apps/api/tests/.../
   injection-corpus.test.ts` feeds each fixture as OCR input and asserts: output parses as menu JSON, the
   per-fixture sentinel is **absent** from the output, prices match grounded tokens. This is the proof the
   parser ignores injected directives — it runs in the existing `test:unit` set and on parser-path PRs.

**Residual (accepted, named):** a sufficiently obfuscated loader that base64-decodes bytes at runtime
could defeat a static content scan. This is the same residual ADR-0011 already accepts (R1: no hard
backstop); the behavioural test + human draft review remain the floor. Owner: Parser owner.

### H2 — FIX. Write the actual license logic + replace the closed env allowlist with a pattern detector.

**Grounded:** `compliance-gate.ts` checks A–D are PII-columns / env-subprocessors / raw-PII-on-sink / DPIA
(`:50-121`). There is **no** license logic and `walk()` collects only `.ts/.tsx` (`:36`). Check B
(`:80-89`) iterates a hardcoded `SERVICE_ENV` array — a new `SKYVERN_BASE_URL`/`CONFIDENT_AI_API_KEY`/
`DEEPEVAL_API_KEY` is not in the array, so the loop never examines it. Both the Breaker H2 and the Counsel
epistemic flag are correct: "AGPL guarded" and "check B already covers it" are false as written.

**Fix (proposal §8.2/§8.3, G5):**
- **Real license check (new arm E in compliance-gate or a sibling `scripts/guardrail-license.mjs`):**
  parse `pnpm-lock.yaml` (the resolved production dependency closure) for each package's `license` field
  (fall back to reading `node_modules/<pkg>/package.json` `license` when the lockfile omits it). **Exit 1**
  on any `AGPL-*`/`GPL-*` (and `UNLICENSED`/missing-license) package **in the production dependency
  closure** of any `apps/*`/`packages/*` workspace. **Allow** AGPL **only** for an explicitly fenced
  sidecar that is *not* a dowiz dependency (Skyvern lives in its own container/repo, never in
  `pnpm-lock.yaml`'s closure) — the fence is "absent from the lockfile + absent from any `import`".
  Plus the existing intent: fail on any source `import` of a forbidden module (Skyvern, and the M3 list).
- **Env pattern detector (replaces the closed allowlist):** scan `packages/config/src/index.ts` for any
  identifier matching `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/` that names an **external** service
  (heuristic: read from `process.env` and used to reach a host/credential), and require each to be present
  in `compliance/subprocessors.md`. The hardcoded `SERVICE_ENV` array stays only as a known-good
  acceptlist; anything matching the pattern and absent from both the acceptlist's documented entry and
  `subprocessors.md` ⇒ exit 1. This makes a new `SKYVERN_BASE_URL`/`DEEPEVAL_*` fire automatically.
- **Red→green proof:** (i) add a fake `"agpl-thing": {license: "AGPL-3.0"}` entry to a fixture lockfile →
  expect exit 1; (ii) add `FOO_API_KEY` to a fixture config with no subprocessor entry → expect exit 1;
  remove both → exit 0. Row in the ledger.

### H3 — FIX. Define the corpus-growth ritual (PII-scrub → human review → synthetic paraphrase); name the owner; add a fixture-content PII gate.

**Grounded:** proposal §6 says "New real-world misses are *added* to the corpus (grow-only)" while §8.3
says "**only synthetic fixtures** … No customer PII … **by construction**." A real-world miss is a real
scraped page, which `menu-region.ts`/`pii-redactor.ts` exist precisely because it carries incidental
third-party PII. DeepEval would then egress that PII to the judge LLM. The two clauses contradict; this
also answers the Counsel open question ("who owns growing the corpus").

**Fix (proposal §6, §8.3, R3/new R8):**
- **Ritual (explicit):** a real near-miss never enters the corpus as raw bytes. The path is:
  `prod near-miss / human-review catch` → **PII-scrub** (run `pii-redactor` + drop any `menu-region`
  NON_MENU section) → **human review** confirms no residual third-party PII → author a **synthetic
  paraphrase** that reproduces the *attack shape*, not the original text → commit the paraphrase as the new
  fixture. The raw scraped page is **never** committed. This preserves "synthetic by construction" while
  satisfying the grow-from-misses feedback edge (kills the Goodhart museum).
- **Named owner of the growth edge:** **Parser owner** holds the pen; the trigger is "any parser incident
  or human-review injection catch → a paraphrased fixture before the incident ticket is closed."
- **Mechanical backstop (new check):** a fixture-content gate (folded into
  `scripts/guardrail-corpus-reachability.mjs` or a sibling) fails CI if **any** fixture file matches the
  PII patterns from `pii-redactor.ts` (email/phone/card/iban/url-with-query). A fixture that trips the
  redactor patterns ⇒ exit 1 — "synthetic by construction" becomes enforced, not asserted. (See L1 for the
  digit-pattern caveat.)
- **Red→green proof:** plant a fixture containing a phone-shaped string → expect exit 1; scrub → exit 0.

### H4 — FIX (enforceable controls) + phase-last. Constrain Skyvern's targets, mandate local-LLM with attestation, jail it, human-review the sample.

**Grounded:** §2.3 aims Skyvern at "the **failing tail** — URLs the current pipeline cannot read"; §8.3
calls it "the same class of data the pipeline already reads." `menu-region.ts:19-20` (`NON_MENU_HEADING`)
**drops** About/Team/Chef/Owner/Contact/Reviews and `scanResidualPii` fails closed at PII density ≥1 — the
failing tail (JS-heavy contact/about pages) is exactly the PII-dense class the parser refuses. Skyvern has
none of these controls and ships full screenshots+content to a vision LLM, with `SKYVERN_LLM`/
`SKYVERN_TELEMETRY` as flippable env defaults with no mechanical lock. Confirmed.

**Fix (proposal §7, §8.3, §9, G4, R4; Counsel care gap folded in):**
- **Phase-last.** Skyvern is the **last** item (Counsel (b) phasing) and stays a one-shot pilot, never a
  wired fallback (unchanged).
- **Target restriction:** the pilot input list is **menu/order pages only** — URLs are pre-classified;
  any URL whose path/section resolves to a NON_MENU class (the `menu-region` drop-list) is excluded from
  the pilot input before Skyvern runs. The pilot does **not** point Skyvern at contact/about/team pages.
- **Local-LLM-only, enforceable attestation (not docs):** the pilot runner asserts `SKYVERN_LLM` resolves
  to a local endpoint (loopback / private host) and refuses to run if it points at an Anthropic/OpenAI/
  cloud host; the attestation (the resolved LLM host + telemetry-off + no-credential) is **emitted as the
  G4 artifact and machine-checked** in the pilot script, not merely written in a README.
- **No dowiz credential:** Skyvern's container env is asserted (by the runner) to contain only its own DB
  string and target URLs — no `DATABASE_URL*`, no RLS-bearing secret. (License/dep guard from H2 already
  forbids any Skyvern `import` into the tree.)
- **Network jail:** Skyvern runs egress-allowlisted to {its target URLs, the local LLM} only (allowlist,
  not denylist — see M1).
- **Human review of a sample (Counsel care gap):** G4 requires human review of a sample of what Skyvern
  *sent* and *received*, explicitly checking for **incidental third-party PII**, before any number is
  trusted. Recorded in the G4 artifact.
- **Sidecar expiry (see M3):** the pilot has a named expiry date + owner; after expiry the container must
  be `docker stop`'d and the measurement frozen.

**Residual (accepted, named):** a one-shot pilot on menu-only public URLs with a local LLM, jailed and
human-reviewed, may still encounter an incidental third-party name on a menu page. This is the same
residual the production parser accepts via redact-by-default; here it is bounded to a one-shot,
human-reviewed sample and never reaches a cloud model. Owner: **Operator**; kill-switch: `docker stop` +
expiry date.

---

## MEDIUM

### M1 — FIX. Egress becomes an allowlist + a local-run fail-closed, not a 4-host denylist.

**Grounded:** §8.3/G2 describe a denylist of PostHog/Sentry/NewRelic/Confident-AI. A denylist is
fail-open: it cannot prove an arbitrary `OTEL_EXPORTER_OTLP_ENDPOINT` (host/IP) is unreachable, and §7's
"no `OTEL_EXPORTER_*` endpoint is reachable" overclaims. Local `deepeval test` runs have no policy at all.

**Fix (proposal §7, §8.3, G2):**
- The CI DeepEval job runs with an **egress allowlist** (network policy / firewall) permitting only the
  Anthropic judge endpoint — every other host (any OTEL endpoint, any analytics host, a transitively-added
  sink) is unreachable by default. Proof flips from "probe a named blocked host → fails" to "probe an
  arbitrary non-allowlisted host → fails."
- **Local-run fail-closed:** the eval entrypoint refuses to run unless (a) `DEEPEVAL_TELEMETRY_OPT_OUT=1`
  AND (b) the fixture-content PII gate (H3) passes AND (c) it is either inside the CI allowlist or invoked
  with an explicit `--synthetic-only` confirmation. A dev running it locally on a non-synthetic file is
  blocked, not silently phoning home.
- **Red→green proof:** CI job log showing an arbitrary-host probe failing; entrypoint refusing without the
  opt-out env.

**Residual (accepted, named):** allowlist correctness depends on the CI network policy being applied; if
CI lacks network-policy support the job runs `--synthetic-only` fail-closed (no non-synthetic data, judge
calls only). Owner: CI maintainer.

### M2 — FIX. `.dockerignore` excludes the corpus; the reachability guard asserts the image is sentinel-free.

**Grounded:** `Dockerfile:12` is `COPY apps ./apps` → the entire `apps/api/tests/fixtures/injection-corpus/`
enters the builder; the runtime stage copies only `/app/dist` (`:35`) + public (`:38-39`) and `tsc`
doesn't emit `.txt`, so the corpus stays out of the runtime **today by toolchain accident**, watched by
nothing.

**Fix (proposal §8.1/M2 note, G1):**
- Add `apps/api/tests/` (or at least `apps/api/tests/fixtures/injection-corpus/`) to **`.dockerignore`**
  so the builder context never receives the corpus.
- The reachability guard (H1, item 2) asserts the sentinel is absent from `**/dist/**` and that no
  `COPY`/asset-copy step in `Dockerfile` references a `tests/` path — a grep over `Dockerfile` for
  `tests` in a `COPY` directive ⇒ exit 1.
- **Red→green proof:** build context listing (or `docker build` + `grep` for the sentinel in the image)
  shows the sentinel absent; planting a `COPY apps/api/tests` line trips the guard.

### M3 — FIX. A forbidden-dependency fence + a sidecar expiry/monitor.

**Grounded:** the ADR DEFERs LangGraph-JS/WorkOS and REJECTs Recall/MoneyPrinter/Glossopetrae/Reverse as
**prose**; R7 claims the no-import/no-credential guards make a Skyvern-fallback fail CI, but per H2 those
guards don't exist. The §9 kill-switch table names owners but no expiry — a "one-shot" sidecar never
stopped becomes the always-on AGPL+egress surface the ADR rejects (Option C).

**Fix (proposal §8.2, §9, G5, R3/R7):**
- The license/dep guard (H2) carries an explicit **forbidden-dependency list**:
  `langgraph`, `@langchain/langgraph`, `@workos-inc/*`, `skyvern`, `recall*`, `moneyprinter*`,
  `glossopetrae*`, and any `AGPL-*`/`GPL-*` license. Any of these appearing in `pnpm-lock.yaml`'s closure
  or as a source `import` ⇒ exit 1. This makes every DEFER/REJECT a mechanical fence, not reviewer memory.
- The Skyvern pilot gets a **named expiry date + owner** in §9; a lightweight check (or a calendar-owned
  operator task) confirms the container is down after expiry. Promotion remains a separate ADR.
- **Red→green proof:** plant `langgraph` in a fixture lockfile → exit 1; remove → exit 0.

---

## LOW

### L1 — FIX. Keep injection/price fixtures clear of the redactor's digit patterns; sentinel is non-numeric.

**Grounded:** `pii-redactor.ts:34-42` — `card` matches 13–19 digit runs, `phone` matches 8+ digit runs;
the parser grounds against `redactedText` (`:426`), so a fixture with a long digit sequence beside a price
can be partially `[REDACTED]`, shifting grounded price tokens and flapping the "exact integer-minor-unit"
assertion.

**Fix (proposal §6, §8.1):**
- The **sentinel token is purely alphabetic** (no digit run) so it never trips the redactor.
- Injection fixtures and price-grounding fixtures are **separate sets**: injection fixtures carry the
  attack directive + the sentinel and a *small* well-spaced price (≤4 digits, no adjacent long numeric
  string); the exact-price-match assertion runs only against the dedicated price fixtures. The H3
  fixture-content PII gate uses the same redactor patterns, so a fixture that *would* be mangled is
  rejected at authoring time — the gate and the determinism property reinforce each other.
- **Proof:** the behavioural test (H1 item 4) is green on the injection set; the price-match assertion is
  green on the price set; no fixture trips the H3 PII gate.

### L2 — FIX. Define the infra-vs-quality discriminator mechanically.

**Grounded:** §7/§9 promise "advisory (judge 5xx/timeout) vs blocking (threshold breach)" with no
described mechanism; DeepEval surfaces both as metric failures.

**Fix (proposal §7, §9, G2):**
- The eval wraps each judge call. A **transport/provider error** (non-200, timeout, rate-limit — i.e. no
  metric could be computed) → the run emits status `NEEDS-RERUN` and **exits 0 advisory** with a visible
  marker (does not block the merge; surfaces in the single-line verdict). A **computed metric below
  threshold** (the judge answered, the score is real) → **exit 1 blocking**. The discriminator is "did we
  get a scored response at all?" — transport failure ≠ quality failure.
- To stop a provider outage from silently masking a real regression, `NEEDS-RERUN` is sticky: a parser-path
  PR that only ever produced `NEEDS-RERUN` (never a green scored run) is flagged for a human re-run, not
  auto-passed.
- **Proof:** a forced 503 from the judge endpoint → exit 0 + `NEEDS-RERUN`; a fixture authored below
  threshold → exit 1.

---

## Counsel advisories

- **(a) provenance — ADDRESS.** The corpus `README` primary-references the **neutral** taxonomy (OWASP LLM
  Top-10 **LLM01: Prompt Injection**, plus academic injection categories); the persona-branded repo
  (elder-plinius/L1B3RT4S) appears only as a secondary footnote, if at all. Proposal §4/R5 and ADR
  Decision §1 updated. Removes the "why is your security test seeded from *that*" reputational snag for a
  GDPR-audited EU company.
- **(b) phase the rollout — ADDRESS.** Adopt the sequencing **Item 1 (corpus + exit-1 guards — zero new
  runtime, load-bearing) → Item 3 (oh-my-mermaid — one-shot, sub-dollar) → Item 2 (DeepEval — first Python
  venv; learn egress-jail discipline isolated) → Item 4 (Skyvern — AGPL + sidecar, heaviest)**. This is
  Counsel's "B-in-spirit" and is added to proposal §4 (Decision) and the ADR. Do **not** fan out four
  toolchains at once.
- **(c) ops/attention tax — ADDRESS.** Add a **maintenance-surface** row to §2.5: which toolchain, who owns
  CVE/update cadence. A Python venv is a new language runtime / CVE stream in a TS shop (the DeepEval OTel
  #2497 defect is the live exhibit); priced as a standing tax, which re-confirms the phasing.
- **(care gap) — ADDRESS.** G4 review is explicitly about **incidental third-party PII** in what Skyvern
  sent/received (folded into H4).
- **(§3.4) public-repo — ADDRESS.** The corpus README records that, if dowiz is/ becomes public, this is a
  *published curated injection corpus* — a decision, not an accident (one line; the taxonomies are already
  public, so marginal).

**ETHICAL-STOPs:** none raised; none introduced. Every fix here *strengthens* the red-lines Counsel
listed as the would-trip lines: synthetic-only is now enforced (H3), the no-corpus-in-source floor is now
authority not advisory (C1/H1), the no-dowiz-credential inverse invariant is now mechanical (H2/H4).

---

## Net effect on the gates

| Gate | Before (claimed) | After (enforceable) |
|------|------------------|---------------------|
| G1 | warn-level ESLint rule + behavioural test | **exit-1 `scripts/guardrail-corpus-reachability.mjs`** (whole-tree content+sentinel scan, dist/image-aware, fixture-PII gate) in `verify:all`+CI+pre-commit; behavioural test load-bearing on parser path; ESLint rule optional at `'error'`, not the floor |
| G2 | telemetry-off + host **denylist** probe | telemetry-off + egress **allowlist** (arbitrary-host probe fails) + local-run fail-closed + infra-vs-quality discriminator (L2) |
| G3 | unchanged | unchanged (one-shot `npx`, no `omm login`) — sequenced **2nd** |
| G4 | recovery number + no-credential attestation | + menu-only targets, machine-checked local-LLM attestation, network allowlist, **human PII review of a sample**, expiry — sequenced **last** |
| G5 | "check B already covers it" (false) | **real license check** (pnpm-lock closure, fail on AGPL/GPL/unlicensed) + **env pattern detector** (`*_URL/*_KEY/*_TOKEN` → subprocessors.md) + **forbidden-dep list** (M3); all `process.exit(1)`, red→green |

All five gates remain "not done until red→green proven + rowed in `docs/regressions/REGRESSION-LEDGER.md`."
No item ships to the production runtime. Phasing: **1 → 3 → 2 → 4.**

---

## RESOLVE round 2 (convergence — 2026-06-29)

> Answering the Breaker **RE-ATTACK round** (`breaker-findings.md` § "RE-ATTACK round", RA-1…RA-8) and the
> Counsel **RE-EXAMINE** liveness advisory (`counsel-opinion.md` § "RE-EXAMINE round (3)"). Every RA finding
> was re-verified against live source at HEAD of `feat/mvp-sensor-seams` before disposition. The round-1
> verdict stands for C1/M3/L1/L2/H1-residual (Breaker himself marks them closed). The three HIGH-origin
> round-1 fixes did **not close** — they *relocated* — their holes; round 2 closes them. Disposition ∈
> {**FIX**, **ACCEPT-RISK (owner)**, **DEFER-FLAG**}.

### Verification basis (read this round)
`scripts/compliance-gate.ts:80-89` (closed `SERVICE_ENV` enum) · `packages/config/src/index.ts` (30 env
identifiers match `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/`; 12 in `SERVICE_ENV`) · `/package.json:3`,
`apps/api/package.json:4` (`private:true`, no `license`) · `scripts/build-apps.ts:95-104` (`fs.cpSync`
asset pipeline) · `scripts/verify-all.ts:13-28` (STEPS has no build step) · `Dockerfile:12,27,35-39`
(`COPY apps ./apps`; runs `build-apps.ts`) · `.dockerignore:7` (bare `tests`) · `pii-redactor.ts:19`
(role-trigger-anchored name regex).

### Summary table

| # | Sev | Finding (one line) | Disposition | Owner |
|---|-----|--------------------|-------------|-------|
| RA-1 | HIGH | env pattern detector storms on ~18 internal envs → re-grown allowlist = relocated H2 | **FIX** (classification manifest, fail-closed) | CI maintainer + reviewer |
| RA-2 | HIGH | license check all-red on clean tree (own `private` pkgs, LGPL≠GPL, dual-SPDX) | **FIX** (third-party-only + SPDX parse) | CI maintainer |
| RA-3 | MED | "image-aware" greps wrong file; dist-scan vacuous (no build step) | **FIX** (collapsed into relocation) | CI maintainer |
| RA-4 | MED | `.dockerignore` bare `tests` ≠ nested `apps/api/tests/` | **FIX** (collapsed into relocation) | CI maintainer |
| RA-5 | MED | fixture-PII gate misses bare names (role-anchored regex) | **ACCEPT-RISK** + reframe (ritual is the floor) | Parser owner |
| RA-6 | MED | DeepEval local fail-closed is wrapper-only; raw CLI bypasses | **ACCEPT-RISK** + reframe (CI is authority) | CI maintainer |
| RA-7 | MED | Skyvern attestation checks host *string* not egress; menu-only unenforceable on SPAs | **ACCEPT-RISK** + reframe (network jail is the floor) | Operator |
| RA-8 | LOW | sentinel self-collision in `scripts/**` scope → self-exclusion = bypass surface | **FIX** (collapsed into relocation; deterministic self-skip) | CI maintainer |
| Counsel | — | corpus-growth ritual needs a LIVENESS heartbeat (recurring trigger) | **ADDRESS** (R11, ride retro/librarian cadence) | Parser owner |

---

### RA-1 — FIX. Replace the false-positive pattern detector with an explicit **classification manifest**, fail-closed.

**Grounded (confirmed):** 30 identifiers in `packages/config/src/index.ts` match
`/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/`; only 12 are in `SERVICE_ENV` (`compliance-gate.ts:80-84`). The
round-1 "flag any matching identifier absent from `subprocessors.md`" storms on ~18 **internal** secrets
(`JWT_PRIVATE_KEY`, `BACKUP_ENCRYPTION_KEY`, `COURIER_PII_ENCRYPTION_KEY`, `DEV_AUTH_SECRET`,
`APP_BASE_URL`, `VAPID_PRIVATE_KEY`, `R2_SECRET_ACCESS_KEY`, `DATABASE_URL_OPERATIONAL`, `JWT_DEV_*` …) —
none of which are subprocessors, so the only ways to green are widening an allowlist or loosening the regex.
The Breaker is right: the fix as written rebuilds the closed allowlist H2 was filed to kill. There is **no
pattern-level signal** separating `JWT_PRIVATE_KEY` (internal) from `SKYVERN_BASE_URL` (external) — both are
`process.env` reads. Do **not** try to auto-distinguish.

**Fix — a COMPLEMENT / classification gate (`scripts/guardrail-license.mjs` or compliance-gate arm F):**
- **One explicit manifest:** `compliance/env-classification.md` — a machine-parsed markdown table that
  classifies **every** `packages/config` env whose identifier matches the pattern as exactly one of
  `internal` | `external-subprocessor`, with a subprocessor name for the external ones:
  ```
  | Env identifier            | Class                 | Subprocessor | Note                         |
  |---------------------------|-----------------------|--------------|------------------------------|
  | JWT_PRIVATE_KEY           | internal              | —            | RS256 signing key            |
  | BACKUP_ENCRYPTION_KEY     | internal              | —            | our backup at rest           |
  | RESEND_API_KEY            | external-subprocessor | Resend       | transactional email          |
  | SKYVERN_BASE_URL          | external-subprocessor | Skyvern      | failing-tail scrape sidecar  |
  ```
- **Gate logic (all `process.exit(1)`):**
  1. Extract every `^\s*([A-Z0-9_]+_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT))\s*:` Zod key from
     `packages/config/src/index.ts`.
  2. **Fail-closed on the unclassified case:** any matched identifier with **no** row in
     `env-classification.md` ⇒ exit 1. A brand-new `SKYVERN_BASE_URL` fails until a human classifies it.
  3. Every `external-subprocessor` row's `Subprocessor` value **must** appear in `compliance/subprocessors.md`
     ⇒ else exit 1. Classifying a new env external **forces** a subprocessor entry.
  4. (advisory) a manifest row whose env no longer exists in config → stale warning (non-blocking).
- **Seed the manifest ONCE** with the existing 30 envs (a single reviewed classification commit — internal
  vs external decided by a human, recorded). Thereafter the manifest only grows by deliberate review.
- **Red→green proof:** (i) add `FOO_BAR_URL` to a fixture config, no manifest row → exit 1; (ii) classify it
  `external-subprocessor: Foo` but `Foo` absent from `subprocessors.md` → exit 1; (iii) add `Foo` to
  `subprocessors.md` → exit 0; (iv) classify `FOO_BAR_URL` `internal` → exit 0. Row in the ledger.

**Why this is not "the H2 bypass moved 6 lines down":** the round-1 detector failed *open* on misclass
(silent allowlist line among 30 in a code array). This gate fails *closed* on the unclassified case — the
default for a NEW env is **block**, not pass. To slip `SKYVERN_BASE_URL` through, a contributor must write the
explicit, semantically-loaded claim "`SKYVERN_BASE_URL` is **internal**" into a dedicated compliance file — a
reviewable lie a reviewer challenges, not an invisible array element. **Honest residual:** a malicious
*misclassification* (external env declared internal) still passes. Accepted: the manifest diff is a
red-line-adjacent compliance artifact reviewed by a human; converting a silent false-negative into an
explicit reviewable assertion is the achievable mechanical bound. Owner: CI maintainer + reviewer.

### RA-2 — FIX. Scan THIRD-PARTY deps only; exempt first-party `private` workspaces; parse SPDX properly.

**Grounded (confirmed):** every workspace `package.json` is `"private": true` with no `license`
(`/package.json:3`, `apps/api/package.json:4`). A round-1 "fail on missing-license in the production
closure" is all-red on the clean tree because the closure *includes the workspace packages themselves*;
`GPL-*` substring-matches the permissive `LGPL-*`; dual `(MIT OR GPL-2.0)` is mishandled.

**Fix — define the scanned set and SPDX handling exactly (`scripts/guardrail-license.mjs`):**
- **Scanned set = THIRD-PARTY production deps only.** Resolve the production closure from `pnpm-lock.yaml`,
  then **exclude first-party workspace packages** — identified by the `workspace:`/`link:` protocol in the
  lockfile and by matching the `pnpm-workspace.yaml` package globs. Our own `private:true` packages are
  **never** scanned (they ship as our code, not a third-party license obligation). Read each remaining
  package's license from `node_modules/<pkg>/package.json` `license`/`licenses` (authoritative post-install;
  the lockfile often omits it).
- **Proper SPDX expression evaluation** (use `spdx-expression-parse` + `spdx-satisfies`, both MIT and tiny,
  or an explicit evaluator with identical semantics):
  - `OR` → OK if **any** disjunct is permissive: `(MIT OR GPL-2.0)` resolves to MIT → **pass**.
  - `AND` → **all** conjuncts must be acceptable.
  - `WITH` → honour the exception: `GPL-2.0 WITH Classpath-exception-2.0` evaluated via the exception.
  - **`LGPL-*` ≠ `GPL-*`** — SPDX IDs are distinct; LGPL (dynamic-link permissive) is **allowed**, never
    substring-matched into the GPL denylist.
- **Denylist (SPDX IDs, not substrings):** `AGPL-1.0*`, `AGPL-3.0*`, `GPL-2.0*`, `GPL-3.0*` (incl.
  `-only`/`-or-later`) in any third-party closure ⇒ exit 1. AGPL is allowed **only** for the fenced Skyvern
  sidecar that is absent from the lockfile closure *and* never `import`ed.
- **Missing license on a THIRD-PARTY dep ⇒ manual-review queue:** exit 1 listing the package, clearable only
  by an explicit `compliance/license-exceptions.md` row (`pkg@version` + reason, reviewed). The carve-out is
  explicit/versioned/third-party-scoped — it can never hide our own first-party code (which is exempt by
  construction, not by carve-out).
- **Red→green proof:** plant a fake `AGPL-3.0` third-party in a fixture closure → exit 1; an `LGPL-3.0` dep →
  exit 0; a `(MIT OR GPL-2.0)` dep → exit 0; a first-party `private` workspace pkg with no license → exit 0
  (exempt); a third-party with no license absent from exceptions → exit 1. Row in the ledger.

### RA-3 / RA-4 / RA-8 — FIX (collapsed). RELOCATE the corpus outside any built/shipped/COPY'd path → non-reachability is STRUCTURAL.

**Grounded (confirmed):** the real asset pipeline is `build-apps.ts:95-104` (`fs.cpSync(apiPublic,…)` /
`fs.cpSync(webDist,…)`) run at `Dockerfile:27` — a `Dockerfile`-text grep sees none of it (RA-3);
`verify-all.ts:13-28` STEPS has no build step so a `**/dist/**` scan is vacuous in `verify:all` (RA-3);
`.dockerignore:7` bare `tests` does **not** exclude nested `apps/api/tests/`, and `Dockerfile:12 COPY apps
./apps` pulls the corpus into the builder (RA-4); the scan scope `scripts/**` includes the guard script
itself, whose sentinel literal self-collides (RA-8). Round-1's "image-aware grep" inspected neither the
image nor `build-apps.ts`.

**Fix — relocate, making the corpus question moot rather than mechanically checked:**
- **Canonical location moves from `apps/api/tests/fixtures/injection-corpus/` to the repo-root test-only dir
  `tests/injection-corpus/`** — which is **not** under `apps/`, `packages/`, or `scripts/` (the *only* source
  trees `Dockerfile` `COPY`s, lines 11-13), and is already excluded by `.dockerignore:7` (bare `tests`
  correctly matches a **root-level** `tests/`). `build-apps.ts`'s only `cpSync` sources are
  `apps/api/public` and `apps/web/dist` — neither is an ancestor of `tests/injection-corpus/`. **The corpus
  is therefore structurally absent from the build context — no `COPY`, no `cpSync`, no `.dockerignore` edge
  case reaches it.** RA-3 (image/bundler inspection) and RA-4 (nested-ignore) are **moot**: there is nothing
  to inspect because the corpus is not in any build input.
- **The guard's structural assertion (cheap, no built dist needed):** the reachability guard asserts the
  corpus canonical path is the repo-root test-only dir, and **exit 1** if the corpus dir is ever found under
  any `COPY`/`cpSync`-reachable path (`apps/**`, `packages/**`, `scripts/**`, or either `build-apps.ts`
  copy-source). This inspects the *actual* asset-pipeline source paths (the RA-3 gap), not Dockerfile text.
- **dist/sentinel scan demoted to defense-in-depth.** The `**/dist/**` sentinel scan and whole-tree content
  scan remain as a *secondary* corroboration (catch a future regression that re-imports the dir), explicitly
  **not** the load-bearing control — the structural relocation is. This retires RA-3's "vacuous green"
  concern: a vacuous secondary check is acceptable when the primary control is structural.
- **RA-8 self-collision — single deterministic self-skip, no allowlist.** The scanner skips exactly two
  paths, both constants, neither growable: (a) its own resolved `__filename`
  (`fileURLToPath(import.meta.url)`), and (b) the single canonical corpus dir constant. Every *other*
  sentinel occurrence in scanned source ⇒ exit 1. No open/extensible self-exclusion list exists, so it cannot
  become a parking spot for a reaching loader.
- **The behavioural/parser test still reaches the corpus from the test context** via an explicit repo-root
  resolve — `path.resolve(repoRoot, 'tests/injection-corpus')` (a small test helper resolves repo-root from
  `import.meta.url`). Tests run from the repo root in dev/CI, never from the shipped image, so test
  reachability and runtime non-reachability are cleanly separated.
- **Red→green proof:** move the corpus dir under `apps/api/src/` (or add it to a `cpSync` source) → guard
  exit 1; canonical repo-root location → exit 0; the parser behavioural test loads it green from the test
  context. Row in the ledger.

### RA-5 — ACCEPT-RISK (owner) + reframe. The authoring ritual is the load-bearing PII floor; the regex gate is honestly defense-in-depth.

**Grounded (confirmed):** `pii-redactor.ts:19` matches a TitleCase name **only** behind a role trigger
(`Chef|Owner|Manager|…|By|Meet|Served by|Prepared by`). A bare third-party name with no trigger ("ask for
Maria", "Hoxha family recipe", a reviewer byline "— Arben") is **not** matched. Names are inherently hard to
regex; the Breaker is right that round-1 over-marketed the gate as "enforced, not asserted."

**Disposition — accept the residual, correct the claim:**
- **Primary (load-bearing) control = the synthetic-paraphrase authoring ritual** (H3/R8): fixtures are
  **authored synthetic paraphrases**, never raw scraped bytes, and a **mandatory human review** confirms no
  residual third-party PII *before commit*. A bare name surviving requires the author to *invent* a real
  person's name into a synthetic string and a reviewer to miss it — low but nonzero.
- **The fixture-content regex gate is demoted to defense-in-depth corroboration**, and the proposal/gate text
  is corrected to say so honestly: the gate catches **structured** PII (emails, digit-runs ≥8 → card/phone,
  query-URLs, IBANs, role-triggered names); it does **not** reliably catch **bare personal names**. "Synthetic
  by construction" is enforced *for structured PII* by the gate and *for bare names* by the ritual + human
  review (the honest floor, asserted-by-human).
- **Accept-risk:** residual bare-name leakage to the Anthropic judge on a synthetic fixture. **Owner: Parser
  owner** (holds the authoring ritual + review). Bounded: synthetic-only-on-disk means the worst case is an
  *invented* name in a paraphrase, not a real customer record. This is the same class as ADR-0011 R1 (no hard
  backstop; human review is the floor).

### RA-6 — ACCEPT-RISK (owner) + reframe. CI is the enforceable authority; local raw-CLI is best-effort, bounded by synthetic-only-on-disk.

**Grounded (confirmed by design):** the venv exposes the raw `deepeval` binary; a dev running
`deepeval test run …` directly skips the dowiz wrapper's opt-out / PII-gate / `--synthetic-only`. Locally
there is no network policy (the allowlist is a CI-only construct). Round-1's claim that *local* runs are
fail-closed is an overclaim.

**Disposition — relocate enforcement to where it is real, correct the overclaim:**
- **CI is the authority.** The egress allowlist is a CI network-layer construct the raw CLI **cannot escape**
  regardless of invocation; CI is the only sanctioned path that touches non-synthetic data (and it doesn't —
  see H3). The wrapper is documented as **local best-effort convenience, not the security boundary.**
- **Local is bounded by synthetic-only-on-disk.** The fixture-PII gate + authoring ritual guarantee no real
  customer PII is ever committed to the fixture dirs, so a raw local `deepeval` run only exposes **synthetic**
  data to the judge. Worst case locally = synthetic fixture + telemetry on → leaks synthetic data + the fact
  an eval ran; **not** a PII red-line crossing.
- **Correct the proposal** (§7 row 2, §8.3): replace "local runs are fail-closed" with "CI is the authority;
  local raw-CLI is best-effort and bounded by synthetic-only-on-disk." **Owner: CI maintainer.** Honest
  residual accepted.

### RA-7 — ACCEPT-RISK (owner) + reframe. The network egress allowlist is the floor; host-string attestation and menu-only are demoted to best-effort.

**Grounded (confirmed by design):** the runner's `SKYVERN_LLM` "local/loopback host" check is a **string**
check — a `localhost:8000` reverse-proxy/tunnel to Anthropic vision satisfies "loopback" and is then also
permitted by the `{targets, local LLM}` allowlist. Menu-only pre-classification is impossible on JS-heavy
SPAs (no fetched text to classify); the failing tail is exactly that class.

**Disposition — name the real control, demote the theatre, accept the residual:**
- **Load-bearing control = a NETWORK-LEVEL egress allowlist** (container/network policy) permitting only
  {explicit target URLs, the local LLM endpoint} and dropping everything else at the network layer. A
  reverse-proxy to Anthropic is blocked because Anthropic is **not** in the allowlist — the network policy
  doesn't trust the host *label*, it drops the *destination*.
- **The host-STRING attestation is demoted to a cheap secondary sanity check**, honestly labeled "not proof
  of egress." **menu-only target restriction is demoted to a best-effort URL-path heuristic** (`/contact`,
  `/about`), explicitly acknowledged as unenforceable on single-page sites.
- **Remaining floor = human review of a sample** of bytes sent/received for incidental third-party PII +
  **named owner + expiry + kill-switch.** Skyvern is an **out-of-band one-shot pilot (phase 4 / last)**,
  never wired as a fallback. **Honest residual:** an operator who deliberately runs a malicious proxy *at the
  allowlisted local-LLM endpoint* forwarding to a cloud model defeats the network jail; bounded by one-shot
  scope, the sampled human read of full pages, expiry, and named accountability. **Owner: Operator**;
  kill-switch: `docker stop` + named expiry date.

### Counsel RE-EXAMINE — ADDRESS. Add a corpus-liveness heartbeat (R11) riding the existing retro/librarian cadence.

**Advisory:** a single named owner is the right *accountability* answer but the wrong *liveness* answer — a
quiet/vacant Parser-owner role lets the corpus silently freeze (the "museum" reforms with no alarm), and the
reactive event-trigger never imports newly-published OWASP LLM01 classes.

**Address (new R11; non-blocking, rides existing machinery — no new ceremony):**
- **Recurring trigger complements the event trigger.** Fold a "corpus freshness" line into the existing
  **Council retro / `librarian` curation** cadence (fires on stage-close per the self-improvement loop) and
  the periodic **agent-health** pass, asking two questions each cycle: *(i) has every parser incident since
  last review produced its fixture?* (closes the reactive-edge leak), and *(ii) is the corpus still
  representative vs the current OWASP LLM01 taxonomy?* (closes the proactive-refresh gap).
- **Optional mechanical nudge (advisory, non-blocking):** a `tests/injection-corpus/LAST-REVIEWED` date that
  a guard **warns** on (not exit 1) past N days — a liveness signal, not a gate.
- The Parser owner still holds the pen; the recurring check confirms the pen is being used. **Owner: Parser
  owner** (pen) + retro/librarian cadence (pulse).
- **Acknowledged (not new mechanisms):** Counsel's two minor edges — (a) require each *new* paraphrase to be
  shown to make a deliberately-vulnerable parser fail (a red baseline) before joining the green set, proving
  it still carries the injection (already folded into the H1 behavioural-test discipline); (b) note the
  PII-gate-evasion residual in the corpus README (one honest line, same class as RA-5).

---

### Net effect — round 2 (HARD-EXIT)

| Finding | Sev | Round-1 state | Round-2 disposition |
|---|---|---|---|
| RA-1 | HIGH | relocated H2 bypass (false-positive storm → re-grown allowlist) | **FIXED** — fail-closed classification manifest; misclass residual accepted (reviewable) |
| RA-2 | HIGH | gate unshippable (all-red on clean tree) | **FIXED** — third-party-only + SPDX parse + explicit exceptions file |
| RA-3 | MED | greps wrong file; vacuous dist-scan | **FIXED** — structural relocation makes it moot; guard inspects real cpSync sources |
| RA-4 | MED | nested `tests` not ignored | **FIXED** — corpus moved out of any COPY path |
| RA-5 | MED | bare-name hole, over-marketed gate | **ACCEPT-RISK** (Parser owner) — ritual is floor, gate honestly DiD |
| RA-6 | MED | wrapper-only local enforcement | **ACCEPT-RISK** (CI maintainer) — CI is authority, local bounded by synthetic-only |
| RA-7 | MED | host-string ≠ egress; menu-only unenforceable | **ACCEPT-RISK** (Operator) — network jail is floor, rest best-effort |
| RA-8 | LOW | self-collision → exclusion bypass | **FIXED** — deterministic two-constant self-skip, no allowlist |
| Counsel liveness | — | reactive-only, no heartbeat | **ADDRESSED** — R11, retro/librarian cadence + advisory LAST-REVIEWED |

**Result: 0 unresolved CRITICAL, 0 unresolved HIGH.** Both HIGH (RA-1, RA-2) are genuinely closed (fail-closed
manifest; third-party-scoped SPDX gate). The five MEDIUM/LOW are either FIXED (RA-3/4/8 via structural
relocation) or ACCEPT-RISK with a named owner and an honest residual (RA-5/6/7). The corpus is now
**structurally non-reachable** (repo-root `tests/injection-corpus/`, outside every `COPY`/`cpSync` path), not
merely scan-checked. **HARD-EXIT reached.**
