# sptbad oracle (FORMAT-BEBOP-IMAGE-v2 §3.1): what st_open must answer on each damaged image of
# bench/vs_rust/std_tests/sptbad.bp, derived from the format's rule, not from the program.
# A superblock is live when its magic and crc15 hold. The newest live one is picked. If its PartTab
# fails its crc the open REFUSES (trap 77), whether or not the image is anchored: a fallback is
# allowed only past a bad magic/crc15, or past a torn last commit onto a generation whose PartTab holds.
RECORDS = 20

def open_image(magic_ok, parttab_ok, torn, older_parttab_ok):
    if not magic_ok:
        return (0, RECORDS - 1)
    if not parttab_ok:
        return (77, None)
    if torn:
        return (0, RECORDS - 1) if older_parttab_ok else (77, None)
    return (0, RECORDS)

CASES = [
    ("e", True, True, False, True),    # untouched
    ("a", True, False, False, True),   # anchored, PartTab payload bit flipped
    ("b", True, False, False, True),   # anchor zeroed, the same flip
    ("c", False, True, False, True),   # live magic zeroed: the legitimate fallback
    ("d", True, True, True, False),    # newest record torn, older generation's PartTab flipped
]

out = []
for name, *state in CASES:
    rc, n = open_image(*state)
    out.append("%s%d=%s" % (name, rc, "" if n is None else n))
print(" ".join(out))
