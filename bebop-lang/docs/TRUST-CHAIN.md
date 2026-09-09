Status: 2026-09-09 CURRENT (D5 / T89, lane C, session 30). Grounded at HEAD 982855f. Every number below was measured on this box on 2026-09-09 by `tools/ddc.sh` (`--chain`, `--why`, `--measure`, `--full`); nothing is quoted from an older document, and where an older document's number is now wrong this file says so. This file is the authority on what the project's trust claim IS. The MANIFESTO and the ROADMAP must not exceed it.

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
the row's central premise is false.** The row says the witness "already exists
in-tree (`selfhost/expr_compile.bp`), so the fix ADDS NO DEPENDENCY". Measured:
the witness emits code for the **retired `exec_words` stack machine** and its
output does not run — it fails on `fn main() -> i64 { 42 }` with SIGBUS (§4).
What D5 delivers is therefore this document, the measurement behind it, and
`tools/ddc.sh`, which is **RED by design** and says so rather than printing a
green line. The chain is better documented and **no more witnessed than before**.

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

**(a) The witness targets a retired execution model.** `selfhost/attic/expr_compile.bp`
was written against the `exec_words` stack machine, where "the stack machine only
clobbers x0/x1" and a runner holds the arena in x27/x28. For the smallest program
in the language, `fn main() -> i64 { 42 }`, it emits 92 bytes that push the
result and never pop it:

    stp x29,x30,[sp,#-16]!  ;  mov x29,sp  ;  sub sp,sp,#0x4000
    mov x0,#42  ;  sub sp,sp,#0x10  ;  str x0,[sp]        <-- pushed, never popped
    add sp,sp,#0x4000  ;  ldp x29,x30,[sp],#16  ;  ret    <-- x30 = the pushed 42

`sp` is 16 bytes low across the epilogue, `ldp` reloads x29/x30 from the wrong
slot, and the seed dies with **SIGBUS (exit 135)**. `bench/vs_rust/invariants.sh`
step **(ix)** records the model as retired: `push_words == 0`
(REGISTER-MODEL-BLUEPRINT §7). Measured over the frozen construct corpus:
**1 of 75 constructs agree** with the reference compiler, and the one that agrees
(`c90_symalias`) has frozen `EXPECT=0`, which a broken binary also produces. The
honest count of constructs this witness witnesses is **zero**.

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
