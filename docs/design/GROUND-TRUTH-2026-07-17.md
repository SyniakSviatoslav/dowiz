# GROUND TRUTH — dowiz repository state (2026-07-17, live `git`)

> Single source of truth. Captured live via `git`, NOT from memory or older docs.
> Supersedes any earlier "wave plan" / roadmap status that contradicts this.
> Re-verify with `git` before trusting any pasted "verified" status.

## Canonical anchors (verified this session)
- **`origin/main` = `9f78b91d5`** — `docs(integration): finalize off-main feat/* wave plan — all 12 resolved`
- Kernel tests: **452 passed** (default, serde-free) + **107 passed** (`--features pq`, NIST-ACVP KAT)
- No decommissioned `packages/`/`apps/` resurrected in `main` tree (grep-confirmed 0).

## CORRECTIONS to earlier (false) reports
1. **p08 WAS merged.** `kernel/src/typed_metrics.rs` is present on `main` (commit `df9aa6ac1`
   "P08 typed-metrics pure core"). The earlier is-ancestor check returned "NOT on main" ONLY
   because the **remote** branch `feat/p08-typed-metrics-prior-run` does not exist (it is a
   *local* branch); the content reached `main` via merge `eeaf3dd0b`. No content lost.
2. `feat/pq-crypto-tier1` is OFF-MAIN as a branch, but its value is NOT lost: the `kernel/src/pq/`
   subsystem was extracted into `main` as the opt-in `pq` feature (107 KAT tests green). The
   branch itself is left intact for reference.

## feat/* branch inventory — 28 total
### ON-MAIN (20) — content already in `main`; branch refs not deleted, harmless
p01-ci-truth-floor, p02-canon-repair, p07-money-reversal, p18-public-flip-prep, p19-growth-engine,
hermetic-remediation, spectral-energy-flow-evolution, agentic-mesh-protocol-2026-07-17,
decentralized-pq-protocol, remove-legacy-thin-layer, rw-02-delete-channel-js,
rw-03-kernel-money-authority, kalman-organ, markov-attractor-signal, agent-capability-boost,
bebop-governance-port, ci-security-gates, harness-llm-backend, kernel-fsm-graph-analysis,
p08-typed-metrics-prior-run (local only; content on main).

### OFF-MAIN (8) — unique kernel files vs main in parentheses
| branch | kernel | docs | verdict |
|---|---|---|---|
| feat/pq-crypto-tier1 | 24 (extracted→`pq` feature) | 3 | RESOLVED — value on main as opt-in `pq` |
| feat/kalman-organ | 2 (old kalman/mat, already on main) | 0 | RESOLVED-in-favor-of-main (stale) |
| feat/agentic-system | 0 | 521 | stale docs fork — RESOLVED-in-favor-of-main |
| feat/golive-remediation | 0 | 618 | stale docs — RESOLVED-in-favor-of-main |
| feat/mvp-sensor-seams | 0 | 845 | stale docs — RESOLVED-in-favor-of-main |
| feat/plane-telemetry-closed-loop | 0 | 715 | stale docs — RESOLVED-in-favor-of-main |
| feat/product-media-seam | 0 | 688 | stale docs — RESOLVED-in-favor-of-main |
| feat/sovereign-core-phase-zero | 0 | 0 | marker/empty — RESOLVED-in-favor-of-main |
| feat/v1-hardening | 0 | 535 | stale docs — RESOLVED-in-favor-of-main |

**Risk of loss:** NONE for kernel code (0 genuinely-new kernel files outside pq, already
extracted). The 521–845 `.md` files per stale branch are OLD versions (branches are ~882 commits
behind main) of design docs that exist in newer form on `main`. Branches are left intact on remote
so nothing is destroyed; extraction of any specific doc is possible on demand.

## Next waves — finishing upgrade (pending operator go)
- **Public-flip readiness (P18):** `feat/p18-public-flip-prep` already ON-MAIN. Flip = operator
  action (GitHub visibility). Pre-flip doc tidy-up = this file + MEMORY.md sync.
- **Remaining real work on main:** P06 key_V signed done-gate (blocks H3/E3-Phase-B), B2
  settlement full, B1 Wasmtime fuel wiring — all operator-gated, NOT in these branches.
- **Stale-branch cleanup (optional):** 7 OFF-MAIN stale forks can be deleted once operator
  confirms (content already on main; branches retained for now per "no delete without say-so").

## Verification evidence (live commands)
- `git rev-parse --short origin/main` → `9f78b91d5`
- `cargo test --lib` (main) → 452 passed, 0 failed
- `cargo test --features pq pq::` (main) → 107 passed, 0 failed (NIST ACVP byte-exact)
- `git branch -r | grep feat` → 28 branches; reachability loop → 20 ON-MAIN / 8 OFF-MAIN
