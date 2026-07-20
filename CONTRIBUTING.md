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

dowiz is Rust-first (~270 `.rs` files). The former TypeScript/JS frontend and its pnpm/turbo
stack were removed 2026-07-15 ("drop js") — there is no `pnpm install` step and no `apps/api`,
`apps/web`, or `packages/` anymore. **There is no root `Cargo.toml` / cargo workspace** — each
crate is standalone; you must `cd` into a crate directory before running cargo there.

- Kernel (Rust/WASM, source of truth): `cd kernel && cargo test`
- Engine (physics render engine): `cd engine && cargo test`
- One-shot local gate before pushing: `bash scripts/verify-kernel-engine.sh`
- Format/lint (run inside the crate dir): `cd kernel && cargo fmt --check && cargo clippy`
- Zero-dependency web demo (renders only; all math in the kernel wasm): `cd web && npm run serve`
- Adapter boundary: no crate outside `kernel/` re-implements kernel math; the kernel is
  authoritative. See the root `CLAUDE.md` for the full crate map and build model.

## Local-first / offline-first principle

Contributions must not introduce mandatory network, vendor, or cloud dependencies
for core functionality. Prefer the standard library and offline algorithms.

## Trademark

"dowiz" / "DeliveryOS" are trademarks of the project owner (see `NOTICE`). Code
contributions do not grant trademark rights. See `TRADEMARK.md` for usage policy.

## Reporting security issues

Do not open public issues for security vulnerabilities. Contact the owner privately.
