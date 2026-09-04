
M=(1<<64)-1
def s(x):
    x &= M
    return x if x < (1<<63) else x-(1<<64)
def fp_mul(a,b):
    na=1 if a<0 else 0; aa=-a if na else a
    nb=1 if b<0 else 0; ab=-b if nb else b
    a1=aa>>32; a0=aa-(a1<<32); b1=ab>>32; b0=ab-(b1<<32)
    hi=a1*b1; mid=a1*b0+a0*b1; ah=a0>>16; al=a0-(ah<<16); bh=b0>>16; bl=b0-(bh<<16)
    ll_h=ah*bh; ll_m=ah*bl+al*bh; ll_l=al*bl
    low=ll_h+((ll_m+(ll_l>>16))>>16)
    p=(hi<<32)+mid+low
    flip=(na+nb)%2
    return -p if flip else p
def isqrt(ss):
    rem=root=0
    for i in range(31):
        sh=60-i*2
        rem=(rem<<2)+((ss>>sh)&3)
        tst=(root<<2)+1
        take=1 if tst<=rem else 0
        root=(root<<1)+take
        rem=rem-take*tst
    return root
def lcg(r):
    return s(s(r*s(6364136223846793005))+s(1442695040888963407))
def spmv(rp,ci,vv,n,x,out):
    for j in range(n): out[j]=0
    for i in range(n):
        for k in range(rp[i],rp[i+1]):
            j=ci[k]
            out[j]=s(out[j]+fp_mul(vv[k],x[i]))
    return n
def norm(x,n):
    ss=0
    for i in range(n):
        ai=abs(x[i]); t=ai>>8; ss+=t*t
    nrm=isqrt(ss)
    if nrm==0: nrm=1
    rr=s(72057594037927936//nrm)
    for i in range(n): x[i]=fp_mul(x[i],rr)
    return (nrm<<14)&M
def csr_from_dense(a,n,rp,ci,vv):
    nnz=0
    for i in range(n):
        rp[i]=nnz
        for j in range(n):
            w_=a[i*n+j]
            if w_!=0:
                ci[nnz]=j; vv[nnz]=w_; nnz+=1
    rp[n]=nnz
    return nnz
def all_eq_off(a,off,b,n):
    eq=1
    for k in range(n):
        m=1 if a[off+k]==b[k] else 0
        eq*=m
    return eq
def eigentime(rp,ci,vv,n,D=30,tmax=2000):
    rng=s(-7046029254386353131)
    x=[0]*n
    for j in range(n):
        rng=lcg(rng)
        # R3(b): the .bp emits `(rng >> 11) >> 20` as LSR for the
        # loop-reassigned local rng -> mirror unsigned (rng&M)>>31.
        frac=((rng&M)>>11)>>20
        x[j]=s(frac+frac-4294967296)
    norm(x,n)
    hist=[0]*(D*n)
    ax=[0]*n
    for j in range(n): hist[j]=x[j]
    t=0; per=0; td=0
    while per==0:
        spmv(rp,ci,vv,n,x,ax)
        norm(ax,n)
        for j in range(n): x[j]=ax[j]
        t+=1
        for p in range(1,D+1):
            m=t-p
            idx=m-(m//D)*D
            if idx<0: idx+=D
            if all_eq_off(hist,idx*8,x,n):
                per=p; td=t; break
        if t>=tmax: per=3
        m=t
        idx=m-(m//D)*D
        for j in range(n): hist[idx*8+j]=x[j]
    ab=1
    for _ in range(16):
        spmv(rp,ci,vv,n,x,ax)
        norm(ax,n)
        for j in range(n): x[j]=ax[j]
        ok=0
        for p in range(1,D+1):
            m=t-p
            idx=m-(m//D)*D
            if idx<0: idx+=D
            if all_eq_off(hist,idx*8,x,n):
                ok=1; break
        ab*=ok
        t+=1
        m=t
        idx=m-(m//D)*D
        for j in range(n): hist[idx*8+j]=x[j]
    return td*1000+per*10+ab

one=4294967296
def ring_dense(a,n,o,loopw):
    for i in range(n):
        for c in range(8):
            a[(o+i)*8+c]=0
        a[(o+i)*8+(o+i)]=loopw
        ip1=i+1-(n if i+1==n else 0)
        im1=i-1+(n if i==0 else 0)
        a[(o+i)*8+(o+ip1)]=one
        a[(o+i)*8+(o+im1)]=one
def ones_dense(a,n,o):
    for i in range(n):
        for c in range(8):
            a[(o+i)*8+c]=one
def eigentime_dense(a,n):
    rp=[0]*16; ci=[0]*64; vv=[0]*64
    csr_from_dense(a,n,rp,ci,vv)
    return eigentime(rp,ci,vv,n)

a=[0]*64
ring_dense(a,8,0,one)
es=eigentime_dense(a,8)
ones_dense(a,8,0)
ef=eigentime_dense(a,8)
fold=es*10000+ef
print(fold)
