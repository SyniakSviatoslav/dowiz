# Oracle for gate dagc_hit_gen (DG4; SPEC §4.2 M-3): A written at generation 1, B at 2, a third
# compile (generation 3) hits A -- A's last_hit_gen becomes 3 in the NEW index, B keeps 2.
gen_a, gen_b = 3, 2
print(gen_a * 10 + gen_b)
