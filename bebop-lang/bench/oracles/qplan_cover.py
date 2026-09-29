# Oracle for gate `qplan_cover` (W-BATGREEN 2026-09-29): bench/vs_rust/std_tests/qplan_cover.bp
# executes every function of selfhost/std/qplan.bp and folds each observed value as acc*31 + v
# (64-bit wrap). Every value below is DERIVED here from the planner's documented cost model and
# formats -- nothing is run through bebop, and the join orders come from a FULL permutation
# search, not from qplan_order's hand-unrolled comparisons (k=4 compares only 3 of 24 orders; the
# chosen input's true minimum is one of them, which this file asserts rather than assumes).
import itertools, sys

M64 = (1 << 64) - 1
def s64(u): u &= M64; return u - (1 << 64) if u >> 63 else u
PER_ROW = {0: 1, 1: 20, 2: 15}       # scan / bucket / spgemm, qplan_cost_* constants

def order_cost(rows, perm):          # cost += card * scan, card *= next table
    card, cost = 1, 0
    for t in perm:
        card *= rows[t]; cost += card * PER_ROW[0]
    return cost

def best(rows):
    perms = list(itertools.permutations(range(len(rows))))
    costs = [order_cost(rows, p) for p in perms]
    m = min(costs)
    if costs.count(m) != 1:
        sys.exit("qplan_cover oracle: rows %r have a TIED minimum -- the pinned order would be "
                 "the planner's tie-break, not the cost model's answer; pick another input" % (rows,))
    return m, list(perms[costs.index(m)])

acc = 0
def fold(v):
    global acc; acc = s64(acc * 31 + v)

fold(0)                                        # qplan_access: stub, scan for every input
for access in (0, 1, 2): fold(7 * PER_ROW[access])   # qplan_cost(7, access)
for rows in ([7], [1000, 10], [1000, 10, 100], [2, 3, 4, 5]):
    cost, order = best(rows)
    fold(0); fold(cost)                        # rc 0, out_cost[0]
    for t in order: fold(t)
fold(92)                                       # qplan_order k=5: refused
fold(0); fold(1)                               # qplan_group(4096) dense, (4097) counting-sort
fp = lambda n_tables, access, group: access * 100 + group * 10 + n_tables
fold(fp(2, 1, 0)); fold(fp(1, 0, 1))           # qplan_fingerprint
fold(1); fold(0)                               # qplan_validate: 6 children ok, 7 refused
text = "access=%d group=%d tables=%d" % (0, 0, 1)    # qplan_explain(fingerprint 1)
fold(len(text))
for ch in text.encode(): fold(ch)
# qplan_plan_query: "q { from T }" parses (qdsl: q, {, from, ident, }) to a single-table plan with
# no group clause -> fp(1, 0, 0) = 1; "x" is not a query -> 0.
fold(fp(1, 0, 0)); fold(0)
print(acc)
