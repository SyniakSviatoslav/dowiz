# Oracle for gate `dl_personal_neg` (row DG8, A-9 negative twin): dl_common.gate(4, 1, 2000) -- the first event
# whose CORRECT relations lose a row (the head replacement the mutation drops), reported for the lowest stratum
# as t * 10^9 + predicate * 10^6 + lost rows * 10^3 + the first lost row's first column.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
print(C.gate(4, 1, 2000))
