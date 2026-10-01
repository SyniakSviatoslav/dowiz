# Oracle for gate `dl_neg_unsafe` (row DG8, SPEC-DATALOG-AND-CODEC A-3): std_tests/dl_neg_unsafe.bp's rule
# set judged by dl_common.first_unsafe (written apart from dl.bp's dl_safe_rule). Prints the exit line the
# gate compares: `exit:125 <the E125 line>`.
import sys, pathlib
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import dl_common as C
kinds, ops = [0, 0, 1, 1], [0, 0, 0, 0]
rules = [(2, [0], [(0, 0, [0])]), (3, [0, 1], [(0, 0, [0]), (1, 1, [1])])]
r = C.first_unsafe(kinds, ops, rules)
print("exit:125 error[E125]: rule %d unsafe -- variable %d not bound by a positive atom" % r if r else "exit:0 safe")
