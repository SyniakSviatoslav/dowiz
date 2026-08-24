# Bebop as a Hypervector-first runtime

Target state: every subsystem either CONSUMES hypervector primitives
(1024-bit VSA, hyper.c), PRODUCES them (encoders), or VERIFIES them
(self-hosted .bp VSA kernels). Nothing stays isolated.

## Layers

    L4  applications      hv_stream anomaly watch | PQC radio (NTT+HV keys)
                          associative memory | glyph-HUD status glyphs
    L3  self-hosted VSA   k6.bp bind+popcount (pure .bp, native AArch64)
                          -> eventual hv_* builtins in the language itself
    L2  services          bundle/permute pipelines, similarity search,
                          shift-invariant matching (NTT cross-correlation)
    L1  encoders          adc.c -> sensor HVs | glyph.c -> visual HVs
                          morse.c -> timing HVs | text tokens -> lexical HVs
    L0  core              hyper.c: code/bind/bundle/permute/hamming
                          + NEON eor/cnt fast paths + W^X-coherent JIT exec

## Wiring map (existing -> target)

| module        | now                    | becomes                        |
|---------------|------------------------|--------------------------------|
| adc.c         | raw MMIO reads         | L1 encoder feeding hv_stream   |
| hyper.c       | L0 + bench             | L0 + NEON default paths        |
| ntt.c         | standalone math        | L2 shift-invariant similarity; |
|               |                        | PQC radio partner of L0        |
| glyph.c       | terminal renders       | L1 visual encoder (7x7 bitmap  |
|               |                        | -> 49-bit seed -> HV)          |
| memristor.c   | crossbar sim           | L1 array-state HVs (NEON only) |
| tensor.c      | numeric tensors        | L2 batch ops over word-slices  |
| spectral.c    | FFT analysis           | L2 spectral features -> HV     |
| exec_words    | runs scalar kernels    | runs VSA kernels (k6+)         |
| compilewords  | caches artifacts       | caches VSA kernels too         |

## Rules

1. Any new telemetry/signal source lands WITH its L1 encoder.
2. Similarity questions never compare raw scalars when an HV distance
   exists; thresholds come from calibration windows (hv_stream pattern).
3. The .bp compiler must track C capability parity for L3: xor/popcount/
   permute kernels stay in bench/vs_rust/kernels until native-correct,
   then graduate into std/.
4. NEON variants are the default on aarch64; scalar fallbacks exist only
   for verification diffs.

## Open defects blocking L3 (tracked in BUGFIXES.md)
- native `^` wrong for words > 2^31 (repro /tmp/opencode/k6x.bp:
  interp=711315 native=551361)
- sc-class stale-index patchwriter (0xa9ff03e0 mid-stream)

## Status
- L0 done (hyper.c, NEON, W^X clean).
- L2/L4 first services live: hv_stream (anomaly), ntt_filter (exact cyclic
  FIR). Both in make test sweep.
- L3 first kernel written (k6.bp); blocked by the two defects above.
