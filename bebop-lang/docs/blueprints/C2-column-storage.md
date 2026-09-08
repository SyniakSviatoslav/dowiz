Status: 2026-09-08, owner main session, grounded at 1ed3719 / bebop.bin 7939ad7e. Derived from docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md §3 (proposal 2, ECS, verdict NARROW). Attaches to ROADMAP A8. PROPOSAL pending operator decision.

# C2 per-type column storage -- "columns per type, not instead of types"

## 0. Goal

A type declares its storage as `column` or `object`. Scan-shaped tables become columns with **one
header per column instead of one per row**; entity-shaped types stay objects. This is the
survivable half of the operator's ECS proposal.

Gate (the size row, which is the falsifiable one): 1M three-field records stored as columns land
**<= 1.2x sqlite's 34.1 MB, i.e. <= 41 MB**, where objects land at 72.4 MB after compaction. And
the point-lookup row must **not regress past 2x** of today's 450 ns -- if it does, that type stays
an object.

## 1. The win nobody had claimed for it here

Every store object pays a 16-byte header: a length+digest word and a crc+generation word
(`selfhost/prelude/store.bp:3-5`). A three-field record is therefore 40 bytes of which 16 are
header -- 40 % overhead. The G7 file-size row is exactly where this shows: the store **loses
2.5x/2.1x to sqlite** (ROADMAP.md:181), and `LANG-DB-DESIGN.md §0.10` already predicted that loss
and attributed it to i64 cells against sqlite's varints. The header is the part nobody costed.

A column table stores one header per COLUMN. At 1M rows and 3 fields that is 48 bytes of header
in total instead of 16,000,000.

This is not a new mechanism. The graph library is already SoA: a `GbMatrix` is separate `rp`/`ci`/
`vv` arrays (`selfhost/std/gb.bp:8-10`), and `RESEARCH-GRAPHBLAS-2026-09-06.md:82-83` records
`arr i64` per column as the representation. C2 makes that a **declarable property of a type**
rather than a hand-rolled shape.

## 2. The loss, stated honestly

A point read of an entity with k components is **k random cache lines instead of 1**. That is the
whole reason "no records" is refuted rather than adopted: the thesis sentence and the point-lookup
row both depend on records existing. `LANG-DB-DESIGN.md:431-432` already measures the shape of
this for the window query ("{u,v} pairs: -8 lines" for AoS).

So the rule is per type, decided by shape:

| shape | storage | example |
|---|---|---|
| scanned end to end, few fields touched per pass | `column` | W's order-event log; `lineitem` in B7 |
| read one entity at a time, most fields touched | `object` | the FSM state per order; `GbMatrix` headers |

## 3. Design

The layout string already carries the per-type choice pattern (`LANG-DB-DESIGN.md §4d:224`, "the
user's choice per type"), and the compiler already digests layouts
(`selfhost/prelude/store.bp:280-285`).

A column table is a header object `{n, ref col_0, ..., ref col_k}` pointing at k+1 payload
objects. **That is a `GbMatrix`-shaped header**, which `gb.bp:8-10` already is -- so the object
graph needs no new kind, only a layout tag saying "these refs are columns of one table of length
n".

Consequences that must be designed, not assumed:
- **append** touches k+1 objects instead of 1; under B4's CoW that is k+1 row-block copies, and
  B4's block granularity is what decides whether that is cheap. This is the interaction to check
  first.
- **compaction** (the Cheney copy, `store.bp:355-416`) walks refs; columns are ordinary refs, so
  it needs no change.
- **A7's `(off,len)` byte handle** gives variable-width fields a home without breaking the 8-byte
  cell, which is why "raw-byte components" is refuted separately: it is already solved.

## 4. Files and functions touched

| file | change |
|---|---|
| `selfhost/prelude/store.bp` | the layout tag; a `st_col_*` accessor family over the header object |
| `bebop.bp` (A8's typed tables) | `column` in the type surface, so `[T]` access on a column resolves through the header |
| `bench/vs_rust/` | the G7 size row measured for both layouts; the point-lookup row for both |
| `tools/bpref.py` | mirror |

## 5. Steps

1. **The refutation first** (§7). It is ~30 lines and it can kill the row before any design lands.
2. Layout tag + accessors in `store.bp`, no compiler change: a column table built by hand, the
   G7 size row measured both ways.
3. Surface syntax under A8.

## 6. Cost against the invariants

D0: none -- no dependency, no network, no crypto surface, and the store stays position-
independent. Zero-dependency cost: none. The one real cost is B4's: k+1 CoW copies per append,
which step 1 must not skip.

## 7. Cheapest refutation, and it exists in the tree today

The T100 window query has **both layouts already**: `nnidx.bp` is SoA arrays, and
`LANG-DB-DESIGN.md §8` estimates the `{u,v}` AoS record. Write the AoS variant of `nnidx.bp`
(~30 lines) and measure the 4.0 us window row.

If AoS is **not** faster on the point query, the "k random lines" cost is being hidden by
out-of-order overlap, and pure columns are safe for this workload too -- which would widen C2 from
"per type" to "columns by default". If AoS **is** faster by more than 2x, the per-type rule stands
and the entity types keep objects.

Either outcome is worth having, and it costs 30 lines. Run it before anything else in this row.
