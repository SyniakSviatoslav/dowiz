# thr: direct-threaded mini-ISA interpreter; cell = op<<56|a<<48|b<<40|d<<32|link; fold = r2*10^6 + exec*100 + jtok*10 + zok
def cell(op, a, b, d, link):
    return (op << 56) | (a << 48) | (b << 40) | (d << 32) | (link & 0xFFFFFFFF)

prog = [cell(0, 0, 1, 2, 1), cell(1, 2, 1, 2, 1), cell(2, 2, 0, 2, 1),
        cell(3, 5, 0, 0, 2), cell(0, 3, 0, 3, 1), cell(4, 0, 0, 0, 0)]
regs = [0] * 8
regs[0], regs[1] = 3, 4
p = exec_ = jtok = 0
while True:
    w = prog[p]
    op, a, b, d = (w >> 56) & 255, (w >> 48) & 255, (w >> 40) & 255, (w >> 32) & 255
    link = w & 0xFFFFFFFF
    if link >= 1 << 31:
        link -= 1 << 32
    if op == 4:
        break
    exec_ += 1
    va, vb = regs[a], regs[b]
    if op == 0: regs[d] = va + vb
    elif op == 1: regs[d] = va * vb
    elif op == 2: regs[d] = va - vb
    elif op == 3 and va == 0:
        jtok += 1
        p += link
        continue
    p += 1
print(regs[2] * 1000000 + exec_ * 100 + jtok * 10 + (1 if regs[3] == 0 else 0))
