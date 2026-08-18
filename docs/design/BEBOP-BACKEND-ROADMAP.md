# Bebop Backend v2-5 — Roadmap

> Status: WASM + AArch64 native live; x86_64/GPU/FPGA planned.

## v1 (live): AArch64 native
- Direct AArch64 encoding, W^X, PAC, CSEL branchless, svc #0 syscalls.
- Bump-pointer heap (x14), 512B frame, eval stack via sp.

## v2 (live): WASM
- `codegen_wasm` emits valid module (magic `\0asm`, version 1).
- Covered: bool, let, arithmetic, function bodies (`codegen_wasm_fn`).
- Verified by `bebopc codegen`.

## v3 (planned): x86_64 + AVX-512
- Encoding table mirrors AArch64 (hand-rolled opcodes).
- Register set: rax/rbx/rcx/rdx/rsi/rdi/r8-r15 (callee-saved subset).
- Syscall: `syscall` instruction (nr in rax).
- CSEL → cmov; PAC → Intel CET (endbr64/rdssp).
- AVX-512 for Hypervector (512-bit = Vector<8,u64>), `#[bit_identical]`.

## v4 (planned): GPU (Vulkan compute)
- Lower `Vector<W,T>` loops to SPIR-V compute shaders.
- `∥` parallel composition → single dispatch.
- Hypervector bundling/binding as parallel reduction.
- Memory: staging buffer → device-local, zero-copy where UMA.

## v5 (planned): FPGA/ASIC (Calyx/CIRCT)
- Bebop → MLIR → Calyx → Verilog → silicon.
- `△data` (dataflow) → systolic arrays.
- NTT `⟲⟳` → fixed-point FFT butterflies.
- Determinism contracts map to static timing.

## Cross-cutting
- All backends share the QTT kernel + proof erasure (QTT 0).
- `#[bit_identical]` proven per-backend by the compiler.
- Contracts → SMT is backend-independent (checks semantics, not codegen).
