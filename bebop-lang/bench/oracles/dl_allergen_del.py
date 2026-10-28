# Oracle for gate `dl_allergen_del` (LV-INC-F, AX plan A1): the sushi EDB of rule set 5 rebuilt by
# dl_common.sushi_rows, the IDB derived by direct set semantics (dl_common.derive), and the allergen
# stratum's WEIGHT computed as what it is -- the number of derivations of allergen(D,A) :- recipe(D,S,C),
# carries(S,A), i.e. the recipe lines (D,S,C) times the carries rows (S,A) that join -- before and after
# deleting produced(105, 120). Never copied from the bebop program.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C

rows, _ = C.sushi_rows()
F = [set() for _ in range(C.NPREDS[5])]
for p, r in rows:
    F[p].add(r)


def state(F):
    idb = C.derive(5, F)
    carries = {(r[0], r[1]) for r in idb[0]}
    weight = sum(1 for d, s, c, _ in F[0] for (x, a) in carries if x == s)
    return idb, weight


gone = C.row(105, 120)
assert gone in F[2], "fixture changed: produced(105, 120) is not in the sushi EDB"
before, wa = state(F)
F[2].discard(gone)
after, wb = state(F)
a, b = len(before[1]), len(after[1])
h = 1 if C.row(23, 3) in after[1] else 0
hc = 1 if C.row(105, 3) in after[0] else 0
print((((a * 1000 + b) * 10000 + wa) * 10000 + wb) * 10000 + ((h * 10 + hc) * 10 + 1) * 10 + 1)
