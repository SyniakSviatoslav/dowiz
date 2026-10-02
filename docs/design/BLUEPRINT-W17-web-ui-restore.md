# BLUEPRINT W17 — web-UI wasm-bridge restore (Rust-native, NO TS)

> **STATUS 2026-10-02 (lane W-ROADMAP, applying the operator's decision of 2026-10-02 — "усе в роадмап, усе потрібно, ніяких видалень, усе обновити": everything into the roadmap, nothing deleted, everything updated).** This is a July-2026 planning document. The architecture it plans (per-node rusqlite, dtn7/BPv7, a QUIC bearer, pgrust, Astro, the native SPA server, mesh crates as a runtime) has **no code on `main` at `9ace8f28`**: `rusqlite` appears only in `tools/deep-clean`; `dtn7`, `bp7`, `quinn`, `pgrust`, `wgpu` have 0 hits under `workers/` and `crates/`; `tools/native-spa-server` was deleted on 2026-10-01 (`b1865597`). **It is history, not status, and it is kept unchanged below this banner — not archived, not deleted (operator 2026-10-02).** What the product became and what is next: `docs/design/ROADMAP-2026-09-22.md` (entry point; its NEXT section is dated 2026-10-02). Where this file's themes went: the event store/log → the hub's append-only log, phases 1-7 of `docs/design/BLUEPRINT-HUB-COST-AND-ORDER-LOG-2026-09-20.md` (SHIPPED) and the bebop DAG store (DG1-DG10, LANDED); the money law and the order FSM → `crates/dowiz-core/src/order_machine.rs` + the conservation audit (8 laws, live); telemetry → DW6/W-TELEM (done, unmerged) and BN8; PQ/crypto → ML-KEM-768 ACVP (`8ae71778`), F15-F17 (Wave N3-A), the seal waits for the operator's keypair (OA-3); mesh/DTN/P2P → refused with measured numbers (`docs/research/2026-10-01-fundamental-bottlenecks-orders-of-magnitude.md` §7.3); local AI/LLM → Wave L decisions (no cloud voice), the VOICE research row; GPU/WebGPU/tensors → withdrawn by the operator (C-1, `7cdcf0ab`); Kani/TLA+/Lean → Lean as a cross-check only (DG6 `01783eb4`, `sorryAx` 0); product UI → Phases A-C of the live roadmap, all LANDED; CI/gates → the 28 gates under `tools/gates/` run by `run-all`; MCP/agents → P14-P20 (Waves N2-B, N4.1). The 13 `crates/dowiz-core` modules of this era that nothing calls are **kept** and get a caller or a header under row N4.2 (never a deletion).


## WHY
JS/TS purge (`f9ab28ff`) deleted the only files that called the kernel's wasm exports
(spectral/order_machine/geo). ORGANISM-STATUS 07-15: those 2 organs regressed from
"wired" to "stranded". `web/src` is now EMPTY (0 .mjs/.js). The product has no working
front-end nerve-endings. Must restore kernel-driven UI WITHOUT reintroducing TS/JS compute
(kernel owns all math authority; AGENTS invariant).

## WHAT (acceptance)
- `web/src/lib/kernel/kernel_client.mjs` — env-agnostic bindKernel + fail-closed (kernel
  Result-rejection → null/ok:false). Reuses `dowiz-kernel` wasm (pkg-web).
- `web/src/app.mjs` — boots kernel, calls `spectral_radius_js` / `geo_progress_flat_js` /
  `fsm_graph_report_js`, renders ρ / drift-class / FSM-signature from kernel math ONLY.
- `web/index.html` + `web/serve.mjs` (zero-dep, correct `application/wasm` MIME) — already
  scaffolded 07-14; verify + fix if drifted.
- `packages/ui/dist/lib/geo-anim.js` stays DEPRECATED/LEGACY (gitignored dist artifact).

## RED→GREEN
- RED: `web/src` empty → browser smoke fails to render kernel output.
- GREEN: `node web/serve.mjs` + headless browser (or `web/package.json` `npm test`) shows
  live kernel render: ρ=1, gap=0, drift=Resonant, FSM acyclic, route snapped. 0 JS re-impl
  of geo/spectral/FSM (grep proof: no haversine/eigen in web/src).

## FILES (Owns — disjoint from all other waves)
- Create: `web/src/lib/kernel/kernel_client.mjs`, `web/src/app.mjs`
- Modify: `web/index.html`, `web/serve.mjs`, `web/package.json`, `web/README.md`
- Test: `web/src/lib/kernel/kernel.test.mjs` (fail-closed assertions)

## RISKS
- pkg-web not built → `npm test` needs `wasm-pack`/built glue. Mitigate: build kernel wasm
  (`cargo build --target wasm32-unknown-unknown --features wasm`) + copy pkg, OR use the
  existing `kernel/pkg-web` if present. Verify glue path resolves.
- No browser in CI → use `node` + a wasm host (wasmtime/node wasm) for smoke; full browser
  render is manual-verify (documented).

## NON-GOALS
- No Svelte/React/TS rewrite. No re-adding deleted legacy UI source.
