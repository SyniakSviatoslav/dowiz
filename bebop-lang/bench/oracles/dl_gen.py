# Oracle for gate `dl_gen` (row DG8, A.1): the five programs gen_dl.bp writes must each print the fold of
# their rule set's gate -- dl_common.gate(s, 0, 2000) for s = 1..5 -- combined acc*31 + v (wrapping i64)
# in set order, as std_golden's dl_gen block combines the five generated programs' outputs.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
acc = 0
for s in range(1, 6):
    acc = C.s64(acc * 31 + C.gate(s, 0, 2000))
print(acc)
