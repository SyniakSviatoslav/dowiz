# Oracle for gate dagc_torn_image (DG4; SPEC §4.2 M-5): the image's file is cut by one cell inside the
# last commit's range; the reopen REFUSES it with reason 2 (torn last commit: the anchor crc fails and
# the store falls back), the image is re-created empty (usable 0), so the span that hit before (1)
# misses after (0). Printed as reason*100 + usable*10 + hit_before + hit_after*1000.
print(2 * 100 + 0 * 10 + 1 + 0 * 1000)
