Status: 2026-09-09 CURRENT (D5 landed as d166006; D5b step 1 in progress, lane C, session 30). Grounded at HEAD 982855f. Every number below was measured on this box on 2026-09-09 by `tools/ddc.sh` (`--chain`, `--why`, `--measure`, `--full`); nothing is quoted from an older document, and where an older document's number is now wrong this file says so. This file is the authority on what the project's trust claim IS. The MANIFESTO and the ROADMAP must not exceed it.

# The trust chain

## 0. The one-paragraph version

Bebop has **zero runtime dependencies**, and that is true. It does **not** have a
159-line trust root, and that has been claimed in this repo more than once. The
trust root today is a **1,480-byte committed loader** (**488 bytes** of it are
`.text`; it compiles nothing), a **170,312-byte committed compiler binary** that
the loader mmaps and jumps into, and **CPython**, on which every semantic gate in
the tree depends through `tools/bpref.py`. `gen3 == gen4` is a
**reproducibility** gate, not a trust gate: Thompson's attack survives any
fixpoint by construction, because a compiler that re-inserts a backdoor into
itself reproduces that backdoor byte-for-byte and the md5s agree (Thompson 1984;
arXiv:1004.5534).

**D5 set out to close that with diverse double-compiling and it did not, because
the row's central premise was false as written.** The row said the witness
"already exists in-tree (`selfhost/attic/expr_compile.bp`), so the fix ADDS NO
DEPENDENCY". Measured 2026-09-09: the witness emitted code for the retired
`exec_words` stack machine and failed on `fn main() -> i64 { 42 }` with SIGBUS —
**1 of 75** frozen constructs "agreed", and that one only because its frozen
`EXPECT` is 0, which a broken binary also yields, so the non-vacuous count was
**0 of 72**.

**D5b (2026-09-09) repaired the witness and it now runs.** Steps 1-2 and part of
step 4 have landed; the non-vacuous count went **0 → 46 of 72**. Step 3 (`use`)
is scoped but NOT done (§7.2a). The full step ledger is §4a.

**D5b step 1 repaired the calling contract.** `emit_epilogue` never popped the function result off the eval stack
(§4a). One `pop(insns, n, 0)` moved the measurement to **44 of 75 raw, 43 of 72
non-vacuous**, and `fn main() -> i64 { 42 }` now returns 42 under the seed. That
is a working second code generator over a real subset — it is **not** yet
diverse double-compiling, because the two-stage form still cannot run (§4b-c)
and 29 non-vacuous constructs still diverge. `tools/ddc.sh --gate` stays RED and
stays out of `battery.sh` until the count is total.

## 1. Every artifact, named and hashed

Measured 2026-09-09 at HEAD `982855f`. Reproduce with `tools/ddc.sh --chain`.

| # | artifact | bytes | md5 | how it is produced | who gates it |
|---|---|---|---|---|---|
| 1 | `seed/seed.S` | 4,370 (159 lines) | `5ab2699bc6376575d7781ae766a77b51` | hand-written AArch64 asm | reviewed by eye; no gate on the SOURCE |
| 2 | `seed/build/seed` | 1,480 | `a86c4bb0e25babe680b96875df8c87a4` | `as seed/seed.S \| ld -static` | **`bench/vs_rust/invariants.sh` (viii)** |
| 2a | `seed/build/seed` `.text` | **488** | `759d3a7129c834ac90e675a83acd99d4` | `objcopy -O binary -j .text` | same rung: rebuilt `.text` must be `cmp`-identical |
| 3 | `seed/pack.py` | 810 | `875b385a78669594484166395e15cd9d` | CPython helper (build-time only) | none |
| 4 | `bebop.bp` | 354,550 (7,356 lines, 287 top-level fns) | `f0525dc8f263afb69721996d79daa764` | hand-written Bebop | typecheck census, std gates, fuzz + bpref |
| 5 | `selfhost/prelude/sha256.bp` | 4,814 | `30c3752e14600fe5f2afaea7b9136c54` | hand-written Bebop; pulled in by `bebop.bp` line 1 `use` | same |
| 6 | **`bebop.bin`** | **170,312** | `a5858877e8aa473f6c01eaebb81d693c` | `seed bebop.bin compile bebop.bp` — i.e. **by itself** | `tools/chain.sh --codegen`: gen3 == gen4 |
| 7 | `bench/golden/bebop-f86bee7.bin` | 301,052 | archived | an older promoted binary kept for reference | `bench/golden/bebop-f86bee7.sha256` |
| 8 | witness src `selfhost/attic/expr_compile.bp` | 125,625 (3,128 lines, 111 fns) | `16b9755aca7352e7bcfa3f79db8831b1` | hand-written Bebop; **retired 2026-09-04 (T45)** | `tools/ddc.sh` (new, D5) |
| 9 | witness driver `selfhost/attic/ec_driver.bp` | 2,465 | `bf14f351062cdeb845d1c7f74358efdb` | hand-written Bebop | same |
| 10 | witness binary `$BEBOP_TMP/ddc/witness.bin` | 74,696 | `b373ca59491adf1324f269e26f2719a7` | `seed bebop.bin compile (8 ++ 9)` | rebuilt and re-hashed on every `tools/ddc.sh` run |
| 11 | CPython | — | `Python 3.14.4` on this box | the OS package | **nothing. This is the largest un-gated element of the trust root.** |

