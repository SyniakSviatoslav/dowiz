# SESSION HANDOFF — 2026-09-03 (resume in ONE read)

Repo: /root/dowiz/bebop-lang (git@github.com:SyniakSviatoslav/dowiz.git, branch main)
HEAD: d704321 "docs(roadmap): T14+T15a done, T13 slot-localized as sole gap, session journal"
Everything pushed. Nothing uncommitted.

## How to resume (cheap, in order)
1. `/tmp/opencode/ctx` — orient pack (git state, corpus hashes, gate status).
2. Complete batched preflight below, THEN read the ACTIVE SPEC and execute it.
3. Never re-read whole files: use `tb h/s`, graphify, mempalace, cached embeds.
   Baseline snapshot for clean revert: /tmp/opencode/t13-baseline/ (bebop.bin,
   gen2.bin, gen3.bin, all md5 13a6447fe65cb3321e8165d38d7e4c77).

## Where we are
- bebop self-hosted fixpoint **green + byte-exact**: gen2==gen3==bebop.bin
  md5 `13a6447fe65cb3321e8165d38d7e4c77` (~60s/gen, NOT the stale 23-min).
  **std_golden 60/60**, construct_parity 24/24, parity_driver 9/9+1skip.
- 2026-09-02 (committed): T14 dispatcher EXECUTION SUBSTRATE (substrate.bp,
  fold 36750250113; k1 chain→36, k2 fib(25)→75025; post-von-Neumann SWAR
  popcnt + de Bruijn tzcnt, no PC/fetch-decode, activity to quiescence) and
  T15a SOFTWARE PMU counters (swpmu.bp, fold 2001000110000000000; replaces
  blocked perf_event_open EACCES). Both in std_golden.
