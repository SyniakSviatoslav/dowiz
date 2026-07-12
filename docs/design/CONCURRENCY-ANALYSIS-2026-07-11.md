# CONCURRENCY ANALYSIS — what runs in parallel vs sequential (2026-07-11)

> Companion to ROADMAP-GROUND-TRUTH-2026-07-11. Grounded in actual local branch inventory
> (27 in-flight branches on /root/dowiz + bebop main) and the Tier gates. Rule: ground truth
> outranks plans; gate conditions are falsifiable, not calendar dates.

## 0. HIDDEN PREREQUISITE (sequential blocker) — RESOLVED 2026-07-12 (verified)

> **Status correction (verified, not claimed):** the blocker described below no longer exists.
> Re-checked 2026-07-12 by direct inspection + `pnpm -r typecheck`:
> - `TourProvider` IS exported by the `@deliveryos/ui` barrel
>   (`packages/ui/src/components/molecules/index.ts:12` → re-exported at `packages/ui/src/index.ts:12`),
>   so `apps/web/src/main.tsx` imports resolve.
> - the string `bebopSkinAttr` does NOT appear anywhere in the tree (no code, no broken ref) — the
>   cited `CourierRoutes.tsx:3` import simply isn't present in the current tree.
> - `pnpm -r typecheck` (apps/web) exits **0** (green).
>
> Conclusion: there is no typecheck-RED blocking merge to main on the current branch. The 2-line fix
> is moot. Keep this section as a reminder that pre-merge typecheck must stay green; do NOT treat it
> as an open blocker.

## 1. PARALLEL-SAFE (Tier 0, zero-pivot-risk, independent files/branches)
These touch disjoint files and can run as concurrent sessions. Each owns its branch; none merges
until §0 clears.

| ID | Work | Branch | Files (disjoint) | Depends on |
|----|------|--------|------------------|-----------|
| P0-1 | Remove 3 money-tweens + P1 token-flip (bebop-skin ON) | feat/design-p1-tokens | ClientLayout, Dashboard, Analytics, EarningsPage, tokens.css, bebop skin | §0 |
| P0-2 | sw.js push handler (courier out-of-app signal) | feat/reliability-push | public/sw.js, web-push subscribe | §0 |
| P0-3 | degrade-storm ratchet (boot-ratchet LD0-2) | feat/ci-security-gates | boot-ratchet.ts, ratchet tests | §0 |
| P0-4 | gitleaks + gate-diffs CI | feat/ci-security-gates | .gitleaks.toml, ci gate | §0 |
| P0-5 | OG <300KB + channel prod-reader + QR kit reader | feat/gtm-channel | meta/og, channel-reader, qr | §0 |
| P0-6 | protocol W/A/H library lines (SEPARATE crate names) | bebop main (branch docs/roadmap-rules) → new crate branches | bebop2 crates | none (own repo) |
| P0-7 | GDPR trio DI-merge prep (verify, don't merge) | fix/audit-remediation | storage/Anonymizer wiring | §0 |

NOTE: P0-3 and P0-4 share feat/ci-security-gates → run as ONE agent (sequenced within branch),
not two. So parallel SESSIONS = 6 (design, reliability, ci-security[3+4], gtm, bebop, gdpr-prep).

## 2. SEQUENTIAL-GATED (cannot parallelize; gate condition must hold)
| ID | Work | Gate (falsifiable) |
|----|------|--------------------|
| S1 | Fix red-main imports (§0) | typecheck green on main |
| S2 | Deploy /claim + G03 to prod | Fly/AWS access + S1 green |
| S3 | Restart prod worker + rotate secret | secrets access (operator) |
| S4 | Tier 1 GDPR trio merge to main | S1 + Tier0 landed |
| S5 | Tier 2 quality bars (13-pt stable, 8-pt gtm) | S4 |
| S6 | Tier 3 G11 = first REAL order | S5 + walk-in demo live |
| S7 | Tier 4 local-first substrate (protocol node R/X) | S6 GREEN |
| S8 | Tier 5 earn-it (money-crypto, messenger, .onion) | S7 |

## 3. CROSS-SESSION CONFLICT WATCH
- apps/web shared by P0-1, P0-2, P0-3/4, P0-5 → each on its own branch; integrate via rebase after S1.
- bebop P0-6 is a SEPARATE repo (no collision with dowiz sessions).
- NO-COURIER-SCORING (operator, final): bebop reputation.rs scoring DROPPED. P0-6 protocol X must
  enforce no-scoring in code (CI gate). Do not build any courier rating.

## 4. EXECUTION WAVE PLAN (max parallel = 3 concurrent per delegate cap)
- WAVE 1 (now): P0-1 (design) · P0-2 (reliability sw.js) · P0-5 (gtm og/qr)  [disjoint web areas]
- WAVE 2: P0-3+4 (ci-security, one agent) · P0-6 (bebop protocol) · P0-7 (gdpr-prep)
- S1 (sequential, me): fix red-main imports → land to main → unblocks all merges.
- After S1 green: integrate waves, then S2..S8 in order.

## 5. WHAT AGENTS MUST NOT TOUCH
- Secrets, prod deploy, worker restart (S2/S3 = operator only).
- Any courier scoring/reputation code (DRIFT R2 → forbidden).
- main directly (work branches; PR after S1).
