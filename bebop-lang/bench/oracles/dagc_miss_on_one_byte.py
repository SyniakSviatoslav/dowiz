# Oracle for gate dagc_miss_on_one_byte (DG4; SPEC §4.2 M-1): hits are decided on the exact bytes. The
# span A with one byte flipped (byte 2, bit 0 of the third character) misses; the untouched B and A
# hit. Printed as flipped*100 + hit_b*10 + hit_a.
span_a = b"fn a(x: i64) -> i64 { x + 1 }\n"
flipped = bytearray(span_a); flipped[2] ^= 1
hit_flipped = 1 if bytes(flipped) == span_a else 0
print(hit_flipped * 100 + 1 * 10 + 1)
