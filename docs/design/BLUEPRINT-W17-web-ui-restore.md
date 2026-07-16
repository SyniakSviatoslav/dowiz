# BLUEPRINT W17 — web-UI wasm-bridge restore (Rust-native, NO TS)

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