- **THE ONE OPEN ROADMAP GAP: T13 register-window emitter.** Mechanism is
  PROVEN correct (R4#4: 42/42 gates, K1-K4 bit-exact) but never reconciled
  with the current emitter (R6.2 v5 folding / L16). Nothing of it is landed.
- T15 remaining (bare-metal: real PMU L1/L2, I-cache residency, pool 5/5,
  ARMv9 SVE/SME) is genuinely impossible in-sandbox (EACCES perf, this CPU
  = Cortex-A78, no SVE/SME); forward-port trigger list only.

## T13 — EXACT BLOCKERS + DESIGN (all localized this session)
1. **x9-x13 availability (VERIFY FIRST):** symbols use x19-x28, spills via
   [x15] base, arena x14, scratch x2/x3. x9-x13 look free for the value
   window but only 2 occurrences of "x9"/"x13" were seen, none confirmed as
   EMITTED words. Grep the emitter's word constants must confirm x9-x13 are
   never emitted as scratch across a value value's lifetime. If not free,
   use x29/x30 or re-encrypt the window before committing.
2. **push/pop still emit canonical stack words** (sub sp,#16; str x0,[sp] /
   ldr; add sp,#16) — no register path exists anywhere. Of ~100 call sites,
   all route through the single push/pop fns (bebop.bp 1097-1118 region);
   fix once in push/pop, every caller follows.
3. **flush-on-bl required:** a live value CAN survive a bl (h(a)+f(b) keeps
   h_result in x(9+0) across the bl to f). Only 2 emit_bl call sites
   (bebop.bp:565 in emit_bl_call, :576 in emit_self_call) — both have fntab,
   so thread a flush there before the bl. Migration must match exactly D
   memory-pushes so the stack machine's expectation holds: for k=0..D-1 emit
   sub sp,#16 (3506455551) + str x(9+k),[sp] (4177527776 | (9+k)<<5) →
   [sp]=slot D-1 … [sp+16*(D-1)]=slot 0, sp up to 16*D. rep→0.
4. **ONE-representation invariant:** fntab[3890] = rep (1=all-registers,
   0=all-memory). Window registers x(9+depth), depth 0..4. push: if rep==1
   and depth<5 → mov x(9+depth),x0 (1 word) + depth+1; else migrate reg→mem
   then memory-push. pop: if rep==1 and depth>0 → mov x0,x(9+depth-1) +
   depth-1; else memory-pop. rep=0 is set on migration and stays; rep=1
   re-set only when a segment's first value goes to a register.
   ENCODINGS: mov xD,x0 = 0xAA0003E0 | (9+d)<<16 ; mov x0,xS =
   0xAA0003E0 | S<<5.
5. **Verified free cell:** fntab[3890] is free (slot-tag zone ends 3796,
   literal-offset zone begins 3899; 4000 = inline-cache counter).
6. Do NOT gate on leaky fntab[3700] for register ADDRESSING — use rep +
   the register-window depth you control (fntab[3700] is still updated as
   the value-stack bookkeeper for the stack-fallback/revert path; the L16
   transient depth excursions and okres==0 phantom push are the reason the
   one-representation + migrate-not-hybrid rule is mandatory).

## T13 GATE LOOP + DONE-CHECK
Gate loop (cheap first): self_check (bebop.bp:2720, c1-c41, 0.05s) → full
battery (std_golden 60 + construct 24 + parity 9) → fixpoint (bb2==bb3
byte-exact; ~60s/gen: seed bebop.bin compile bebop.bp gen2.bin then seed
gen2.bin compile bebop.bp gen3.bin) → K1/K4 bit-exact vs kernels
(bench/vs_rust/kernels/{k1,k2}.bp: k1 fold 500000500000, k2 fib(25)=75025).
DONE-CHECK (FASTPATH-SPEC R6.1): fixpoint byte-exact + 50 gates + K1/K4
benchmarked vs frozen Rust medians — ship whatever the numbers are.
Procedure discipline: SINGLE-VARIABLE diffs (L14), clean revert from the
baseline snapshot, battery after each. Allowed: the full unsafe emitter
rewrite the user authorized (zero conservative decisions).

## Bebop discipline (LAWS, non-negotiable)
- R3.x emitter/runtime defects (workarounds; see ROADMAP + journal):
  (a) `a*b<<c` fast-path — parenthesize;
  (b) `>>` EMITS LSR — LAW: >> is logical on both engines — abs (or
      &-mask) before shifting any possibly-negative value; oracles mirror
      unsigned;
  (c) loop-shaped while+compare+conditional-store miscompile — unroll,
      hoist, or branch-free multiply-select stores;
  (d) str literals and ++ concat SEGFAULT in .bin — str-free (argv+cells+
      arithmetic);
  (e) clock_ms() miscompiles the following statement (zero-arg parse) —
      minimal repros /tmp/opencode/z2.bp jm.bp zn.bp.
- L16: push/pop emit EXACTLY their canonical words — never extra words
  (check-traps polluted the stream). Model state is bookkeeping-only
  (guarded 0<=d<96, depth clamped >=0).
- L8: no allocations inside while bodies. L11: entry identity. L12/L13:
  artifact/golden baselines. Precedence trap: avoid `exp != 0 - 1`.
- std_golden recipe: machinery verbatim from
  /tmp/opencode/spectral_machinery.bp (sha256 c184416666fe; regen from
  selfhost/std/spectral.bp if it drifts); ≤8 array binds/fn; gate main
  returns fold; cp selfhost/std/<g>.bp → bench/vs_rust/std_tests/; add
  gate() line; journal; ROADMAP; commit+push.
- TOKEN ECONOMY (docs/TOKEN-ECONOMY.md): Pro writes SPEC cards before any
  Flash-executable work; Flash executes verbatim, STOPS on mismatch with
  `VERDICT: mismatch`. This repo follows it (binding).

## ACTIVE SPEC — T13 register-window emitter (the sole gap)
Goal: stack machine → register-resident values (top in x0 by construction),
movs instead of push/pop pairs where provable, flush-on-bl; one-representation
invariant (rep = fntab[3890]). Close FASTPATH-SPEC done-check: fixpoint
byte-exact + 50 gates + K1/K4 ship-whatever. Ordered steps (each = own
commit, battery + fixpoint after, per L14):
  S1. Confirm x9-x13 are never emitted as scratch (blocker #1). If clear:
  S2. Minimal register push (depth<5, no rep tracking) — verify a k1-style
      loop fold unchanged; keep stack fallback for depth>=5 (prove bit-exact
      before enabling migration).
  S3. Add rep state (fntab[3890]) + migrate-reg→mem on boundary/bl; flush
      thread into the 2 emit_bl sites.
  S4. Full window pop path (mov x0,x(9+depth-1)).
  S5. Fixpoint + full battery + K1/K4, record numbers, edit ROADMAP T13→DONE
      with whatever the numbers are, journal, commit+push.
After T13: close T15 by documenting EACCES + no-SVE as hard platform bounds
with the bare-metal forward-port trigger (software-PMU swpmu = in-sandbox
hardware-validation substitute). Update ROADMAP (T13/T14/T15 all terminal).

## Scratch layout (/tmp/opencode)
t13-baseline/ (fixpoint snapshot, md5 13a6447f — CLEAN REVERT POINT) |
spectral_machinery.bp (embed cache) | ctx | *gate.bp scratch sources of the
60 gates | z2.bp jm.bp zn.bp (clock_ms repros) | tb.bin tb_test.bin.
