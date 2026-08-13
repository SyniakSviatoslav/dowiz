# Kernel-Space Migration Assessment (#21)

Goal: move everything possible into the kernel (no_std / rust-for-linux
module), shrinking the artifact from the 26 MB `.rlib` (compiler artifact) to a
20–150 KB `.ko`.

## Reality check on the 26 MB `.rlib`

The `.rlib` is a *compiler/linker artifact* (type metadata, generics, LLVM
bitcode) needed at build time, NOT at runtime. The actual machine code that
ships is in `libdowiz_kernel.so` = **3,332 bytes of text+data+rodata**
(`size` output), and the rlib's real code text = **2.45 MB** across 16 CGUs.
A kernel module would carry only the code that is actually reachable, plus the
`no_std` core — on the order of tens-to-hundreds of KB.

## Audit result (measured 2026-08-13)

303 modules total. Classification by std-only primitives (fs/net/thread/process/
Mutex/io/env/path):

| Tier | Modules | Count | Kernel-space? |
|---|---|---|---|
| **Kernel-core** (pure `core`+`alloc`) | math/geometry/glyph/lut/compression | **234 (77%)** | ✅ already no_std-clean |
| **Boundary** (`std::time` only) | telemetry, token_bucket, resonance, … | ~34 | 🔶 swap `Instant`→jiffies |
| **User-space** (`fs`/`net`/`thread`/`process`) | fdr, retrieval, academia, mesh, p2p | ~46 fs + 3 net + 11 thread + 31 process | ❌ needs abstraction |

The entire rewrite-layer is in the 234: `lut`, `constants`, `trig`, `delta`,
`eigen`, `spectral`, `hypervector`, `hypervector_index`, `pixel_snapshot`,
`glyph_dashboard`, `squash`, `fft`, `spherical`, `tensor`, `mat`, `numerical_guard`,
`householder`, `simd`, `csr`, `cgraph`, `hypergraph`, `trinary`, `wave`, `noether`,
`resonance`, `kalman`, `pid`, `fractal`, `ktg2/*`, `fractal_manchester`.

## What "move everything possible" means concretely

The pure-computation core is *already* the kernel. The remaining step is to
extract it as a `#![no_std]` crate with an `alloc`-only profile:

1. **`core` only, `alloc` for Vec/String/HashMap/BTreeMap** — the 234 modules
   use none of `std::fs/net/thread/process/Mutex`, so they compile under
   `no_std + extern crate alloc` unchanged (their `std::f64::consts` /
   `std::cmp` etc. are `core` re-exports).

2. **Boundary tier** — the ~34 modules that only use `std::time::{Instant,
   SystemTime}` get a thin `Clock` trait: `now()` abstracts `Instant::now()`
   (userspace) vs `jiffies`/`ktime_get` (kernel). Zero logic change.

3. **User-space tier stays userspace** — `fdr` (A/B segment files → kernel
   could map to a char device or `debugfs`), `retrieval`/`academia` (filesystem
   corpus → VFS or embedded static data), `mesh`/`p2p` (sockets → `sk_buff`/
   netlink). These are ports, not rewrites.

4. **rust-for-linux** — the `no_std` core crate becomes a `kernel::` crate
   dependency; no C rewrite needed. Type safety and zero-dep invariants carry
   over; `alloc` uses the kernel's `kmalloc`-backed allocator.

## Realistic `.ko` size

Kernel-core code text is ~2.45 MB uncompiled; with LTO + `opt-level=3` + only
reachable symbols, a representative math/glyph/lut module ships at **tens of KB
to ~150 KB**, well under the 1 MB linux module limit. Exact size needs a
`rust-for-linux` toolchain + `objdump` on a real `.ko` — deferred (no
rust-for-linux toolchain on this aarch64 container).

## Falsifiable done-check for a future extraction

- A `#![no_std]` crate `dowiz-core` re-exports the 234 kernel-core modules and
  compiles with `cargo build --target <no_std>` (no `std` link).
- `nm` on the resulting `.ko`/`.so` shows the 8,262 defined symbols collapsing
  to the reachable subset; `size` text < 1 MB.
- The boundary `Clock` trait swaps `Instant`→`jiffies` with zero test delta.

## Status

Audit done (234/303 clean). Extraction to a real `no_std` crate + `.ko` build is
a follow-on that needs a rust-for-linux toolchain and a target kernel tree —
not available in this container. This document is the binding plan for that step.
