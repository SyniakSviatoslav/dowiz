# Reflection — rebuild wave-2: channel-adapter integration + Phase-A S1 Rust surface landed

- **Date:** 2026-07-04
- **Trigger:** qualified change — 5 commits, ≥3 code files, red-line-adjacent surfaces
  (money/orders header path, config env, Rust API layer). "Continue rebuilding, max lanes."
- **Class:** rebuild-program execution / lane-harvest discipline.

## WHAT

Landed 5 commits on `fix/audit-remediation`: (1) QR/NFC + TMA dark channel adapters,
(2) PR-3 voice UI kit, (3) rebuild wave-2 (Astro scaffold + S1/S2 OpenAPI SSOT + programmatic
env-census extractor + S2-auth & WS-authz council DRAFTs + E2E skip triage), (4) E2E hardening,
(5) the S1 storefront-read surface in Rust (20 ops, 97+30 tests, clippy -D clean).

## WHY (causal, not just where)

Three things that would have bitten silently, each caught by a deterministic signal, not by care:

1. **The env-census regression pins caught the program's own drift.** The R4 extractor's
   live-tree pin (`68 raw reads`) went red the moment this session's own TMA `process.env.TMA_ENABLED`
   raw read landed — exactly the map-coverage promise ("nothing missed" is a CI property, not a
   claim) firing on the author, one hour after it was written. The right move was to *move the pin*
   (with a comment tracing the +1 to TMA and the operator-gated EnvSchema entry that will flip it to
   `both`), NOT to loosen the assertion — the pin did its job.

2. **The staging E2E failure was environmental, and the way I knew was an unrelated oracle.**
   3/4 channel tests passed; the 4th (full checkout → x-channel header) failed at
   `getByTestId('checkout-phone')`. Rather than weaken it, I ran the unrelated parity-oracle spec
   `flow-simpl-s1` — it failed at the *identical* DOM step. That decorrelated the failure from my
   change: the storefront add-to-cart→checkout flow is broken on staging demo data, pre-existing.
   The test is left correct-but-red (not skipped, not weakened); the header propagation stays
   unit-proven (web 8/8, api 12/12). Lesson reinforced: an unexplained red on a new test is
   confirmed environmental only when an *independent* committed test reproduces it.

3. **The config red-line gate fired on a legitimate one-line dark flag** and I did NOT fight it.
   The TMA `TMA_ENABLED` EnvSchema entry tripped `post-edit-gates.sh` (packages/config red-line).
   The server flag reads `process.env.TMA_ENABLED` directly (same convention as its sibling dark
   `TG_*` flags), so the feature works dark today; the EnvSchema line is queued operator-gated.
   Correct posture: propose the protected line, don't force it — matches the standing gated-queue rule.

## WHERE

- `scripts/rebuild-map/__tests__/extract-server-flags-envs.test.mjs` (moved pins + trace comment)
- `e2e/tests/channel-attribution.spec.ts` (poll-not-race, rate-limit-aware nav, honest 4th-test note)
- `apps/web/src/lib/tma.ts` + `packages/config/src/index.ts` (dark flag; EnvSchema half operator-gated)
- `rebuild/` (S1 Rust surface harvested; domain money/order-status/tenant byte-identical verified)

## FLAGS (escalate, not inline-fix)

- **Pre-existing staging checkout-flow breakage** — add-to-cart→checkout fails the demo storefront
  E2E (`flow-simpl-s1` + channel test #4 identically). Not this change. Needs its own investigate loop.
- **Live 3-kind 422 confirmed statically** — FE offers 6 messenger kinds, order-create Zod
  (`packages/shared-types/src/legacy.ts:48`) accepts 3 → phone/signal/simplex 422. Operator-gated
  fix (shared-types is protected; the `MessengerKind` unification is the queued draft).
