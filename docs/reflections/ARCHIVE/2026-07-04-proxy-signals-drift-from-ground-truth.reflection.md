# Reflection — proxy signals drift silently from ground truth (verify before acting)

**Date:** 2026-07-04 · **Slug:** proxy-signals-drift-from-ground-truth
**Qualifying trigger:** ≥3 code files + red-line surfaces (S4 media, GDPR anonymizer purge, S3 catalog) in one long session; also a token-reduction program.

## CONTEXT
Long session: rebuilt S3 catalog + S4 media surfaces (Rust, dark), shipped a red-line GDPR fix
(`delivery_photo_key` purge on erasure), then a token-reduction program (7 external-repo teardowns +
a quantified harness audit + a codegraph-rust bake-off) and codified the agentic map-reduce rule.
Two independent incidents this session shared one causal root, which is worth banking.

## DECISIONS
- Ran a harness token-cost audit that estimated ~36.5K reclaimable tokens from "6 duplication clusters"
  in the memory corpus; queued a librarian pass to merge them.
- Integrated S4 from a worktree lane; two reviewers (the build lane itself, then the SSG guardian)
  each raised a HIGH/red-line flag that the GDPR anonymizer purge "had not shipped."
- In both cases I VERIFIED against ground truth before acting, rather than acting on the signal.

## WHERE
1. Token audit `docs/research/2026-07-04-harness-token-audit.md` (memory-dedup row) — grouped clusters
   by FILENAME similarity.
2. S4 build lane + SSG guardian (`invariant-guardian`) — reviewed the worktree copy of
   `apps/api/src/lib/anonymizer/index.ts`.

## WHY (causal — not just where)
Both signals were **proxies that had silently diverged from the real thing, and the divergence was
large and invisible from inside the signal.**
- The dedup estimate used **filename grouping as a proxy for semantic redundancy.** Content-level
  Jaccard (the ground truth) showed only 2 of 6 clusters were genuinely redundant → ~1,330 tokens, a
  **~27× overclaim**. Same-topic filenames are not duplicate content; 4 "clusters" were distinct
  sessions or live reference hubs whose merge would have destroyed reusable pointers.
- The stale red-line flags came from **worktree base drift as a proxy for current code.** The lanes'
  worktrees copied their `apps/api` base BEFORE commit `5ded9f19` landed the fix in main. Reviewers
  faithfully reported the code they saw — but that code was not `main` HEAD. The proxy (worktree copy)
  had drifted from ground truth (`main`), so a resolved red-line got re-flagged twice.
The common root: **an estimate or a review is only as true as the signal it reads; when that signal is
a proxy (filename, a stale checkout), it can be off by an order of magnitude or invert a
resolved/unresolved verdict — silently, because nothing inside the proxy reveals the gap.**

## CONFIDENCE
High. The 27× gap is measured (audit estimate vs the librarian's content-verified count). The stale-flag
mechanism is confirmed (grepped `delivery_photo_key` present in `main` at `5ded9f19`; the flagging
reviewers read worktrees whose base predated it).

## NEXT-TIME
- Before deleting/merging on an estimate, verify against the REAL signal (content, not filename). The
  three shipped measurement scripts (`scripts/memory-{metrics,health}.mjs`, `context-tax.mjs`) now do
  the content check — they are the deterministic guardrail that caught this.
- When dispatching a review/SSG lane against a worktree, either sync the worktree base to `main` HEAD
  first, OR tell the reviewer the exact commit to diff against, OR cross-check any red-line flag against
  `main` before treating it as live. A "red-line not fixed" flag from a lane on a stale base is a
  false positive until confirmed against `main`.

## LINK
- Guardrail already shipped: the 3 measurement scripts (commit `cbf0d088`).
- Related: [[audit-remediation-orchestration-2026-07-03]] (worktree-swarm integration hazards),
  the token-reduction synthesis `docs/research/2026-07-04-token-reduction-synthesis.md` §3 (the ~27×
  correction), and the agentic map-reduce rule `docs/operating-model/agentic-map-reduce.md` (a mapper
  returns distillate the reducer TRUSTS — which only holds if the mapper read ground truth, making
  "what base did the mapper read" the load-bearing question).
