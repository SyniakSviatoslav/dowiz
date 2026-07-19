# Kernel Hardening Checklist — Standing Law

- **Status:** MANDATORY, CI-enforced (`hardening-gate` job, `scripts/hardening-gate.sh`).
- **Source:** `SPACE-GRADE-KERNEL-ARCHITECTURE-SYNTHESIS-2026-07-19.md` §4, with the §10/P7, P5(b),
  P6 corrections built in. Blueprint: `BLUEPRINT-ITEM-06-hardening-checklist-ci-2026-07-19.md`.
- **Machine-readable manifest:** [`HOT-PATHS.tsv`](./HOT-PATHS.tsv) — the gate reads this, not prose.

## The designation rule

Any **new or changed algorithmic hot path** must satisfy the four items below. "Algorithmic hot
path" is **crypto or not** — graph algorithms, scheduler math, GCRA arithmetic, hand-rolled parsers
and escapers all qualify. The designated zones are the `@ZONE` lines in `HOT-PATHS.tsv`. A diff that
touches a file under a zone with **no manifest row** fails CI: new algorithmic code must register
itself here before it can merge.

## The four items

1. **An oracle.** Exhaustive where the input space permits (the 12-state FSM permits it); otherwise a
   large randomized corpus differentially checked against a simple reference implementation, with the
   reference retained forever as a test-only crate-internal module (or, where a live reference was
   removed for a stated reason — item-5's `regex`, dropped to reach zero-dep — a vendored test corpus,
   recorded as such in the manifest row, not pretended into the strong form).
2. **A dudect-style gate** where secret-dependent timing is conceivable — including a **planted-leak
   self-test proving the gate itself works** (`src/ct_gate.rs`, per bebop's `ntt_ct_gate` standard).
   CI-time harness, not linked into release.
3. **Debug-mode differential cross-check** (`debug_assert!`/`debug_assert_eq!` against a per-call
   oracle) compiled out of release — continuous verification at zero production cost. Applies **only
   where a callable per-call reference exists**; for corpus-only oracles there is no per-call
   reference, and the manifest records `N/A(corpus-oracle)` rather than faking it.
4. **A binary/assembly spot-check on every compiler version bump** for branch-free paths
   (arXiv:2410.13489 incident class). PMU counters remain optional/experimental.

## The §10/P7 correction — re-execute, never presence-check (load-bearing)

Presence-checking is self-certifiable: an empty oracle file "passes." The gate therefore **never**
decides GREEN by reading a report/artifact file. Every verdict comes from a live process exit code
plus **parsed live test counts** in the CI run itself:

- **Named-filter re-execution with a minimum-count assertion.** `cargo test <filter>` with a filter
  matching **zero** tests exits 0 — bare exit-code checking is a presence-check one level down. Every
  manifest row carries `min_tests`; the gate parses cargo's `N passed` and asserts `N >= min_tests`.
  **A filter matching zero tests is RED.** This is the anti-forgery core.
- **`min_tests` is a floor.** Adding tests only raises the count (always safe). Removing/renaming an
  oracle test below the floor goes RED — lowering a floor requires a deliberate, visible manifest edit.
- **Self-tests run in the same invocation.** The dudect gate runs *including* its planted-leak
  self-test (`ct_gate ... -- --ignored`, release): a deliberately leaky comparator must be rejected by
  the same Welch-t machinery, or the step is RED.
- **The gate has a proven RED path.** Before first merge the executor demonstrated (a) a diff touching
  a zone with no manifest row → exit 1, and (b) a manifest row whose filter matches zero tests → exit
  1, plus (c) a clean touch → exit 0. See the blueprint §2.4 and the item-6 commit body.
- **Determinism (P6).** Every cargo invocation is `--locked --offline`; the gate asserts the
  `Cargo.lock` hash is unchanged after the run.
- **One sanctioned presence-check exception:** `toolchain-bump-gate` checks for the *presence* of the
  human assembly-audit artifact (`docs/audits/toolchain/spot-check-<ver>.md`) — an assembly audit is a
  human judgment record, not re-executable by grep. That is acceptable there and **only** there;
  item 7 (Kani) is what upgrades checklist item 4 to a deterministic re-executed check.

## Honest gaps (ledgered, visible in the manifest's own `gap` column)

- **item 2 (dudect):** harness now EXISTS (`ct_gate.rs`) and is wired to `ct_eq` as proof-of-mechanism.
  Coverage of the crypto surface is item 7/8 work.
- **`kem.rs` / `hybrid.rs` variable-time tag compares** (`KNOWN-RED(P91.2)`): pre-existing, compiler-
  independent; a dudect gate over them would honestly go RED. Do **not** silently fix crypto in item 6
  — the CT fix (adopting `ct_eq`) is the gate's first customer, passing through the checklist it triggered.
- **item 3 (debug cross-check):** wired for `order_machine` (FSM_ADJ dual-representation) and
  `householder::eig2x2` (Vieta trace/det). Corpus-oracle rows carry `N/A(corpus-oracle)`.
- **`token_bucket` GCRA differential oracle** (`MISSING(item-8)`): item 6 only *designates* the hot
  path so item 8's atomic-GCRA swap cannot merge without the mutex-vs-atomic parity oracle.
- **ML-KEM-768 ACVP official vectors** (`MISSING(P91.2)`): self-documented deferral (`kem.rs:5`).
