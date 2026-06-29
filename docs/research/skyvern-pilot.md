# Skyvern failing-tail pilot — G4 (tooling-integration-eval, Phase 4)

**STATUS: SCAFFOLDED — PENDING OPERATOR RUN.** Out-of-band, one-shot measurement. Not wired into the
app, not in CI, not a fallback. This environment has no Skyvern instance and no curated failing-URL
corpus, so the recovery numbers below are a template the operator fills on a real run.

**Owner:** Operator. **Expiry:** run within 30 days of starting the sidecar; on expiry `docker stop`
the container and freeze this artifact. Promotion to a real production fallback is a SEPARATE ADR.

## What it measures

Recovery-rate + cost-per-URL of [Skyvern](https://github.com/Skyvern-AI/skyvern) (AGPL-3.0,
self-hosted) over the restaurant URLs the current Playwright + brand-extractor scrape pipeline
**fails** on (empty/garbage menu extraction). Decides whether the failing-tail recovery justifies the
operational + licensing overhead of a fallback — measured BEFORE any wiring.

## Boundary & controls (ADR-tooling-integration-eval G4; Breaker H4 / RA-7)

- **AGPL stays OUT OF TREE.** Skyvern runs as its own container; dowiz reaches it ONLY over HTTP
  (`SKYVERN_BASE_URL`). No `import`, no vendored source, no dependency — `guardrail-license` (G5)
  forbids `skyvern` in the closure or as a source import.
- **Load-bearing: network egress allowlist** at the sidecar's container/network layer — only
  `{explicit target URLs, the local-LLM endpoint}` reachable; everything else dropped. A `localhost`
  reverse-proxy to a cloud model is blocked because the *destination* is not allowlisted (RA-7).
- **No dowiz credential** in the sidecar env — machine-checked: `scripts/skyvern-pilot/no-credential-attest.mjs`
  (red→green proven: a `DATABASE_URL` in the env → exit 1; clean → exit 0).
- **`SKYVERN_TELEMETRY=false`**; `SKYVERN_LLM` = a local model.
- **Human third-party-PII review** of a SAMPLE of what Skyvern sent/received — the failing tail is
  disproportionately JS-heavy contact/about pages that carry owner names/phones the menu-parser
  deliberately drops; the menu-only URL heuristic is unenforceable on SPAs (RA-7), so this human read
  is the real backstop. Record the review note below.
- **Residual (accepted, Operator):** a malicious proxy at the allowlisted local-LLM endpoint defeats
  the jail — bounded by one-shot scope, the sampled human read, and expiry.

## Run

```
# 1. Stand up Skyvern out-of-tree (its own container; telemetry off; local LLM; egress firewall).
# 2. Attest the sidecar env holds no dowiz secret:
node scripts/skyvern-pilot/no-credential-attest.mjs <sidecar.env>
# 3. Measure over the failing URLs:
SKYVERN_BASE_URL=http://<sidecar> SKYVERN_ENV_FILE=<sidecar.env> \
  scripts/skyvern-pilot/run-measurement.sh failing-urls.txt
```

## Results (operator fills)

| metric | value |
|---|---|
| failing URLs tested | _TODO_ |
| recovered (usable menu) | _TODO_ |
| recovery rate | _TODO_ |
| avg cost / URL (vision-LLM) | _TODO_ |
| verdict (wire fallback? sep. ADR) | _TODO_ |

### Human third-party-PII review note
_TODO — sample of sent/received content reviewed for incidental third-party PII; findings here._

LAST-REVIEWED: 2026-06-29
