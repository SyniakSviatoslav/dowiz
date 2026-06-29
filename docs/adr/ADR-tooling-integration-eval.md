# ADR — Tooling integration: defensive injection corpus, CI eval, docs pilot, scrape sidecar

- Status: **Proposed — RESOLVE round 2 applied 2026-06-29 (HARD-EXIT: 0 unresolved CRITICAL/HIGH)** (design-time; no production code)
- Date: 2026-06-29
- Deciders: DeliveryOS System Architect (Breaker + Counsel rounds complete; 0 ETHICAL-STOP)
- Resolution: `docs/design/tooling-integration-eval/resolution.md`
- Slug: `tooling-integration-eval`
- Proposal: `docs/design/tooling-integration-eval/proposal.md`
- Relates: ADR-0011 (`docs/adr/0011-menu-parse-eval-and-grounding.md`),
  ADR-0012 (`docs/adr/0012-agent-token-economy-dev-tooling.md`),
  `apps/api/src/lib/ai-ocr-parser.ts:508-544`, `scripts/compliance-gate.ts`,
  `tools/eslint-plugin-local/src/index.js`.

## Context

The AI menu-parser ingests untrusted scraped/OCR content and concatenates it into a Claude prompt
(`ai-ocr-parser.ts:508-542`, sink `:544`). ADR-0011 established the prompt-injection threat and was honest
that no hard automated backstop exists (human draft review is the floor). A research pass produced a
pre-triaged tool shortlist. Naive adoption risks **license contamination** (AGPL into the dowiz tree),
**PII egress** (eval/scrape tools phoning home), and **new attack surface on a 🔴 red-line**
(auth/PII/RLS/money/migrations). Only a safe subset is adopted, each behind a mechanical boundary.

Triage taken as input: **ADOPT** synthetic injection corpus, DeepEval (CI-only), oh-my-mermaid (local
pilot). **PILOT** Skyvern (out-of-band sidecar). **DEFER** LangGraph-JS, WorkOS/auth.md. **REJECT**
Recall, MoneyPrinterTurbo, Glossopetrae, Reverse/zhaoxuya520.

## Decision

> **Phasing (RESOLVE / Counsel):** adopt in risk×ownership order — **Item 1 → Item 3 → Item 2 → Item 4** —
> do NOT stand up four heterogeneous toolchains at once (a Python venv is a new CVE stream in a TS shop).

1. **Defensive injection-resistance corpus** — ~20–30 synthetic, paraphrased fixtures authored in-house
   (categories seeded by a **neutral taxonomy — OWASP LLM Top-10 LLM01 primary**, a persona-branded repo a
   footnote at most; **zero verbatim vendoring**), stored as inert `.txt` at the **repo-root test-only dir
   `tests/injection-corpus/`** (RESOLVE-2 RA-3/4/8 — relocated out of `apps/api/tests/` so it is outside
   every Docker `COPY` / `build-apps.ts` `cpSync` path → **structurally non-reachable**, not merely
   scan-checked). **Defensive-only.** Growth from real misses follows the
   scrub→human-review→synthetic-paraphrase ritual (raw page never committed); the ritual + human review is
   the load-bearing PII floor (the fixture regex gate is defense-in-depth, blind to bare names — RA-5); a
   recurring retro/`librarian` freshness check gives it a liveness heartbeat (RESOLVE-2 R11). Owner = Parser
   owner.
