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
def topk(rp,ci,vv,n,k,iters,evals,evecs):
    x=[0]*192; ax=[0]*192; tmp=[0]*192
    found=0; m=0
    while m<k:
        rng=s(-7046029254386353131)
        for j in range(n):
            rng=lcg(rng); frac=((rng>>11)>>20)&M
            x[j]=s(frac+frac-4294967296)
        norm(x,n)
        om=0
        while om<found:
            proj=0
            for j in range(n): proj=s(proj+fp_mul(evecs[om*n+j],x[j]))
            for j in range(n): x[j]=s(x[j]-fp_mul(proj,evecs[om*n+j]))
            om+=1
        norm(x,n)
        it=0
        while it<iters:
            spmv(rp,ci,vv,n,x,ax)
            om=0
            while om<found:
                proj=0
                for j in range(n): proj=s(proj+fp_mul(evecs[om*n+j],ax[j]))
                for j in range(n): ax[j]=s(ax[j]-fp_mul(proj,evecs[om*n+j]))
                om+=1
            norm(ax,n)
            for j in range(n): x[j]=ax[j]
            it+=1
        spmv(rp,ci,vv,n,x,tmp)
        om=0
        while om<found:
            proj=0
            for j in range(n): proj=s(proj+fp_mul(evecs[om*n+j],tmp[j]))
            for j in range(n): tmp[j]=s(tmp[j]-fp_mul(proj,evecs[om*n+j]))
            om+=1
        lam=0
        for j in range(n): lam=s(lam+fp_mul(x[j],tmp[j]))
        om=0
        while om<found:
            proj=0
            for j in range(n): proj=s(proj+fp_mul(evecs[om*n+j],x[j]))
            for j in range(n): x[j]=s(x[j]-fp_mul(proj,evecs[om*n+j]))
            om+=1
        norm(x,n)
        fpos=0
        for j in range(n):
            mag=abs(x[j]); big=1 if mag>65536 else 0; seen=1 if fpos!=0 else 0
            if seen==0: fpos=x[j]*big
        neg=1 if fpos<0 else 0
        for j in range(n): evecs[found*n+j]= (-x[j]) if neg else x[j]
        evals[found]=lam; found+=1; m+=1
    kr=[0]*64
    for i in range(1,found):
        key=evals[i]
        for jj in range(n): kr[jj]=evecs[i*n+jj]
        ke=abs(key); j=i-1; go=1
        while go==1:
            inb=1 if j>=0 else 0; je=abs(evals[j]); lt=1 if je<ke else 0
            move_=lt*inb
            if move_==1: evals[j+1]=evals[j]
            for jj in range(n):
                if move_==1: evecs[(j+1)*n+jj]=evecs[j*n+jj]
            j=j-move_; go=move_
        evals[j+1]=key
        for jj in range(n): evecs[(j+1)*n+jj]=kr[jj]
    return found
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

one=4294967296
a=[0]*64
for i in range(8):
    a[i*8+i]=one
    a[i*8+((i+1)&7)]=one
    a[i*8+((i+7)&7)]=one
rp=[0]*16; ci=[0]*64; vv=[0]*64
csr_from_dense(a,8,rp,ci,vv)
evals=[0]*8; evecs=[0]*64
topk(rp,ci,vv,8,4,64,evals,evecs)
print("evals:", [s(x) for x in evals[:4]])
print("evecs row0:", [s(x) for x in evecs[:8]])
print("evecs row1:", [s(x) for x in evecs[8:16]])
w=[0]*256
c1=[1,1,1,1,1,1,1,-1]
c2=[1,1,1,1,-1,-1,-1,-1]
c3=[1,1,-1,-1,1,1,-1,1]
w[0:8]=c1; w[8:16]=c2; w[16:24]=c3
w[80:88]=c3
for jj in range(8): w[96+jj]=c3[(jj+2)%8]
for jj in range(8): w[104+jj]=c2[(jj+5)%8]
def vdot(off,row,n=8):
    return sum(w[off+j]*evecs[row*8+j] for j in range(n))
for ssl in range(3): w[128+ssl]=vdot(ssl*8,0)
w[131]=vdot(80,0); w[132]=vdot(96,0); w[133]=vdot(104,0)

w[136],w[137],w[138]=w[132]-w[128+0],w[132]-w[128+1],w[132]-w[128+2]
w[139],w[140],w[141]=w[133]-w[128+0],w[133]-w[128+1],w[133]-w[128+2]
for i in range(136,142): w[i]=abs(w[i])
pa=1 if w[137]<w[136] else 0; va=w[137] if pa else w[136]; ida=pa
pb=1 if w[138]<va else 0; ida=2 if pb else ida
pb=1 if w[140]<w[139] else 0; vb=w[140] if pb else w[139]; idb=pb
pc=1 if w[141]<vb else 0; idb=2 if pc else idb
la,lb=w[138],w[140]
lay=1 if w[131]-w[128+2]==0 else 0
oo=0
for p0 in range(3):
    for p1 in range(p0+1,4):
        dot=sum(evecs[p0*8+j]*evecs[p1*8+j] for j in range(8))
        oo+=abs(dot)>>28
rot=(1 if la<=1048576 else 0)*2+(1 if lb<=1048576 else 0)
("ob:",oo<=200)
print(ida*1000000+idb*10000+lay*100+rot*10+(1 if oo<=200 else 0))
