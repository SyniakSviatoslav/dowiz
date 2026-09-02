# SESSION HANDOFF — 2026-09-02 (resume in ONE read)

Repo: /root/dowiz/bebop-lang (git@github.com:SyniakSviatoslav/dowiz.git, branch main)
HEAD: (this commit) "docs: roadmap final status - pull complete, 41 gates".
Everything pushed.

## How to resume (cheap, in order)
1. `/tmp/opencode/ctx` — orient pack (git state, corpus hashes, gate status).
2. Read the ACTIVE SPEC below and execute it (Flash tier) or extend (Pro tier).
3. Never re-read whole files: use `tb h/s`, graphify, mempalace, cached embeds.

## Where we are
- bebop self-hosted compiler fixpoint bebop.bin (dfaf06c3). **std_golden 41/41**
  gates. New this session (journal 1788288208-1788288229): SS-17 seigtime
  (24th, 1233012011, ring-30 eigentime), SS-18 srepl (25th, 8449214), SS-8
  sinc (26th), SS-1 kalman (27th), SS-5 calcbound (28th), SS-2 vecinv (29th),
  SS-4 fir (30th), SS-7 qlora (31st), SS-12 bitmat (32nd), SS-9 attn (33rd),
  SS-3 lcres (34th), SS-11 genarena (35th), SS-10 stride (36th), SS-13
  pieblock (37th), N3 ringvsa (38th), N7 msuper (39th), N8 spacetime (40th),
  Neural Operator Core fno (41st, 111152971008019 — FWHT/NTT/KLT three-level
  stack, user-specified roadmap item 1).
- ROADMAP: the pull is COMPLETE at gate level. Remaining (honest, marked
  innovate: with triggers): SS-14 direct-threaded (emitter rework + I-cache
  bench), hardware halves of SS-3/9/10/11/13 (clock syscall, PMU, mmap
  surface, cold start), SME/SVE2 forward port (real ARMv9 silicon), R3.x
  emitter defects (a)-(d) — the only open correctness work.

## Bebop discipline (LAWS, non-negotiable)
- T0 expected-vs-got on every run; one-line journal H:DID:GOT:VERDICT.
- L8: no allocations inside while bodies. L11: entry identity for .bin runs.
- R3.x emitter/runtime defects (workarounds, see ROADMAP + journal):
  (a) `a*b<<c` fast-path — parenthesize;
  (b) **UPDATED (1788288210, 1788288216): `>>` EMITS AS LSR on locals —
      THE LAW IS NOW: >> is logical on both engines — abs (or &-mask)
      before shifting any possibly-negative value; oracle mirrors shift
      UNSIGNED.** First seed-sensitive gate (seigtime) exposed the split;
      calcbound confirmed it on locals.
  (c) loop-shaped while+compare+conditional-store miscompile — unroll,
      hoist to locals, or branch-free multiply-select stores;
  (d) str literals and ++ concat SEGFAULT in .bin runtime — str-free
      programs (argv + cells + arithmetic only).
- Precedence trap (1788288220): `exp != 0 - 1` misparses — use explicit
  flags/sels instead of negative-constant comparisons.
- fp_div (long division) needs an INTEGER-PART pre-loop for ratios >= 1
  (the r<b restoring invariant escapes otherwise; 1788288222).
- Fold discipline: each gate = ONE i64 fold == python mirror bit-exact;
  ×3 only at first freeze.
- std_golden gate recipe: embed machinery verbatim from
  /tmp/opencode/spectral_machinery.bp (sha256 c184416666fe; regenerate from
  selfhost/std/spectral.bp if it drifts); ≤8 array binds/fn; gate main
  returns fold; cp selfhost/std/<g>.bp → bench/vs_rust/std_tests/;
  add gate() line to std_golden.sh; journal; ROADMAP; commit+push.
- Spectral topk: need UNIQUE |lambda| in the top-k block; seed shift emits
  LSR — mirrors shift unsigned; folds quantize away LSB seed noise.

## Toolstack (always on)
- tb = `tb` wrapper at ~/.local/bin/tb: tb h <path> (crc32), tb ctx j r g,
  tb s <needle> <path>, tb c (stdin compressor), no args = self-test fold
  1111000 (gate 23).
- graphify (uv tool): query/path/explain on graphify-out/graph.json.
  mempalace: search/wake-up; re-mine journal after commits. snap: text→PNG
  (vision models only). rtk plugin auto-activates after restart.
- Token economy: docs/TOKEN-ECONOMY.md (binding). Tier routing: Pro =
  reasoning/planning/debugging/synthesis; Flash = SPEC card execution.

## ACTIVE SPEC — R6 fast path + R3.x(e) clock defect (the open work)
- R6 register-aware emitter: docs/FASTPATH-SPEC.md (binding) - R6.0 root-
  causes the R4 model-write failure FIRST, then source-level register/
  constant awareness; NO in-place word-stream rewrites (all of R4/R5 killed
  by layout-sensitive self-compile crashes, journal 1788288234/36).
- R3.x(e): clock_ms() miscompiles the following statement (zero-arg builtin
  parse) - minimal repros /tmp/opencode/z2.bp jm.bp zn.bp; SS-3 jitter half
  blocked on it (journal 1788288238).
- Remaining tail: SS-14 direct-threaded, SME/SVE2 forward port (ARMv9).

(a) fast-path `a*b<<c` miscompile (1788288190);
(b) `>>` logical/arithmetic selector split — emitted LSR on locals; the
    language LAW is ">> is logical; abs first" (1788288210/216);
(c) loop-shaped miscompile (1788288197);
(d) str literals/++ concat segfault (1788288206).
Then, in order: SS-14 direct-threaded dispatch + SS-3/9/10/11/13 hardware
halves when the clock/PMU/mmap surface exists; SME/SVE2 forward port on
real ARMv9 silicon.
Design: fix each in bebop.bp emitter/runtime, rebuild the fixpoint
(bb2==bb3 required), add a minimal-repro regression gate per defect (frozen
fold == oracle), keep all 41 gates green. Defect (b) fix = always ASR on
signed operands or make the shift semantics uniform + mirror unsigned in
oracles.

## Scratch layout (/tmp/opencode)
spectral_machinery.bp (embed cache, c184416666fe) | tb.bin + tb_test.bin |
ctx | snap + snapenv/ | ss15_oracle.py ss16_oracle.py ss17_oracle.py
ss18_oracle.py | sg_*/tp1 probes (stale, safe to delete) | *gate.bp scratch
sources of this session's 41 gates (seigtime srepl sinc kalman calcbound
vecinv fir qlora bitmat attn lcres genarena stride pieblock ringvsa msuper
spacetime fno).
