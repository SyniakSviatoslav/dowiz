# dowiz · kernel-driven field UI

Minimal real UI that loads the **Rust `dowiz-kernel` wasm** and renders:

- **geo progress** — courier query point → perpendicular distance to route, snapped
  point on the route, segment index (from `kernel::geo`, via `geo_progress_flat_js`).
- **spectral drift** — spectral radius ρ, spectral gap γ, Fiedler λ₂, drift class
  (from `kernel::spectral`, via `spectral_flat_js`).
- **FSM signature** — vertex/edge count + acyclic flag from the order-lifecycle FSM
  (from `kernel::order_machine`, via `fsm_graph_report_js`). The drift-gate
  (`verify_fsm_signature`) is what keeps this red/green on silent lifecycle change.

> This shell **never re-implements** geo/spectral/FSM math in JS/TS. The kernel is
> the single source of truth; `src/lib/kernel/kernel_client.mjs` only decodes the
> kernel's flat-bridge protocol and fails closed on malformed input.

## Run (zero dependencies)

The dev server serves the **repo root** (not `web/`) so `app.mjs`'s relative import
`../../kernel/pkg-web/dowiz_kernel.js` resolves to the real glue. Requires the
kernel wasm surface to have been built (see below).

```
cd web
npm run serve           # → http://localhost:8099/web/index.html
npm test                # VbM contract tests (20 assertions)
```

## Build the kernel wasm surface

From the repo root (emits `kernel/pkg` + `kernel/pkg-web`, both gitignored):

```
bash scripts/build-kernel-wasm.sh
```

The web glue (`kernel/pkg-web/dowiz_kernel.js`) auto-loads
`dowiz_kernel_bg.wasm` relative to itself via `import.meta.url`.

## Files

- `index.html` — DOM + styling (kernel output target only).
- `src/app.mjs` — boots the kernel wasm, binds it, renders + animates.
- `src/lib/kernel/kernel_client.mjs` — env-agnostic adapter over the kernel wasm
  (node glue for tests; web glue via `bindKernel` in the browser). Fail-closed.
- `src/lib/kernel/kernel.test.mjs` — VbM contract tests mirroring the engine bridge.
- `serve.mjs` — zero-dep static server with correct `application/wasm` MIME.
