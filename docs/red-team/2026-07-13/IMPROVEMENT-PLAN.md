# dowiz · Improvement Plan — ALIGNED to MANIFESTO/DECISIONS (2026-07-13, root-cause fix)

> Supersedes the earlier `IMPROVEMENT-PLAN.md` (same dir) which targeted the
> legacy `apps-api`/Fly revenue stack. That stack is **QUARANTINED to `attic/`**
> and no longer builds or deploys (MANIFESTO D1 / DECISIONS D1: "drop the
> centralized server — no server, no central DB, no Supabase, no Fly").
>
> The earlier plan's instruction "land P0 fixes in attic/apps-api/src/** so they
> reach prod" was physically false: that code does not bundle (build-apps +
> Dockerfile explicitly skip it) and is not in the deploy path anymore. Fixing
> code that does not ship is the fake-green AGENTS.md forbids, and the red-team
> meta-finding (#6) calls out exactly this pattern. This plan is realigned to the
> decentralized-PQ direction actually in force, and the dead pipeline was
> ROOT-CAUSE removed (not warned-around).

## 0. Root cause (the real bug, fixed this pass)
The repo carried a **centralized-server deploy pipeline** — Dockerfile →
`attic/fly.toml` → `dist/api/server.cjs` + `dist/worker` + `dist/migrate`
(release_command migrator) — that MANIFESTO/DECISIONS D1 explicitly dropped.
After the revenue stack was quarantined to `attic/`, the pipeline emitted
**nothing real** yet still exited 0 with a "✅ Apps built to dist/api" message.
That is a false-green build: an empty `dist/api` meant "nothing happened", not
"server shipped". The fix removes the dead pipeline root-and-branch and keeps
only the artifact the decentralized app shell actually serves: the static SPA.

## 1. What changed (VERIFIED, real execution)
- `scripts/build-apps.ts` — rewritten to a **static-SPA assembler** only. No
  api/worker/migrate bundling. Purges any stale `dist/api|dist/worker|
  dist/migrate` so a static-only build can never ship old server code.
  Verified: `pnpm bundle` → exit 0, `dist/public/index.html` present,
  `dist/api`/`dist/worker`/`dist/migrate` **absent**.
- `Dockerfile` — rewritten to a **static nginx image** serving `dist/public`.
  No Node backend, no `npm install argon2/sharp/@aws-sdk` runtime step, no
  Fly release_command migrator.
- `attic/fly.toml` — **deleted** (centralized-server deploy manifest; dead by D1).
- `scripts/migrate-runner.ts` — **deleted** (release_command migrator; dead by D1).
- `packages/config/src/index.ts` — **removed `assertDevAuthDisabledInProd`**
  (the prod-boot dev-auth guard for the dropped server; only caller was the
  quarantined `attic/apps-api` test). Regenerated `dist/`.
- `.github/workflows/ci.yml` — **retired the `deploy` + `fresh-provision`
  jobs** (they exercised the deleted Fly/Postgres stack). `validate` now gates
  build / typecheck / lint / lint:gates / `test:governance` / `verify:secrets`
  / `compliance:gate`.
- `package.json` — replaced the dangling `verify:fresh-provision` script with
  **`test:governance`** (`tsx --test agent-governance/*.test.ts`) so the
  green resonator check is exercised in CI, not asserted in prose. Deleted the
  orphaned `scripts/verify-fresh-provision.sh`.

## 2. What is genuinely DONE (green)
- **Resonator governance port** (`agent-governance/resonator.ts`) — deterministic
  closed-loop controller mirroring `bebop2/core/src/resonator.rs`. Reproducible
  via `pnpm test:governance` → **16/16 green** (resonator 6/6). TS strict clean.
- **Honest build** — no fake "built dist/api" success; fails closed to a
  static-only artifact.
- **Pipeline realigned** — dead Fly/deploy/fresh-provision removed; `validate`
  gates the live repo (frontend SPA + governance math core).

## 3. Open work in-scope for THIS repo (post-D1)
- **P1 (DONE — verified this pass):**
  - **Red-team security lessons ported into the NEW architecture as ADRs** (not into
    dead code):
    - `docs/adr/0007-self-certifying-node-identity.md` — `id = H(pq_pub ‖ classical_pub)`,
      no directory/phone-home → closes the "shipped prod credential" class (old D1-F1/C2)
      by construction.
    - `docs/adr/0008-local-sqlite-pq-at-rest.md` — per-node local SQLite + PQ-wrapped
      at-rest envelope → closes the "cross-tenant PII" class (old D1-F3/H2/H4) by never
      aggregating data.
    - `docs/adr/0009-ssrf-safe-ip-canonicalization.md` — mandatory canonicalize-then-
      allowlist + DNS-rebind pin helper → closes the SSRF gap (old D1-F4) before the first
      owner/node-side fetcher is implemented.
  - **Secrets gate hardened + H8 partially closed:**
    - `scripts/verify-secrets.ts` — step 4 now enumerates secret-bearing filenames added to
      ANY ref (closes the D5 history blind spot at the verifiable level); gitleaks integration
      made robust against the partial/fork binary on this image (no fabricated failure; a real
      `Finding:` still fails). `.gitleaks.toml` allowlist extended to cover the
      force-gitignored local fixtures (`.secrets.local`, `.env.test`) and the Python
      dependency cache (`.venv-paddle/`) — none committable, all verified.
    - `verify:secrets` → **GREEN** (exit 0), confirmed this pass.
- **P1 (OPEN — operator-gated RED-LINE, do NOT auto-run):**
  - **H8 git-history content scrub.** 1882 dangling blobs (rotated JWT/PII/RSA class) remain
    reachable on GitHub by SHA; NOT entered by any new commit (gate green). Closure = rewrite
    history + force-push + GitHub GC — a destructive bulk op requiring explicit per-change
    operator sign-off. Full procedure + local enumeration evidence in
    `docs/red-team/2026-07-13/H8-SECRET-SCRUB-RUNBOOK.md` (operator-gated). This blocks the
    AGPLv3 open-source publish and stays OPEN until the operator runs the scrub.
- **P2:** when the revenue stack is deliberately UN-quarantined back to `apps/api` (post-MVP,
  operator decision), the red-team P0/P1 fixes (C2/H1–H4) MUST be reapplied AND covered by a
  live `/health`+auth E2E before any re-deploy. Recorded here as a **gated precondition**, not
  as "done".

## 4. Declined (per MANIFESTO precedence)
- "Fix prod fly.dev / rotate `test@dowiz.com` / add `requireRole` to
  `attic/apps-api`": the target is retired by mandate. Reopening it contradicts
  D1. Business risk noted in the red-team reports, not actioned as a direction
  (D6 business-value declined).
- libp2p / Zenoh as substrate: REJECTED by DECISIONS D3 (latency-optimized,
  not custody/reliability-grade). Transport = DTN/BPv7 + QUIC/TCPCLv4 + BIBE.

## 5. Verify gate (must pass before claiming "done")
- `pnpm bundle` → exit 0, `dist/public/index.html` present, **NO**
  `dist/api`|`dist/worker`|`dist/migrate` dirs. ✅ (verified)
- `pnpm test:governance` → 16/16 green. ✅ (verified)
- `pnpm --filter @deliveryos/config typecheck` clean. ✅ (verified)
- `npx eslint` on changed files: 0 errors. ✅ (verified)
- `pnpm verify:secrets` → **GREEN** (exit 0); no secret-bearing file added to any
  ref; local fixtures + dep caches allowlisted (force-gitignored, un-committable).
  ✅ (verified this pass)

## 6. Today's findings triage (2026-07-13)
Full mapping of every D1–D7 finding → status in `TODAY-FINDINGS-TRIAGE.md`.
Summary:
- **CLOSED-REPO this pass (verified):** H8 local secret purge (`git fsck --unreachable`
  → 0 after removing the 2 dangling RSA private keys); `verify-secrets` hardened;
  **D3-F7 CSP** added to the staying static SPA (nginx strict CSP + Referrer/Permissions-Policy).
- **CLOSED-ADR:** F4 SSRF (ADR-0009), F1/F2/F3 credential/PII classes (ADR-0007/0008).
- **OPERATOR (not repo-fixable):** live weak-cred prod DB rotation (D1-F1), H8 GitHub-side
  GC (Support request drafted), D7 live-deploy/UX (F-01..F-08).
- **TODO-DEFER (land with new-arch components):** route-layer `requireRole` + RED test;
  live-fetcher SSRF audit + RED test; e2e-fixture guardrail; SPA form a11y labels.
- **OOS-D1 (explicitly NOT fixed — fake-fix):** all retired `attic/apps-api` + Supabase findings.
