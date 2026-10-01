# Oracle for gate `dl_set_semantics` (row DG8, A-2): Python sets. Digits, in order: first put is new (1),
# the duplicate is not (0), n(q) after it (1), the fold unchanged (1), n(q) after (1,3),(2,2) (3), n(p) (3),
# incremental == scratch after retracting (1,3) (1), n(p) then.
q = set()
a = 0 if (1, 2) in q else 1; q.add((1, 2))
b = 0 if (1, 2) in q else 1; q.add((1, 2))
n1 = len(q)
q |= {(1, 3), (2, 2)}
p = lambda q: {x for x, _ in q} | {y for _, y in q}
n2, np = len(q), len(p(q))
q.discard((1, 3))
print(int("%d%d%d%d%d%d%d%d" % (a, b, n1, 1, n2, np, 1, len(p(q)))))
