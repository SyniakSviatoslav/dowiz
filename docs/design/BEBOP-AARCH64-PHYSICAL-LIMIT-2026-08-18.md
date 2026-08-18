# Bebop — Absolute Physical Limit on AArch64 (engineering map)

Status: living engineering playbook. Last touched 2026-08-18.
Scope: Bebop native compiler (C bootstrap → self-host) and its runtime kernels.

The goal is to squeeze absolute physical maximum out of AArch64 silicon. Focus
shifts from pure code logic to the physics of the CPU: cache behaviour, pipeline
loading, memory-bus throughput.

## Hard law
Bebop must be faster than Rust (dowiz-core) EVERYWHERE. Any benchmark where
Bebop is slower than the Rust reference is a bug. Compile flags must match
Rust's release profile: `-O3 -flto -funroll-loops` (Rust: opt-level=3 +
lto=fat + codegen-units=1).

## 1. Memory subsystem & cache lines
- Alignment (64B): arenas, VSA vectors, PID states aligned to cache line.
- AoS → SoA for vector/hot loops (ntt, fft, hv_permute) — homogeneous NEON loads.
- Hardware prefetch PRFM (`__builtin_prefetch`) for sort/ntt large arrays.

## 2. Pipeline & ILP
- Software pipelining in dense loops (fft, mobius_reduce).
- Hardware predication CSEL/CCMP — kill branches (`(cond)?a:b` → CSEL).
- Branchless rotate `(x>>rot)|(x<<((64-rot)&63))` → ROR/EXTR (applied in rng.c).

## 3. NEON vectorization
- hv_permute via TBL/TBX / VSRI/VSLI (was 8× gap vs Rust).
- NTT/FFT butterflies via FMLA (1 cycle).
- hv_bind/hv_hamming already hand-NEON (veorq/vcntq/vaddvq).

## 4. Microarchitecture arsenal
- Non-temporal STNP/LDNP for streaming > L2/L3 arrays.
- DC ZVA hardware zeroing of cache lines for bulk buffers/arenas.
- Loop alignment .align 6 (64B) for hot loops.
- Port contention: interleave load/store/arithmetic across ports.

## 5. Codegen backend (bebopc)
- No spills: hot variables in X0–X30 / V0–V31.
- Custom internal calling convention (fixed callee-saved X19–X25 for hot ptrs).
- Macro-op fusion ordering.

## 6. SWAR for scalar paths (4×16-bit lanes in one GPR).

## 7. Aerospace / formal tier (future)
- SEU mitigation: software TMR + ECC arenas.
- Formal WCET via Lean 4.
- ARM MTE.
- Zero-downtime CoW self-healing.

## Priority decision rule
Answer the "which bottleneck next" question with reasoning tied to measured
gaps: cache/memory > NEON expansion > register allocation. Re-benchmark after
each change; let numbers pick the next target. Never optimize blind.

## Verification
- `make clean && make` (0 warnings).
- `make test` — 45 suites PASS.
- `./build/bebopc bench` vs `cargo run --release` (rust-bench) — Bebop ≥ Rust everywhere.
