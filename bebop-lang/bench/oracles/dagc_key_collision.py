# Oracle for gate dagc_key_collision (DG4; SPEC §2.2 K-1): a K64 is an index, never a proof. The
# program looks up the bytes of B under the stored key of A; the key is found (1) and the lookup
# misses because the bytes differ (1 - 0). The two spans have equal length (else +100).
a = b"fn a(x: i64) -> i64 { x + 1 }"; b = b"fn a(x: i64) -> i64 { x + 2 }"
found, hit = 1, (1 if a == b else 0)
print(found * 10 + (1 - hit) + (0 if len(a) == len(b) else 100))
