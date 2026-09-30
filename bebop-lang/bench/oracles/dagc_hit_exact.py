# Oracle for gate dagc_hit_exact (DG4; SPEC-BEBOP-DAG-RUNTIME §4.2 M-1, §2.4 K-4). The rule, not the
# program: after a reopen, the same span bytes under the same context and compiler digest are a hit
# (1); the same bytes under a different context (a header gained a parameter) are a miss (0); the
# same bytes under another compiler digest are a miss (0) -- K-4 treats that image as empty.
same, ctx_changed, compiler_changed = 1, 0, 0
print(same * 100 + ctx_changed * 10 + compiler_changed)
