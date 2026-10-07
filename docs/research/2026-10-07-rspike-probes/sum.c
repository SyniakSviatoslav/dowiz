// Reduction (the shape dowiz's folds/catalogue have): sum of N=1,048,576 i64.
// dense loop (vectorised) | tree-DAG spike incremental with k changed leaves | algebraic delta O(k).
// k sweeps 1, 16, 256, 1024, 10486 (1%), 104858 (10%).
#include "common.h"
#define LOG 20
#define N (1u<<LOG)
#define REPS 20
static u64 *leaf; static u64 *tree; // tree: level arrays; tree[(1<<(LOG))+ j] style heap: node 1 = root, leaves at N..2N-1
static u64 sum_loop(const u64*a){ u64 s=0; for(u64 j=0;j<N;j++) s+=a[j]; return s; }
static void tree_build(void){ for(u64 j=0;j<N;j++) tree[N+j]=leaf[j]; for(u64 i=N-1;i>=1;i--) tree[i]=tree[2*i]+tree[2*i+1]; }
static uint32_t *queue; static uint8_t *dirty;
static u64 tree_spike(const uint32_t*idx,u64 k){ // each changed leaf spikes its parent; a node fires once per sweep level (dirty bit), no early cutoff possible for +
  uint32_t qh=0,qt=0; for(u64 t=0;t<k;t++){ u64 p=(N+idx[t])>>1; if(!dirty[p]){ dirty[p]=1; queue[qt++]=p; } }
  while(qh!=qt){ uint32_t i=queue[qh++]; dirty[i]=0; u64 nv=tree[2*i]+tree[2*i+1]; if(nv!=tree[i]){ tree[i]=nv; if(i>1){ u64 p=i>>1; if(!dirty[p]){ dirty[p]=1; queue[qt++]=p; } } } }
  return tree[1];
}
int main(int argc,char**argv){
  leaf=malloc(N*8); tree=malloc(2*N*8); queue=malloc(2*N*4); dirty=calloc(2*N,1);
  u64 rng=99; for(u64 j=0;j<N;j++){ rng=rng*6364136223846793005ull+1442695040888963407ull; leaf[j]=rng>>20; }
  tree_build(); u64 base=tree[1];
  double t0=now_ms(); u64 s=0; for(int r=0;r<REPS;r++) s+=sum_loop(leaf+opaque(0)); RES("sum_dense_loop",now_ms()-t0,REPS,s/REPS);
  u64 ks[]={1,16,256,1024,10486,104858};
  for(int ki=0;ki<6;ki++){ u64 k=ks[ki]; uint32_t*idx=malloc(k*4); u64*dv=malloc(k*8);
    for(u64 t=0;t<k;t++){ rng=rng*6364136223846793005ull+1442695040888963407ull; idx[t]=(rng>>33)%N; dv[t]=1+(rng>>40)%1000; }
    char nm[64];
    // spike incremental: apply +dv then -dv alternately so the graph returns to base after an even number of reps
    t0=now_ms(); u64 r1=0; for(int r=0;r<REPS;r++){ for(u64 t=0;t<k;t++) tree[N+idx[t]] += (r&1)? (0-dv[t]) : dv[t]; r1=tree_spike(idx,k); }
    snprintf(nm,64,"sum_tree_spike_k%llu",(unsigned long long)k); RES(nm,now_ms()-t0,REPS,r1);
    if(r1!=base) printf("MISMATCH tree k=%llu %llu base %llu\n",(unsigned long long)k,(unsigned long long)r1,(unsigned long long)base);
    // algebraic delta: s += sum(dv)
    t0=now_ms(); u64 r2=base; for(int r=0;r<REPS*100;r++){ u64 d=0; for(u64 t=0;t<k;t++) d+=dv[t]; r2 += (r&1)? (0-d): d; }
    snprintf(nm,64,"sum_delta_k%llu",(unsigned long long)k); RES(nm,now_ms()-t0,REPS*100,r2);
    // dense recompute with the same change applied (so the comparison is fair): apply, loop, revert
    t0=now_ms(); u64 r3=0; for(int r=0;r<REPS;r++){ for(u64 t=0;t<k;t++) leaf[idx[t]]+=(r&1)?(0-dv[t]):dv[t]; r3=sum_loop(leaf); }
    snprintf(nm,64,"sum_dense_k%llu",(unsigned long long)k); RES(nm,now_ms()-t0,REPS,r3);
    free(idx); free(dv);
  }
  (void)s; return 0;
}
