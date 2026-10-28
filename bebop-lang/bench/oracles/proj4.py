# LV-INC-A proj4 oracle: the self-test of bench/vs_rust/std_tests/proj4.bp, derived from the SPEC
# (crates/bebop-store/src/proj/kinds.rs header; docs/FORMAT-BEBOP-IMAGE-v2.md §9.3), never from the program:
#   1000  the two OUT layout digests proj.bp pins are st_digest of their layout strings -- st_digest is the
#         low 32 bits (big-endian) of sha256 over the string (proj.rs digests_are_st_digest_of_their_layouts);
#         recomputed here and compared with the values the spec names (OUTB 4157677073, OUTZ 85289679).
#    300  st_proj_eval kind 2 on a KV whose root is v3 (a delta chain) answers how[0] = 3 (refold) ...
#     50  ... with why 5 = "kind 2 on a v3 KV", because pj_kv_fold folds the BASE only (proj.bp pj_refuse_v3);
#      1  ... and the value 0 - 1 (refused, no memo written).
import hashlib


def st_digest(s):
    return int.from_bytes(hashlib.sha256(s.encode()).digest()[28:32], "big")


dig = int(st_digest("OUTB{i64,i64,i64,[i64]}") == 4157677073 and st_digest("OUTZ{i64,i64,i64,i64,[i64]}") == 85289679)
how0, why, refused = 3, 5, 1
print(1000 * dig + 100 * how0 + 10 * why + refused)
