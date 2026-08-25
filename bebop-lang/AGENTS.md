# Agent Process Rules

Written after a metacognitive audit of the M1–M3 session (2026-08-25).
Each rule cites the incident that bought it. These are process laws —
they exist because violating them cost real cycles in practice.

## 1. Zero hand-typed instruction words
Incident: 6+ wasted compile/run cycles on transcription errors — `382d695f`
typed as `3b0d695f`; hex read as big-endian int (`int('382d695f',16)` =
942415199 instead of LE 942500191); `mov x13,x0` embedded where `mov x0,x13`
was dumped; an entire emitter block pasted from memory instead of from
objdump output.

Rules:
- Never type or copy words manually. Pipeline: `.s` → objdump → script
  parses hex column → `int.from_bytes(bytes.fromhex(h),'little')` → insert
  into target via one script run. Hex→word is ALWAYS little-endian.
- After inserting words into any emitter, re-disassemble the generated
  stream at the insertion site and diff against the reference block.
  One command (~10 s) catches what otherwise costs minutes per cycle.
- If you already dumped correct words, embedding them is a mechanical
  step. The moment you "remember" a constant, stop and re-dump.

## 2. Syscall wrapper register contract
Incidents: sys_readbuf clobbered len(x1) with the scratch address leaving
x2 garbage; sys_slurp shipped with the SAME missing-x2 bug hours later.
Both passed review because nothing forced enumeration.

Rule: before emitting any svc/bl sequence, write the register table as a
comment FIRST and give every argument register an explicit producer:
```
// x0=fd(pop0)  x1=scratch(mov x10)  x2=len(mov x2,x1!)  x8=63
```
If a table cannot be written cleanly, the emitter is wrong. Interp mirrors
hide this class (C variables don't clobber) — only JIT executes the contract.

## 3. Harness semantics before differential debugging
Incident: fd=-2 appeared only outside gdb; burned several cycles on
instrument-vs-clean differences before reading exec_words main loop —
which revealed warmup+ref double invocation, i.e., state leaks across
calls (scratch pollution, open fds).

Rule: when behavior differs across runs/instruments/invocations, read the
runner's execution model first (how many calls, what resets). Any global
state — IO scratch zone, arena cursor, fds, buffers handed to the kernel —
must be re-initialized or explicitly terminated EVERY call, never rely on
fresh mmap zeros.

## 4. Divergence checklist (interp ≠ native/JIT) — ordered by frequency
Check in this order before inventing new theories:
1. Live symbols > 8 → spill machinery active? Shrink the probe to ≤8
   bindings FIRST (syscall builtins × spills is a known-broken combo).
2. Fast-path bail-outs: if-with-call-in-condition (fpC retargets literal
   branches into `mov x0,x0` copies of the condition value).
3. Scratch-zone overlap: anything writing near x28-8192 vs live data.
4. Wrong/stale artifact being executed (verify timestamp + word count).
5. Register protocol violations (x19–x26 symbols; x15 spill base; x27/x28
   arena cursor/end; caller-saved x0–x14 across bl).
The project's own BUGFIXES.md lore predicts most divergences — consult it
before hypothesizing. Writing probe variants that isolate ONE hypothesis
each (A/B/C/D/E style) beats patching the probe six times toward luck.

## 5. Two strikes ⇒ change axis, not magnitude
Incident: seed buffer — 64 KB limit hit by beboSelf (480 KB), then
mmap(16 MB)+read → ENOMEM, then mmap(1MB)+read → ENOMEM again. Only
after three failures did the design pivot to file-backed mmap, which
worked immediately.

Rule: two consecutive failures along one axis falsify the axis. Stop
tuning sizes/constants; enumerate alternative designs and pick the one
that eliminates the failing constraint entirely.

## 6. Paired structures get equality asserts, not eyeballs
Incident: fn-name list (regex, 176 entries) vs OFF offsets (138) were BOTH
printed and their mismatch ignored → entry offset pointed mid-function →
SIGSEGV → another debug cycle. The offline mirror tool itself then died on
an unimported module — an unverified tool driving decisions.

Rule: any analysis whose conclusion depends on two lists aligning must
`assert len(a)==len(b)` and print diffs. Run the tool to completion once,
sanity-check its output counts, THEN consume it. A diagnostic you print
but do not check against an expectation is noise, not evidence.

## 7. gdb-on-JIT recipe (do not improvise)
- Fixed-address breakpoints fail before the JIT mapping exists ("Cannot
  insert breakpoint"). Anchor on `__clear_cache` (runs post-mmap), then
  set base+offset breakpoints computed from THAT run.
- Crash triage: `info proc mappings` → find the rwx mapping →
  offset = pc − map_base → word# = offset/4 → disassemble that word range
  of the .bin. Two commands locate any JIT crash precisely.
- exec_words is stripped: no function symbols. Don't try symbol breaks;
  use `break __clear_cache`, `file:line` won't resolve either.

## 8. Cache model is known — trust it
.becache keys are crc32(compiler)+crc32(kernel); edits invalidate
automatically. Do not spend cycles suspecting staleness without positive
evidence (print the keys if unsure). When output doesn't change after an
edit, the edit didn't reach the executed path — verify insertion site,
not the cache.

## 9. Evidence hygiene
Most bugs above printed their smoking gun at least one cycle before
detection (the 176≠138; interp len=12 vs jit=13 pointing straight at the
missing terminator; result values decomposing exactly into w/nr/ok parts).
Rule: every printed diagnostic gets an explicit expected-value comparison
in the same breath. Decompose suspicious results arithmetically (what w,
nr, ok would produce this?) before running more experiments.

## 10. Keep what works (positive patterns from this session)
- Distinct exit codes per failure branch in seed (90 open / 91 read /
  92 mmap) plus errno propagation (`neg x0`) — failure localized instantly.
- Minimal repro ladder: p2 (open alone), p4 (+zeros), p5 (+read) — each
  delta isolates one mechanism.
- Variant bisection A–E of a misbehaving probe: five tiny programs beat
  one accreting program.
- `assert old.count==1` on every scripted source patch — saved silent
  misapplication repeatedly; make it universal.
- Execution-first verification through the real runtime path (seed) —
  assembler-correct words can still be context-wrong; only execution is
  ground truth.
