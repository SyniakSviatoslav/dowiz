# Oracle for gate `dl_strata` (row DG8, A-4/A-5): std_tests/dl_strata.bp's predicate graph stratified by
# dl_common.stratify (Tarjan SCCs + Kahn over the condensation, ties to the smallest predicate index;
# builtins 0). Prints the vector one decimal digit per predicate, predicate 0 first.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
kinds = [0, 2, 1, 1, 1, 1, 1, 0]
e, lt, a, b, c, d, f, g = range(8)
rules = [(a, [(e, 0)]), (a, [(b, 0), (e, 0)]), (b, [(a, 0), (lt, 0)]), (c, [(g, 0), (a, 1)]),
         (d, [(c, 0), (f, 1)]), (f, [(g, 0), (g, 0), (c, 1)]), (f, [(f, 0), (f, 0)])]
ok, st = C.stratify(8, kinds, rules)
assert ok == "ok"
print(int("".join(str(x) for x in st)))
