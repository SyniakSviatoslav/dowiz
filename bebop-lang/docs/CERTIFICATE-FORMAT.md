# Bebop Certificate Format Specification

Status: 2026-09-14 — normative specification for the text certificate format
used by `tcheck.bp` (F6) and `tools/certcheck.py` (reference implementation).
2026-09-14: sections 3.2/3.3 gain the CLAUSE STRIDE and HINT BOUND, and section
9 gains requirement 7, THE GOAL CHECK. All three were already enforced by
`tcheck.bp` and none was enforced by `tools/certcheck.py`; the two committed
certificates that exploited the gap are `bench/cert_neg/n05_wrong_clauses.cert`
and `bench/cert_neg/n06_clause_too_wide.cert`.

This is the ONLY normative document for the certificate format. Text form is
normative; any binary cache carries this text's sha256 and is re-derived,
never trusted.

## 1. Overview

A Bebop certificate is a text file that discharges one QF_ABV obligation
(variables are 64-bit bitvectors; arrays eliminated in the VC generator).
The certificate contains:

1. A CNF problem declaration.
2. Input clauses from the bit-blaster.
3. Derived clauses, each with **mandatory hints** naming antecedents in
   resolution order.
4. Optional deletion records.
5. An obligation binding (`x` line) tying the certificate to exactly one
   obligation via sha256.

**Central property:** hints make checking a loop. Every derived clause names
its antecedents in resolution order, so the checker walks a linear chain with
no unit propagation, no watched literals, no occurrence lists, no search.
That is why the Bebop checker can be tens of functions rather than a solver.

## 2. Text Form (Normative)

### 2.1 Encoding

- Line endings: LF only (byte `0x0A`).
- Comments: lines starting with `%` (byte `0x25`) are not hashed and are
  ignored by the checker.
- Blank lines: ignored by the checker; not hashed.
- Tokens: separated by whitespace (space `0x20` or tab `0x09`).
- No trailing whitespace after the last token on a line.

### 2.2 Canonical Text

The sha256 of an obligation or certificate is taken over its **canonical
text** (section 5, 6). Canonical text is computed identically in
`tools/certgen.py`, `tools/certcheck.py`, and `selfhost/tcheck.bp`:

1. Split on LF.
2. Strip each line.
3. Remove blank lines and comment lines (starting with `%`).
4. Collapse whitespace between tokens to a single space.
5. Join with LF, append final LF.
6. Encode as UTF-8 bytes.

**MUST match exactly** between producer, checker, and all tools. Any change
invalidates all committed oids and Merkle roots.

## 3. Record Types

Every non-comment, non-blank line starts with a single-character tag
followed by tokens separated by whitespace.

### 3.1 `p` — Problem Declaration

```
p <nvars> <nclauses>
```

- `nvars`: total number of CNF variables (positive integer).
- `nclauses`: total number of input clauses (informational; the checker does
  not enforce this count against the actual number of `c` lines).

Exactly one `p` line per certificate. Must appear before any `c` or `r` lines.

### 3.2 `c` — Input Clause

```
c <id> <lit1> <lit2> ... <litN> 0
```

- `id`: unique positive integer identifying this clause.
- `lit1..litN`: signed integers; positive = variable asserted, negative =
  variable negated. Each `|lit|` is in `[1, nvars]`.
- Terminated by `0`.
- **At most 64 literals** (the CLAUSE STRIDE). A checker MUST REFUSE a wider
  clause, never store a prefix of it: a truncated clause is a DIFFERENT clause,
  and a resolution chain checked against it is a chain checked against the wrong
  formula. `tcheck.bp` refuses at `add_input`/`add_derived` (`cl_stride`, a
  runtime value in `n_cl[2]`, 64 on the certificate path); `certcheck.py`
  refuses with `MAX_CLAUSE_WIDTH`.

**All `c` lines MUST precede the first `r` line.** The input block is what the
goal check (section 9.7) compares against the obligation; an input clause
smuggled in after a derivation step would be an unchecked axiom, which is the
hole the goal check exists to close.

Input clauses are asserted as axioms (always true under the given encoding).
They come from the bit-blasting of the 64-bit operators.

### 3.3 `r` — Derived Clause (Resolution Step)

```
r <id> <lit1> ... <litN> 0 <hint1> ... <hintM> 0
```

- `id`: unique positive integer for the derived clause.
- `lit1..litN`: literals of the derived clause, terminated by `0`.
- `hint1..hintM`: clause ids of antecedents, terminated by `0`.

