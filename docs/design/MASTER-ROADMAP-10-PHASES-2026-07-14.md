# MASTER ROADMAP — dowiz / DeliveryOS + bebop (10 phases)

> Source of truth: this doc + `ROADMAP-GROUND-TRUTH-2026-07-14.md` + `MASTER-ROADMAP-MVP-2026-07-12.md`
> + `MANIFESTO.md` + `DECISIONS.md`. Newest operator instruction outranks older doc.
> Precedence: D8 … D0–D7 (DECISIONS) > 2026-07-11/12 docs. Mesh + PQ are NOT deferred (D6 over C8).
> Ground truth (grep/git/cargo) always outranks this plan. Re-verify before trusting any DONE line.

## 0. Verified baseline (live, 2026-07-14 — autopilot)

| Repo | Branch | Tests | Notes |
|---|---|---|---|
| dowiz | `feat/kernel-fsm-graph-analysis` | kernel **152** / engine **25** green | kernel = math authority (geo/spectral/FSM); web/src kernel-driven UI shipped |
| dowiz-pq | `feat/pq-crypto-tier1` | **144** green | ML-DSA-65 / ML-KEM-768, KAT-gated, hybrid X25519⊕ML-KEM |
| bebop-repo | `feat/logic-governance` | workspace **751** green | fmt + logic-governance |

Spine complete + green: kernel FSM/spectral/geo math → engine bridge (geo, spectral) → web UI
renders kernel output only. JS/TS is legacy. Cross-repo full suite is the FINAL gate.

## The 10 phases

### P1 — Sovereign core (Rust/WASM, event-sourced) — DONE
Order-lifecycle FSM as the growth substrate. `decide` composes machine → actor-gate →
cc1 → pricing. Red-line (`kernel::decide`) is the keystone. FSM graph-analysis
(drift-gate `verify_fsm_signature`) closes silent-lifecycle-drift. **Status: DONE + pushed.**

### P2 — Kernel math authority (geo / spectral / FSM) — DONE
`kernel::geo` (haversine/lerp/bearing/progress/eta/snap/is_arriving/point_in_polygon/
is_out_of_order), `kernel::spectral` (Faddeev-LeVerrier + Durand-Kerner general
eigensolver: ρ, SLEM, γ, Fiedler λ₂, DMD drift class), `kernel::order_machine`
(topological_order / reachable / cyclomatic μ / spectral_radius / fsm_graph_report).
All exposed via wasm. **Status: DONE (152 kernel tests).**

### P3 — Engine consumption bridge — DONE
`engine/src/bridge.rs::{geo, spectral}` mirror-pin the kernel flat-bridge protocol;
engine NEVER re-implements math. `LoopDriftDetector` + `CourierMarker` + fail-closed
decoders. **Status: DONE (25 engine tests).**

### P4 — Field UI (kernel-driven, no JS math) — DONE
`web/index.html` + `web/src/app.mjs` boot the kernel wasm and render geo progress +
spectral drift + FSM signature from kernel math only. `web/serve.mjs` (zero-dep,
`application/wasm` MIME), `web/package.json` (serve/test), 20-assert VbM test.
**Status: DONE (committed `64753bc0` + `b28d9e5d`).**

### P5 — PQ transport envelope (protocol, not primitives) — DONE (dowiz-pq)
ML-DSA-65 sign/verify, ML-KEM-768 (FIPS 203 KAT), X25519 (RFC 7748 corrected), hybrid
X25519⊕ML-KEM (both required, no classical-only fallback), entropy seam (OS default +
Anu QRNG quantum⊕OS, never errors), fractal fingerprint (tag, not key). Bundle-shaped
store-and-forward / replay / lifetime / custody in `node`. **Status: DONE (144 tests).**

### P6 — Mesh / real decentralized (bebop2) — PARKED per invariant until dowiz carries it
PHASE-1 done (event_log MESH-06, pgrust compat gate, 4 CI gates green). Honest ceilings:
iroh-QUIC carrier = stub (dalek-conflict + offline-policy); MESH-12 genesis-policy =
operator-gated (HUMAN enum default Unspecified, fail-closed). **Operator gate: mesh
protocol only advances when dowiz has a real flow to carry it.**

### P7 — Frontend surface depth (Sea & Sheet interfaces) — OPEN
`packages/ui` / `apps/web` are empty/untracked (legacy source absent). The kernel-driven
`web/src` UI (P4) is the canonical substitute. Next: grow it into the "Sea" (ambient
field) + "Sheet" (brand-SDF) interfaces per `docs/design/dowiz-interfaces/` (DZ-*),
standing on the kernel math. **No calendar date; gated on operator approving the UI
direction.** Tier-1 canonical prod OG/demo still has no active-stack target (legacy
`attic` only).

### P8 — Ops / reliability / single-pane — OPEN (partial)
`pgrust` immediately; resurrect from `attic` (health/Sentry/notification/backup/
rate-limit) rather than rebuild. Monitoring = VictoriaMetrics + Grafana + Netdata +
Gatus + DEAD-MAN-SWITCH. Degrade-closed circuit breakers (payment → cash). Backups
3-2-1-1-0 (off-Hetzner = top gap). **Honest: no canonical prod yet → these are
spec'd, not deployed.**

### P9 — Self-development / growth substrate (PRIMARY FOCUS, operator 2026-07-13) — IN PROGRESS
Biggest focus = reflection, metacognition, ethics, agnostic/rational inquiry; kernel as
growth substrate. Reverse-engineered `kernel::spectral` (the #1 missing primitive the
hydraulic-loop design named) and proved it correct against hand-derived Laplacian
spectra (P4: λ₂ = 2−√2; periodic Markov: ρ=1, gap=0) — 5/5 ad-hoc proof 2026-07-14.
Research queue (physics-math-exploration.md §2): spectral graph theory → mesh consensus,
Bayesian calibration, integer/overflow laws, causal inference, category theory of
kernel↔wasm↔UI functorial mapping, info-geometry of self-improvement gradient.
**Continue: deepen spectral graph theory (cycle C_n spectrum, SLEM→mixing time τ),
log each exercise in self-improvement-log.md.**

### P10 — Open-source readiness (ADR-020, gated) — OPEN
AGPLv3 + TM + DCO. Gated on secrets scrub + EUTM. SECURITY INCIDENT (creds in git
history) rotated; REMOTE scrub force-push = open gate → HARD blocker for prod push.
~20 missing 2026-07-11 reports: UNVERIFIED manifest filed honest (not fabricated).

## Tier spine (from operating rules)
stabilize v1 → ship prod truth → quality bars → first real order (G11 GREEN) → only
then rewrite substrate. **First real order is the only true gate; everything else is a
falsifiable condition, not a date.**

## INVARIANT
Build DOWN from the first real order, not UP from the protocol. Gates are falsifiable
conditions. Ground truth (grep/git/cargo) always outranks this plan.
Main-merge is an operator gate (frozen `origin/main` anchor); canonical-stack ships on
feature branches. Push-plans-first: plans committed + pushed before execution.

*Generated 2026-07-14. Re-verify before trusting any DONE line.*
