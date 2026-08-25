# Bug retrospective & rules update — last 7 days

Every native-codegen or toolchain bug that cost debugging time this week,
with the rule each one earns. Rules marked **[RULE]** are binding for future
sessions on this codebase.

## A. Encoding / constants class

**A1. movrr base constant wrong by heart-math.** Symptom: garbage register
moves. Root cause: decimal constants derived mentally from hex.
Fix: all encodings now taken from objdump of assembler reference programs.
**[RULE] No AArch64 constant enters any source file without (a) objdump
derivation and (b) exec_words execution computing a known result. Python
computes decimals; hands never convert hex.**

**A2. UBFM/lsl formula "verified" against a wrong hand-typed decimal.**
The python-side check passed because both sides shared the same wrong base.
Fix: anchor samples come only from the assembler, never from memory.
**[RULE] Verification anchors must be externally generated (assembler,
QEMU, silicon), never reproduced from the same mental model under test.**

**A3. msub/msub-field order confusion.** Fixed by assembling the exact
instruction and transcribing objdump output. Covered by A1 rule.

## B. Lazy-folding / binding class

**B1. Missing materialization points (three separate incidents).** Constants
riding in cx cells silently produced garbage-register reads when a consumer
lacked a materialization branch (binop sides, cmp sides, if arms, call args,
while conds, tail position).
**[RULE] Every new consumer of an expression value MUST be audited against
the materialization checklist before it can be considered done; add its test
to the probe ladder.**

**B2. Zero-word const rhs binds stale x0 (`emit_let_stmt`).** `bind_reg`
fired for pure-constant fast values, capturing whatever was in x0. Guard:
`vwords = n[0] > nb_v`; const path must `fpC_lit(...)` into the bound reg
(spill regs via x0 + bind_reg). Status: fix drafted, NOT landed — see D1.
**[RULE] "Nothing emitted" is a semantic state, not an optimization detail:
any code path that binds/returns/persists a value must handle the zero-word
case explicitly.**

**B3. Const-rhs lets skip stab-visible register state entirely; readers of
folded names observe stale registers nondeterministically** (worked at -O0,
failed at -O2). Interacts with B2.
**[RULE] Interpreter/native divergences that flip with unrelated compiler
flags = memory-layout bug; stop measuring, start dumping frames.**

## C. Frame / ABI class

**C1. Spill banks outside the frame.** x15=sp+4096 while frame reserves
1264B: every ≥10-local fn wrote caller stack. Layout-dependent crashes.
Candidate fix (x15=sp+1024 inside frame) alone broke ≤9-local fns ⇒ there is
a second live user of the old x15 value or the G1-NOP interaction is not
understood yet. Status: OPEN, forensics below.

**C2. zeros() emitted literal 0 as base pointer.** Arrays were interpreter-
only in practice; parity corpus contained no array kernels so nothing caught
it. Validated fix sketch exists (x27 arena cursor + OPT-A pair keep) but the
full combo needs its own focused round. Status: OPEN with reproducers.
**[RULE] Any language feature added to the interpreter must ship with a
compiled-native probe in the same commit, even if the emitter is expected to
bail to legacy.**

**C3. Callee-saved pair NOPing (OPT-A) counts bound vars, not used regs.**
x27 became live through zeros() but the pair stp/ldp x27/x28 was still
patched to NOP → ABI violation → -O2-only flakiness. Fix validated: keep the
x27/x28 pair whenever the arena feature is compiled in.
**[RULE] Register-liveness analyses must be driven by actual instruction
scan, not proxies like variable count.**

## D. Process / tooling class

**D1. Combined-fix instability.** Landing three interacting fixes (arena +
x15 + bind guard) in one pass destabilized previously-green probes. The
session reverted to keep the tree green and queued a focused round.
**[RULE] Codegen fixes land ONE hypothesis per commit, each with its probe
ladder run before the next lands. Interaction matrix first, combo last.**

**D2. Phantom bug from a clobbered scratch file.** A probe .bp got
overwritten with the compiler source itself; two sessions worth of "fast
path bails on nested lets" chased a ghost.
**[RULE] Probe files are generated fresh into content-addressed names
(/tmp/probes/<sha>.bp); a surprising benchmark is re-derived before it is
explained.**

