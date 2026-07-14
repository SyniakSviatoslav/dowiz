# ROADMAP + GROUND TRUTH — dowiz & bebop (product) & dowiz-pq — 2026-07-14 UPDATE (rev 2)

> **Supersedes** `ROADMAP-GROUND-TRUTH-2026-07-11.md` and the 2026-07-14 rev-1.
> Re-verified against live disk on 2026-07-14 (autopilot session, operator full
> autonomy granted). Memory-first + push-plans-first still apply. Ground truth
> (grep/git/cargo) always outranks this plan.

## 0. GROUND TRUTH — verified this session (autopilot, full autonomy)

| Repo | Active branch | Tests | Status |
|---|---|---|---|
| /root/dowiz | `feat/kernel-fsm-graph-analysis` (= `origin/main`) | kernel **122 green**, engine **18 green** | all waves landed + pushed to main |
| /root/dowiz-pq | `feat/pq-crypto-tier1` | **178 tests** | DONE, pushed `da99e4e4` |
| /root/bebop-repo | `feat/logic-governance` | workspace **751 green** | fmt+logic, pushed `152596f` |

- **`origin/main` is now the live integration branch** — force-pushed (force-with-lease,
  recovery tag `backup/main-pre-merge-20260714`→`7c5e816b`) through the autopilot
  waves. Current main tip: `b2803a46`. The "640-commits-behind stale main" in rev-1 is
  RESOLVED; main tracks the canonical kernel work.
- **Tier-1 legacy gate UNBLOCKED**: `PROVISION_OPS_SECRET` minted (openssl rand -hex 32)
  into gitignored `.env` (verified untracked; cannot reach origin). The gate in
  `attic/apps-api/dist/modules/acquisition/ops-auth.js` verified fail-closed→open with
  real execution. NOTE: this is the **legacy attic** provisioning surface; the canonical
  Tier-1 prod OG/demo has NO live target in the active kernel/web stack (only `dist/`
  artifacts). So Tier-1 prod is "unblocked at the gate level" but not shipped as a
  canonical path.
- **Free-LLM widened** (operator authorized bypass of hermes config guard): 6 fallback
  providers, default `tencent/hy3:free` unchanged.

## 1. DONE (verified this session)

- **FSM graph analysis** (`feat/kernel-fsm-graph-analysis`): `has_cycle` (DFS),
  `cyclomatic_number` (μ), `topological_order` (Kahn), `reachable` (BFS bitmask),
  `spectral_radius` (ρ), `fsm_graph_report()` + `fsm_graph_report_js`. Plus
  `FSM_GOLDEN_SIGNATURE` drift-gate (`verify_fsm_signature`) — fail-closed catch of
  silent lifecycle drift.
- **field-ui-engine (b) — GEO HALF DONE**: `kernel/src/geo.rs` gained
  `polyline_length_meters`, `progress_along_route(poly,pos)->RouteProgress` (replaced
  wrong t.clamp), `is_out_of_order`, `eta_seconds(rem,total,base)` (5 m/s fallback),
  `should_snap(prev,next,thr)` (lat/lng pair), `ARRIVE_THRESHOLD_M=150`/`SNAP_THRESHOLD_M=500`.
  `kernel/src/wasm.rs` exposes 9 `geo_*_js` fns. Kernel 121→122 green. wasm32 release
  build Finished. Commit `b2803a46` (incl. next item).
- **DT_STABLE contract (this rev)**: kernel `DT_STABLE=0.02` (authoritative, + pin test)
  + engine mirror pin; fixed dangling `field::DT_STABLE` reference. Closes silent-drift
  gap. Engine no-dep mandate respected (both sides pin the literal).
- **MESH-12 RESOLVED**: operator-signed-root genesis policy
  (`docs/design/mesh-real/MESH-12-RESOLVED-2026-07-14.md`), reuses dowiz-pq ML-DSA-65.
- **~20 missing 2026-07-11 brief reports**: accepted UNVERIFIED
  (`docs/design/MISSING-REPORTS-MANIFEST-2026-07-14.md`) — NOT fabricated (AGENTS.md).
- **bebop logic-governance**, **dowiz-pq tier-1**: pushed (see §0).
- **bebop2 `field_build`/`sinc`**: correctly PARKED per operator invariant (build down
  from real order; protocol work stays parked). Kernel reuses only plain geo kinematics.

## 2. OPEN / BLOCKED (honest — not auto-faked)

1. **field-ui-engine G-detach / G-replay (FRONTEND)**: BLOCKED — `packages/ui` has NO
   `.ts` source outside `dist/` (only built `geo-anim.js` artifact exists). Cannot
   repoint `use-courier-marker` off `geo-anim.js` without source. Needs the `packages/ui`
   source tree (or a decision that `dist/` is the only deliverable).
2. **Tier-1 canonical prod OG/demo**: only legacy `attic` surface exists; no active-stack
   target. Not a code-blocker, but not shippable as canonical.
3. **~20 missing reports**: still UNVERIFIED (manifest filed, accepted as such).
4. **bebop protocol work**: PARKED per invariant until dowiz carries it.

## 3. PARALLEL-SAFE vs SEQUENTIAL (structure before code)

- **PARALLEL-SAFE (can run now, own branch):** any new kernel/engine feature; doc
  refresh (done); further RW-* consolidation.
- **SEQUENTIAL GATES (operator):** main-merge (DONE this session via force-with-lease),
  MESH-12 (RESOLVED), Tier-1 prod (BLOCKED on active-stack target).

## 4. INVARIANT

Build DOWN from the first real order, not UP from the protocol. Gates are falsifiable
conditions, not calendar dates. Ground truth (grep/git/cargo) always outranks this plan.

*Generated 2026-07-14 rev-2. Re-verify before trusting any DONE line.*
