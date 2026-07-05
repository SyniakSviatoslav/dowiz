# DeerFlow deep-research pilot — out-of-tree harness tool

**STATUS: SCAFFOLDED — PENDING OPERATOR RUN.** Out-of-band dev/harness experiment. Not wired into the
app, not in CI, not a dependency. **Owner:** Operator. **Expiry:** freeze the artifact if not run in 30 days.

## What it is
[ByteDance DeerFlow](https://github.com/bytedance/deer-flow) (MIT) — a long-horizon "SuperAgent" harness
(research/code/create) built on **LangGraph + LangChain (Python)**. Evaluated as a possible richer engine
for the deep-research lane vs the current `deep-research` skill / ODR.

## Boundary & controls (G5; mirrors the Skyvern precedent)
- **LangGraph/LangChain stay OUT OF TREE.** This project's council already **DEFERRED LangGraph**
  (tooling-integration-eval). DeerFlow runs as its OWN Python process; dowiz never imports it, vendors it,
  or adds it as a dep. `scripts/guardrail-license.mjs` FORBIDDEN-DEP now blocks `deerflow`/`deer-flow`/
  `langchain`/`@langchain/`/`langgraph` in the lockfile or as a source import (red→green proven).
- **No dowiz credential / data** in the sidecar env — `node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>` (the shared dowiz-secret attestation; exit 1 on any DATABASE_URL/JWT/secret).
- **Local LLM + telemetry off**; egress allowlisted to the model endpoint + explicit research targets only.
- **No dowiz tenant/PII** fed to it — research inputs are public/synthetic only.

## What it measures
Research quality + cost on a fixed question set vs the existing deep-research lane. Decides whether
DeerFlow's depth justifies standing up a Python sidecar. Promotion to anything wired = a SEPARATE ADR
(and would still not bring LangGraph into the tree).

## Run
```
# 1. Stand up DeerFlow out-of-tree (own venv/container, local LLM, telemetry off, egress firewall).
# 2. node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>   # must exit 0
# 3. Run the fixed question set; record quality/cost below.
```
_Results: (operator fills on a real run)._
