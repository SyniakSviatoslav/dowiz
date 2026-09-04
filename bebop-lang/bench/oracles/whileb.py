# whileb: while-boundary shapes; fold = mem[3]*1000 + (j==4)*100 + (k==3)*20 + (a==6)*3 + (q==2)*4.
mem = [0] * 8
i = 0
while i < 4:
    mem[i] = i * 2 + 1
    i += 1
j = 0
while j < 3:
    j += 2
k = 0
while k < 3:
    k += 1
a = 0
while a < 4:
    a += 3
q = 0
while q < 2:
    w = 0
    while w < 2:
        w += 1
    q += 1
print(mem[3] * 1000 + (100 if j == 4 else 0) + (20 if k == 3 else 0) + (3 if a == 6 else 0) + (4 if q == 2 else 0))
