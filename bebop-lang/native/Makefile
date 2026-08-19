# Bebop native compiler — Phase 1 (C bootstrap)
# Native, not Rust. Zero external dependencies (libc only). Self-hosts later.
CC      ?= cc
CFLAGS  ?= -O3 -flto -std=c11 -Wall -Wextra -Wpedantic
BUILD   ?= build
BIN     := $(BUILD)/bebopc

SRC := src/main.c src/glyph.c src/calyx.c src/memristor.c src/adc.c src/compute.c src/lexer.c src/parser.c src/morse.c src/mesh.c src/qtt.c src/ntt.c src/ntt32.c src/hyper.c src/mem.c src/lmem.c src/hydra.c src/expr.c src/verify.c src/verifier.c src/vsa.c src/codegen.c src/graph.c src/chain.c src/native.c src/money.c src/fft.c src/arena.c src/event.c src/complex.c src/modular.c src/sort.c src/token_bucket.c src/checksum.c src/hex_util.c src/trig.c src/rng.c src/stats.c src/pid.c src/markov.c src/math_native.c src/spectral.c src/autonomic.c src/typereg.c src/atomic.c src/bench_all.c src/vir.c src/vir_umulh2.c src/theorem.c src/pac.c src/effect.c src/jittable.c src/supervise.c src/session.c src/syscall.c src/typereflect.c src/termination.c src/smt.c src/contract.c src/comptime.c src/fmt.c src/power.c src/x86_64.c src/gt.c src/gt_switch.S src/noether.c src/oracle.c src/tensor.c src/pool.c src/pq.c src/zlib.c src/x25519.c src/sha256.c src/aes_gcm.c
LDLIBS := -lm -lpthread

all: $(BIN)

$(BIN): $(SRC) src/glyph.h src/calyx.h src/arena.h src/memristor.h src/adc.h src/compute.h src/lexer.h src/parser.h src/morse.h src/mesh.h src/qtt.h src/ntt.h src/ntt32.h src/hyper.h src/mem.h src/lmem.h src/hydra.h src/expr.h src/verify.h src/verifier.h src/vsa.h src/codegen.h src/graph.h src/chain.h src/native.h src/money.h src/fft.h src/arena.h src/event.h src/complex.h src/modular.h src/sort.h src/token_bucket.h src/checksum.h src/hex_util.h src/trig.h src/rng.h src/stats.h src/pid.h src/markov.h src/math_native.h src/spectral.h src/autonomic.h src/typereg.h src/atomic.h src/bench_all.h src/vir.h src/theorem.h src/pac.h src/effect.h src/jittable.h src/supervise.h src/session.h src/syscall.h src/typereflect.h src/smt.h src/gt.h src/noether.h src/oracle.h src/tensor.h src/pool.h src/pq.h src/zlib.h src/sha256.h src/x25519.h src/aes_gcm.h
	@mkdir -p $(BUILD)
	$(CC) $(CFLAGS) -o $@ $(SRC) $(LDLIBS)

