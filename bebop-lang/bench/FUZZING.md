# Bebop Parser Fuzzing Resilience Report

**Date:** 2026-08-19
**Scope:** run-and-report only. No `native/src/*.c` (including `fuzz.c`) or any other
source file was modified; nothing was committed or pushed. The harness was built
and executed, and this report written.

---

## 1. Bottom line

| Metric | Value |
|---|---|
| Harness | `native/src/fuzz.c` → `native/build/fuzz` |
| Total inputs tested | **300,000** |
| Wall-clock duration | **339.99 s** (~5.7 min) |
| Crashes (signal death) | **0** |
| Hangs (timeouts) | **0** |
| Assertion failures | **0 signal-aborts observed** (not separately counted — see §5) |
| **Crashes-per-million-inputs** | **0.0** |
| Result | **PASS** (exit 0) |

The parser front-end survived 300,000 adversarial inputs with **zero crashes and
zero hangs**. At the 95% confidence level this puts the true crash rate below
**≈10 per million inputs** (rule-of-three upper bound: `3 / 300000`).

---

## 2. Exact commands

Toolchain: `cc` = GCC **15.2.0** (`cc (Ubuntu 15.2.0-16ubuntu1)`), host
**aarch64** (Linux 6.17.0). Build flags (from `native/Makefile`):
`-O3 -flto -std=c11 -Wall -Wextra -Wpedantic -Werror -Wshadow -Wstrict-prototypes -Wmissing-prototypes -Wundef -Wformat=2`.

```sh
cd /root/dowiz/bebop-lang/native

# (1) build the fuzz harness — writes ONLY build/fuzz (does not touch build/bebopc)
make fuzz

# (2) run it: 300,000 inputs, fixed default seed 0x9E3779B97F4A7C15ULL
./build/fuzz 300000
```

Run output (exit 0):

```
fuzz: 300000 inputs | ok=61572 parse_err=238428 crashes=0 hangs=0
PASS: no crashes or hangs
```

The fuzz target is built from `src/fuzz.c` + the full compiler source list with
`src/main.c` (the CLI driver) filtered out (`FUZZ_SRC := $(filter-out src/main.c,$(SRC))`).

---

## 3. Results

### 3.1 Headline run (`./build/fuzz 300000`)

| Counter | Count | Share of 300,000 |
|---|---|---|
| `ok` (parse succeeded) | 61,572 | 20.5% |
| `parse_err` (clean reject, no fault) | 238,428 | 79.5% |
| `crashes` (child died to a signal) | **0** | 0.0% |
| `hangs` (child hit 1 s alarm) | **0** | 0.0% |
| **Total** | 300,000 | 100% |

- **Crashes per million inputs: 0.0** (`0 / 300000 × 1e6`).
- Throughput: ≈ **882 inputs/s** (300,000 in 339.99 s), limited by one `fork()` +
  `waitpid()` per input, not by parse cost.
- Duration: **339.99 s** (~5.7 min), just past the ~5-minute target.

### 3.2 Determinism / reproducibility checks

The harness uses a fixed-seed xorshift64* PRNG (`rng_state = 0x9E3779B97F4A7C15ULL`),
so runs are bit-for-bit reproducible. Confirmed:

| Run | Inputs | ok | parse_err | crashes | hangs |
|---|---|---|---|---|---|
| `./build/fuzz 120000` (invocation 1) | 120,000 | 24,749 | 95,251 | 0 | 0 |
| `./build/fuzz 120000` (invocation 2) | 120,000 | 24,749 | 95,251 | 0 | 0 |
| `./build/fuzz 300000` | 300,000 | 61,572 | 238,428 | 0 | 0 |

The two 120,000-input runs are byte-identical, and the 300,000-input run is a
deterministic prefix-extension (same seed → same first 120,000 inputs).

---

## 4. What the fuzzer actually exercises (honest note)

This is a **parser/lexer front-end robustness fuzz only** — it is *not* a
full-compiler fuzz, despite the target building against the full compiler object
set. Concretely, per input the forked child executes exactly:

```c
alarm(1);                       // 1-second hang bound
int r = bp_parse(buf, &prog, &err);   // lex + parse
bp_program_free(&prog);         // AST teardown (free)
_exit(r == 0 ? 0 : 1);
```

So the only code paths exercised are the **lexer, the parser, and the AST
destructor** (`bp_parse` / `bp_program_free`). The fuzz does **not** reach:

