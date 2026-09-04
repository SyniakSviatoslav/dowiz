M = (1<<64)-1
def s(x):
    x &= M
    return x if x < (1<<63) else x-(1<<64)
def sw(x):  # signed add wrap helper -> signed
    return s(x)

def fwht(x, b):
    xb = x[b:b+8]
    d=[s(xb[0]+xb[1]), s(xb[0]-xb[1]), s(xb[2]+xb[3]), s(xb[2]-xb[3]),
       s(xb[4]+xb[5]), s(xb[4]-xb[5]), s(xb[6]+xb[7]), s(xb[6]-xb[7])]
    t = [0]*8
    t[0]=sw(d[0]+d[2]); t[1]=sw(d[1]+d[3]); t[2]=sw(d[0]-d[2]); t[3]=sw(d[1]-d[3])
    t[4]=sw(d[4]+d[6]); t[5]=sw(d[5]+d[7]); t[6]=sw(d[4]-d[6]); t[7]=sw(d[5]-d[7])
    d[0]=s(t[0]+t[4]); d[1]=s(t[1]+t[5]); d[2]=s(t[2]+t[6]); d[3]=s(t[3]+t[7])
    d[4]=s(t[0]-t[4]); d[5]=s(t[1]-t[5]); d[6]=s(t[2]-t[6]); d[7]=s(t[3]-t[7])
    for i in range(8): x[b+i]=d[i]

def fdiv(v, sh):  # floor-div on possibly-negative: 0-(((0-v)+2**sh-1)>>sh) if v<0 else v>>sh
    if v < 0: return -((((-v)+(1<<sh)-1)) >> sh)
    return v >> sh

def seqeq(x, a, c):
    for i in range(8):
        if x[a*8+i] != x[c*8+i]: return 0
    return 1
def seqeq_m(x, b, mm):
    for i in range(8):
        if x[b+i] != mm[i]: return 0
    return 1

def gen(w):  # lsm_gen: state lives at w[161]; op2 shift emits LSR (array-load rule)
    y = s(w[161] ^ (w[161] << 13))
    z = s(y ^ ((y & M) >> 7))
    w2 = s(z ^ (z << 17))
    w[161] = w2
    return w2

def lsm_step(w, u):
    pre = [0]*8
    for i in range(8):
        acc = 0
        e = w[0+i]
        while e < w[1+i]:
            acc = s(acc + s(w[73+e] * w[137 + w[9+e]]))
            e += 1
        pre[i] = s(acc + s(u * w[145+i]) * 32768)
    for j in range(8):
        v = pre[j]
        d4 = fdiv(v, 4)
        sval = s(w[137+j] + d4)
        h = fdiv(sval, 1)
        w[137+j] = h

def lsm_build(w):
    w[161] = 1234567
    for i in range(8):
        w[1+i] = w[0+i]
        for _ in range(8):
            z = gen(w)
            wr = (z & 7) % 3 - 1
            if wr != 0: w[1+i] = s(w[1+i] + 1)
    for i2 in range(8):
        loc = 0
        for j2 in range(8):
            z = gen(w)
            wr = (z & 7) % 3 - 1
            if wr != 0:
                w[9 + w[0+i2] + loc] = j2
                w[73 + w[0+i2] + loc] = wr
                loc += 1

def oracle():
    mm = [7,-3,5,-11,13,-17,19,-23]
    arena = [0]*32; pert = [0]*32; w = [0]*192
    # h_picture(m, arena)
    for sq in range(4):
        for c in range(8): arena[sq*8+c] = mm[c]
        fwht(arena, sq*8)
    # h_perturb
    for q in range(4):
        for c2 in range(8):
            v = mm[c2]
            pert[q*8+c2] = v+1 if c2==0 else v
        fwht(pert, q*8)
    # h_dan
    w[171] = 0
    for k in range(32):
        if arena[k] != pert[k]: w[171] += 1
    # h_trim
    for z in range(8): arena[8+z] = 0
    arena[26] = 0
    # h_recover
    for r in range(4):
        fwht(arena, r*8)
        for i in range(8): arena[r*8+i] = arena[r*8+i] // 8   # /8 C-trunc (exact here)
    # h_consensus
    for a in range(4):
        w[172+a] = 0
        for c in range(4):
            e = seqeq(arena, a, c)
            add = e if c != a else 0
            if add == 1: w[172+a] += 1
    # h_pick
    s0,s1,s2,s3 = w[172],w[173],w[174],w[175]
    bm = s1 if s0>s1 else s0
    bm = s2 if s2>bm else bm
    bm = s3 if s3>bm else bm
    bt = 4
    t = 1 if s0==bm else 0; u0 = 1 if bt==4 else 0; v0 = t*u0; bt = v0*0 + bt*(1-v0)
    t = 1 if s1==bm else 0; u0 = 1 if bt==4 else 0; v0 = t*u0; bt = v0*1 + bt*(1-v0)
    t = 1 if s2==bm else 0; u0 = 1 if bt==4 else 0; v0 = t*u0; bt = v0*2 + bt*(1-v0)
    t = 1 if s3==bm else 0; u0 = 1 if bt==4 else 0; v0 = t*u0; bt = v0*3 + bt*(1-v0)
    pk = bm*16 + bt
    best = pk & 15
    # h_inv
    pf = 0
    rec_ok = 1 if w[172+best] != 0 else 0
    pf = rec_ok
    pf = s(pf + s(seqeq_m(arena, best*8, mm))*2)
    pf = s(pf + s(1 - seqeq_m(arena, 8, mm))*4)
    pf = s(pf + s(1 - seqeq_m(arena, 24, mm))*8)
    w[162] = pf
    # lsm_build / lsm_win
    lsm_build(w)
    for i in range(8): w[145+i] = 1 - (i & 1)
    # h_exec
    acc = 0
    for it in range(8):
        z = gen(w)
        u = (z & 1) + (1 if arena[best*8+it] < 0 else 0)
        lsm_step(w, u)
        for k in range(8): w[153+k] = w[137+k]
        fwht(w, 153)
        wb = 0
        for ib in range(8):
            wb |= ((1<<ib) if w[153+ib] > 0 else 0)
        acc = s(s(acc*131) + s(wb*17) + s(w[153] + w[160]))
        for j in range(8): w[137+j] = 0
    acc = s(s(acc*131) + s(w[162]*1000) + w[171])
    return acc, (pk & 15), pf, w[171], sorted(w[0:9])

print(oracle()[0])
