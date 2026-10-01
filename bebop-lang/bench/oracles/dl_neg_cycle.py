# Oracle for gate `dl_neg_cycle` (row DG8, A-4): `p(X) :- q(X), not p(X)` -- dl_common.stratify (Tarjan)
# finds the negated atom inside its own SCC. Prints `exit:125 <the E125 line>`.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
r = C.stratify(2, [0, 1], [(1, [(0, 0), (1, 1)])])
assert r[0] == "negcycle"
print("exit:125 error[E125]: negation inside a cycle -- %d depends negatively on %d in the same stratum" % r[1:])
