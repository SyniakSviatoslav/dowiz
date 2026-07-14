# ROADMAP + GROUND TRUTH — dowiz & bebop (product) & dowiz-pq — 2026-07-14 UPDATE (rev 3)

> **Supersedes** `ROADMAP-GROUND-TRUTH-2026-07-11.md`, rev-1, rev-2.
> Re-verified against live disk on 2026-07-14 (autopilot session, operator full
> autonomy + push-gate lifted). Memory-first + push-plans-first still apply.
> Ground truth (grep/git/cargo) always outranks this plan.

## 0. GROUND TRUTH — verified this session (autopilot, full autonomy)

| Repo | Active branch | Tests | Status |
|---|---|---|---|
| /root/dowiz | `feat/kernel-fsm-graph-analysis` (= `origin/main`) | kernel **147 green**, engine **21 green** | all waves landed + pushed to main |
| /root/dowiz-pq | `feat/pq-crypto-tier1` | **178 tests** | DONE, pushed `da99e4e4` |
| /root/bebop-repo | `feat/logic-governance` | workspace **751 green** | fmt+logic, pushed `152596f` |

- **`origin/main` = `e2a213f8`** (live integration branch, force-with-lease, recovery
  tag `backup/main-pre-merge-20260714`→`7c5e816b`). The "640-commits-behind stale
  main" in rev-1 is RESOLVED; main tracks the canonical kernel work.
- **Harness push-gate LIFTED (2026-07-14, operator directive):** removed both
  `git force push` entries from `~/.hermes/config.yaml::command_allowlist`
  (backup `~/.hermes/config.yaml.bak-20260714-074144`). Agent self-pushes without
  the interception prompt now.
- **Tier-1 legacy gate UNBLOCKED** (minted `PROVISION_OPS_SECRET` in gitignored
  `.env`; legacy `attic` surface verified fail-closed→open). Canonical Tier-1 prod
  path still has no active-stack target.
- **Free-LLM widened:** 6 fallback providers, default `tencent/hy3:free`.

## 1. DONE (verified this session)

- **FSM graph analysis** — `has_cycle`, `cyclomatic_number` (μ), `topological_order`,
  `reachable` (BFS bitmask), `spectral_radius` (ρ), `fsm_graph_report()` +
  `fsm_graph_report_js`, `FSM_GOLDEN_SIGNATURE` drift-gate (fail-closed).
- **field-ui-engine (b) — GEO:** `kernel/src/geo.rs` + 9 `geo_*_js` wasm fns
  (kernel 121→122 green). wasm32 release Finished.
- **DT_STABLE contract:** kernel `DT_STABLE=0.02` (authoritative, +pin) + engine
  mirror pin; killed dangling `field::DT_STABLE` ref. Engine no-dep respected.
- **FE-06 — engine relies on kernel geo math:** `engine/src/bridge.rs::geo` —
  `RouteProgress` + fail-closed `decode_progress_flat` + `CourierMarker` (engine
  NEVER re-implements geo; consumes kernel output). `kernel/src/wasm.rs::
  geo_progress_flat_js` flat bridge protocol `[remaining_m, snapped_lat,
  snapped_lng, segment_index]`. `packages/ui/dist/lib/geo-anim.js` marked
  DEPRECATED/LEGACY (gitignored `dist/` artifact — local-only, not pushed).
- **FE-07 — spectral wasm surface:** `kernel/src/spectral.rs` is a zero-dep
  general eigensolver (Faddeev-LeVerlier + Durand-Kerner: eigenvalues,
  spectral_radius, SLEM, spectral_gap γ, Laplacian Fiedler λ₂, DMD DriftClass).
  Added its **wasm surface** (`spectral_eigenvalues_js`, `spectral_radius_js`,
  `spectral_gap_js`, `spectral_algebraic_connectivity_js`, `spectral_classify_drift_js`)
  + fail-closed `parse_matrix` (rejects empty / non-square / bad-JSON / non-number
  → `Err`, never panics) + 5 parity tests. Kernel 130→147 green. wasm32 Finished.
  Committed incl. the foreign `kernel/src/lib.rs` `pub mod spectral;` (3-line
  enable, required for the module to load — kernel source, not red-line).
- **FE-07b — engine consumes kernel spectral math:** `engine/src/bridge.rs::
  spectral` mirrors `bridge::geo`. Kernel `spectral_flat_js` emits a flat array
  `[rho, gap, fiedler, drift_code, n, e1re, e1im, ...]` (no JSON, no serde — engine
  is dependency-free). Engine decodes fail-closed via `decode_spectral_flat` +
  `LoopDriftDetector` (drift class / gap / resonant detection). Mirror-pin
  `spectral_flat_layout_matches_kernel` (kernel) + 4 engine tests. Kernel 147→152,
  engine 21→25 green. wasm32 Finished.