**D3. Stale runner binary masked a real regression** (exec_words rebuilt by
hand, Makefile silent). Fixed by building explicitly after edits.
**[RULE] After ANY change to a runtime component, verify the timestamp of
the binary actually being invoked before interpreting results.**

**D4. Duplicate compilemany commits** — committed twice with identical
message due to interrupted flow.
**[RULE] git log --oneline -3 before every push; amend-or-rebase duplicates
away locally before they reach origin.**

## Session-2 forensics (post-retrospective round)

**C1 root cause RECLASSIFIED.** The >=10-local crashes are NOT spill-bank
corruption: stream decoding shows the optimizer rewriting a `[movz][sdiv]`
window so that `let half_total = n / 2` loses its `bind_reg` (the sdiv
writes the target register directly, the bind word disappears; next let's
bind then aliases the slot). Spill encodings themselves verified correct
against assembler (`str/ldr [x15,#slot*8]`). Next single-hypothesis probe:
disable the A/B window peephole when the window contains a division, or
make bind_reg fire after any op-with-rd==target-reg.

**C2 scope correction.** se/sf ("9 vs 10 locals") were red herrings: every
crashing probe called zeros(); locals count was incidental. Validated-green
state exists: zeros-trio (arena cursor init past NOP window + x27/x28 pair
keep + emit_zeros bump sequence) turned tg/tf/tc/td AND se/sf green
deterministically. It was reverted only because:

**C2a. Arena-in-frame cannot hold the compiler's own buffers.** 64KB reserve
< insns[262144]=2MB -> SIGSEGV inside self_check; raising reserve to 16MB
puts the cursor below the 8MB stack mapping -> immediate fault. Correct
design (next round): x29-relative callee-saved saves + sp-dynamic alloca +
epilogue `mov sp,x29` restructure, or a process-lifetime heap page obtained
once via mmap-less .bss anchor in the runner.

**[RULE] A per-call reservation can never exceed one frame; anything sized
by the PROGRAM (compiler buffers, big arrays) needs storage whose lifetime
and address range outlive the frame — decide this per feature before wiring
allocation.**

**[RULE] When three probes flip color together, list their shared feature
first (all sd-class probes call zeros; none of the green ones do). Locals
count was correlated, not causal.**

## Current open queue (priority order)
1. SOLVED THIS SESSION: hash-wrap identifier rejection (C1-class crashes
   were mostly this + zeros); zeros landed via runner bump arena x27/x28.
   se/sf/sd/tg/tf/tc/td/p* all green deterministic; parity 340/340,
   fuzz PASS, selfcompile warm 41ms.
