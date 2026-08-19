#!/usr/bin/env bash
set -euo pipefail
NATIVE=/root/dowiz/bebop-lang/native
COV=/root/dowiz/bebop-lang/bench/r5/cov
mkdir -p "$COV"
cd "$COV"
# Rebuild source list with absolute paths (78 files from SRC := in Makefile)
SRCS="src/main.c src/glyph.c src/calyx.c src/memristor.c src/adc.c src/power_telemetry.c src/compute.c src/lexer.c src/parser.c src/morse.c src/mesh.c src/qtt.c src/ntt.c src/ntt32.c src/hyper.c src/mem.c src/lmem.c src/hydra.c src/expr.c src/verify.c src/verifier.c src/vsa.c src/codegen.c src/graph.c src/chain.c src/native.c src/money.c src/fft.c src/arena.c src/event.c src/complex.c src/modular.c src/sort.c src/token_bucket.c src/checksum.c src/hex_util.c src/trig.c src/rng.c src/stats.c src/pid.c src/markov.c src/math_native.c src/spectral.c src/autonomic.c src/typereg.c src/atomic.c src/bench_all.c src/vir.c src/vir_umulh2.c src/theorem.c src/pac.c src/effect.c src/jittable.c src/supervise.c src/session.c src/syscall.c src/typereflect.c src/termination.c src/smt.c src/contract.c src/comptime.c src/fmt.c src/power.c src/x86_64.c src/gt.c src/gt_switch.S src/noether.c src/oracle.c src/tensor.c src/pool.c src/scale.c src/startup.c src/pq.c src/zlib.c src/x25519.c src/sha256.c src/aes_gcm.c src/tls.c"
ABSSRCS=""
for f in $SRCS; do ABSSRCS="$ABSSRCS $NATIVE/$f"; done
cc --coverage -O0 -g -std=c11 -Wall -Wextra -Wpedantic -Werror -Wshadow -Wstrict-prototypes -Wmissing-prototypes -Wundef -Wformat=2 -o bebopc $ABSSRCS -lm -lpthread
echo "BUILD_OK"
ls -la bebopc
ls *.gcno | wc -l
