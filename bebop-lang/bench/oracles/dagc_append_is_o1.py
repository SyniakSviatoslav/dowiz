# Oracle for gate dagc_append_is_o1 (DG4; SPEC §4.2 M-2): three "compiles", one commit each, so the
# image's generation is 3; the third append grows the arena by at least what its own objects need and
# by at most twice that (no whole-image rewrite). Printed as gen*10 + within.
print(3 * 10 + 1)
