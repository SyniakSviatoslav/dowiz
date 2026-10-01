# Oracle for gate `dl_unavail` (row DG8, SPEC-DATALOG-AND-CODEC A.6 rule set 1): dl_common.gate(1, 0, 2000) --
# the fixture rebuilt from the same LCG, the 2000 events applied to the EDB, the IDB derived once by direct
# set semantics, folded like dl_fix.bp dl_idb_fold.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
print(C.gate(1, 0, 2000))