2. REMAINING red class (sc only): legacy div/mod helper inside loop bodies
   emits an madd/sub sequence through x14 whose prologue setup interacts
   with G1 scanning (decoded: stp-shaped word + sdiv + madd x14 + sub).
   PROGRESS: G1-gate probe (count sdiv/madd shapes in scan) applied - no
   effect, rule out scratch-setup theory. gdb ground truth: the faulting
   word is 0xa9ff03e0 (LDP x0,x0,[sp],#writeback) appearing mid-stream;
   objdump confirms shape, grep confirms NO source line emits any matching
   constant -> the word is WRITTEN BY A PATCHER through a stale index
   (prime suspect: two-pass program layout re-emission overlapping windows,
   or A/B-peephole wl2+reg rewrite reading across a truncated boundary).
   Next probe: checksum every insn[] word after pass-1 vs pass-2 in
   compile_program_to; diverging index = corrupting writer.
3. Language ops: early return, unary !, compound assignment family
   (% already shipped end-to-end).
4. k5 NTT kernel (algorithm validated interp=python=759186635) lands after
   item 2; then VSA-over-ADC demo, glyph completion, NEON deep pass.

## Session: spill-forensics (sc-class reopened and narrowed again)

Fixed this round:
- [FIXED] x27/x28 collision: sym_bind handed symbols the runner's arena
  cursor/end registers. Symbols now occupy x19..x26 only; 9th+ binding
  spills to frame slots. This was a REAL latent killer for any program
  with >8 live names using zeros().
- [HARDENED] emit_var/bind_reg now load xzr / no-op on reg<0 instead of
  encoding garbage registers (the historic 0xa9ff03e0 = mov x0,x[-1]).
- [FIXED] fp_loadvar LDR base was off by one imm12 step
  (4181720064 -> 4181721088); fast-path spilled loads were malformed.

Still open, narrowed hard:
- Programs with >=2 spilled symbols AND an array-write loop crash
  (SIGSEGV; at fault x14 holds garbage despite prologue add x14,sp,#768
  executing — proven by marker-injection: a movz x14,#0xabcd inserted
  after the prologue survives to the faulting instruction).
- Forensics ruled out: enum-ctor mis-dispatch of zeros (disabled:
  still crashes), OPT-G1 prologue elision (disabled: still crashes),
  stream corruption (full word-level decode shows clean instruction
  stream), stale patch indices in A/B fusA/fusB (patterns verified).
- [ROOT-CAUSED AND FIXED] The entire crash family (p20/p21/mi3/lc/...)
  was emit_ident treating `ident {` as a struct literal — for
  `while k < c7 {` the legacy RHS parse of `c7` saw the loop's `{` and
  ATE THE LOOP BODY as struct-construction fields (str[x14,i*8],
  mov x0,x14, add x14,#24 signature). It stayed latent because OPT-B's
  fast path handled conds whenever no spills forced the legacy retry;
  the x27/x28 fix lowered the register budget and exposed it everywhere.
  Struct-literal branch disabled (feature unused by all kernels/tests).
  ALL former crashing reproducers now match interp bit-exact:
  lc=28, mi3/full3/p20/p21, xx=24, spill2=621, k9=60.
- Reproducers: /tmp/opencode/mi3.bp (11 syms + write-loop),
  /tmp/opencode/spill2.bp (2 spills, NO arrays -> PASSES, isolating
  arrays+spills interaction). mi2 (1 spill) passes.

## Round: x15 call-save closes k5; k6 narrowed to let-chain stab ordering

- [FIXED] k5 NTT kernel: root cause was caller-saved x15 (spill base)
  clobbered by callee prologues across bl. emit_bl now wraps every call
  in push/pop of x15 (assembler-verified words 3506455551/4177527791/
  4181722095/2432713727). k5 = 759186635 native, bit-exact with interp
  and the python reference. Sanity checksums resynced (helper-program
  stream grew by 4 words/call site): c35=146919800484, c36=198951693688.
- [FIXED — THE ACTUAL ROOT OF THE WHOLE FAMILY] emit_zeros ladder word
  #1 was 3548187681 which disassembles to `ubfiz x1,x1,#3,#4` — a
  4-bit field extract, NOT `lsl x1,x1,#3`. For array sizes <=15 words
  the truncation accidentally rounded correctly; for >=16 words the
  cursor advance COLLAPSED, so consecutive zeros() blocks overlapped:
  b aliased a, reads returned the other array's data ("b reads a",
  address deltas of 0), k6 popcount = 0. Assembler-verified true LSL#3
  = 3548246049 now emitted. k6 = 236 native == interp; k1..k5 re-verified.
  The earlier "stab ordering" suspicion was wrong — it was built on
  misread register maps from scratch-file index shifts (see RULES).
- [INFRA] exec_words runner: call_jit loads x27/x28 with inline asm in
  the same block as blr + -ffixed-x27/-ffixed-x28 build flags (GCC
  keeps global register-asm vars in memory otherwise). Protocol proven
  by magic stream (mov x0,x27 -> arena base).
- [NOTED] interpreter zeros() handle semantics: `c - a` on two zeros()
  handles returns 0 in interp while native gives honest pointer delta
  (64 for two 4-element blocks). Harmless for kernels; audit before
  ever relying on handle identity.

## Round: register protocol PROVEN; runner hardening

- [PROVEN] The x27/x28 register contract WORKS end-to-end: a hand-made
  stream returning mov x0,x27 gives the arena base back through
  call_jit. GCC on aarch64 keeps global register-asm vars in MEMORY and
  ignores them as registers, so exec_words now loads x27/x28 with inline
  asm in the SAME block as blr (call_jit), built with -ffixed-x27
  -ffixed-x28. Build command for the runner is now:
    gcc -O2 -Wall -Wextra -ffixed-x27 -ffixed-x28 -o build/exec_words bench/exec_words.c
  (JITBASE printed to stderr for gdb work; under gdb ASLR-off the base
  was stable at 0x7ff7ffc000.)
- [OPEN, narrowed to absurdity] yj/yb/xr family: two zeros(n) blocks +
  address delta returns 0 natively while the emitted stream disassembles
  textbook-perfect (objdump-verified: two ladders, two cursor-adds,
  sub). gdb at the cursor-add shows x27=0 despite magic-stream proof.
  Contradictory evidence across runs suggests my probe tooling (stale
  binaries, count-line index shifts in hand decodes) is itself part of
  the noise — per BUGFIXES rules, next session must start from a FRESH
  single-purpose harness, not reuse lv.full-style scratch files.
- k5 stays green through all of this; make test 79/0; parity 40/40;
  fuzz_selfhost PASS with new sanity checksums.

## Round: SWAR popcount lands; unary ! in C host; cmp-value bug fixed

- [DONE] popcount() as a pure-.bp stdlib function (branch-free SWAR via
  the bitwise tier): k6 v2, 20us -> 5.8us (3.4x). Parity
  interp==native==python on 0/255/2^32/2^62/0x5555...
- [FIXED] C-host comparison evals (three sites) set only .b/.bval and
  left .ival=0 — any comparison USED AS A VALUE (not just in if/while
  conds) silently read as zero. All six now set both fields.
- [PARTIAL] unary ! : C host desugars !e to (e == 0) in parse_primary
  (guarded against !=); works in interp. Selfhost emitter attempted
  (emit_apply_not chain branch) but the branch fired TWICE per use for
  reasons not isolated this session (cell-guard did not change it) —
  reverted from selfhost to keep streams honest; native !x currently
  yields 0 silently. Queued with the double-emission trace notes.

## Wave-2 audit complete

- CLI modes vs self-tests: 105 modes / 69 self-tests; ZERO orphaned
  self-tests (wave-1 wired the four isolated modules; re-inventory
  confirms full coverage). The 36 test-less modes are tooling/demo
  surfaces (compilewords, run, size, hv_stream, ntt_filter, ...), not
  library modules.
- Selfhost compiler dead code: removed ctor_index, count_params,
  fp_expr_step, emit_fast, mkarr (5 fns, 129 -> 124). emit_offsets and
  self_check are called by name from the C host and stay. One variant
  (fp_expr_stepX) flagged for the next pass.

## Next-session scope notes (compound assignment +=)

Three parser surfaces must agree; touch points mapped:
1. run/interp path: lexer.c + qtt.c (AstProgram/TERM_*) — statement
   dispatch lives where TERM_LET is built; desugar `x op= e` to
   LET(x, BIN(op, VAR x, e), rest); scalar form only.
2. selfhost compiler: emit_body item dispatch (is_let/is_while/...):
   add is_compound = ident-start && after-ident ws one of +-*/% followed
   by '='; emitter order: sym_bind(name) EARLY -> emit_var(reg) ->
   emit_cmp(rhs) -> binop -> bind_reg(reg); consume ';'.
3. fuzz corpus: extend with x+=/-=/(*=)//=/%= scalar loops.
Array-element compound (a[i] += e) stays out of scope until scalars are
proven. C-host expr.c parse_seq experiment reverted (run path does not
use it).

## Compound-assign root cause found (for next session)

C-host seq-level desugar worked for FINAL statements but broke
mid-sequence: TERM_LET's scalar-value env rollback (`env_i =
saved_env_i` after eval of t->b) erases the shadow-rebind when the LET
is used as an EXPRESSION VALUE inside the seq wrap chain. Correct
design: compound stmt must lower to a MUTATING form (the in_while
in-place mutation branch) or seq must not wrap value-position LETs.
Until then: no += in the parity corpus (both backends must agree).

## HV-everywhere infrastructure

- BEBOP_PRELUDE=<file>: compilewords prepends the prelude source to any
  kernel (both single-source and batch paths) — hv_stdlib.bp ships
  SWAR popcount, seeded generator, XOR bind, Hamming as plain .bp.
- k7 VSA associative memory: 8 key->value 1024-bit HVs, noisy query,
  nearest-key resolve; interp==native==python; 35.7us native.

## NEON hvham — verified words + emitter half-built (next session's first task)

Assembler-verified AArch64 sequence (objdump-checked), 15 distinct words:
  lsr x4,x2,#3        = 3544448068
  cbz x4,+tail        = 3019899236   (offset for layout: [lsr][cbz][movi][body8][sub][cbnz][uaddlv][umov][mov])
  movi v4.16b,0       = 1325458436
  ldp q0,q1,[x0],#32  = 2898330624
  ldp q2,q3,[x1],#32  = 2898332706
  eor v0,v2           = 1847729152
  eor v1,v3           = 1847794721
  cnt v0              = 1310742528
  cnt v1              = 1310742561
  add v4,v0           = 1310753924
  add v4,v1           = 1310819460
  sub x4,#1           = 3506439300
  cbnz x4,-9          = 3053453028   (target: first ldp)
  uaddlv h5,v4.16b    = 1848653957
  umov w6,v5.h[0]     = 235027622
  mov x0,x6           = 2852520928   (NOT 2852520704 — that decodes to orr x0,x24,x6!)
[RESOLVED SAME SESSION] Single-path emitter restored from this table
and shipped. TWO real bugs found past the rollback point:
1. CHUNK SIZE: `ldp q0,q1` covers 32 BYTES = 4 WORDS per iteration,
   not 8 — the chunk counter must be `lsr x4,x2,#2` = 3544382532
   (not #3/3544448068). The "x2=n-8" theory was wrong; a two-marker
   discriminating probe (markers in chunk1 AND chunk2) separated the
   hypotheses in one run.
2. STRICT BRANCHES confirmed again: both tail variants always emit;
   never dispatch emitters through if/else — use single-path forms.
Status: hh=12, hh2=8, disc=16, hz2=8 all interp==native; parity corpus
+1 (hvham_neon). k7neon variant uses the builtin (same checksum;
bounce-copy of rows currently caps the win at ~20% — direct row-slice
args are the follow-up).

## hvham2 session: emitter built, strict-branch trap CONFIRMED mechanically

Built hvham2(a,ao,b,bo,n) — offset-args NEON hamming (removes k7's
bounce copy). C-host side COMPLETE and correct (symmetric pair nodes
ARRAY[arr,off] in slots a/b; eval with bounds). Selfhost emitter:
single-chunk proofs passed, then a crash spiral produced the decisive
mechanical proof of the branch-evaluation law:

  While emit_hvham2/emit_hvham sat as if-branches in the SHARED
  emit_call_or_ctor dispatch, EVERY zeros() call in ANY program
  strictly evaluated BOTH emitters: their argument parsers consumed
  source text past the zeros ')' and emitted garbage words. k6 — which
  never calls hvham* — broke. Diff vs HEAD showed wholesale register
  remapping of its stream.

CONSEQUENCE LAW (now binding): in this meta-language an if-chain
selects VALUES but every branch's function CALLS execute. Any builtin
dispatcher must therefore keep at most ONE call-capable branch per
dispatch site, or route through flag-multiplied word emission (OPT-G1
style) — never side-effecting parser calls in untaken branches.

Correct hvham2 selfhost strategy for next attempt (no dispatcher edit):
parse args via the SAME single generic emitter by extending emit_zeros
pattern into one parameterized fn emit_hv_neon(argcount, layout-id)
selected BEFORE any call site exists (hash->fn pointer table is not
available; instead generate per-builtin wrapper fns each containing its
OWN full body, dispatched from separate let-bound cells evaluated
unconditionally but parsing NOTHING unless their hash matches — guard
with early 'if name != HASH then 0 else <full body>' where the full
body sits INSIDE the then-arm and contains no competing call).

C-host hvham2 + selfhost hvham(3-arg) remain green: hh/hh2/hz2/disc
were interp==native before today's spiral; re-verify after re-land.
