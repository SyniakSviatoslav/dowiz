# Toolchain spot-check — Rust 1.96.1

- **Date:** 2026-07-19
- **Auditor:** Opus executor (roadmap item 14), autonomous session.
- **Previous → new version:** `<absent>` → `1.96.1` (this is the *baseline* artifact — the
  introduction of `rust-toolchain.toml` itself, which the `toolchain-bump-gate` treats as a bump
  `<absent> → 1.96.1` and therefore requires to carry this file).
- **Compiler audited:** `rustc 1.96.1 (31fca3adb 2026-06-26)` / `cargo 1.96.1 (356927216 2026-06-26)`
  — the exact version now pinned in `/rust-toolchain.toml`, and the version already installed on the
  dev box, so this pin is behaviourally a no-op for the current build (verified below).

> **Honesty preface (read this first).** The *source-level* constant-time review below is real work
> that was actually performed against the live files. The *assembly-level* audit that synthesis §4
> ultimately calls for (disassemble every secret-dependent path under the new compiler and prove no
> secret-dependent branch/memory-access was introduced by optimisation — the "Breaking Bad"
> arXiv:2410.13489 failure mode) is **only partially performed here** and is **explicitly deferred**
> for its exhaustive, taint-tracked form to **Tier 2 item 7 (Kani wiring)**. This artifact does not
> claim "audited, clean" at the instruction level. It records exactly what was and was not checked,
> so the audit *series* starts honest. A future compiler bump inherits this same shape and must
> upgrade the assembly section to a full per-branch analysis once item 7 lands the tooling.

## Assembly spot-check

Per synthesis §4 (post-compiler-bump binary audit; `-O` has been shown to break constant-time code
in Kyber/HQC reference implementations, arXiv:2410.13489): the intended check is, for each
branch-free / secret-dependent path, to disassemble under the NEW compiler and confirm no
secret-dependent branch or memory access appeared. Paths and findings (paths **verified to exist**
at audit time — none had moved):

### Surface inventory (verified live)

| Surface | File | Nature |
|---|---|---|
| ML-DSA-65 NTT / ring arithmetic | `kernel/src/pq/dsa.rs` | hand-rolled, branch-free arithmetic |
| ML-KEM-768 FO implicit-rejection tag-compare | `kernel/src/pq/kem.rs` | hand-rolled re-encryption consistency check |
| Keccak-f[1600] (copy A) | `kernel/src/pq/keccak.rs` | hand-rolled permutation |
| Keccak-f[1600] (copy B, until item 25 dedup) | `kernel/src/event_log.rs` (`fn keccak_f`, ~L67) | hand-rolled permutation |
| x25519 scalar-mult ladder | `kernel/src/pq/x25519.rs` | **delegated** to `curve25519-dalek` |
| hybrid combine / confirm-tag compare | `kernel/src/pq/hybrid.rs` | hand-rolled `combine()` + tag check |

### Source-level constant-time findings (real, performed)

- **`dsa.rs` — NTT / reduction: branch-free, as intended.** `montgomery_reduce`, `reduce32`,
  `poly_pointwise_montgomery` and the `ntt`/`intt` butterflies are straight-line integer arithmetic
  over fixed loop bounds (`0..N`, `0..24`-style), with **no data-dependent branch**. The conditional
  add `caddq(a) = a + ((a >> 31) & Q)` is the textbook constant-time masked-add (arithmetic-shift
  sign mask, no branch). ✅ at source level.
  - **Caveat (real, not a regression):** `poly_chknorm` (dsa.rs ~L146) contains an early-return
    `if t >= b { return true; }`. This is the reference Dilithium rejection-sampling norm check; it
    is *intentionally* variable-time in the reference and leaks only "did the rejection loop restart",
    not secret-key coefficients. Flagged for completeness; unchanged by the compiler.
- **`kem.rs` — FO tag-compare is NOT constant-time (pre-existing, self-documented).** `decaps_internal`
  (kem.rs ~L468) does `if c_prime != c_fixed { return implicit_reject }` — a plain `!=` byte-slice
  compare, which **short-circuits at the first differing byte** and is therefore variable-time. The
  code itself admits this: *"Data-independent enough for the red-line gate here; … P91.2 will tighten
  this once the ACVP gate is in place."* This is a genuine constant-time gap that **predates and is
  independent of the compiler** — the 1.96.1 pin neither introduces nor cures it. Owed to P91.2, not
  item 14. Recorded here so it is on the audit ledger, not hidden.
- **`hybrid.rs` — same class.** `hybrid_decaps` (hybrid.rs ~L101) does `if tag != ct.confirm` — again
  a variable-time `!=` on the confirmation tag. `combine()` itself is SHAKE256 over a fixed-length
  concat (constant-time). Same pre-existing, compiler-independent gap as `kem.rs`.