2. **DeepEval (Apache-2.0)** — a **CI-only** Python job in an **isolated venv** (never in `apps/api`
   runtime), scoring the parser on a fixed fixture corpus; telemetry hard-off, egress denylisted,
   OTel-hijack (#2497) neutralized, **synthetic data only** until confirmed. Avoid the deepeval-ts SaaS
   wrapper.
3. **oh-my-mermaid (MIT)** — one-shot local pilot via pinned `npx` (no global install, no `omm login`),
   generating `.mmd` docs for `apps/api/src/routes`, compared to the repowise overview.
4. **Skyvern (AGPL-3.0)** — one-shot **out-of-band sidecar** measuring failing-tail scrape recovery on
   30–50 URLs; self-hosted, `SKYVERN_TELEMETRY=false`, local/cheap LLM, AGPL kept as a separate HTTP
   service (no reach into the dowiz tree). **Not** wired as a production fallback in this change. RESOLVE
   controls (H4; RESOLVE-2 RA-7): **load-bearing = a network-level egress allowlist** ({target URLs, local
   LLM} only, dropping all else at the network layer — a reverse-proxy to a cloud model is blocked by
   *destination*, not by host label) + no `DATABASE_URL` + **human review of a sample** for third-party PII +
   **named expiry date + owner**; the `SKYVERN_LLM` host-*string* check and menu-only URL heuristic are
   honestly **secondary/best-effort** (the latter unenforceable on JS-heavy SPAs). Residual (malicious proxy
   at the allowlisted local endpoint) accepted, bounded by one-shot scope + sampled human read + expiry.

Topology = **Option A (quarantine-and-CI-gate everything)** with cadence trims: $0 static guards on every
PR, paid behavioural/eval jobs on parser-path/nightly triggers. Rejected: **Option C (sidecar mesh)** —
productionizes unproven AGPL tooling into the request path; **Option B (minimal)** — defers cheap,
isolated, real-gap-closing wins for no risk reduction.

**Explicitly DEFER:** LangGraph-JS (redundant with the certified in-house harness), WorkOS/auth.md (adds
attack surface to the auth red-line; do **not** host an `auth.md`, AuthKit SaaS = PII-egress/lock-in).
**Explicitly REJECT:** Recall (PII to immutable ledger → erasure impossible), MoneyPrinterTurbo
(out-of-scope + copyright liability), Glossopetrae (executable AGPL covert-channel; need met by trivial
in-house encodings), Reverse/zhaoxuya520 (offensive RE — wrong threat model).

## Consequences

- (+) Adversarial corpus continuously corroborates that the parser ignores injected directives, on the
  🔴 PII/injection red-line, with a **$0 static reachability guard** as the floor.
- (+) Quality eval (hallucination/faithfulness/G-Eval) closes the gap ADR-0011 left, CI-only and isolated.
- (+) Living architecture docs piloted at sub-dollar one-shot cost.
- (+) Failing-tail scrape recovery becomes a *measured number* instead of a guess — without committing to
  AGPL in the runtime.
- (−) Adds a Python venv to CI and a guard rule to maintain.
- (−) Recurring eval/injection nightly spend ≤ ~$105/month (capped by fixture limit + parser-path
  trigger).
- (−) Honest: no item gives a hard injection guarantee — human draft review (ADR-0011) remains the floor.

## Data / migrations

**N/A for all four items.** Test-only fixtures, CI report artifacts, `.mmd` docs, and a sidecar with its
**own** ephemeral storage. **No dowiz schema, no migration, no RLS surface, no tenant data path.** The
relevant invariant is the inverse: **no tool may obtain a dowiz `DATABASE_URL`/RLS-bearing credential**
(enforced by the no-AGPL-import + license guard).

## Money

No money path is touched. The one absolute-determinism point inherited from ADR-0011: in the eval, price
correctness is **exact integer-minor-unit match, zero tolerance**.

## Security / red-lines

- **Reachability (RESOLVE C1/H1/M2; RESOLVE-2 RA-3/4/8):** **structural non-reachability is primary** — the
  corpus lives at the **repo-root test-only dir `tests/injection-corpus/`**, outside every `Dockerfile`
  `COPY` and `build-apps.ts` `cpSync` source, so it is absent from the build context (RA-4's nested-`tests`
  ignore and RA-3's image-inspection questions are *moot*). The floor `scripts/guardrail-corpus-reachability.mjs`
  is an **exit-1 standalone script** in `verify:all`+CI+pre-commit (NOT a warn-level ESLint rule). It (1)
  asserts the corpus dir is never under a `COPY`/`cpSync`-reachable path (inspecting the real
  `build-apps.ts` copy-sources, not Dockerfile text); (2) does a defense-in-depth sentinel/content scan with
  a **deterministic two-constant self-skip** (own `__filename` + the one corpus dir, no growable allowlist —
  RA-8); (3) an import-edge pass. The behavioural test (fixture→menu-JSON-only, sentinel absent, prices
  grounded), loaded from the test context via `path.resolve(repoRoot, 'tests/injection-corpus')`, is the
  load-bearing proof on the parser path.
- **AGPL reach (RESOLVE H2/M3; RESOLVE-2 RA-2):** real, **shippable** license logic (was vaporware *and*
  would have been all-red as first fixed). `scripts/guardrail-license.mjs`/compliance-gate scans the
  **third-party** production closure of `pnpm-lock.yaml` (**exempting first-party `private` workspace
  packages**), evaluates **SPDX properly** (LGPL ≠ GPL; `(MIT OR GPL-2.0)` resolves to MIT; `WITH`
  exceptions), fails on `AGPL/GPL` SPDX IDs, and routes a missing third-party license to an explicit
  `compliance/license-exceptions.md`; plus a **forbidden-dependency list** (langgraph/@workos-inc/skyvern/
  recall/moneyprinter/glossopetrae). Skyvern reached over HTTP only, never `import`ed.
- **PII egress (RESOLVE M1/H3/H4; RESOLVE-2 RA-1/5/6/7):** DeepEval telemetry off + egress **allowlist**
  (only the Anthropic judge endpoint reachable — a denylist is fail-open); **CI is the enforceable
  authority, the local wrapper is best-effort** bounded by synthetic-only-on-disk (RA-6). Skyvern: network
  **egress allowlist** is the load-bearing control (host-string check + menu-only are secondary/best-effort
  — RA-7), no dowiz credential, human PII review of a sample. New external-service env vars are caught by an
  **env CLASSIFICATION gate** — `compliance/env-classification.md` classifies every `*_URL/*_KEY/*_TOKEN/…`
  identifier `internal`|`external-subprocessor`, **fails closed on the unclassified case**, and forces every
  external one into `subprocessors.md` (a *pattern detector* would storm on ~18 internal secrets and re-grow
  the allowlist — RA-1). The fixture-content PII gate is defense-in-depth for *structured* PII; bare names
  are held by human review (RA-5, accepted residual).

## Gates (deterministic "done" proof)

- **G1** **exit-1 `scripts/guardrail-corpus-reachability.mjs`** in `verify:all`+CI+pre-commit, red→green:
  **structural** assertion that the corpus (`tests/injection-corpus/`) is never under a `COPY`/`cpSync` path
  (inspects the real `build-apps.ts` sources, RA-3) + a defense-in-depth sentinel/content scan with a
  deterministic two-constant self-skip (RA-8) + import-edge pass + a fixture-content PII gate for *structured*
  PII (bare names held by human review, RA-5). Behavioural test (fixture as OCR → menu-JSON only, sentinel
  absent, prices grounded), loaded from the test context, is the load-bearing proof on the parser path. (An
  ESLint `no-corpus-in-source` rule may exist at `'error'` for fast feedback but is NOT the floor.)
- **G2** DeepEval CI pre-flight asserts telemetry-off + egress **allowlist** active (arbitrary-host probe
  fails) + #2497 neutralized → exits non-zero before loading fixtures if any fails; local-run fail-closed;
  infra-vs-quality discriminator (transport error → `NEEDS-RERUN` advisory; computed breach → exit 1);
  venv absent from `apps/api` runtime deps.
- **G3** committed `.mmd` for `apps/api/src/routes` + repowise-overview comparison note; no `omm login`.
- **G4** `docs/research/` measurement of recovery-rate + cost/URL + **load-bearing controls (RA-7):** a
  network egress allowlist ({targets, local LLM}), no dowiz credential, human third-party-PII review of a
  sample, named expiry; the `SKYVERN_LLM` host-string check + menu-only URL heuristic are honestly
  secondary/best-effort. Promotion gated on a separate ADR.
- **G5** **shippable** license/PII boundary check, all `process.exit(1)`, red→green: (a) AGPL/GPL via proper
  **SPDX** evaluation over the **third-party** closure only (first-party `private` workspaces exempt; LGPL≠GPL;
  dual-OR handled; missing license → `compliance/license-exceptions.md`) — RA-2; (b) forbidden-dependency
  list; (c) env **classification** gate (`compliance/env-classification.md`: every `*_URL/*_KEY/*_TOKEN/…`
  classified internal|external; **fail-closed on unclassified**; external ⇒ must be in subprocessors.md) —
  replaces both the false "check B already covers it" and the storming pattern detector (RA-1).

Each gate must be rowed in `docs/regressions/REGRESSION-LEDGER.md` before the corresponding item is
"done". No item ships to the production runtime — everything is CI-only, local-only, or out-of-band.
