# dowiz → Rust/WASM Migration Plan

**Status:** ACTIVE · **Owner:** operator (SyniakSviatoslav) · **Date:** 2026-07-13
**Precedence:** supersedes any plan that keeps TypeScript/JavaScript/Node.js in this repo.
**Source of truth:** `bebop-repo` (`bebop2-core` cdylib/WASM + `crates/bebop` host agent).
**Method:** phased waves, max independent lanes per wave, every blueprint verified RED→GREEN
before merge. No single destructive sweep — replace layer-by-layer as the Rust lands.

---

## 0. Mandate (verbatim intent)

> "rewrite all ts to rust/wasm/kernel - no ts, js, nodejs - give permission to fully replace
> all ts/js/nodejs code. rust/wasm. Keep working step by step with lanes - do not rush
> everything at once."

Consequence: **zero TS/JS/Node remains at the end.** The SvelteKit storefront, Next.js admin,
Supabase/Node server, Stripe glue, and all `agent-governance` TS are replaced by Rust/WASM
binaries + a thin WASM host. The governance/math core is NOT ported to TS — it lives once, in
Rust, and is consumed via `wasm-bindgen` / `cdylib`.

---

## 1. Architecture (target end-state)

```
                        ┌─────────────────────────────────────┐
                        │   WASM HOST (tiny static HTML/JS     │
                        │    shell — generated, not hand-maintained) │
                        └───────────────┬─────────────────────┘
                                        │ wasm-bindgen
                        ┌───────────────▼─────────────────────┐
                        │   bebop2-core (cdylib, no_std+alloc)  │
                        │   PQ crypto · field/VSA · algebra ·  │
                        │   resonator · lyapunov · dmd ·        │
                        │   entropy_ledger · admit · …          │
                        └───────────────┬─────────────────────┘
                                        │ rlib
                        ┌───────────────▼─────────────────────┐
                        │   crates/bebop (host agent)           │
                        │   governor · coherence · persistence  │
                        │   orthogonality · renormalizer ·      │
                        │   audit · snapshot · wiring           │
                        └───────────────┬─────────────────────┘
                                        │ axum / tonic
                        ┌───────────────▼─────────────────────┐
                        │   Rust server (replaces Supabase fn +│
                        │   Node/Next) — Postgres via SQLx,     │
                        │   PQ-signed frames (proto-cap), DTN   │
                        └───────────────────────────────────────┘
```

No TypeScript compiler, no `node_modules`, no `pnpm`. The only JS that may survive is the
**generated** WASM loader glue (emitted by `wasm-pack`/`wasm-bindgen`, not authored).

---

## 2. Phases & lanes

### Phase 0 — Governance/math KERNEL (IN PROGRESS)
Source: `docs/design/hydraulic-loop-v2/` blueprints. Target: `bebop-repo`.
- Wave 0: BP-01 resonator defrost ✅ · BP-02 geodesic metric ✅ · BP-22 TS-port DELETED (superseded)
- Wave 1 (math-correctness 🔴): BP-03 Francis QR · BP-04 diffusion sign ✅ · BP-05 PID Jury · BP-06 entropy ledger ✅
- Wave 2: BP-07 online DMD · BP-08 admit() · BP-09 persistence · BP-10 orthogonometer · BP-11 renormalizer
- Wave 3: BP-12 AuditLog · BP-13 salience-decay · BP-14 semantic field-veto · BP-15 guard-bash · BP-16 snapshot · BP-17 money (RED-LINE)
- Wave 4: BP-18 6-layer mount · BP-19 dashboard · BP-20 orchestration · BP-21 Kalman · BP-23 yellow batch

### Phase 1 — Agent-governance module → WASM binding
Replace `dowiz/agent-governance/*.ts` (already mostly deleted) with a `wasm-bindgen` wrapper
around `bebop2-core::resonator` + `crates/bebop` modules. No TS remains.

### Phase 2 — Server → Rust
Replace Supabase Edge Functions / Node API (`dowiz/apps/api`, `attic/apps-api`) with an Axum
server in `bebop-repo` (or a `dowiz/server` Rust crate). Postgres via SQLx. PQ-signed frames
reuse `bebop2/proto-cap` + `bebop2/proto-wire`. Stripe moved to a Rust crate (or dropped if
out of scope).

### Phase 3 — SPA → Rust/WASM frontend
Replace SvelteKit storefront (`dowiz/apps/web`) + Next.js admin (`dowiz/apps/admin`) with a
Leptos/Yew WASM app calling the Phase 0/1 kernels. The only authored web artifact is `index.html`
+ generated wasm loader.

### Phase 4 — Deletion
Once Phases 0–3 land and verify green, delete all `*.ts`/`*.js`/`*.svelte`/`package.json`/
`pnpm-lock.yaml`/`node_modules`. Repo becomes Rust-only.

---

## 3. Invariants (every lane must hold)
1. **Verify-first:** real `cargo test` / `wasm-pack test` gate per blueprint; never fake-green.
2. **Rust sole source:** no TS/JS re-implements governance math.
3. **Red-line (money/auth/RLS):** BP-17 + Phase 2 money paths need per-change confirmation.
4. **Fail-closed:** a missing/ambiguous input → rejected, never coerced.
5. **No data loss:** never delete a TS layer until its Rust replacement verifies.

---

## 4. Progress ledger (verified)
| Phase | Item | Status | Evidence |
|-------|------|--------|----------|
| 0 | BP-01 resonator | GREEN | bebop2-core 6/6 |
| 0 | BP-02 geodesic metric | GREEN | bebop2-core 7/7 |
| 0 | BP-04 diffusion sign | GREEN | coherence 4/4 + mass gate |
| 0 | BP-06 entropy ledger | GREEN | crates/bebop 11/11 |
| 0 | BP-22 TS port | DELETED | resonator.ts/test.ts removed, 0 refs |
| 0 | BP-03 Francis QR | IN FLIGHT | — |
| 0 | BP-05 PID Jury | IN FLIGHT | — |

Updated as lanes complete.
