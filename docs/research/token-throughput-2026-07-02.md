# Token throughput / bandwidth levers (research + first measurement, 2026-07-02)

Operator ask: raise token-processing bandwidth with ready free tools, zero quality-gate cuts.
Full verdicts per the 12-rule tooling grammar; measured with ccusage over the local usage JSONL.

## Measurement (ccusage, MIT, npx-only — ADOPTED as on-demand observer)
Week 06-25→07-02 totals: ~4.58B tokens (~$4,110 API-equivalent). Today (07-02):
input 908k · output 1,908k · cache-create 12.9M · cache-read 365.9M.
**Reading:** cache-read:fresh-input ≈ 400:1 — prompt caching already absorbs the fan-out
preamble replication; compression middleware would only break it. The real spend is OUTPUT
tokens + per-lane cache-creation. Command: `npx -y ccusage@latest daily|--instances`.

## Verdicts
| Tool | License | Verdict | Why |
|---|---|---|---|
| ccusage (ryoppippi/ccusage) | MIT | **ADOPT (on-demand)** | read-only, local JSONL, zero egress; the measurement everything else depends on |
| Anthropic Batch API (pattern: s2-streamstore/claude-batch-toolkit) | MIT | **PILOT** | 50% off in+out, <1h typical; fits councils/librarian/rituals/bulk transforms (latency-tolerant); needs API key = operator billing decision |
| Per-agent model tiering (`model:` frontmatter, .claude/agents) | native | **ADOPT-pending-operator** | haiku for mechanical agents (sweeps, log-triage), Sonnet/Opus for judgment; CCR's routing win, zero proxy risk; `.claude/agents` = protect-path → operator applies |
| claude-code-router (musistudio) | MIT | PARK-with-trigger | proxy MITM + tool-use fidelity risk on cheap models; trigger = working cheap-model lane + deterministically-gated mechanical class |
| Claude Code OTEL telemetry | first-party | PARK-with-trigger | needs always-on collector; trigger = ccusage rollups insufficient |
| LLMLingua/-2 (microsoft) | MIT | **REJECT (harness path)** | no injection seam; breaks exact-prefix caching (net token INCREASE); silent context mutation = quality-gate risk; repowise distill already owns the artifact-compression lane |
| GPTCache (zilliztech) | MIT | **REJECT** | semantic cache on changing repo state = false-green generator; maintenance stalled; Anthropic prompt caching is the correct cache here |
| ollama local offload | MIT | REJECT on this box | 3B @ ~10-12 tok/s on the same 4 cores that build — throughput goes DOWN; trigger = ≥8 dedicated cores/GPU or Ubicloud lane |

## Structural sink + fix (confirmed by measurement)
Fan-out preamble replication is real but ~fully cache-served. Remaining levers, in order:
1. **Batch lane** for every non-interactive round (council critics, librarian, Sunday rituals,
   per-item transforms) — halves marginal cost, frees the interactive budget. Operator: API key.
2. **Model tiering** in agent frontmatter — cuts OUTPUT-token price on mechanical lanes.
   Operator: edit .claude/agents/*.md (protect-path).
3. Keep subagent context lean (deferred tool schemas, Repowise skeletons over raw Reads —
   already policy; verify compliance with ccusage --instances periodically).
Related: docs/operating-model/lane-capacity.md (lane budget this pairs with).
