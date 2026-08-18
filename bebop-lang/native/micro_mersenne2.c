#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MOD  2147483647U
#define ROOT 7U

static inline uint32_t reduce(uint64_t x) {
    uint32_t r = (uint32_t)(x & MOD) + (uint32_t)(x >> 31);
    if (r >= MOD) r -= MOD;
    return r;
}
static inline uint32_t mul(uint32_t a, uint32_t b) { return reduce((uint64_t)a*b); }
static uint32_t powm(uint32_t a, uint32_t e) {
    uint32_t r=1; while(e){if(e&1)r=mul(r,a);a=mul(a,a);e>>=1;}return r;
}
static uint32_t inv(uint32_t a) { return powm(a, MOD-2); }

void ntt(uint32_t *a, size_t n, bool invert) {
    for(size_t i=0;i<n;i++) if(a[i]>=MOD) a[i]%=MOD;
    size_t j=0;
    for(size_t i=1;i<n;i++){
        size_t bit=n>>1;
        while(j & bit){ j ^= bit; bit >>= 1; }
        j ^= bit;
        if(i<j){ uint32_t t=a[i]; a[i]=a[j]; a[j]=t; }
    }
    static uint32_t roots[4096];
    uint32_t *rts = (n/2<=4096)?roots:malloc((n/2)*sizeof(uint32_t));
    uint32_t wprim=powm(ROOT,(MOD-1)/(uint32_t)n);
    if(invert) wprim=inv(wprim);
    rts[0]=1;
    for(size_t k=1;k<n/2;k++) rts[k]=mul(rts[k-1],wprim);
    for(size_t len=2;len<=n;len<<=1){
        size_t half=len/2, step=n/len;
        for(size_t i=0;i<n;i+=len)
            for(size_t k=0;k<half;k++){
                uint32_t w=rts[k*step], u=a[i+k], v=mul(a[i+k+half],w);
                a[i+k]      = (u+v>=MOD) ? (u+v-MOD) : (u+v);
                a[i+k+half] = (u>=v) ? (u-v) : (u+MOD-v);
            }
    }
    if(rts!=roots) free(rts);
    if(invert){
        uint32_t inv_n=inv((uint32_t)n);
        for(size_t i=0;i<n;i++) a[i]=mul(a[i],inv_n);
    }
}

int main() {
    uint32_t ca[5]={1,2,3,4,5}, cb[3]={1,1,1}, co[7]={0};
    size_t alen=5, blen=3, n=alen+blen-1, size=1;
    while(size<n) size<<=1;
    printf("size=%zu\n", size);
    uint32_t *fa=calloc(size,4), *fb=calloc(size,4);
    memcpy(fa,ca,alen*4); memcpy(fb,cb,blen*4);
    printf("fa:"); for(size_t i=0;i<size;i++) printf(" %u",fa[i]); printf("\n");
    ntt(fa,size,false);
    ntt(fb,size,false);
    printf("NTT(fa):"); for(size_t i=0;i<size;i++) printf(" %u",fa[i]); printf("\n");
    printf("NTT(fb):"); for(size_t i=0;i<size;i++) printf(" %u",fb[i]); printf("\n");
    for(size_t i=0;i<size;i++) fa[i]=mul(fa[i],fb[i]);
    ntt(fa,size,true);
    printf("result:"); for(size_t i=0;i<size;i++) printf(" %u",fa[i]); printf("\n");
    printf("expected: 1 3 6 9 12 9 5 0\n");
    free(fa); free(fb);
    return 0;
}
