#include <stdint.h>
#include <stdio.h>
#include <time.h>
#include <stdlib.h>
#include <string.h>

#define MOD 998244353ULL
#define MU  18479187002ULL

/* AArch64 umulh — multiply-high, one insn. */
static inline uint64_t umulh(uint64_t a, uint64_t b) {
    uint64_t r;
    __asm__ __volatile__("umulh %0, %1, %2" : "=r"(r) : "r"(a), "r"(b));
    return r;
}

static inline uint64_t barrett_asm(uint64_t x) {
    uint64_t q = umulh(x, MU);
    uint64_t r = x - q * MOD;
    if (r >= MOD) r -= MOD;
    return r;
}

static inline uint64_t barrett_128(uint64_t x) {
    uint64_t q = (uint64_t)(((__uint128_t)x * (__uint128_t)MU) >> 64);
    uint64_t r = x - q * MOD;
    if (r >= MOD) r -= MOD;
    return r;
}

static uint64_t pow_mod(uint64_t a, uint64_t e) {
    uint64_t r = 1;
    while (e) { if(e&1) r=barrett_asm(r*a); a=barrett_asm(a*a); e>>=1; }
    return r;
}

static uint64_t inv_mod(uint64_t a) { return pow_mod(a, MOD-2); }

/* NTT with ASM Barrett */
__attribute__((optimize("O2")))
void ntt_asm(uint64_t *a, size_t n, int invert) {
    for (size_t i=0;i<n;i++) if(a[i]>=MOD) a[i]%=MOD;
    size_t j=0;
    for(size_t i=1;i<n;i++){
        size_t bit=n>>1;
        while(j&bit){j^=bit;bit>>=1;}
        j^=bit;
        if(i<j){uint64_t t=a[i];a[i]=a[j];a[j]=t;}
    }
    static uint64_t rts[4096];
    uint64_t *roots = (n/2<=4096)?rts:malloc((n/2)*8);
    uint64_t wprim=pow_mod(3,(MOD-1)/n);
    if(invert) wprim=inv_mod(wprim);
    roots[0]=1;
    for(size_t k=1;k<n/2;k++) roots[k]=barrett_asm(roots[k-1]*wprim);
    for(size_t len=2;len<=n;len<<=1){
        size_t half=len/2, step=n/len;
        for(size_t i=0;i<n;i+=len)
            for(size_t k=0;k<half;k++){
                uint64_t w=roots[k*step], u=a[i+k];
                uint64_t v=barrett_asm(a[i+k+half]*w);
                a[i+k]      =(u+v>=MOD)?(u+v-MOD):(u+v);
                a[i+k+half] =(u>=v)?(u-v):(u+MOD-v);
            }
    }
    if(roots!=rts)free(roots);
    if(invert){
        uint64_t inv_n=inv_mod(n);
        for(size_t i=0;i<n;i++) a[i]=barrett_asm(a[i]*inv_n);
    }
}

void conv(const uint64_t *a, size_t al, const uint64_t *b, size_t bl, uint64_t *out) {
    size_t n=al+bl-1,s=1; while(s<n)s<<=1;
    uint64_t *fa=calloc(s,8), *fb=calloc(s,8);
    memcpy(fa,a,al*8); memcpy(fb,b,bl*8);
    ntt_asm(fa,s,0); ntt_asm(fb,s,0);
    for(size_t i=0;i<s;i++) fa[i]=barrett_asm(fa[i]*fb[i]);
    ntt_asm(fa,s,1);
    memcpy(out,fa,n*8);
    free(fa); free(fb);
}

int main() {
    printf("umulh NTT self-test\\n");
    // identity test
    uint64_t id[256]; for(int i=0;i<256;i++) id[i]=i;
    ntt_asm(id,256,0); ntt_asm(id,256,1);
    int ok=1; for(int i=0;i<256;i++) if(id[i]!=i) {ok=0;break;}
    printf("identity: %s\\n",ok?"PASS":"FAIL");

    // convolution
    uint64_t ca[5]={1,2,3,4,5}, cb[3]={1,1,1}, co[7];
    conv(ca,5,cb,3,co);
    printf("conv: %llu %llu %llu %llu %llu %llu %llu\\n",
        (unsigned long long)co[0],(unsigned long long)co[1],(unsigned long long)co[2],
        (unsigned long long)co[3],(unsigned long long)co[4],(unsigned long long)co[5],
        (unsigned long long)co[6]);
    uint64_t expected[7]={1,3,6,9,12,9,5};
    int cok=1; for(int i=0;i<7;i++) if(co[i]!=expected[i]) {cok=0;break;}
    printf("conv check: %s\\n",cok?"PASS":"FAIL");

    // benchmark n=1024
    int n1024=1024, inner=64;
    uint64_t *a=calloc(n1024,8), *b2=calloc(n1024,8), *o2=calloc(2*n1024,8);
    for(int i=0;i<n1024;i++){a[i]=i; b2[i]=i*7;}
    for(int w=0;w<3;w++) conv(a,n1024,b2,n1024,o2);
    struct timespec t0,t1;
    clock_gettime(CLOCK_MONOTONIC,&t0);
    for(int k=0;k<inner;k++) conv(a,n1024,b2,n1024,o2);
    clock_gettime(CLOCK_MONOTONIC,&t1);
    double ns = (t1.tv_sec-t0.tv_sec)*1e9+(t1.tv_nsec-t0.tv_nsec);
    printf("n=1024: %.0f ns/op  sink=%llu\\n", ns/inner, (unsigned long long)o2[512]);
    free(a);free(b2);free(o2);
    return (ok&&cok)?0:1;
}
