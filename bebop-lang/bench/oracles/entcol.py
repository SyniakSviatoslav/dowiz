# entcol: L-system A->AB, B->B at depths [2,4,6,8]; diversity = min(#A,#B)*10//len < 3 => collapse; fold = collapsed*1e9 + freed*1e3 + 7
collapsed = freed = 0
for depth in [2, 4, 6, 8]:
    s = "A"
    for _ in range(depth):
        s = "".join("AB" if c == "A" else "B" for c in s)
    n = len(s)
    if min(s.count("A"), s.count("B")) * 10 // n < 3:
        collapsed += 1; freed += n
print(collapsed * 10**9 + freed * 1000 + 7)
