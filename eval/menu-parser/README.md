# Menu-parser eval (G2) — DeepEval, CI-only

Regression-grade LLM-as-judge eval of the AI menu-parser (`apps/api/src/lib/ai-ocr-parser.ts`):
**faithfulness** (extracted items grounded in the OCR source — no hallucinated dishes) +
**price-grounding** (every price appears verbatim in the source — no invented prices).

Council-approved (`tooling-integration-eval`, Phase 2 of 4). DeepEval is Apache-2.0 but runs in an
**isolated Python venv in CI only** — never an `apps/api` runtime dep (G5 `guardrail-license` asserts
this), so it cannot drift into the shipped image.

## Safety boundary (the load-bearing part)

`preflight.py` is a **fail-closed** gate that runs BEFORE any fixture loads or `deepeval` imports.
It exits non-zero unless ALL hold (Breaker M1/H3/RA-6):

1. `DEEPEVAL_TELEMETRY_OPT_OUT=1` (telemetry is ON by default) + error-reporting off.
2. No `CONFIDENT_AI_API_KEY`/`DEEPEVAL_API_KEY` (Confident-AI cloud sync OFF).
3. `OTEL_SDK_DISABLED=true` + no off-allowlist OTLP endpoint — neutralizes the reported #2497
   global-TracerProvider hijack/exfiltration.
4. `EVAL_EGRESS_ALLOWLIST` set and **allowlist-only** (`api.anthropic.com`; a denylist is fail-open).
5. Fixtures carry no structured PII (defense-in-depth; the authoring ritual is the floor).

**CI is the enforceable authority** (the network allowlist is a CI-layer construct the raw `deepeval`
CLI cannot escape). Local runs are best-effort, bounded because **only synthetic fixtures exist on
disk** — a raw local run leaks synthetic data, not customer PII. Judge calls go only to Anthropic.

## Run

```
./run.sh           # preflight → generate parser outputs → DeepEval scoring
```

Self-test (no key): `LLM_PROVIDER=heuristic pnpm exec tsx eval/menu-parser/generate-outputs.ts` uses
the deterministic heuristic structurer. In CI with `ANTHROPIC_API_KEY` the live LLM path is exercised.

## Infra-vs-quality discriminator (Breaker L2)

- transport/provider error (no metric computed) → `NEEDS-RERUN`, **exit 0** (advisory, human re-run);
- a COMPUTED metric below `EVAL_THRESHOLD` (default 0.7) → **exit 1** (blocking).

## Files

- `preflight.py` — fail-closed safety gate (stdlib only; red→green proven).
- `generate-outputs.ts` — runs the real `AiOcrParser` on each fixture → `outputs/` (gitignored).
- `eval_menu_parser.py` — DeepEval Faithfulness + GEval price-grounding (judge = Anthropic).
- `fixtures/*.json` — **synthetic** menu OCR + expected JSON (no PII).
- `requirements.txt` — pinned `deepeval` (re-review the #2497 neutralization on any bump).
- `ci-workflow.menu-parser-eval.yml` — the CI workflow (APPLY: operator copies into
  `.github/workflows/` — protected zone; see the header).
- `run.sh` — orchestration (fail-closed order).

LAST-REVIEWED: 2026-06-29
