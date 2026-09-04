# attn: HDC attention — argmin_j popcount(Q^K_j) (lowest index on ties), out = V_win ^ Q;
# fold = win*10^9 + bestdist*10^6 + (out&0xFFFF)*100 + uniq
M = (1 << 64) - 1
K = [6148914691236517205, -6148914691236517206, 8608480567731124087, -8608480567731124088]
V = [2654435761, 2246822519, 3266489917, 668265263]
Q = 8608480567731124159
d = [bin((Q ^ k) & M).count("1") for k in K]
bestdist = min(d)
win = d.index(bestdist)
uniq = 1 if d.count(bestdist) == 1 else 0
out = (V[win] ^ Q) & M
print(win * 10**9 + bestdist * 10**6 + (out & 0xFFFF) * 100 + uniq)
