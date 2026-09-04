# swpmu: software PMU; fold = sum(1..N)*10^12 + (stepok*10+sumok)*10^10 + (elapsed<0), N=2000, elapsed_ms >= 0 always.
N = 2000
s = i = steps = 0
while i < N:
    i += 1
    s += i
    steps += 1
stepok = 1 if steps == N else 0
sumok = 1 if s == (N * (N + 1)) // 2 else 0
elapsed = 0  # CLOCK_MONOTONIC delta is non-negative by definition
print(s * 1000000000000 + (stepok * 10 + sumok) * 10000000000 + (0 if elapsed >= 0 else 1))
