# Bebop Certificate Format Specification

Status: 2026-09-09 — normative specification for the text certificate format
used by `tcheck.bp` (F6) and `tools/certcheck.py` (reference implementation).

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
7. Exit with non-zero code on any violation.

A conforming checker MUST NOT:
- Perform unit propagation over the whole database.
- Search for proofs.
- Trust any producer output.

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
| `tools/certgen.py` | Certificate producer (untrusted, will be deleted) | 302 |
| `tools/certcheck.py` | Reference checker (untrusted, specification) | 220 |
| `selfhost/tcheck.bp` | Trusted checker (Bebop, in-tree) | ~350 |
| `docs/CERTIFICATE-FORMAT.md` | This document | ~200 |
| `docs/RESEARCH-VERIFICATION-2026-09-09.md` | Design rationale (sections 7, 11-15) | 311 |
