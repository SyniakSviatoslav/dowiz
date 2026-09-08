Status: 2026-09-08, owner main session, grounded at 1ed3719. Derived from docs/RESEARCH-BYOK-ECS-DATAFLOW-2026-09-08.md §8 (proposal 7, "memory as Git", verdict ALREADY with three gaps). A B4 step-3 footnote, not a phase of its own. PROPOSAL pending operator decision.

# C3 commit chain, heads, and st_open_at(gen)

## 0. What is already true, so that the gap is visible

The operator's "memory as Git" is the store's design sentence, already implemented:

| Git idea | in the tree | file:line |
|---|---|---|
| immutable append-only history | append-only bump arena, objects written once | `selfhost/prelude/store.bp:1-14` |
| atomic commit | two superblocks, root swap, write the header last | `store.bp:1-14`, `st_commit_m` |
| never mutate in place | CoW-append MVCC with exact freed accounting | `selfhost/std/mvcc.bp:3-4`, `prev` edge `:53` |
| a reader sees one whole world, no locks | the mapping IS the reader token; the kernel's inode refcount releases it | `LANG-DB-DESIGN.md:200-206` |
| GC only when no reader holds the old world | compaction = Cheney copy into a fresh file + rename | `store.bp:355-416` |
| time travel | B4's `prev` row | `docs/blueprints/B4-functional-tensor-updates.md:22-23, :37-38, :61` |

Two claims in the proposal are therefore already answered and must not be re-litigated:
**reader registration is not needed** (the inode refcount does it), and **"MVCC's undo logs and
locks wreck graph performance" does not apply** -- this store has no undo log, no redo log and no
reader table (`RESEARCH-NOPOINTERS-SQL:177-179`).

The cost is also already on the books, measured, not feared: the file grows by every update until
compaction -- **85.2 MB after 10^5 updates against sqlite's 34.1 MB** (ROADMAP.md:181).

## 1. The three gaps

1. **No commit-object chain.** The superblock keeps generation `g` and `g-1` only
   (`store.bp:9-12`); older roots survive only through a type's own `prev` edge. So "check out
   commit N" is not expressible for the store as a whole.
2. **No named heads/branches.** There is one `root` cell.
3. **No `st_open_at(gen)`.** A reader can take the live root or nothing.

## 2. Design

The superblock has free cells 9..14 (`st_sb_write_m` zeroes them). C3 spends two of them, and note
that **B1's follow-up card already claims cell 9** for its verification anchor -- so C3 starts at
cell 10 and the two rows must be landed in a known order.

- **cell 10 = `commit`**: the offset of a commit OBJECT written by every `st_commit`, holding
  `{gen, root, prev_commit_off, timestamp?}`. That is one extra small object per commit, appended
  like any other, and it makes the history a real chain rather than a two-deep window.
- **cell 11 = `heads`**: the offset of a heads table object, `{name_digest -> commit_off}`. Absent
  (0) means "one unnamed head", i.e. today's behaviour.
- **`st_open_at(base, gen, tmp)`**: walk the commit chain from the live commit to the requested
  generation and return that root. O(commits walked), which is the honest cost of a linked list
  and the reason a heads table exists at all.

~60-100 lines in `store.bp`, per the research document's estimate.

## 3. Gate

- A store committed N=1000 times, then reopened at generations 1, N/2 and N: each returns a root
  whose fold equals the fold recorded at that generation (the folds are captured during the
  writing loop, so the test is self-checking).
- `st_open` of the live generation is byte-identical in behaviour and **no slower** than today --
  the chain is written, not read, on the hot path.
- Compaction drops commits no head can reach, and the freed accounting matches `mvcc.bp`'s exact
  numbers.

## 4. Where this does NOT go

Branches per node plus merge is the shape that would matter on a MESH -- and there it becomes
Automerge-class CRDT work, which `LANG-DB-DESIGN.md §1 [w23]` explicitly scopes out of a
single-writer store. C3 is local history, not distributed history. Anything more is a separate
operator decision with D0's mesh invariant attached.
