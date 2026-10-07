# REPORT-cold — honest source->result row (R4), cold self-compile phase profile (R5/DG18), honest.sh DVFS column (R6)

Status: 2026-10-07 CURRENT. Lane L2 rfast-cold (docs/research/2026-10-07-bebop-fastest-runtime.md §12), saved by main.
bebop.bin 8a0b4325. Box: proot aarch64, core 4 = A78, governor walt; the box was BUSY (other lanes compiling) and core 4
moved 691-2304 MHz — read ratios, not absolute times. Raw outputs: main session 4384f304 scratchpad cold-*.out.

## 1. Toolchains

| toolchain | version | used |
|---|---|---|
| bebop | `seed bebop.bin compile`, 8a0b4325 | yes |
| tcc | 0.9.27 (AArch64 Linux) | yes |
| gcc -O0 / -O2 | Ubuntu 15.2.0-16ubuntu1 | yes |
| zig Debug / ReleaseFast | 0.14.1 | yes |
| go | 1.26.0 linux/arm64 | yes |
| rustc -O | 1.93.1 | yes |
| clang | Termux (Bionic) clang-21 | NO — glibc builds would link Android libs |

## 2. Cold source -> result

Method: `bench/vs_rust/cold_twins.sh` — new process per compile and per run, `taskset -c 4`, median of 5, interleaved; every
output checked against a python oracle (0 mismatches); bebop's `<out>.dag` and `.becache` removed before each compile; a
fresh nonce comment per compile so no cache hits the program itself (prebuilt std/libc/GOCACHE std/zig global cache stay
warm, warmed once with another program outside the timing). K1-K4 = kernels/k{1..4}.bp folds (Rust twins rust_once/k*.rs;
C/Go/Zig twins generated, inputs hidden from the optimiser). BIG = generated 600 fn x 10 `let` integer chain (bebop
282,817 B; C 339,217; Rust 570,870; Go 240,273; Zig 321,945), all print -5449054496967356537.

compile / run / **TOTAL** ms:

