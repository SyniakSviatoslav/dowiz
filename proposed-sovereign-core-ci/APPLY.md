# Sovereign-core CI gate — manual apply (`.github/workflows/ci.yml` is a protected zone)

Closes the gap: **the Rust core (`rebuild/crates/domain`) runs in NO CI today.** `ci.yml` is
entirely Node/pnpm — nothing compiles, tests, or lint-checks the sovereign core. This wires in the
Phase-Zero Step-1 enforcement (Manifesto Hard Laws) as a required check.

Everything below is proven locally (2026-07-05, rustc/cargo 1.96.1):

- `bash rebuild/scripts/sovereign-gate.sh` → **exit 0**
  - Gate 1 wasm32 build: core links on `wasm32-unknown-unknown` (no OS/sockets/fs/threads/entropy).
  - Gate 2 disallowed-methods: no clock/entropy calls in production core.
- `cargo test --manifest-path rebuild/crates/domain/Cargo.toml` → **35 passed / 0 failed**.
- Red→green proof of the ban: injecting `SystemTime::now()` into `lib.rs` made Gate 2 fail with
  `error: use of a disallowed method 'std::time::SystemTime::now'`; reverting made it pass.
- The wasm gate caught a **real** latent violation on first run: the core's `uuid` dependency had
  the `v4` (entropy) feature enabled crate-wide. Fixed by moving `v4` to `[dev-dependencies]`
  (`rebuild/crates/domain/Cargo.toml`) — production core is now provably entropy-free; tests keep
  `new_v4` via feature-unification on the test target.

## Add a new top-level job to `.github/workflows/ci.yml` (sibling of `validate` / `fresh-provision`)

```yaml
  # Phase-Zero Step 1 — mechanical enforcement of the dowiz-core Hard Laws (Sovereign Core).
  # The core (rebuild/crates/domain) must stay pure math: no clock, no entropy, no IO. This job
  # is the machine-checkable definition of "sovereign" and is REQUIRED to merge. Scope is the core
  # crate ONLY — the `api` shell crate is allowed to touch the outside world and is not gated here.
  sovereign-core:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust toolchain + wasm target + clippy
        run: |
          rustup toolchain install stable --profile minimal --component clippy
          rustup target add wasm32-unknown-unknown
      - name: Sovereign-core gate (wasm32 build + disallowed clock/entropy methods)
        run: bash rebuild/scripts/sovereign-gate.sh
      - name: Core unit tests (transition matrix, money, error taxonomy)
        run: cargo test --manifest-path rebuild/crates/domain/Cargo.toml
```

## Notes

1. **Why a dedicated job, not a step in `validate`:** `validate` sets up Node/pnpm; the core needs
   the Rust toolchain + the `wasm32-unknown-unknown` target. Keeping it separate lets it run in
   parallel and fail independently with a clear signal.
2. **Rename-proof:** the gate references the core by manifest path (`rebuild/crates/domain/Cargo.toml`),
   not by package name, so it survives the Phase-Zero Step-2 `domain → dowiz-core` package rename
   with no CI edit.
3. **Make it required:** after the first green run, mark `sovereign-core` a required status check on
   the `main` branch protection rule so a purity regression cannot merge.
4. **Config scoping (do NOT move):** `rebuild/crates/domain/clippy.toml` must stay under the core
   crate dir, never at the workspace root — the `api` shell crate calls
   `Instant::now()`/`Uuid::new_v4()` 800+ times legitimately, and a root-level ban would break it.
   The gate loads the config via `CLIPPY_CONF_DIR` against `--lib` only.
