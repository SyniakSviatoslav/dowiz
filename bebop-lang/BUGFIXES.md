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
- [NARROWED FURTHER] Not the fast-path bail: forcing legacy-only conds
  (spill gate) still crashes. The minefield is the STACK-MACHINE
  comparison of two BARE identifiers (`j < n`) when >=2 symbols are
  spilled: the cond compiles to a single bogus word instead of the full
  load/load/cmp/cset sequence, and later heap-style words
  (str [x14], mov x0,x14, add x14,#24 = struct-literal shape) appear.
  Rewrites that give the RHS a non-atom shape DO pass:
    while j < n * 1   -> correct results
    while j - n < 0   -> correct results
    while n > j       -> still crashes
  Workaround until fixed: write loop conditions in one of the passing
  forms; keep live-name counts <= 8 to avoid spills entirely.
- Reproducers: /tmp/opencode/mi3.bp (11 syms + write-loop),
  /tmp/opencode/spill2.bp (2 spills, NO arrays -> PASSES, isolating
  arrays+spills interaction). mi2 (1 spill) passes.