| toolchain | K1 | K2 | K3 | K4 | BIG |
|---|---|---|---|---|---|
| **bebop** | 45/14/**71** | 34/19/**58** | 39/15/**51** | 31/15/**46** | 341/13/**356** |
| tcc | 76/24/**111** | 82/18/**99** | 64/15/**80** | 51/26/**73** | 64/14/**76** |
| gcc -O0 | 341/17/**357** | 368/14/**380** | 268/15/**283** | 321/22/**344** | 1752/14/**1765** |
| gcc -O2 | 279/16/**292** | 364/12/**375** | 270/14/**284** | 272/22/**290** | 1171/13/**1184** |
| zig Debug | 5307/20/**5332** | 5026/17/**5045** | 4422/12/**4431** | 4438/23/**4465** | 4882/11/**4893** |
| zig ReleaseFast | 1446/13/**1459** | 2063/11/**2074** | 1439/11/**1453** | 1447/14/**1459** | 33412/15/**33429** |
| go | 7375/20/**7407** | 7087/21/**7104** | 6478/17/**6500** | 5341/24/**5361** | 3455/18/**3472** |
| rustc -O | 769/20/**788** | 812/14/**827** | 717/15/**733** | 740/31/**766** | 30009/14/**30023** |

bebop TOTAL / other TOTAL (< 1.00x = bebop faster):

| vs | K1 | K2 | K3 | K4 | BIG |
|---|---|---|---|---|---|
| tcc | 0.64x | 0.58x | 0.64x | 0.63x | **4.71x (bebop slower)** |
| gcc -O0 | 0.20x | 0.15x | 0.18x | 0.13x | 0.20x |
| gcc -O2 | 0.24x | 0.15x | 0.18x | 0.16x | 0.30x |
| zig Debug | 0.01x | 0.01x | 0.01x | 0.01x | 0.07x |
| zig ReleaseFast | 0.05x | 0.03x | 0.04x | 0.03x | 0.01x |
| go | 0.01x | 0.01x | 0.01x | 0.01x | 0.10x |
| rustc -O | 0.09x | 0.07x | 0.07x | 0.06x | 0.01x |

Every run is 11-31 ms = the proot process floor, so the totals are compile times.

**Claim, from measured columns only:** on AArch64 (this box) bebop goes from integer source to result 11-17x faster than
rustc -O on four kernels and 84x faster on a 283 KB program, 3-8x faster than gcc -O0/-O2, and >= 10x faster than Go and
Zig; **tcc 0.9.27 compiles a 283 KB program 4.7x faster than bebop.** "Fastest source->result" is NOT supported.

## 3. Cold self-compile phase profile

10 variant compilers, `let _ = sys_exit(0);` after one phase (SPEEDUP §7.1 method); control `v10-full` md5 = 8a0b4325 =
bebop.bin; artifacts checked (v1 leaves only `.use`, v9 writes a.bin without `.becache`, v10 all four).
Cold compile of bebop.bp, pinned core 4, median of 7:

| exits after | ms | phase | share |
|---|---|---|---|
| 1 read + use expansion | 52 | +52 | 5% |
| 2 dag_tables | 60 | +7 | 1% |
| 3 dag_probe | 71 | +12 | 1% |
| **4 dag_plan (planning pass)** | 502 | **+431** | **45%** |
| 5 dag_layout (incl. scan_literals) | 509 | +6 | 1% |
| **6 dag_emit (emission pass)** | 972 | **+464** | **49%** |
| 7 dag_litdata | 932 | -40 (noise) | 0 |
| 8 dag_store (.dag write) | 954 | +22 | 2% |
| 9 stub + bytes + export | 946 | -8 (noise) | 0 |
| 10 full (.becache) | 948 | +2 | 0 |

BIG (336 ms): expand ~30, plan +142, emit +142.

Fn-count probe (generator differs from 2026-09-28: 175 KB at 701 wide fns vs 109 KB then), median of 5, ms:

| fns | narrow (1 call/fn) | wide (8 calls/fn) |
|---|---|---|
| 1 | 29 | 32 |
| 51 | 47 | 36 |
| 101 | 54 | 49 |
| 201 | 48 | 74 |
| 401 | 67 | 133 |
| 701 | 105 | 267 |

Control: 701 wide fns took 2,063 ms on 2026-09-28, 267 ms today. Whole-compile exponent in fn count 1.27-1.37 (wide),
1.1-1.2 (narrow), down from 1.85. Per phase (wide, 201/401/701): planning 11 -> 39 -> 110 ms, exponent **1.83-1.86**;
emission 23 -> 47 -> 108 ms, exponent 1.0-1.5.

**Phase to attack: `dag_plan`** — 45% of the cold compile, the only superlinear phase, and it exists only to learn each
fn's word count. The DAG already relocates `bl`, `&f` and literal words for memo hits (`dag_rel`/`dag_patch`): one
emission pass that records relocations and patches at the end removes planning. ESTIMATE 948 -> ~520 ms on bebop.bp
(R5 target <= 0.5 s); BIG 336 -> ~195 ms, still ~3x tcc. Why planning is superlinear is a HYPOTHESIS (lazy whole-source
tables + callee resolution before `fntab` is published).

## 4. honest.sh: p05..p95 ratio range and scaling_cur_freq (applied 2026-10-07)

Adds a `ratio p05..p95 range` column ([bebop p05 / Rust p95, bebop p95 / Rust p05]), a `core MHz before -> after` column
read around each kernel's runs, a `?` after the ratio when the frequency moved > 10% or could not be read
(`NOT MEASURED`), the governor in the header; `HONEST_CPUFREQ=<dir>` overrides the sysfs path (mutation input).
Proof: natural ramp on real sysfs marked only K1H (`1.0x ? | 0.94..1.14x | 960 -> 2054`); steady run: no `?`; M1
(scaling_cur_freq rewritten 2054400 -> 1190400 mid-run) marked only the straddling K4 row; M2 (missing dir) marked all
five `NOT MEASURED ?`.

## 5. Instrument found broken

`tools/perf.py` `selfcompile.one()` removed `x.bin`, `.becache`, `.use` but not `x.bin.dag`, so every timed run after the
warmup was a memo replay: as written 105/676/102/102/103 ms, with `.dag` removed 1538/1017/1007/1057/1052 ms. Every
`selfcompile_*` value since DG4 is ~10x low. Fixed 2026-10-07 (the tuple now includes `x.bin.dag`).