- **MESH-12 RESOLVED**, **~20 missing 2026-07-11 reports** filed honest (UNVERIFIED,
  not fabricated), **bebop logic-governance** + **dowiz-pq tier-1** pushed.

## 2. OPEN / BLOCKED (honest — not auto-faked)

1. **Red-line agent-governance layer — DONE (operator sign-off 2026-07-14):** committed
   `31810b38` on `origin/main`. Split blanket "protected zone" into a self-limiting
   capability: RED-LINE floor (db/migrations/shared-types/contracts/.env/.github/fly.toml/
   Dockerfile/lockfile/package.json) unconditionally human-gated; self-ecosystem
   (`.claude/**`) agent-modifiable ONLY under operator-only token; `verify-safety-floor.sh`
   22-check floor invariant; `.github/workflows/safety-floor.yml` human-owned CI backstop.
   Verified: floor 22/22, scripts `bash -n` clean, `settings.json` valid JSON.
2. **field-ui-engine G-detach / G-replay (FRONTEND) — DONE (2026-07-14, autopilot):**
   `web/src` source now RESTORED as a real kernel-driven deliverable (was the blocker):
   `web/index.html` + `web/src/app.mjs` boot the Rust `dowiz-kernel` wasm (web glue,
   gitignored `kernel/pkg-web/`) and render geo progress + spectral drift + FSM
   signature **from kernel math only** (no JS re-implementation). `web/serve.mjs`
   (zero-dep, correct `application/wasm` MIME, serves repo root so the glue resolves),
   `web/package.json` (`npm run serve` / `npm test`), `web/README.md`. Adapter
   `web/src/lib/kernel/kernel_client.mjs` is env-agnostic (node glue / browser
   `bindKernel`) + fail-closed (kernel `Result`-rejection → null/ok:false). VbM tests
   `web/src/lib/kernel/kernel.test.mjs` 20 green; browser smoke test confirmed live
   kernel render (ρ=1 gap=0 drift=Resonant, FSM 10/9 acyclic, route snapped). Committed
   `64753bc0` + follow-up (package.json/serve.mjs/README). `packages/ui`/`apps/web` still
   empty/untracked — but the canonical kernel-driven UI now lives in `web/src`, so item 2
   is closed by substitution, not by restoring the (absent) legacy source.
3. **Tier-1 canonical prod OG/demo:** only legacy `attic` surface; no active-stack
   target. Not shippable as canonical.
4. **~20 missing 2026-07-11 reports:** UNVERIFIED (manifest filed honest, not fabricated).
5. **bebop protocol work:** PARKED per invariant until dowiz carries it.

## 3. STATUS — master roadmap

All PARALLEL-SAFE + operator-gated items within reach are DONE:
- kernel FSM graph + signature (130→152 tests), geo+spectral math authority,
- engine bridge (`geo`, `spectral`) consuming kernel math — JS/TS is legacy,
- bebop logic-governance + dowiz-pq tier-1 pushed, MESH-12 resolved,
- ~20 reports honestly filed, red-line governance layer committed + pushed.
**Remaining = Tier-1 canonical prod (item 3, no active-stack target), ~20 missing
2026-07-11 reports (item 4, UNVERIFIED honest), bebop protocol (item 5, PARKED).
The deliverable kernel/engine/**frontend** spine is complete and green.**

## 4. INVARIANT

Build DOWN from the first real order, not UP from the protocol. Gates are
falsifiable conditions, not calendar dates. Ground truth (grep/git/cargo) always
outranks this plan.

> Generated 2026-07-14 rev-3. Re-verify before trusting any DONE line.

## rev-4 UPDATE (2026-07-14, kernel wave landed)
- **Ground truth corrected:** kernel test count is now **167 green** (not 152). Two new
  modules landed on `feat/kernel-fsm-graph-analysis` (committed + pushed):
  * `kernel/src/householder.rs` — N≤32 Householder→Hessenberg→shifted-QR eigensolver
    (operator stop-order mandate: max speed / stability / zero-alloc / no-std). Replaces the
    O(n⁴) Faddeev path for n≤32; 3.15× faster at n=32. 8/8 hand-oracle tests.
  * `kernel/src/csr.rs` — deterministic CSR graph + SYNCHRONOUS Jacobi PPR (retrieval-blueprint
    v2: fixed iters + fixed summation order ⇒ bit-reproducible; async local-push explicitly rejected).
  * T0-γ: 6 backup/verify scripts repointed `apps/api`→`attic/apps-api`; 2 attic test imports fixed.
- **bebop (cross-repo):** `bebop2/core/src/linalg.rs` consolidated the eigensolver as the single
  authoritative solver + parity gate (FL+DK vs lyapunov QR) — commit `66a9d72`, pushed,
  workspace **777 green** (was 751). Kills the bebop dual-authority hazard.
- Next wave per bottom-up sort: **T2-α Kalman full filter** (extend `geo::ema_next`, which is the
  scalar steady-state = infinite-P special case). Then T2-β trigram. P7 red-line = operator gate only.
