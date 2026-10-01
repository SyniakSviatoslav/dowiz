# Oracle for gate `dl_safe` (row DG8, A-3 positive twin of dl_neg_unsafe): the rule set is safe
# (dl_common.first_unsafe -> None), has 4 strata (dl_common.stratify), and over q = {0..5}, r = {1,3}
# derives s = q and p = q x (q \ r). Prints (strata * 10^6 + |s| * 10^3 + |p|) * 31 + fold(p), wrapping i64.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
kinds, ops = [0, 0, 1, 1], [0, 0, 0, 0]
rules = [(2, [0], [(0, 0, [0])]), (3, [0, 1], [(0, 0, [0]), (0, 0, [1]), (1, 1, [1])])]
assert C.first_unsafe(kinds, ops, rules) is None
st = C.stratify(4, kinds, [(h, [(p, n) for p, n, _ in b]) for h, _, b in rules])[1]
q, r = set(range(6)), {1, 3}
s = {C.row(x) for x in q}
p = {C.row(x, y) for x in q for y in q - r}
print(C.s64((max(st) * 1000000 + len(s) * 1000 + len(p)) * 31 + C.fnv_rows(p)))
