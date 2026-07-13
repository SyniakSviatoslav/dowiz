# Dowiz Remediation Plan — 2026-07-13 (corrected)

> Corrected after independent re-verification: the original red-team synthesis filed
> F1–F4 against `attic/apps-api`, a tree **quarantined in commit e1505e1d and excluded
> from the build**. Patching it is a no-op. See ESCALATION-RETIRED-TREE.md for the
> full truth table. This plan lists only what is real and fixable.

## DONE (verified green)

- [x] **CI supply-chain pin** — `ci.yml:150` `setup-flyctl@master` → `@v1` (real tag).
  No other `@master`/`@main`/`@latest` refs remain in `.github`.

## ESCALATED (not fixable in-repo; needs operator)

- [ ] **F1** seeded owner cred — manual prod DB check (no egress here).
- [ ] **F2/F3** role gate — live seam mints+Zod-validates `role`; real gap is at the
      route layer, which is not in the built `platform` package yet. Add `requireRole`
      guard + RED test when HTTP routes land.
- [ ] **F4** SSRF IPv6 — audit the LIVE outbound fetcher (attic one is retired).

## NOT DONE (intentionally)

- No edits to `attic/` (retired, not built → fake-fix).
- No fabricated verification of F1/F4.

## Verification

- `ci.yml` change is a one-line pin; YAML parses (no structural change).
- bebop2 remediation (C1 + honest PQ leg) verified by `cargo test --workspace`: 598 pass, 0 fail.