- type-checking / elaboration, name resolution, universe checks,
- codegen (WASM / aarch64 native / x86_64 encoder / JIT),
- the proof kernel / conversion checker (`theorem`),
- the contract / SMT / termination verifiers,
- any numeric kernels (NTT, FFT, hypervectors, crypto, …),
- the self-host compiler, or the CLI driver (`main.c` is excluded by the Makefile).

"Passing" therefore means: *the parser accepted or cleanly rejected every input
without faulting or exceeding 1 second.* It is strong evidence of front-end
memory-safety, and **no evidence at all** about the rest of the compiler.

### Input corpus (6 generator classes, uniformly mixed via `rng_next() % 6`)

| # | Generator | What it produces |
|---|---|---|
| 0 | `gen_ascii` | 0–65535 bytes of biased interesting ASCII (`CHARSET`) |
| 1 | `gen_bytes` | 0–65535 raw arbitrary bytes (incl. NUL-free high bytes) |
| 2 | `gen_truncated` | a valid-ish `.bp` seed truncated at a random non-zero length |
| 3 | `gen_mutated` | a seed with 1–8 byte-level mutations (flip / insert / delete / overwrite / duplicate-span) |
| 4 | `gen_spliced` | two seeds spliced with random glue, optionally truncated |
| 5 | `gen_token_bomb` | 33,000 `;` bytes — deliberately past the 32,768 lexer token cap |

The 15 SEEDS include structs, enums, fns, recursion, arrays, `while`, UTF-8 `λ`
glyphs, `theorem`, and a multi-item module — so the mutation/splice generators
walk *near-valid* program shapes, not just garbage.

---

## 5. Methodology notes & honesty caveats

1. **"Assertion failure" is not a distinct counter in this harness.** The parent
   classifies each child by `waitpid` status only: exit 0 → `ok`, exit 1 →
   `parse_err`, `SIGALRM` → `hang`, any other signal → `crash`. An `assert()`
   abort in the child raises `SIGABRT` and would be counted as a **crash**; a
   soft internal check that returns a parse error would be folded into
   `parse_err` and is **indistinguishable from a legitimate syntax rejection**.
   Since zero signal-deaths of any kind were observed, zero assert-aborts occurred,
   but "assertion failures" that present as clean parse rejects cannot be ruled out
   and are not counted separately.
2. **`parse_err` ≠ bug.** 79.5% of inputs were rejected, which is expected: the
   `gen_ascii`/`gen_bytes`/token-bomb generators produce overwhelmingly invalid
   source. A "clean rejection" is the correct outcome and is not a failure.
3. **Fork-per-input design.** Each input runs in a fresh `fork()`ed child under
   `alarm(1)`. A segfault/abort in the parser therefore kills only the child and
   is recorded, not propagated. This is what makes the 0-crash result meaningful
   rather than a single crash ending the campaign.
4. **Hang bound is 1 s per input.** `alarm(1)` means a hang costs at most ~1 s and
   is counted; the 0-hang result also confirms no input drove the parser into a
   >1 s stall (the token bomb's 33,000 chars are the most likely candidate and
   completed without timing out).
5. **Timing variance.** The headline run took 339.99 s (~882 inputs/s); an earlier
   120,000-input confirmation run took 129.07 s (~930 inputs/s). The difference is
   scheduler/load noise from concurrent workers on the shared host, not workload
   difference — the deterministic seed means both runs exercised identical input
   streams for their shared prefix.
6. **Reproducibility.** Every number in §3.1 is reproducible with the exact
   command in §2 on the same toolchain; there is no nondeterminism (fixed seed, no
   ASLR-dependent behavior in the counters).
7. **Concurrent-work caveat.** This run happened alongside other agents working in
   the same repository. `git status` at the time of the run showed pre-existing
   modifications to `selfhost/bebopc.bp` and `selfhost/compiler_main.bp` — neither
   produced by this task, and neither affects the fuzz build (which compiles the
   C `native/src/*.c` sources only).

---

## 6. Bottom line (repeat)

- **300,000 inputs, 0 crashes, 0 hangs, 0 signal-aborts** → **0.0 crashes per
  million inputs** (95% upper bound ≈ 10/M via rule of three).
- The **lexer + parser + AST destructor** are robust against 5 minutes of
  adversarial, mutated, spliced, truncated, byte-garbage, and token-bomb input.
- This says **nothing about type-checking, codegen, the proof kernel, or the
  verifiers** — those are outside the harness and would need a separate fuzz target.
