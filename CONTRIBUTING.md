# Contributing to dowiz / DeliveryOS

Thank you for your interest in contributing. This project is licensed under the
**GNU Affero General Public License v3.0 (AGPLv3)** with the **Developer Certificate
of Origin (DCO)**. By contributing you agree to the DCO (see the root `DCO` file).

## Sign your commits (DCO)

Every commit MUST include a `Signed-off-by` trailer certifying the DCO 1.1 terms:

    git commit -s -m "feat: short description"

If you forget, amend and re-sign:

    git commit --amend -s

CI rejects commits without a valid `Signed-off-by`.

## Development setup

dowiz has two main parts:

### Web PWA (JS, no build step)
```bash
cd web
# Serve locally (any static file server)
python3 -m http.server 8080
# Or: npx serve .
```
- Single-file app: `web/src/app.js`
- Styles: `web/src/styles/` (tokens.css, base.css, animations.css)
- Kernel bridge: `web/src/lib/kernel/kernel_client.mjs`
- Telemetry: `web/src/lib/telemetry/`
- No bundler, no npm install needed
- Open http://localhost:8080 in browser

### Kernel (Rust → WASM)
```bash
cd kernel
cargo test
cargo fmt --check && cargo clippy
```
- Source of truth for geo/spectral/FSM math
- Zero-dependency wasm boundary

### Pre-submit checklist
- `node --check web/src/app.js` (JS syntax)
- `cd kernel && cargo test` (kernel green)
- `bash scripts/verify-kernel-engine.sh` (full gate)

## Local-first / offline-first principle

Contributions must not introduce mandatory network, vendor, or cloud dependencies
for core functionality. Prefer the standard library and offline algorithms.

## Trademark

"dowiz" / "DeliveryOS" are trademarks of the project owner (see `NOTICE`). Code
contributions do not grant trademark rights. See `TRADEMARK.md` for usage policy.

## Reporting security issues

Do not open public issues for security vulnerabilities. Contact the owner privately.
