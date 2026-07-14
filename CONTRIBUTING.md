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

- Kernel (Rust/WASM, source of truth): `cd kernel && cargo test`
- Web/apps: `pnpm install && pnpm -r typecheck && pnpm -r build`
- Adapter boundary: JS/TS never re-implements kernel math; the kernel is authoritative.

## Local-first / offline-first principle

Contributions must not introduce mandatory network, vendor, or cloud dependencies
for core functionality. Prefer the standard library and offline algorithms.

## Trademark

"dowiz" / "DeliveryOS" are trademarks of the project owner (see `NOTICE`). Code
contributions do not grant trademark rights. See `TRADEMARK.md` for usage policy.

## Reporting security issues

Do not open public issues for security vulnerabilities. Contact the owner privately.
