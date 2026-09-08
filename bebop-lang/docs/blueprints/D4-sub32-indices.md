Status: 2026-09-08, owner main session, grounded at 0b823d0. From docs/RESEARCH-CORPUS-IDEAS-2026-09-08.md §7.3 (proposed there as A8b). CONDITIONAL on A8 and A9. PROPOSAL pending operator decision.

# D4 sub-32-bit column indices

## 0. The claim, and the row it depends on

After a locality relabel (D3), Diagonally-Addressed Matrix Nicknack (2307.06305)
reports that **"> 95 % of these matrices fit into the DA-CSR format using 16 bit
column indices"** (abstract). A `ci` slot then costs 2 bytes instead of 8.

This is the one place where i64-only cells are expensively wrong, and it is
*storage*, not arithmetic: `docs/RESEARCH-CORPUS-IDEAS-2026-09-08.md §7.3` reads
the corpus as "strong against i64 for storage, weak against it for arithmetic".

## 1. Why it must wait

- **A8** brings u32 cells; without typed tables there is no place to put a
  narrower cell.
- **A9** brings the NEON builtins. A scalar `ldrh` per slot may cost more cycles
  than it saves bytes: the decode-throughput law in 2606.22423 puts the ridge at
  roughly 20-27 bits at 12 GB/s, which says **u32 is safely bandwidth-bound but
  16-bit needs a vector decoder to stay ahead**.
- **D3** first, because the 95 % figure is conditional on the relabel.

Landing this before those three would be measuring a decoder, not a format.

## 2. Gate

Bytes per slot **<= 3** with ns/slot **not above** A8's u32 number, fold
identical.

## 3. Cheapest experiment that kills it

A 16-bit-`ci` variant of the frontier BFS against the u32 one, same graph, same
slot. **If ns/slot rises at all, the row waits for A9's NEON decoder** rather
than being argued about.

## 4. Cost against the invariants

Zero dependencies: none. One-pass: one extra `ldrh` word on the access path
under the typed-table dispatch A8 introduces. D0: none.