**Two of the D5 row's own headline numbers are stale, and they are the numbers
the argument is made with.** `bebop.bin` is **170,312 bytes**, not 159,196 —
that figure comes from the 2026-09-08 research documents and the binary has
grown 7 % since. And the executable part of the loader is **488 bytes**, not
1,480; the remaining 992 bytes are the ELF wrapper. The honest sentence is
*"a 488-byte loader inside a 1,480-byte ELF, plus a 170 KB committed compiler,
plus CPython"*.

## 2. What was ALREADY gated before D5

This matters, because the D5 row reads as though the chain were ungated, and
part of it was not. Verified by reading the gates, not by trusting the row:

* **`seed.S` → `seed/build/seed`.** `bench/vs_rust/invariants.sh` step **(viii)**
  (landed as D12-D) runs `as seed/seed.S | ld -static`, `objcopy -j .text` on
  both the rebuild and the committed binary, and `cmp`s them. Link 1→2 of the
  chain is **already reproducible from source and already gated**; the claim in
  `docs/ANALYSIS-2026-09-06.md:165` that "no gate rebuilds it and compares" is
  **stale**. Comparing only `.text` is the right call: the ELF wrapper drifts
  with the linker version and the executed bytes do not.
* **`bebop.bp` + `bebop.bin` → `bebop.bin`.** `tools/chain.sh --codegen` requires
  gen3 == gen4 byte-exact; promotion additionally runs `invariants.sh --freeze`
  and the full battery. This gates **reproducibility**, and the tree says so.
* **`bebop.bin` semantics vs `bebop.bp` semantics.** Every std gate, construct
  parity row and fuzz case is checked against `tools/bpref.py`, an independently
  written CPython implementation of the language. This is a **real** independent
  check — it is differential testing, and it is precisely why CPython sits in the
  trust root rather than merely in the dev toolchain.

What is **not** gated, before D5 or after it: any check that the *code generator*
inside `bebop.bin` corresponds to the code generator described in `bebop.bp`, by
any route that does not pass through `bebop.bin` itself.

## 3. What D5 actually delivers

1. **This document** — the artifact chain with no gap (§1) and the limits stated
   in the same breath as the claim (§5).
2. **`tools/ddc.sh`** — `--chain` prints §1; `--why` prints the evidence for §4;
   `--measure` runs the witness against the frozen construct corpus and reports
   what it gets right; `--full` attempts Wheeler's two-stage DDC so the negative
   claim in §4 is falsifiable rather than asserted. `--gate` runs all of it and
   **exits non-zero with `DDC: NOT ESTABLISHED`**. It is deliberately NOT wired
   into `battery.sh` or `std_golden.sh`: a red gate nobody can turn green is a
   broken build, and the finding belongs in a document, not in the build.
3. **A refutation of the D5 row's premise**, below, with the disassembly.

## 4. Why DDC does not run here

