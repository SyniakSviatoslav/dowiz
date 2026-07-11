# Master Execution Plan — Gap Blueprints 2026-07-11

> Synthesis of the 13 gap blueprints in this directory (G01–G13), each produced by an independent
> research session on 2026-07-11 against the full-project audit
> (`docs/research/2026-07-11-full-project-audit-dowiz-bebop.md`). This document sequences them,
> maps cross-gap dependencies, and consolidates every operator decision into one queue.
> ZERO code was changed producing any of this — everything below is plan, not action.

---

## 1. What the research changed about the picture

The 13 deep-dives materially *corrected* several audit-level assumptions. Read these first —
they reshape the sequencing:

| # | Correction | Source |
|---|---|---|
| 1 | **The "rewrite-aware merge problem" does not block the GDPR trio.** `fix/audit-remediation` forks exactly at origin/main's tip `c8b2d5a0` (97 ahead / 0 behind, same hash lineage). The 500+ add/add conflict problem applies only to post-scrub paleo/sovereign branches. The trio ships as a curated file-set PR — no history surgery needed. | G01 §2 |
| 2 | **The GDPR photo purge would silently no-op even if merged**: `workers.ts:100` constructs `AnonymizerService(pool, messageBus)` with NO storage wired, in every lineage. A ~6-line DI fix is a required 4th change. | G01 §5 |
| 3 | **An active process re-dirties any scrub**: a scheduled cloud loop pushes the pre-scrub secret-bearing lineage to `origin/fix/audit-remediation` roughly 6-hourly (last: 2026-07-11 12:07Z). It must be paused before any force-push. Also: the pre-scrub backup bundle is GONE — origin is currently the sole full copy of pre-scrub history. | G02 §4–5 |
| 4 | **The checkout bug is two bugs**: the 3-kind enum (returns 400, not 422) AND a missing `receiver{}` in the `.strict()` schema — every "deliver to someone else" order fails for all 6 kinds. No DB migration needed (074 already applied the 6-kind CHECK on prod). ~15 LOC in one protect-paths-blocked file. | G03 |
| 5 | **Staging cutover state is NOT unknown** — probed live: S1/S3/S5/S7/S9/S10 still serve Rust today (matches the 07-05 frame). The real unknown is behavioral parity. But draft migrations 085/086 now have a **numbering collision** with formal 085/086 placed 07-07, and the 085 watermark date passed in the unsafe (double-pay) direction. | G04 §1,4 |
| 6 | **The FE-blocking "8 kB budget" was requirements drift** — 8 kB is the Paraglide i18n overhead check (measured 972 B); the authoritative storefront budget is 60–90 kB gz and the current page is 21.6 kB. The blocking decision dissolves. Dependency is inverted: the cutover's S1 re-flip *waits on* Astro parity, not vice versa. | G05 §1,6 |
| 7 | **Sovereign Core verification debt is worse than "not run"**: `hub_checkout` gates nothing (telemetry-only), `replay-parity-check.sh` is a placeholder, `cause_hash` is the literal string "placeholder", the staging Playwright suite is vacuous (wrong field names, zero real assertions). | G06 §2 |
| 8 | **Two real bugs sit in the staged ML-DSA-65** (found by bebop's own overlap review 07-11 16:26Z): `w1_encode` double-highbits zeroes the Fiat-Shamir commitment; latent wrong `make_hint` arg. Both one-line fixes, specified. The crypto is now STAGED (not just working-tree). bebop sessions **cannot write memory** — `.claude/settings.json` denies Edit/Write. | G08 §1,3 |
| 9 | **bebop2's PQ code is not FIPS-interoperable by construction** (coefficient-domain keys, CBD-sampled matrix A, 32-byte challenge vs 48) — bespoke schemes wearing FIPS names; cannot be validated against official vectors until re-derived. Timing hotspots include the exact KyberSlash class. | G09 |
| 10 | **The claim funnel is broken at the hinge**: `GET /claim` = 404 on prod AND staging (`SPA_ROUTES` omission — one-line fix, failing spec already exists). 11/12 demo storefronts don't exist on prod. Both candidate domains unregistered. **Prod worker machine stopped since 07-03.** | G11 §1–2,5 |
| 11 | The 06-27 "forgotten security trio" is mostly LOW (courier hash = login-DoS at worst; CORS wildcard largely inert; pii-cipher latent). The real item is **B3: prod runs as `dowiz_app` with BYPASSRLS — ~103 RLS policies dormant** (latent-critical, XL, red-line, sequenced last). | G10 |
| 12 | Several "open" items are already closed: `lint:gates` fixed; GH #9 stale (all 9 scripts exist on origin/main); guard-bash FP fix + loop-registry circuit landed in `4077c11d` (audit §7.7 partially falsified). The `checkout-phone` break is test debt (testid renamed by ADR-0016), not a product break. The pickup "fix" everyone assumed (persist PICKED_UP) would be WRONG — kill the phantom broadcast instead. | G12, G13 |
| 13 | The stale worktrees' staged diffs would **re-insert a real Supabase credential over a REDACTED placeholder** — discard after a rotation check; harvest only 2 commits by hunk-apply. | G12 §7 |
| 14 | **ADR-020 ("the FINAL goal") was never committed** — referenced in 8+ docs, exists in no ref's history. Zero PARKED markers exist in 152 memory files. | G07 §1 |

---

## 2. Execution waves

Dependencies force this shape. Waves 0–1 are days, not weeks.

### Wave 0 — Protect & stop the bleeding (same day, mostly S-effort)
No dependencies; every item is either protective or stops an active hazard.

| Step | From | Gate | Effort |
|---|---|---|---|
| 0.1 Pause the ~6-hourly cloud push loop re-dirtying origin (and the daily plane-maintainer pushes) | G02 | operator (it's their scheduled trigger) | S |
| 0.2 Protect the bebop crypto WIP: tar+bundle+patch trio backup → apply the 2 one-line ML-DSA fixes → 3-model attest → commit+push (incl. the test-count doc bump the pre-commit pins) | G08 Ph.1 | operator runs it (bebop sessions can't Write) | S–M |
| 0.3 Take a fresh `--mirror` bundle of origin (sole copy of pre-scrub history) | G02 Ph.2 pre-step | — | S |
| 0.4 Install gitleaks + CI hard-fail + canary RED proof (kills the verify:secrets false-green; also fixes the bogus `-i "."` flag) | G02 Ph.1 | pre-approved ("highest value / zero risk") | S |
| 0.5 Land the 3 uncommitted gate diffs (with the 2-line P7 regex amendment) + close stale GH #9 with drafted evidence + META-CONTROLLER.md line swap | G13 Ph.1–3 | — | S (~2h all G13) |
| 0.6 Restart the stopped prod worker machine; rotate the lost `PROVISION_OPS_SECRET`; check rotation status of the Supabase cred in the worktree diffs | G11, G12 | operator (Fly access) | S |

### Wave 1 — The prod vehicle (days 1–2) — THE critical path
One curated PR onto origin/main (same-lineage, no history surgery), operator-gated merge, CI auto-deploys prod. Contents:

1. **G01**: GDPR trio as curated file-set (2 blobs verbatim + surgical webhook diff + 5 tests + 4 ledger rows) **+ the AnonymizerService storage-DI fix** + webhook `secret_token` preflight/re-issue.
2. **G03 ride-along**: MessengerKind 6-kind enum + `receiver{}` schema + handle max 120→500 (operator applies the draft — protect-paths file).
3. **G11 ride-along**: `/claim` added to `SPA_ROUTES`; OG-card commit `6a89d6e8`; demo storefront provisioning for prod.

Proof stack: each fix has its falsifiable RED/GREEN spec in its blueprint (photo-purge R2 assertion, webhook 401 RED, 4×201 GREEN + `icq` 400 RED run against unfixed prod first, `/claim` spec that fails today). Rollback: 3 revertable commits, zero migrations/env.

**This wave discharges the only live legal exposure and unblocks the revenue funnel in the same merge.**

### Wave 2 — Validation week (days 3–7, operator-personal)
G11 days 3–7: QR sheets (dep already present), operator-identity footer, buy the domain, unfurl dry-run, then concierge outreach ArtePasta → Dubin & Sushi → Apollonia.
GREEN = a real order row from a non-operator customer on a claimed venue. RED = 0 claims after 10 contacts across 5 venues → pre-committed stop/pivot.
**G07's arbiter doc should be signed at the start of this wave** — its ranking (validate-first; then Sovereign MVP > rebuild > bebop > OSS) is exactly this sequencing; review date 2026-07-25.

### Wave 3 — The scrub window (after Wave 1 lands, scheduled)
G02 Phase 2, strictly ordered: freeze (loop paused, Actions disabled — CI deploys prod on main push) → mirror bundle verified → 2 force-pushes + ~30 branch deletions → fresh-clone gitleaks + reachability proofs (RED-run against the pre-push bundle first) → GitHub Support gc / `gh api` 404 probe. Unblocks ADR-020 gate 1. Consider the fresh-repo-swap alternative (unusually cheap: 0 forks, 2 issues) — operator decision D-G02-1.

### Wave 4 — Program hygiene & gated tracks (parallel, post-Wave-1)
| Track | Plan | Effort |
|---|---|---|
| Sovereign Core (ranked #1 active by G07) | G06 Option B "D1-complete": close verification debt first (~3 days: real hub_checkout flag, real replay-parity, real cause_hash, non-vacuous Playwright), build 2.1 + 1.4 content-hash oracle, activate 0b-6 CI, PARK 1.3/2.4 signed | 8–11 lane-days, 4 operator touchpoints |
| Rebuild cutover | G04: Phase 0 re-baseline (probe script already 9/9 GREEN) → operator picks Path A (resume; only if the full A0 gate sheet is banked up front) or **Path B mothball (default)**; either way: renumber draft migrations 085–089→087–091, disarm the degrade-storm (boot-grace + restart RED test + real alert), never verbatim-apply the passed watermark | B: 1–2 sessions; A: 12–18 |
| Astro FE | G05: FE-0 only (budget re-anchor 25/35/60 kB signature + CI gz gate + fix the 3 scaffold defects); FE-1+ only if the arbiter confirms the rebuild | FE-0: 1–2 sessions |
| Security edges | G10 sequence: ledger PENDING rows + `guardrail-security-debt-review.mjs` (the "never again" mechanism) → cheap fixes (courier `LIMIT`/dead-code, CORS origin pin, pii-cipher version byte) → OR-1 two-mode boot-guard → **B3 staged NOBYPASSRLS flip last (red-line, XL)** | S→XL |
| Ops queue | G12 batches: E2E truth-pass (checkout-comm testids, rate-limit allowList) → guard/hook infra (staleness-guard 3 blind spots, pre-commit Docker→CI) → pickup phantom-broadcast kill → worktree harvest (2 hunk-applies) then prune (operator, after cred check) → GH #19 reroute via Actions | M total |
| bebop hygiene | G08 Ph.2–4: bootstrap memory corpus (drafts ready-to-paste; needs the settings carve-out D4), 12-file detritus relocation + deterministic guard, doc truth-pass | M |
| bebop2 assurance | G09: in-repo ladder (Wycheproof, differential-vs-oracle for the interoperable set, dudect, fuzz) + adopt the 4-tier value-bearing policy + decide PQ re-derivation + NLnet/NGI Zero application | M–L + operator decisions |

---

## 3. Dependency graph (why this order)

```
0.1 pause push-loop ──────────────┐
0.3 mirror bundle ────────────────┼──► Wave 3 scrub ──► ADR-020 gate 1 ──► OSS track (parked)
0.4 gitleaks ─────────────────────┘         ▲
                                            │ (G01 first: legal harm > dead-cred hygiene)
Wave 1 PR (G01+G03+G11 rides) ──► prod ─────┘
        │                          │
        │                          └──► Wave 2 validation week ──► first real order (GREEN)
        │                                        ▲
0.6 worker restart + /claim fix ─────────────────┘

G07 arbiter signature ──► funds ONE active track (Sovereign) ──► G06 D1-complete
                                                └──► G04 defaults to mothball; G05 stops at FE-0
G08 0.2 protect-WIP ──► bebop memory bootstrap ──► detritus guard (also feeds G07's P12 spec)
```

Inverse dependencies discovered: G05 is NOT blocked by G04 (S1 re-flip waits on Astro, not vice versa); G01 is NOT blocked by the history bifurcation; G03 needs NO migration.

---

## 4. Consolidated operator decision queue

Every gate across the 13 blueprints, deduplicated, in recommended answering order.
(Details + full option analysis in each blueprint's §6.)

| # | Decision | Blueprint | Default/Recommendation |
|---|---|---|---|
| D1 | Approve Wave-1 PR to main (= prod deploy): GDPR trio + DI fix + G03 + /claim + OG/demos | G01/G03/G11 | YES — only live legal exposure |
| D2 | Scope: include the one-line `orders.metadata.client_ip_hash` strip (GDPR gap #1) in Wave 1? | G01 | YES (one line, same subsystem) |
| D3 | Pause/retire the 6-hourly re-push loop + daily plane-maintainer pushes | G02 | PAUSE now, retire after scrub |
| D4 | bebop `.claude/settings.json` carve-out so sessions can write memory | G08 | YES (scoped to memory paths) |
| D5 | Run the crypto protect-and-commit sequence (0.2) today | G08 | YES — largest unprotected WIP |
| D6 | Sign the arbiter doc (ranking: validate-first; B>A>D>C; rules R1–R5; review 2026-07-25) | G07 | SIGN, or re-rank explicitly |
| D7 | Rebuild: Path A (resume, bank full A0 gate sheet) vs Path B (mothball, signed PARKED + monthly keep-alive) | G04 | B unless A0 banked |
| D8 | FE budget re-anchor signature (25/35/60 kB working targets) | G05 | SIGN — unblocks FE-0 at zero code |
| D9 | Sovereign exit gate redefinition to "D1-complete" (park 1.3/2.4 signed) | G06 | YES |
| D10 | Scrub mechanics: in-place force-push vs fresh-repo swap; scrub window date | G02 | swap is viable (0 forks); pick a window |
| D11 | B3 NOBYPASSRLS program: greenlight the staged council sequence (after cheap wins) | G10 | YES, sequenced last |
| D12 | Worktree prune after harvest + cred-rotation check (destructive) | G12 | YES after harvest verified |
| D13 | Domain purchase (porosite.al / dowiz.app) + who does outreach days 5–7 | G11 | operator-personal, this week |
| D14 | bebop2 PQ: re-derive to FIPS interop? adopt 4-tier value policy? NLnet application? | G09 | policy YES; re-derive before any value-bearing use |
| D15 | GH #19 egress: allowlist policy vs GH-Actions reroute | G12 | reroute via Actions |

---

## 5. Honest accounting

- **Total execution estimate if everything is funded**: Waves 0–1 ≈ 2–3 days; Wave 2 = the week
  (mostly operator-personal); Wave 3 ≈ 1 scheduled window; Wave 4 ≈ 25–40 lane-days spread across
  tracks — which is exactly why G07's WIP-limit (one active future) is the load-bearing decision.
- **What this plan deliberately does not do**: build WS4 video, payments/fiskalizimi code, new
  Astro islands beyond FE-0, bebop2 kernel/cli/reloop, or any Phase-C/D rebuild work — all parked
  behind the arbiter doc's re-entry criteria.
- **The one metric that matters** (per G07/G11 and the repo's own Business-Value-Sort): a real
  order row from a non-operator customer on a claimed venue. Every wave above either protects
  existing work or shortens the path to that row.

*Produced 2026-07-11 by the gap-research program (13 read-only Fable sessions + this synthesis).
No code, config, git state, or deployment was changed. All action items above await operator
execution/approval per the repo's never-bypass-human-gates rule.*
