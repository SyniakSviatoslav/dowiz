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
            rng=lcg(rng)
            frac=((rng&M)>>11)>>20   # LSR mirror (R3.b)
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
        for j in range(n): evecs[found*n+j]=(-x[j]) if neg else x[j]
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
def profile(a,n):
    rp=[0]*280; ci=[0]*1024; vv=[0]*1024
    evals=[0]*n; evecs=[0]*(n*n)
    csr_from_dense(a,n,rp,ci,vv)
    topk(rp,ci,vv,n,n,32,evals,evecs)
    rho=abs(evals[0])
    tol=1048576; one=4294967296
    unst=0
    for k in range(n):
        ev=abs(evals[k])
        over=1 if ev-one-tol>0 else 0
        unst+=over
    dmp=rho-(one-4295)
    unp=rho-(one+4295)
    d=1 if dmp<0 else 0
    u=1 if unp>0 else 0
    cls=d*0+u*2+(1-d)*(1-u)
    return rho,unst,cls,evals

def drift(a0,a1,n):
    rho0,un0,f,ev0=profile(a0,n)
    rho1,un1,t,ev1=profile(a1,n)
    out=[rho1-rho0, un1-un0, f, t]
    return f*4+t, out

one=4294967296
def ringw_dense(a,n,o,w):
    for i in range(n):
        for c in range(8): a[(o+i)*8+c]=0
        a[(o+i)*8+(o+i)]=w
        ip1=i+1-(n if i+1==n else 0)
        im1=i-1+(n if i==0 else 0)
        a[(o+i)*8+(o+ip1)]=w
        a[(o+i)*8+(o+im1)]=w

a0=[0]*64; a1=[0]*64; a2=[0]*64
w=one//4
ringw_dense(a0,8,0,w)
a1=list(a0); a2=list(a0)
e=one//100
a1[0]=s(a1[0]+e)
big=one*2//5
for i in range(8): a2[i*8+i]=s(a2[i*8+i]+big)
t1,o1=drift(a0,a1,8)
drho1=o1[0]; dq1=drho1>>16
t2,o2=drift(a0,a2,8)
drho2=o2[0]; du2=o2[1]; dq2=drho2>>16
fold=dq1*100000+t2*10000+du2*1000+t1*100+dq2
print(fold)
