# DG5 sanchor oracle (docs/design/SPEC-BEBOP-DAG-RUNTIME-2026-09-28.md §6.3, RT S-6). The seven
# bits bench/vs_rust/std_tests/sanchor.bp reports, each derived from the FORMAT, not from the
# program's answer:
#   bit 0  st_commit_x writes cell 9 = (mark << 32) | crc32(cells[mark, used)); mark >= 1024, so an
#          anchored superblock's cell 9 is never 0, and recomputing it over untouched cells matches.
#   bit 1  flipping bit 0 of an object's h1 changes a cell inside [mark, used), so the anchor's crc32
#          no longer matches -> the live superblock is refused and the open falls back one generation.
#   bit 2  the object walk checks h0's length and h1's HIGH 32 bits (the payload crc) only, so an h1
#          low-word flip passes it -- the case only the anchor sees.
#   bit 3  a payload cell change is inside [mark, used) -> refused via the anchor.
#   bit 4  with cell 9 = 0 the open takes the bounded walk; a payload change breaks that object's crc.
#   bit 5  with cell 9 = 0 the h1 flip is not seen (bit 2's blind spot), so the torn generation stays.
#   bit 6  after a refusal the previous generation holds 19 of the 20 records and the refused
#          superblock's magic is 0 (st_reopen_verify stamps it).
# Every bit is expected TRUE, so the value is 2^7 - 1.
print(sum(1 << b for b in range(7)))
