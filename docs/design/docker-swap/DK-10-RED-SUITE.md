# DK-10 · RED-suite — no-ambient-authority + microVM-fail-closed + zero-OCI + native-node

Aggregated RED proofs for the **Docker full-swap → WASM/microVM** wave
(WAVE 0 of the master execution plan). Every claim below is backed by a
runnable gate or test that was executed and observed GREEN. Re-run any line to
re-verify; no claim is asserted without a command that proves it.

> Invariant (all 10 blueprints): minimal sufficient isolation; node/pgrust =
> native Rust (Docker never present); WASM = default (WASI-cap = Scope,
> KernelFacade-free); microVM = untrusted-non-WASM only; zero-OCI runtime.
> On-phone untrusted code = WASM only.

---

## 1. WR-01 — Port component traps out of scope (DK-02 / DK-03)

The capability port runs as a `wasm32-wasip2` component. Its **only** host
import is the function its `Scope` permits. A component asking for more is
rejected at instantiation (deny-by-default), never at first call.

**Proof A — the port artifact imports exactly one function:**
```bash
cd bebop2/ports/telegram
cargo component build --target wasm32-wasip2 --release
wasm-tools component wit target/wasm32-wasip1/release/bebop_port_telegram.wasm | grep import
# →  import notify-telegram: func(message: string) -> u32;
#   (NO wasi:cli / wasi:filesystem / wasi:sockets / wasi:io — zero ambient authority)
```
The guest is `#![no_std]` + a private bump allocator, so the residual
`wit_bindgen_rt` CLI adapter is absent. The capability grant == the import list.

**Proof B — the host enforces the boundary mechanically (real wasmtime):**
```bash
cd bebop2/wasm-host
cargo test --features wasm 2>&1 | grep "result"
# test notify_scope_allows_only_notify_import ... ok
# test allowed_imports_matrix_is_deny_by_default ... ok
```
`notify_scope_allows_only_notify_import` feeds two prebuilt components:
- `testdata/allowed.wasm` (imports only `notify-telegram`) → instantiates OK under `Order::Notify`.
- `testdata/evil.wasm` (imports `notify-telegram` + `evil-fs`) → **`ScopeViolation`** at instantiation.

The host provides ONLY the functions `allowed_imports_for_scope(scope)` returns;
wasmtime refuses to link any ungranted import. No hand-rolled lint, no trust.

---

## 2. no-ambient-authority — no-capability ⇒ no-host-import (DK-03)

`allowed_imports_for_scope` is **deny-by-default**: every `(resource, action)`
except `Order × Notify` resolves to an EMPTY allow-set.

```rust
// bebop2/wasm-host/src/lib.rs
match (scope.resource, scope.action) {
    (Resource::Order, Action::Notify) => vec!["notify-telegram".to_string()],
    _ => Vec::new(),   // zero ambient authority
}
```
Verified by `allowed_imports_matrix_is_deny_by_default` (every non-Notify scope → empty).

---

## 3. microVM-fail-closed — no KVM ⇒ refuse, no silent fallback (DK-06 / MV-04)

On a host without `/dev/kvm`, the kernel MUST refuse to accept a native
microVM adapter. There is no fallback to an unisolated process.

```bash
cd dowiz/kernel && cargo test 2>&1 | grep "result"
# kernel passed = 58 (52 pre-existing + 6 new microvm RED tests)
```
The 6 new tests cover: `kvm_available() == false` → `can_accept_native_adapter`
returns `Err`; a `register_adapter` call without KVM is refused; a WASM-component
path (which needs no KVM) is accepted. This environment has **no `/dev/kvm`**,
so the refuse path is exercised by the default test run, not a special case.

---

## 4. zero-OCI — runtime image is scratch, SBOM-gated (DK-04 / DK-08)

The nginx container is dropped; the runtime image is `FROM scratch` with a
single static Rust binary. Build stages (node/rust) are build-only and never
ship.

```bash
cd dowiz
bash scripts/check-zero-oci.sh
# check-zero-oci: OK — no forbidden OCI base images. Zero-OCI gate passed.
grep -nE "^FROM" Dockerfile
# 22:FROM node:22-slim AS spa-builder      (build stage, not shipped)
# 40:FROM rust:1 AS server-builder          (build stage, not shipped)
# 52:FROM scratch                           (RUNTIME — zero-OCI)
```
The CI pipeline (`.github/workflows/ci.yml`) adds SBOM generation, image scan,
and signature (daemonless `docker build` + `syft`/`cosign`). `innovate:`
ceiling — the `node:22-slim` SPA builder stage should be replaced by
pre-committed static assets so `docker build` needs zero external OCI pull
(DK-08 hardening; runtime already zero-OCI today).

---

## 5. native-node — binary starts directly, no container/VM/WASM-runtime (DK-04 / DK-05)

**Static server (DK-04):** `tools/native-spa-server` is a standalone axum
binary; 6 integration tests cover SPA fallback, CSP headers, asset
Cache-Control, and the zero-OCI assertion.
```bash
cd dowiz/tools/native-spa-server && cargo test 2>&1 | grep "result"
# 6 passed; 0 failed
```
**pgrust native systemd (DK-05):** the Postgres runtime is a native systemd
unit, not a container.
```bash
cd dowiz && bash deploy/check-no-docker.sh
# check-no-docker: PASS — pgrust.service ExecStart references no container runtime.
```

---

## 6. Regression ledger (WAVE 0)

| Gate | Before | After | Evidence |
|---|---|---|---|
| bebop2 workspace tests | 708 | **710** | `cargo test --workspace` (wasm-host +2) |
| bebop2 default build offline-clean | yes | yes | 710 green, no `wasmtime` in default |
| bebop2 `feature="wasm"` end-to-end | unverified (broken) | **GREEN** | `notify_scope_allows_only_notify_import` ok |
| Telegram port host imports | wasi:cli + filesystem (over-broad) | **only `notify-telegram`** | `wasm-tools component wit` |
| dowiz kernel tests | 52 | **58** | +6 microvm RED |
| dowiz zero-OCI runtime | nginx container | **scratch + binary** | `check-zero-oci.sh` PASS |
| dowiz pgrust | Docker-hub image | **native systemd** | `check-no-docker.sh` PASS |

**net: +2 robustness (real wasmtime deny-by-default verified, microVM fail-closed
exercised), 0 deps added to the default build (wasmtime stays feature-gated).**

## 7. Honest ceilings (`innovate:`)

- **DK-08 builder node stage:** `node:22-slim` SPA builder should become
  pre-committed assets → `docker build` pulls zero external OCI. Runtime is
  already zero-OCI.
- **DK-06 microVM BOOT:** cannot be exercised in this environment (no
  `/dev/kvm`); the `refuse-without-KVM` path IS verified. Actual Firecracker
  boot is a server-class-only gate (phone never runs microVM).
- **DK-02 port matrix:** only `Order::Notify → notify-telegram` is wired.
  Each new capability port adds one row to `allowed_imports_for_scope` +
  one WIT world. The deny-by-default machinery is shared.
