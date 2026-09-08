Status: 2026-09-08, owner main session, grounded at 0b823d0. From docs/RESEARCH-CORPUS-IDEAS-2026-09-08.md §4 (proposed there as B4b). PROPOSAL pending operator decision; the payoff is PREDICTED, not measured.

# D3 locality relabelling, paid for by a pass the store already runs

## 0. Why this is nearly free here

Vertex ids in the store are whatever the writer assigned -- sgraph2 uses an LCG
pair stream, W's order log will use arrival order -- and no row changes them.
The graph literature's cheapest locality win is to renumber vertices so
neighbours are near in memory, and it is normally an extra pass nobody wants to
pay for.

**Here it is already paid for twice over:** compaction is a Cheney copy that
assigns every live object a new offset (`store.bp:355-416`), and the BFS order
that most reorderings want is exactly what the frontier kernel produces at
45 ns/slot. The permutation rides along in a pass that already reads every live
object.

There is a second reason on this box specifically: pages are 4 KB and THP reads
`[never]`, so a 10M-slot `ci` array spans ~20,000 pages against an A78 L2 TLB of
order a thousand entries. **Relabelling is the only lever on TLB locality an
unprivileged process has.**

## 1. What the literature says (from abstracts)

Recursive Graph Bisection (1602.08820): "graph reordering is a powerful
technique to increase the locality". BOBA (2306.10410): a linear-time ordering
that "can substantially speed up the conversion from a COO representation to the
compressed format CSR" -- so it also helps the build D1 is about. RCM++
(2409.04171) on the starting-node heuristic; 2012.10026 on RCM plus SIMD for
Graph500 BFS. Memory Hierarchy Sensitive Graph Layout (1203.5675): blocking
across cache line, TLB entry and DRAM page.

**And one warning that must not be skipped:** A Closer Look at Lightweight Graph
Reordering (2001.08448) reports that hot-vertex reorderings "may inadvertently
destroy the inherent community structure". Degree-sort alone can LOSE. That is
why the gate below requires the fold to be identical and the win to be measured,
not assumed from the shape.

## 2. Mechanism

A permutation `pi` -- BFS order from a chosen root, or reverse Cuthill-McKee,
which is BFS with degree-sorted neighbours -- applied during `st_compact`:

    ci'[k] = pi[ci[k]]        rows copied in pi order
    rp rebuilt by the same counting pass the CSR build already uses

O(E) once per compaction, inside a pass that already touches everything.
Entities that carry the id (W's orders) get a forwarding cell in the migration
table, which is the mechanism `LANG-DB §4d` already uses for type changes.

## 3. Gate

Frontier BFS ns/edge-slot on the 1M/10M sgraph2 graph **<= 0.7x** after relabel,
**fold identical**; compaction wall **<= 1.5x** of today's 747 ms row; PK lookup
unchanged.

## 4. Cost against the invariants

Zero dependencies: none. One-pass: none -- this is data, not code. D0: none. A
peer node reordering its own local store changes nothing on the wire, because
`MANIFESTO §2` makes identity `H(pq_pub || classical_pub)`, not a store offset.

**The caveat that decides the row:** W is an order log, and time of arrival is
already a good locality order for it. The win is for the GRAPH phases -- BFS, CC,
TC over the order-courier-merchant graph -- and it must be measured there, not on
the log.

## 5. Cheapest experiment that kills it

No store change at all: in sgraph2's generator, assign vertex ids in BFS order of
the generated graph (a python oracle already computes `bfs_levels`) instead of
LCG order, and rerun the frontier row once under a slot.

**If ns/edge-slot moves less than 20 %, random-id locality was not the cost and
this row dies.** If it moves, the store-side implementation is ~60 lines in
`st_compact`.
