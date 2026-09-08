Status: 2026-09-08, owner main session, grounded at 0b823d0. From docs/RESEARCH-CORPUS-IDEAS-2026-09-08.md §7.1 -- the strongest published argument against a stated commitment. Revives TASKS.md T89, OPEN. PROPOSAL pending operator decision. NO performance effect: this row buys honesty, not speed.

# D5 the trust chain, and what `gen3 == gen4` does not prove

## 0. The claim that does not survive

This project says its trust rests on 159 lines of assembly. **It does not, and
the tree says so if read carefully:**

- `seed/seed.S` line 1 calls itself a **"FROZEN Bebop loader. Zero C."** Its
  whole body is `openat -> lseek -> mmap(PROT_READ|EXEC, MAP_PRIVATE) -> arm
  arena -> call`. It compiles nothing.
- the seed binary is **1,480 bytes**; the compiler it loads, `bebop.bin`, is
  **159,196 bytes and is checked into git**.
- so today's trust root is a committed 159 KB binary that a 1.5 KB loader mmaps
  and jumps into, plus CPython for the `bpref` oracle.

And the fixpoint does not close the gap: **`gen3 == gen4` proves
self-consistency, not trustworthiness.** Thompson's attack (reproduced and
formalised in 1004.5534) survives any fixpoint by construction -- a compiler that
inserts a backdoor into itself reproduces that backdoor byte-for-byte, and the
md5s agree.

The project already knows the answer. **T89 -- "trust chain + diverse
double-compiling without C" -- is OPEN** in `TASKS.md:100`, and neither
`tools/ddc.sh` nor `docs/TRUST-CHAIN.md` exists.

## 1. Why this is cheap rather than expensive

Diverse Double-Compiling needs a SECOND, independent compiler for the same
source language -- and this tree already has a witness: `selfhost/expr_compile.bp`
(the one `pool_parity` and `gb_compile1` build on). DDC does not require the
witness to be fast, complete, or pleasant; it requires it to be INDEPENDENT and
to compile a frozen subset.

**The fix therefore adds no dependency.** That matters: the commitment being
defended is zero dependencies, and the defence must not violate it.

## 2. Mechanism

    W1 = witness(bebop.bp)        # a second, independent compiler
    W2 = W1(bebop.bp)             # the claimed compiler, rebuilt through it
    assert md5(W2) == md5(gen4)   # the golden fixpoint, byte-exact

If W2 differs, either the witness is wrong or the shipped binary contains
something its source does not. `docs/TRUST-CHAIN.md` then lists every hash in
the chain: seed.S -> seed binary -> witness -> W2 -> the promoted `bebop.bin`.

## 3. Gate

`tools/ddc.sh` prints **W2 == golden fixpoint, byte-exact**, and
`docs/TRUST-CHAIN.md` names every artifact hash. Nothing else changes.

## 4. What it costs

Zero dependencies: none. One-pass: none. Performance: none. The real cost is
FREEZING the surface subset the witness must accept, and keeping it frozen as
`bebop.bp` grows -- which is why this is a row and not an afternoon.

## 5. Until it lands

The honest wording, which the ROADMAP and MANIFESTO should use, is: *the trust
root is a 1,480-byte loader plus a checked-in 159 KB binary; the 159-line
`seed.S` is a loader, not a bootstrap; `gen3 == gen4` is a reproducibility gate,
not a trust gate.* Anyone repeating "the trust base is 159 lines" -- including
this session, which did so more than once today -- is overstating it.
