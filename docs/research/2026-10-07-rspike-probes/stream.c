// Memory-system calibration for the spike analysis: ns per element for a sequential sum and for a
// dependent random walk (one cache miss per step), at sizes that sit in L1 / L2 / L3 / DRAM.
#include "common.h"
static u64 seqsum(const u64*a,u64 n){ u64 s=0; for(u64 j=0;j<n;j++) s+=a[j]; return s; }
int main(void){
  u64 sizes[]={4096, 65536, 1<<20, 1<<23};      // 32 KB, 512 KB, 8 MB, 64 MB
  for(int si=0;si<4;si++){ u64 n=sizes[si]; u64*a=malloc(n*8); for(u64 j=0;j<n;j++) a[j]=j;
    u64 total=1ull<<26; int reps=(int)(total/n); u64 s=0; double t0=now_ms();
    for(int r=0;r<reps;r++) s+=seqsum(a+opaque(0),n);
    double ms=now_ms()-t0; printf("STREAM seq n=%llu bytes=%llu ns_per_elem=%.3f GBps=%.2f s=%llu\n",(unsigned long long)n,(unsigned long long)(n*8),ms*1e6/((double)n*reps),(double)n*8*reps/ms/1e6,(unsigned long long)s);
    // dependent random walk: a[j] = next index (a random cyclic permutation)
    u64 rng=5; for(u64 j=0;j<n;j++) a[j]=j; for(u64 j=n-1;j>0;j--){ rng=rng*6364136223846793005ull+1442695040888963407ull; u64 k=(rng>>33)%(j+1); u64 t=a[j]; a[j]=a[k]; a[k]=t; }
    u64 steps= n<=65536? 1ull<<24 : 1ull<<22; u64 p=0; t0=now_ms(); for(u64 i=0;i<steps;i++) p=a[p]; ms=now_ms()-t0;
    printf("STREAM randwalk n=%llu bytes=%llu ns_per_step=%.2f p=%llu\n",(unsigned long long)n,(unsigned long long)(n*8),ms*1e6/steps,(unsigned long long)p);
    free(a); }
  return 0;
}
