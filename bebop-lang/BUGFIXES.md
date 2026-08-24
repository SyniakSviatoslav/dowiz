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

## Current open queue (priority order)
1. C1+C2+B2 combined round: spill-in-frame + arena zeros + const-bind guard,
   with the full probe ladder matrix (≤9/>=10 locals × arrays × calls ×
   nesting), self_check/fuzz/parity gates, then k5 NTT kernel unblocked.
2. Language ops: early return, unary !, compound assignment family.
3. VSA-over-stream demo, glyph completion 121→300, NEON deep pass.