**Hints are mandatory.** Every derived clause MUST have at least one hint.
An unhinted step is a malformed certificate — the checker exits with error.

**At most 1024 hints per step** (the HINT BOUND), and at most 64 literals in the
derived clause (section 3.2's stride). A checker MUST REFUSE past either bound
rather than truncate: `tcheck.bp` SILENTLY TRUNCATED hint lists until `1fa8df8`,
and a reference checker that walks a longer chain than the trusted one is not a
twin of it.

The checker verifies the derivation by:
1. Assuming the negation of the derived clause (each literal `l` sets
   `assign[|l|] = (l < 0)`).
2. Walking hints in order. For each hint clause:
   - If **satisfied** under the assignment: error (bad hint order).
   - If **falsified** (all literals false): conflict found, chain closes.
   - If **unit** (exactly one unassigned literal): extend the assignment
     with that literal's value.
   - If **more than one unassigned literal**: error (not unit).
3. If the chain ends without a conflict: error (derivation incomplete).

### 3.4 `d` — Deletion

```
d <id1> <id2> ...
```

Removes the named clauses from the database. Freed clause ids may be reused
by later clauses (though this is uncommon).

### 3.5 `x` — Obligation Binding (15.1)

```
x <oid>
```

- `oid`: exactly 64 lowercase hexadecimal characters, the sha256 of the
  obligation's canonical text.

Every certificate MUST contain exactly one `x` line. This binds the
certificate to exactly one obligation and prevents replay against a different
obligation (the attack 15.1 exists to stop).

## 4. Clause IDs

- Clause ids are positive integers.
- Each id appears at most once in a certificate (no duplicates).
- Input clauses (`c`) and derived clauses (`r`) share the same id space.
- Deleted clauses (`d`) may be reused.

## 5. Obligation Binding (15.1)

The `x` line carries sha256 of the obligation's canonical text. To verify:

1. Read the obligation file.
2. Compute `canonical_text` (section 2.2).
3. SHA-256 the resulting bytes.
4. Compare the hex digest to the certificate's `x` line.

**Without an obligation**, the derivation is still checked but the certificate
is unbound. A gate must always pass the obligation.

## 6. Obligation Format

Obligations are DAG-structured, one node per line:

```
% comment
obligation <name>
<id> var <name>           -- fresh 64-bit variable
<id> const <decimal>      -- 64-bit constant (two's complement)
<id> add|and|or|xor <a> <b>
<id> not <a>
<id> shl|lshr <a> <k>     -- k literal shift amount 0..63
<id> eq <a> <b>           -- boolean node (one bit)
prove <id>
```

- All node args reference strictly smaller ids (one forward pass checks
  well-formedness and acyclicity).
- `prove <id>` names the boolean goal node.

## 7. Merkle Root over Obligations (15.3)

A Merkle root over a set of obligations makes the gate one number:

1. For each obligation `obl_i`, compute `leaf_i = sha256(canonical_text(obl_i))`.
2. Sort `leaf_0, leaf_1, ...` lexicographically by byte value.
3. Concatenate all sorted leaves.
4. `root = sha256(leaf_0 || leaf_1 || ... || leaf_n)`.

Properties:
- Empty set: root = `000...000` (64 zero hex chars).
- Single obligation: `root = sha256(sha256(canonical_text(obl)))`.
- Deterministic regardless of file order.

The root is used by `--verify-root` in `certcheck.py` and by the gate chain
to make the entire obligation set a single number.

## 8. Trust Model

### Trusted
1. `seed/seed.S` (1,480 bytes): loads and jumps.
2. `tcheck.bp` source and binary: the checker itself.
3. The VC generator: code that says WHAT to prove.
4. The bit-blaster's definition of 64-bit operators.
5. The LRAT soundness argument.

### Not Trusted (Producers)
- Lean 4, its kernel and compiler.
- CaDiCaL or any SMT solver.
- The elaborator.
- `tools/certgen.py` (the bridge producer, will be deleted).

A wrong producer yields a rejected certificate, never an accepted false one.
This is the de Bruijn criterion.

## 9. Checker Requirements

A conforming checker MUST:

1. Parse `p`, `c`, `r`, `d`, `x` records.
2. Enforce clause id uniqueness.
3. Enforce that every `r` line has hints.
4. Walk each hint chain as described (section 3.3).
5. Verify the empty clause is derived.
6. Verify the `x` line matches the obligation's sha256 (when given).
7. **THE GOAL CHECK: re-blast the obligation and verify that the certificate's
   input clauses ARE that CNF** — same count, same order, same literals
   (positional comparison; the blaster and the producer emit gates in the same
   sequence, which is what the alignment bought). When given.
8. Enforce the clause stride (3.2) and the hint bound (3.3) by REFUSING.
9. Exit with non-zero code on any violation: `2` malformed or unverifiable, `3`
   the clauses do not encode the bound obligation.

Requirement 7 is not optional decoration. Requirements 4 and 5 prove the
certificate refutes SOME CNF; requirement 6 proves the checker was handed SOME
obligation TEXT. **Neither proves the CNF encodes that text.** Until 2026-09-14
`certcheck.py` stopped at 6, and `bench/cert_neg/n05_wrong_clauses.cert` — the
clauses and the resolution chain of `or is commutative` carrying the `x` line of
`and is commutative` — replayed perfectly and printed `certcheck: OK`. This is
why the bit-blaster lives INSIDE the checker (trust model, section 8, item 4)
rather than in the producer, and why `certcheck.py` carries its own copy of it
instead of importing `tools/certgen.py`'s: a checker that asks the untrusted
producer what the clauses should be has asked the wrong party.

A conforming checker MUST NOT:
- Perform unit propagation over the whole database.
- Search for proofs.
- Trust any producer output.
- **Truncate** anything it was given. See 3.2 and 3.3.

## 10. Error Messages

Error messages MUST include:
- The line number where the error occurred.
- The clause id involved (for `r`/`c` errors).
- A description of what went wrong.
- For hint errors: the expected vs actual state.

## 11. Versioning

The format is versioned by the document date (2026-09-09). Changes to
canonical text computation, record types, or binding rules invalidate
all committed certificates and require re-generation.

## 12. Files

| File | Role | Lines |
|---|---|---|
| `tools/certgen.py` | Certificate producer (untrusted, will be deleted) | 393 |
| `tools/certcheck.py` | Reference checker (untrusted, specification) + its own bit-blaster | 698 |
| `selfhost/tcheck.bp` | Trusted checker (Bebop, in-tree) | 4124 |
| `tools/certcheck_census.py` | The gate: prints `cert_checked` and `checker_neg` | 146 |
| `bench/cert_pos/**` | Obligations with certificates that MUST check | 7 pairs |
| `bench/cert_neg/**` | Deliberately corrupted certificates that MUST be refused | 6 |
| `docs/CERTIFICATE-FORMAT.md` | This document | ~280 |
| `docs/RESEARCH-VERIFICATION-2026-09-09.md` | Design rationale (sections 7, 11-15) | 311 |

## 13. Measured reach of the format (2026-09-14)

What the 64-literal clause stride actually costs, measured rather than assumed:

| obligation | vars | input clauses | hinted steps | max clause | checks |
|---|---|---|---|---|---|
| `arith_add` (1+1=2) | 1 | 2 | 1 | 1 | yes |
| `shl3_mul8` (x*8 == x<<3) | 65 | 2 | 1 | 1 | yes |
| `lshr_logical` ((x<<1)>>1 == x & 2^63-1) | 65 | 2 | 1 | 1 | yes |
| `de_morgan` (~(x&y) == ~x \| ~y) | 384 | 831 | 194 | 3 | yes |
| `and_comm` (x&y == y&x) | 384 | 831 | 257 | 3 | yes |
| `neg_twos` (0-x == ~x+1) | 440 | 1316 | 640 | 61 | yes |
| `mul_three` (x*3 == x + (x<<1)) | 564 | 2178 | 1403 | 33 | yes |
| `add_assoc` ((x+y)+z == x+(y+z)) | 1080 | 3979 | 20839 | **179** | **NO — past the stride** |

The first three are discharged by the BLASTER, not by the solver: both sides
fold to the same literal vector during bit-blasting, so the CNF is `{T}, {¬T}`
and the whole proof is one step. That is a real result about the encoding rather
than a vacuous one — but only since requirement 7 landed. Before it, ANY
two-clause tautology carrying the right `x` line passed those three rows, which
is exactly the forgery `docs/GATE-PROVENANCE-AUDIT-2026-09-13.md:281` described.

`add_assoc` is the boundary: true, cheap for cadical, and uncertifiable here.
The limit that binds is the clause stride (179 > 64), not the 16384-clause store
and not the solver.