Wheeler's DDC is two-stage, and byte-exactness comes from the second stage:

    W1 = witness(bebop.bp)     # a Bebop compiler carrying the WITNESS's codegen
    W2 = W1(bebop.bp)          # a Bebop compiler carrying BEBOP.BP's codegen
    md5(W2) == md5(gen4)       # byte-exact, because W2 emits what bebop.bp specifies

Stage 2 exists only if stage 1 succeeds. Three independent reasons it does not:

**(a) The witness targeted a retired execution model. REPAIRED, D5b step 1.** `selfhost/attic/expr_compile.bp`
was written against the `exec_words` stack machine, where "the stack machine only
clobbers x0/x1" and a runner holds the arena in x27/x28. For the smallest program
in the language, `fn main() -> i64 { 42 }`, it emitted 92 bytes that pushed the
result and never popped it:

    stp x29,x30,[sp,#-16]!  ;  mov x29,sp  ;  sub sp,sp,#0x4000
    mov x0,#42  ;  sub sp,sp,#0x10  ;  str x0,[sp]        <-- pushed, never popped
    add sp,sp,#0x4000  ;  ldp x29,x30,[sp],#16  ;  ret    <-- x30 = the pushed 42

`sp` was 16 bytes low across the epilogue, `ldp` reloaded x29/x30 from the wrong
slot, and the seed dies with **SIGBUS (exit 135)**. `bench/vs_rust/invariants.sh`
step **(ix)** records the model as retired: `push_words == 0`
(REGISTER-MODEL-BLUEPRINT §7). Measured over the frozen construct corpus:
**1 of 75 constructs agreed** with the reference compiler, and the one that
agreed (`c90_symalias`) has frozen `EXPECT=0`, which a broken binary also
produces — so the honest count was **zero**.

**Step 1 -- the repair is one line**: `emit_epilogue` now begins with `pop(insns, n, 0)`,
which puts the body's value in x0 (the register the seed prints) *and* restores
`sp` so the teardown lines up with `emit_prologue`. Re-measured on the same
corpus: **44 of 75 raw, 43 of 72 non-vacuous** (and `c90_symalias`, the old
vacuous agreement, now correctly shows as a divergence — confirming it was
worth nothing). The 31 remaining divergences fall into four classes: 11 produce
no output at all (frame/spill model drift), 5 return 0 because the feature is
absent (`use` ×2, `cas`, `sys_run`, enum payload), and 15 return a wrong value,
most of them surface added after the witness was retired (`return`, `break`,
unary `-`/`!`, `clz`, `crc32`/`crc32x`, `scan`).

**(b) It cannot read `bebop.bp` even in principle.** It has **no `use` support**
(zero matches for a use handler in its 3,128 lines) and `bebop.bp` line 1 is
`use "selfhost/prelude/sha256.bp"`. Two construct probes show the failure is
silent, not loud: `c44_use24` and `c47_usenest` "compile" through the witness to
**172-byte stubs**. And `selfhost/attic/ec_driver.bp` sizes its function table at
`zeros(256)` while `bebop.bp` has **287** top-level fns; Bebop array bounds are
unchecked, so the overflow corrupts the arena rather than raising.

**(c) Empirically it does not finish.** `tools/ddc.sh --full` on this box,
2026-09-09: stage 1 on the 359,331-byte inlined source returned **rc=124** —
timed out at 600 s without producing a W1.

**This was known.** `docs/ROADMAP-AUDIT*.md:316` blocks T89 on "witness compiler
must compile current surface — it does not: F-D". The D5 blueprint and the D5
ROADMAP row did not carry that forward, and their "the fix ADDS NO DEPENDENCY"
rests on a witness that does not do the job named.

**`tools/bpref.py` is not a substitute.** It is genuinely independent (CPython,
different author-time, different structure) and
`docs/RESEARCH-LITERATURE-2026-09-08.md:215` proposes it as the DDC witness. But
every `sys_*` builtin in it is a stub — `tools/bpref.py:659`,
`return 0  # ponytail: stubs; real fs/mmap/run/wait4 syscalls are out of the
fuzzed surface`. A DDC witness must read a source file and write a `.bin`; bpref
can do neither. It interprets programs; it does not compile them.

**So: is CPython still in the trust root after D5? Yes, unchanged.** Nothing in
this row removes it. `tools/bpref.py` remains the semantic oracle for every std
gate, construct row and fuzz case; `tools/typecheck.py`, `tools/census.py`,
`tools/bpref.py` and `seed/pack.py` are all CPython. D5 must not be quoted as if
it changed that.

### 4a. The D5b repair ledger

Each step was measured on its own before the next was started, so every number
below is attributable. Headline is the NON-VACUOUS count -- agreements on the
three constructs whose frozen `EXPECT` is 0 are excluded, because a binary that
traps or prints nothing also yields 0.

| step | change | raw | **non-vacuous** |
|---|---|---|---|
| — | before D5b | 1/75 | **0/72** |
| 1 | `emit_epilogue`: one `pop(insns, n, 0)` — the body's value was left on the eval stack and the teardown ran 16 B low | 44/75 | **43/72** |
| 2 | `ec_driver.bp` fn tables 256 → 1024 **with the loud trap the compiler already had** (`sys_exit(103)`); `compile_program_to` cap 256 → 1024, keeping its existing `brk #0x57` | 44/75 | **43/72** (no regression) |
| 4a | unary `-`, unary `!`, `0x` hex literals (T99) — all three fell through to `emit_num`, which consumed no characters and emitted literal 0 | 46/75 | **45/72** |
| 4b | array-literal allocation order + `clz` (T105) | 47/75 | **46/72** |
| 4c | `>>>` arithmetic shift (T42(b)) — correct surface, **net zero constructs**: `c32_asr` moved 100011 → 96148 against a frozen 96138 and still diverges on a precedence detail | 47/75 | **46/72** |

Step 2 is worth naming precisely because the scope was corrected mid-flight:
`compile_program_to` **already trapped loudly** on fn-table overflow; only
`ec_driver.bp`'s `zeros(256)` was silent. One trap was added, not two — and the
cap itself was wrong in both places (256 against `bebop.bp`'s 287 fns), so the
witness could never have read its own target even with `use` support.

### 4b. What was deliberately NOT repaired, and why

The witness is worth something **only because it is architecturally different
from `bebop.bp`** — a stack machine against a register model. Every repair that
makes it more like the compiler under test buys agreements and sells diversity.
Each change above keeps the witness's own model: evaluate, `pop` into x0, apply,
`push`. The array-literal fix is the clearest case — it was fixed by pushing the
base on the eval stack and using a post-indexed store, which is *more*
stack-machine, not less. No register-model behaviour was imported.

The 28 remaining divergences are therefore left standing, in three groups:

1. **10 produce no output** (`c23_spillcall`, `c25_matchtail`, `c26_selfrec`,
   `c33_loopalloc`, `c36_break`, `c38_frameheap`, `c78_scan`, `c90_symalias`,
   `c91_letlive`, `c94_fsync`) plus the wrong-value frame cases (`c21_param13`,
   `c43_arena_persist`, `c66_fncap`, `c69_index_roundtrip`, `c92_ptrfree`,
   `c95_symspan`). These are the witness's `x14`/`x15` frame-heap and spill model
   against the current one. **They are the ones that cannot be closed without
   importing the register model, and so they should stay divergent.** A
   construct made to agree by adopting the compiler-under-test's model agrees
   falsely: it no longer constitutes independent evidence about that model.
2. **6 need absent builtins/statements** — `crc32`/`crc32x` (a hardware CRC loop
   emitter), `cas`, `sys_run`, enum payload, and `return`/`break` (both need a
   forward branch patched to the epilogue). Additive, model-preserving, and the
   next cheapest work after `use`.
3. **2 need `use`** (`c44_use24`, `c47_usenest`) — §7.2a.

## 5. Exactly what is witnessed, and what is not

**Witnessed today:**

1. `seed.S` rebuilds `seed/build/seed`'s 488 `.text` bytes byte-exact
   (invariants (viii)).
2. `bebop.bp` compiled by `bebop.bin` reaches a byte-exact fixpoint
   (`chain.sh --codegen`, gen3 == gen4).
3. `bebop.bin`'s *observable semantics* agree with an independent CPython
   implementation on every std gate, construct row and fuzz case (`bpref`).

**Not witnessed:**

1. **That `bebop.bin`'s 170,312 bytes correspond to `bebop.bp`.** (2) is
   self-consistency; (3) is a semantic spot-check of the *compiled programs*,
   not of the compiler's own code generator, and it runs the compiler under test
   to produce what it checks. No second code generator exists in working order,
   so no diverse double-compiling has been performed. **This is the trusting-trust
   hole and it is fully open.**
2. **Even a revived `expr_compile.bp` would close a narrower gap than DDC
   proper**, for two reasons that survive any amount of debugging: the witness is
   *compiled by the compiler under test* (a Thompson attack that recognises the
   witness source defeats it), and two different code generators do not produce
   byte-identical output — measured here, **0 of 89** constructs and kernels
   compiled byte-identically by the two — so the comparison can only be on
   observable values, which catches a backdoor that changes a result and misses
   one that changes only the emitted bytes (timing, a hidden syscall, an
   unreachable payload).
3. **The toolchain below `as`/`ld`.** Step (viii) rebuilds `seed.S` with the
   box's binutils; those binaries are trusted, not gated.
4. **CPython.** See §4.

## 6. The wording that is allowed

Use these. They are what the tree supports.

* ✅ "Zero runtime dependencies." — `ldd bebop.bin` says "not a dynamic executable".
* ✅ "The trust root is a 488-byte loader (in a 1,480-byte ELF) whose source is
  gated to rebuild byte-exact, a 170,312-byte committed compiler binary that is
  reproducible from nothing but itself, and CPython for the semantic oracle."
* ✅ "`gen3 == gen4` is a reproducibility gate, not a trust gate."
* ✅ "`seed.S` → `seed` is gated; `bebop.bp` → `bebop.bin` is not."
* ❌ "The trust argument rests on 159 lines of assembly." — `seed.S` compiles
  nothing; its whole body is `openat → lseek → mmap(PROT_READ|EXEC) → arm arena
  → call`.
* ❌ "The fixpoint proves the binary matches the source."
* ❌ "Bebop is DDC-verified" / "the witness already exists in-tree". — §4.

## 7. What would actually close it

In increasing order of cost. None is an afternoon, and that is the finding:

1. **Repair the witness's calling contract** (pop the result into x0, restore
   `sp`), then measure again. This is the cheapest step and it is the one that
   turns `tools/ddc.sh --measure` from 1/75 into a real number. It is a change to
   a *retired* file targeting a *retired* execution model, so it is worth doing
   only as step 1 of item 2.
2a. **`use` handling (D5b step 3) — scoped, costed, NOT done.** The witness has
   no `use`, and `bebop.bp` line 1 is one. It cannot be added inside the
   compiler as a text substitution: the witness scans a `str` with
   `char`/`str_len` and Bebop has no string concatenation, so there is nowhere
   to splice the included source. The workable design is at the DRIVER level —
   `ec_driver.bp` already slurps the source, so it would scan for `use "..."`,
   build each path as cells, `sys_open` it and `sys_readbuf` the contents into
   ONE contiguous buffer ahead of the main source, then hand that single buffer
   to `compile_program_to`. That is ~60-80 lines of dense Bebop plus several
   correctness iterations. Doing it in the harness instead (as
   `tools/ddc.sh --full` does with `sed`) would be cheating: it would move a
   language feature into the test rig and make `c44_use24`/`c47_usenest` agree
   without the witness understanding anything.

2. **Widen the witness until it accepts `bebop.bp`** — add `use`, raise the
   driver's fn table past 287, port the post-T45 builtins, and make it fast
   enough to finish 359 KB of source. Then run the real two-stage DDC.
   `tools/ddc.sh --full` is the harness it plugs into; it already runs and
   already reports the failure. This is the row's stated cost, "freezing the
   surface subset the witness accepts", and it is the honest price.
3. **Give `tools/bpref.py` real `sys_*` syscalls** so the interpreter can be the
   witness. This removes hole §5.2's first half — the witness would no longer be
   built by the compiler under test — at the price of an unbounded interpreted
   self-compile and a deeper CPython dependency.
4. **A second, independently written compiler not built by `bebop.bin`.** This
   is what Wheeler actually requires and what Camlboot (arXiv:2202.09231) spent
   about a person-month on for OCaml.

Until one of those lands, this document — not the ROADMAP, not the MANIFESTO —
states the guarantee.
