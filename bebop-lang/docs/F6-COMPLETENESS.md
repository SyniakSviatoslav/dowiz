# F6 completeness (and soundness) — measured 2026-09-11

Claim: the hinted certificate format loses no theorems (complete) and
`tcheck.bp` accepts only genuine refutations of the stated obligation (sound).

## Argument

1. **Resolution is complete for UNSAT.** Standard theorem: every unsatisfiable
   CNF has a resolution refutation ending in the empty clause.
2. **Hints are always available.** The producer (today `tools/certgen.py` via
   CaDiCaL LRAT, tomorrow `bebop.bp`) walks its OWN proof and emits the
   antecedents in resolution order. No search is moved into the certificate;
   the order the solver used is the order written down. Hence every UNSAT
   obligation has an acceptable certificate — the format cannot make a true
   claim unprovable.
3. **Each step is re-derived, not trusted.** Both checkers assume the negation
   of the claimed clause and walk the hints: every antecedent must be unit
   (extends the assignment) or falsified (closes the chain). A step that ends
   without conflict, cites a satisfied clause, or has no hints is rejected.
   An unsound step is therefore a malformed certificate, never an accepted one.
4. **Replay is bound out (15.1).** The `x` line carries sha256 over the
   canonical obligation text; a valid certificate filed against a different
   obligation is rejected by both checkers.

## Evidence (this box, bebop.bin 174508 B, /tmp/f6 corpus)

| case | certcheck.py | tcheck.bin (from selfhost/tcheck.bp) |
|---|---|---|
| t1 valid 1-step refutation | OK, 1 step | OK, steps=1, empty=yes |
| t2 valid 2-step chain via derived clause | OK, 2 steps | OK, steps=2, empty=yes |
| n1 unhinted step | reject rc=2 | reject rc=2 |
| n2 satisfied antecedent first | reject rc=2 | reject rc=2 |
| n3 empty never derived | reject rc=2 | reject rc=2 |
| n4 missing x line | reject rc=2 | reject rc=2 |
| t1 bound to matching .obl (oid 304026d6…) | bound | bound (Bebop sha256 == hashlib) |
| t1 cert replayed vs other.obl | OID MISMATCH rc=2 | reject rc=2 |

Differential agreement: **8/8**. The bound case additionally cross-validates
`tcheck.bp`'s `canonical_text` + `sha256_words` against Python hashlib to the
hex digit. Merkle root (15.3) verified through `certcheck.py --merkle` /
`--verify-root`.

## Type-checker leg (term layer reference)

`tools/kcheck.py --corpus bench/kernel_neg`: **kernel_neg 0/13 accepted,
kernel_pos 3/3 accepted, internal 0/16.** The kernel parity target (F7 term
layer inside `tcheck.bp`) reads 0/16 — honest starting value, not a failure.

## Limits (not hidden)

- Certificates here are hand-written small refutations: no CaDiCaL on this
  box (`lean/bin/cadical` absent), so the full certgen→cadical→certcheck
  pipeline is untested here and deferred to F5's producer.
- `tcheck.bp` holds the CERTIFICATE layer only; the term/type layer is F7.
