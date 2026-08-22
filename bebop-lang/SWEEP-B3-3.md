# SWEEP B3-3 — Bebop Selfhost Sweep (strict + check)

- Date: 2026-08-22 (UTC)
- Binary: `../native/build/bebopc` (aarch64, via /lib/ld-linux-aarch64.so.1), run from /root/dowiz/bebop-lang/selfhost
- Scope: selfhost top-level *.bp + std/*.bp (extra/ contains no .bp files)
- Protocol: `bebopc strict FILE` (timeout 60s); if strict passes → `bebopc check FILE` (timeout 90s). Exit 0 = PASS. CODE VERIFICATION ONLY — no source files modified.

## Summary

- Total .bp files: 137 (64 top-level + 73 std + 0 extra)
- Strict pass: 137/137
- Check pass: 137/137
- Failing files: **none**

## Results (top-level)

| file | strict | check | notes |
|------|--------|-------|-------|
| aarch64.bp | PASS | PASS |  |
| adt.bp | PASS | PASS |  |
| alpha.bp | PASS | PASS |  |
| ast.bp | PASS | PASS |  |
| bebopc.bp | PASS | PASS |  |
| branchless.bp | PASS | PASS |  |
| closure.bp | PASS | PASS |  |
| codegen.bp | PASS | PASS |  |
| compile.bp | PASS | PASS |  |
| compile_pipeline.bp | PASS | PASS |  |
| compiler_main.bp | PASS | PASS |  |
| conv.bp | PASS | PASS |  |
| count-l.bp | PASS | PASS |  |
| dependent.bp | PASS | PASS |  |
| djb2.bp | PASS | PASS |  |
| driver.bp | PASS | PASS |  |
| emitter.bp | PASS | PASS |  |
| enum.bp | PASS | PASS |  |
| enum_parse.bp | PASS | PASS |  |
| error_report.bp | PASS | PASS |  |
| eval.bp | PASS | PASS |  |
| expr_compile.bp | PASS | PASS |  |
| expr_parser.bp | PASS | PASS |  |
| field_access.bp | PASS | PASS |  |
| fold.bp | PASS | PASS |  |
| gcd.bp | PASS | PASS |  |
| generics.bp | PASS | PASS |  |
| glyph.bp | PASS | PASS |  |
| hof.bp | PASS | PASS |  |
| infer.bp | PASS | PASS |  |
| ir.bp | PASS | PASS |  |
| lexer.bp | PASS | PASS |  |
| match_eval.bp | PASS | PASS |  |
| name_pool.bp | PASS | PASS |  |
| nat_peano.bp | PASS | PASS |  |
| norm.bp | PASS | PASS |  |
| parse_struct.bp | PASS | PASS |  |
| parser.bp | PASS | PASS |  |
| pattern.bp | PASS | PASS |  |
| precedence.bp | PASS | PASS |  |
| proof.bp | PASS | PASS |  |
| qtt_kernel.bp | PASS | PASS |  |
| qtt_types.bp | PASS | PASS |  |
| quantities.bp | PASS | PASS |  |
| reduce.bp | PASS | PASS |  |
| registry.bp | PASS | PASS |  |
| selftest.bp | PASS | PASS |  |
| selftest_exec.bp | PASS | PASS |  |
| sema.bp | PASS | PASS |  |
| source_map.bp | PASS | PASS |  |
| string_build.bp | PASS | PASS |  |
| string_ops.bp | PASS | PASS |  |
| struct.bp | PASS | PASS |  |
| struct_parse.bp | PASS | PASS |  |
| strutil2.bp | PASS | PASS |  |
| subst.bp | PASS | PASS |  |
| symtab.bp | PASS | PASS |  |
| token.bp | PASS | PASS |  |
| tokenize.bp | PASS | PASS |  |
| type_eq.bp | PASS | PASS |  |
| type_registry.bp | PASS | PASS |  |
| typecheck.bp | PASS | PASS |  |
| universes.bp | PASS | PASS |  |
| wasm.bp | PASS | PASS |  |

## Results (std/)

| file | strict | check | notes |
|------|--------|-------|-------|
| std/automaton.bp | PASS | PASS |  |
| std/base64.bp | PASS | PASS |  |
| std/bignum.bp | PASS | PASS |  |
| std/bitfield.bp | PASS | PASS |  |
| std/bits.bp | PASS | PASS |  |
| std/bitset.bp | PASS | PASS |  |
| std/blas.bp | PASS | PASS |  |
| std/checksum.bp | PASS | PASS |  |
| std/color.bp | PASS | PASS |  |
| std/combinatorics.bp | PASS | PASS |  |
| std/complexf.bp | PASS | PASS |  |
| std/crc.bp | PASS | PASS |  |
| std/date.bp | PASS | PASS |  |
| std/decimal.bp | PASS | PASS |  |
| std/dist.bp | PASS | PASS |  |
| std/dp.bp | PASS | PASS |  |
| std/effect.bp | PASS | PASS |  |
| std/encoding.bp | PASS | PASS |  |
| std/event.bp | PASS | PASS |  |
| std/fft.bp | PASS | PASS |  |
| std/fmath.bp | PASS | PASS |  |
| std/fmath_trig.bp | PASS | PASS |  |
| std/fmt.bp | PASS | PASS |  |
| std/gcra.bp | PASS | PASS |  |
| std/geometry.bp | PASS | PASS |  |
| std/geometry3d.bp | PASS | PASS |  |
| std/graph.bp | PASS | PASS |  |
| std/hash.bp | PASS | PASS |  |
| std/heap.bp | PASS | PASS |  |
| std/hex.bp | PASS | PASS |  |
| std/integrate.bp | PASS | PASS |  |
| std/interpolate.bp | PASS | PASS |  |
| std/interval.bp | PASS | PASS |  |
| std/list.bp | PASS | PASS |  |
| std/log.bp | PASS | PASS |  |
| std/markov.bp | PASS | PASS |  |
| std/math.bp | PASS | PASS |  |
| std/matrix.bp | PASS | PASS |  |
| std/modular.bp | PASS | PASS |  |
| std/money.bp | PASS | PASS |  |
| std/morse.bp | PASS | PASS |  |
| std/nat.bp | PASS | PASS |  |
| std/ntt.bp | PASS | PASS |  |
| std/numeric.bp | PASS | PASS |  |
| std/ops.bp | PASS | PASS |  |
| std/pac.bp | PASS | PASS |  |
| std/permutation.bp | PASS | PASS |  |
| std/pid.bp | PASS | PASS |  |
| std/polynomial.bp | PASS | PASS |  |
| std/primes.bp | PASS | PASS |  |
| std/queue.bp | PASS | PASS |  |
| std/quicksort.bp | PASS | PASS |  |
| std/radix.bp | PASS | PASS |  |
| std/ratelimit.bp | PASS | PASS |  |
| std/ring.bp | PASS | PASS |  |
| std/rle.bp | PASS | PASS |  |
| std/rng.bp | PASS | PASS |  |
| std/roman.bp | PASS | PASS |  |
| std/search.bp | PASS | PASS |  |
| std/session.bp | PASS | PASS |  |
| std/set.bp | PASS | PASS |  |
| std/sort.bp | PASS | PASS |  |
| std/stack.bp | PASS | PASS |  |
| std/statistics.bp | PASS | PASS |  |
| std/stats.bp | PASS | PASS |  |
| std/string.bp | PASS | PASS |  |
| std/strutil.bp | PASS | PASS |  |
| std/tensor.bp | PASS | PASS |  |
| std/token_bucket.bp | PASS | PASS |  |
| std/units.bp | PASS | PASS |  |
| std/uuid.bp | PASS | PASS |  |
| std/vec.bp | PASS | PASS |  |
| std/version.bp | PASS | PASS |  |