- **`keccak.rs` / `event_log.rs` — permutations are branch-free by construction.** Both `keccak_f`
  copies are θ/ρ/χ/ι over fixed 24 rounds and fixed lane counts with only bitwise ops (`^`, `!`, `&`,
  `rotl`); control flow depends solely on the public round/lane indices, never on message or key
  bytes. ✅ at source level. (The two copies are algorithmically identical; dedup is item 25.)
- **`x25519.rs` — delegated to an audited crate.** Scalar multiplication is
  `MontgomeryPoint(u).mul_clamped(k)` from `curve25519-dalek`, an externally-audited constant-time
  implementation. The constant-time property here is inherited from a vetted dependency, not
  hand-rolled; the compiler bump does not change which crate is linked.

### Assembly-level check actually run (partial)

- Built `kernel` in `--release` under 1.96.1 (`cargo build --release --offline`, exit 0). The
  branch-free arithmetic helpers (`montgomery_reduce`, `caddq`, `reduce32`, `keccak_f`, `ntt`) carry
  `#[inline]` or are small `fn`s and were **inlined away** — they have no standalone symbol in
  `libdowiz_kernel.rlib` (`nm`/`objdump` find no such symbol), which is itself consistent with them
  compiling to inlined straight-line arithmetic rather than out-of-line branchy code.
- One reachable symbol was disassembled as a spot demonstration of the method:
  `dowiz_kernel::event_log::sha3_256` (`objdump -d --disassemble=<sym>`). It contains 13 jump
  instructions; these are consistent with the source's fixed-count loop control (24-round permutation
  + fixed-lane absorb/squeeze) — i.e. loop back-edges on public counters, **none scaling with input
  length**. This is a spot check, not a taint-tracked proof that every branch's condition is
  loop-control-only.

### Assembly-audit verdict

**Partial / method-demonstrated, not exhaustive.** No secret-dependent branch was *observed* in the
one function disassembled, and the branch-free surfaces inlined as expected. A full adversarial
per-branch taint analysis across all six surfaces under optimisation — the rigorous form synthesis §4
asks for — was **not** performed here and is **deferred to Tier 2 item 7 (Kani wiring)**, which
brings the tooling to make it deterministic rather than a manual objdump read. The two variable-time
`!=` compares in `kem.rs`/`hybrid.rs` are pre-existing source-level gaps owed to P91.2 (ACVP), wholly
independent of this compiler pin.

## Full-suite re-run

The pin is exact-equal to the installed toolchain, so this baseline artifact's re-run is on the dev
box under `rustc 1.96.1` (identical to what CI will use once the pin lands):

- `cd kernel && cargo test --offline` → **exit 0**; primary binary **902 passed, 0 failed, 3 ignored**;
  all additional test binaries green (0 failures across the suite).
- `cd engine && cargo test --offline` → **exit 0**; **117 passed, 0 failed** (plus 4 further green
  binaries, 0 failures).
- `cd kernel && cargo build --release --offline` → **exit 0** (8 pre-existing style warnings only,
  no errors; warnings are unrelated `unnecessary parentheses` lints, not introduced by item 14).

**CI run URL:** _pending — this baseline was proven locally under the pin's own version (1.96.1). Once
`toolchain-bump-gate` and the pin land on `origin/exec/space-grade-tier0-2026-07-19`, the existing
`cargo-test` / `eqc-proofs` jobs re-execute under this exact toolchain on the runner; that push's
Actions run is the end-to-end `<absent> → 1.96.1` green proof of the bump-detected path. Record its
URL here on first CI execution._

---

### Gate self-test evidence (item 14 §G.8 proof)

The `toolchain-bump-gate` comparison logic was extracted and unit-tested locally against simulated
before/after `channel` values (GitHub Actions was not run locally). Results recorded in the commit
message / `docs/design/BLUEPRINT-ITEM-14-toolchain-pin-2026-07-19.md §3.5`:

- **non-bump diff** (`1.96.1` == `1.96.1`) → `no toolchain bump … gate vacuously green`, exit 0.
- **bump without artifact** (`1.96.1` → `1.96.2`, no `spot-check-1.96.2.md`) → `::error::bump to
  1.96.2 without docs/audits/toolchain/spot-check-1.96.2.md`, **exit 1 (RED — gate bites).**
- **bump with artifact** (add `spot-check-1.96.2.md` with both required headings) → exit 0 (GREEN).
- **absent→present** (`<absent>` base, `1.96.1` head, this file present) → exit 0 (this commit's own
  path).
