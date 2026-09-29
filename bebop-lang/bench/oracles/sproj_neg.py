# DG5 sproj_neg oracle (RT P-3, run-time twin until DG6's `pure` class): st_proj_eval checks the
# fold fn value against proj.bp's registry BEFORE reading or writing anything, so an unregistered
# fn exits 124 (the spec's E124) and the positive twin -- the registered pj_log_step on the same
# fresh image -- is a first read: how code 2 (proj.bp st_proj_eval: 2 = no memo). The gate
# prints "<exit code of the negative>:<output of the positive>".
print("124:2")
