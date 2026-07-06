# Reflection — Rust rebuild program complete (10/10 dark): stage-close retro

- **Date:** 2026-07-05
- **Trigger:** stage-close (the entire strangler build half + all councils S1–S10 + cutover harness + S2 re-ratification). Qualified: touched every 🔴 red-line class (auth/money/RLS/GDPR/WS), ≫3 files, closed the largest stage of the program.
- **Outcome:** all 10 surfaces built dark, council-approved, dual-SSG-gated, integrated (tip `77d7e979`). 3 live prod fixes shipped along the way. 0 regressions to the frozen domain.

## WHY it worked (causal, not just where)

1. **The decorrelated adversary is load-bearing, not ceremonial.** Every surface's dual SSG gate (invariant-guardian + security-sentinel) OR its council-breaker found ≥1 real defect that the builder — competent, tests green — did not see: S8 silent-notification-loss (dedup claimed before send, never released on transient failure), S6 JWT-in-URL leak, S10 ungated-future-admin-route class, S2-amendment C1-family-sticky-is-uncomputable. **WHY:** a builder optimizes for "make it pass"; class-specific failure modes (retry-safety, log-leak, gate-escape, key-availability-at-the-gate) are only surfaced by a reviewer whose sole job is to assume it's broken. Single-perspective review structurally cannot substitute. → CONFIRMS the SSG/council structure; do not thin it under time pressure.

2. **Proxy-vs-ground-truth drift is now RECURRENT (4 instances) → promote past advisory.** A "fact" asserted in a packet/council propagates uncontested until a seat re-reads live source: (a) 085 "watermark landmine" is a DRAFT not a live guard [[proxy-signals-drift-from-ground-truth]]; (b) S10-Q4a B3-blocker is `owner_notification_targets`, not `locations` (`locations` has `public_select USING(true)`); (c) S5 anonymous-create is GUC-LESS (packet claimed the inverse); (d) S2-amendment C1 assumed `family_id` is gate-computable — it is in no claim/token. **WHY:** natural-language packets carry claims without provenance; a downstream reader trusts the prose. The existing reflection captured (a); (b)-(d) are the same root recurring. → This is past the advisory threshold (CLAUDE.md §7: recurrent bug → guardrail).

## Deterministic ratchet candidates (for librarian / council-retro — I did NOT self-enact)

- **G1 (proxy-drift → guardrail).** Council packets must carry a `file:line` for every load-bearing red-line claim, and the breaker's DoD must include INDEPENDENTLY re-reading each cited line (not trusting the packet's quote). Cheapest form: a packet-lint that flags a 🔴 claim with no `path:line` citation. Promotes the [[proxy-signals-drift-from-ground-truth]] lesson from advisory to gate.
- **G2 (serious-gate blind spot — a pure config fix, the ideal ratchet).** The PreToolUse serious-gate regex matches auth/money/RLS/migrations by PATH, but ALL rebuild red-line code lives under `rebuild/crates/**` whose filenames were (correctly, per council authorization) kept out of the regex — so every S6–S10 red-line write went UNGATED by the per-file gate (gated instead by the SSG review + council clearance, the stronger checks, so no harm). But the gate is structurally blind to the new stack. → Add a `rebuild/crates/**` red-line glob so future edits to the Rust auth/money/GDPR code trip the gate. Deterministic, one-line, no judgment.
- **G3 (operational friction, low priority).** Worktree build-lanes start with an empty `.claude/state/serious-cleared` → they stop-report on the first red-line write → lead clears the worktree + resumes (cost: one round-trip per lane). A worktree could inherit main's active clearances at creation. Smooths, not safety.

## Related
[[proxy-signals-drift-from-ground-truth]] (the root, now recurrent) · [[rebuild-wave2-channel-integration]] · the program state is in memory `rebuild-decision-rust-astro-2026-07-04` (cutover half remaining, operator-gated).