test: $(BIN)
	$(BIN) version
	$(BIN) glyphs
	$(BIN) glyph fn
	$(BIN) glyph struct
	@echo "--- tokenize sample ---"
	$(BIN) tokens ../samples/hello.bp
	@echo "--- parse sample ---"
	$(BIN) parse ../samples/hello.bp
	@echo "--- fmt round-trip (idempotent) ---"
	$(BIN) fmt ../samples/hello.bp
	@echo "--- morse codec (utility) ---"
	$(BIN) morse "ntt"
	$(BIN) unmorse "... --- ..."
	$(BIN) unmorse "$$($(BIN) morse 'quantum hypervector')"
	@echo "--- qtt semiring ---"
	$(BIN) qtt
	@echo "--- ntt ---"
	$(BIN) ntt
	@echo "--- hypervector ---"
	$(BIN) hyper
	@echo "--- memory ---"
	$(BIN) mem
	@echo "--- expr (parse→typecheck→eval) ---"
	$(BIN) expr "1 + 2 * 3"
	$(BIN) expr "let y = 2 in y * y"
	$(BIN) expr "if (1 == 1) then 10 else 20"
	$(BIN) expr '(\x:i64. x + 1)(41)'
	@echo "--- verify (bounded contracts) ---"
	$(BIN) verify
	@echo "--- vsa identifiers ---"
	$(BIN) vsa
	@echo "--- codegen (wasm) ---"
	$(BIN) codegen
	@echo "--- native (aarch64 JIT) ---"
	$(BIN) native
	@echo "--- money (integer, overflow-safe) ---"
	$(BIN) money
	@echo "--- fft (radix-2, float) ---"
	$(BIN) fft
	@echo "--- arena (bump allocator) ---"
	$(BIN) arena
	@echo "--- event (C3 event-sourced) ---"
	$(BIN) event
	@echo "--- modular (PSL(2,Z) Möbius) ---"
	$(BIN) modular
	@echo "--- sort (f64 key) ---"
	$(BIN) sort
	@echo "--- token bucket (GCRA) ---"
	$(BIN) token
	@echo "--- checksum (FNV-1a) ---"
	$(BIN) checksum
	@echo "--- hex util ---"
	$(BIN) hex
	@echo "--- trig ---"
	$(BIN) trig
	@echo "--- rng ---"
	$(BIN) rng
	@echo "--- stats ---"
	$(BIN) stats
	@echo "--- pid ---"
	$(BIN) pid
	@echo "--- markov ---"
	$(BIN) markov
	@echo "--- check (elaboration) ---"
	$(BIN) check ../samples/hello.bp
	@echo "--- atomic (lock-free) ---"
	$(BIN) atomic
	@echo "--- proof kernel (conversion) ---"
	$(BIN) conv
	@echo "--- proof kernel (equality / refl) ---"
	$(BIN) proof
	@echo "--- theorem surface (.bp → kernel) ---"
	$(BIN) check ../samples/theorem-sample.bp
	@if $(BIN) check ../samples/theorem-false.bp 2>/dev/null; then echo "FAIL: false theorem was accepted"; exit 1; else echo "theorem rejection: ok"; fi
	@echo "--- proof kernel (Nat / recursor) ---"
	$(BIN) nat
	@echo "--- QTT strings (check/conv/prove) ---"
	$(BIN) str
	@echo "--- arrays (literal/index) ---"
	$(BIN) array
	@echo "--- universes (cumulative) ---"
	$(BIN) universe
	@echo "--- termination (structural) ---"
	$(BIN) termination
	@echo "--- VIR (NEON lowering) ---"
	$(BIN) vir
	@echo "--- PAC (pointer auth) ---"
	$(BIN) pac
	@echo "--- effect (pure/io) ---"
	$(BIN) effect
	@echo "--- JIT table (atomic swap) ---"
	$(BIN) jittable
	@echo "--- supervision tree (CoW rollback) ---"
	$(BIN) supervise
	@echo "--- session types (duality) ---"
	$(BIN) session
	@echo "--- raw syscalls (no libc) ---"
	$(BIN) syscall
	@echo "--- type reflection (sizeof/alignof) ---"
	$(BIN) typereflect
	@echo "--- atomic machine-code (⚛) ---"
	$(BIN) atomicjit
	@echo "--- SMT (native DPLL) ---"
	$(BIN) smt
	@echo "--- contracts (requires/ensures -> SMT) ---"
	$(BIN) contract
	@echo "--- comptime (const eval) ---"
	$(BIN) comptime
	@echo "--- formatter (bp_fmt) ---"
	$(BIN) fmttest
	@echo "--- power (WFI/WFE + PMU) ---"
	$(BIN) power
	@echo "--- x86_64 (encoder) ---"
	$(BIN) x86_64
	@echo "--- green threads (stackful coroutines) ---"
	$(BIN) gt
	@echo "--- pq (Keccak-f[1600] + SHAKE/SHA3, FIPS 202) ---"
	$(BIN) pq
	@echo "--- zlib (deflate/inflate + crc32/adler32) ---"
	$(BIN) zlib

clean:
	rm -rf $(BUILD)

.PHONY: all test clean
