# substrate: event-dispatcher dataflow, LSB-first drain per sweep; fired cell i adds its value into i+1 (chain) or i+1,i+2 (addend) and decrements their need; cells reaching need 0 fire next sweep; iterate to quiescence. k1 chain cells[i]=i -> cell[8]=36 in 9 sweeps; k2 fib ripple -> cell[25]=fib(25)=75025 in 25 sweeps. fold = k1v*10^9 + k2v*10^4 + ok1*100 + ok2*10 + sw1*2 + sw2
def drive(cells, need, ncell, act, addend, target):
    sweeps = 0
    while act:
        nxt = 0
        while act:
            i = (act & -act).bit_length() - 1; act &= act - 1
            for d in (1, 2) if addend else (1,):
                cells[i + d] += cells[i]; need[i + d] -= 1
            for j in range(1, ncell):
                if need[j] == 0: nxt |= 1 << j; need[j] = -1
        act = nxt; sweeps += 1
    return cells[target], sweeps
k1v, sw1 = drive([i if i < 9 else 0 for i in range(64)], [0] + [1] * 8 + [0] * 55, 9, 1, False, 8)
k2v, sw2 = drive([0, 1] + [0] * 62, [0, 0] + [2] * 24 + [0] * 38, 26, 3, True, 25)
print(k1v * 10**9 + k2v * 10**4 + int(k1v == 36) * 100 + int(k2v == 75025) * 10 + int(sw1 == 9) * 2 + int(sw2 == 25))
