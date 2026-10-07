// K1H: s = s*3 + i, i = N..1 (wrapping), N = 1,000,000.
// Versions: loop | spike (unrolled DAG, event queue) | spike_fp (event queue + per-node fn pointer, a
// generic reactive runtime) | closed (3x3 matrix power, O(log N)) | array variants for the incremental case.
#include "common.h"
#define N 1000000
#define REPS 20

static u64 k1h_loop(u64 s, u64 n){ for(u64 i=n;i>0;i--) s = s*3 + i; return s; }

// ---- spike: node j (0..N-1) computes s[j+1] = 3*s[j] + c[j], c[j] = N-j. A node fires when its input changed.
static u64 *sval; static u64 *carr; static uint32_t *queue;
static u64 spike_run(u64 s0, u64 n, int cold){
  // cold=1: every node's stored value is stale (sentinel), so the whole chain fires: this is the cold single run.
  if(cold) for(u64 j=0;j<=n;j++) sval[j]=~0ull;
  sval[0]=s0; u64 fired=0;
  uint32_t qh=0, qt=0; queue[qt++]=0;            // node 0 is "spiked" by the input
  while(qh!=qt){
    uint32_t j=queue[qh++]; fired++;
    if(j>=n) continue;
    u64 nv = sval[j]*3 + carr[j];
    if(nv!=sval[j+1]){ sval[j+1]=nv; queue[qt++]=j+1; }   // propagate only on change (early cutoff)
  }
  return sval[n] + 0*fired;
}
// generic reactive node: op is a function pointer (what a real dataflow/FRP runtime dispatches through)
typedef u64 (*opfn)(u64,u64);
static u64 op_muladd3(u64 a,u64 c){ return a*3+c; }
static u64 op_add(u64 a,u64 c){ return a+c; }
static opfn *ops;
static u64 spike_fp_run(u64 s0, u64 n){
  for(u64 j=0;j<=n;j++) sval[j]=~0ull;
  sval[0]=s0;
  uint32_t qh=0, qt=0; queue[qt++]=0;
  while(qh!=qt){
    uint32_t j=queue[qh++];
    if(j>=n) continue;
    u64 nv = ops[j](sval[j],carr[j]);
    if(nv!=sval[j+1]){ sval[j+1]=nv; queue[qt++]=j+1; }
  }
  return sval[n];
}
// ---- closed form: state (s, i, 1); s' = 3s + i; i' = i - 1.
static u64 k1h_closed(u64 s0, u64 n){
  Mat m={3,{{3,1,0},{0,1,(u64)-1},{0,0,1}}}; Mat p=mat_pow(m,n);
  u64 v[3]={s0,n,1}, o[3]; mat_apply(&p,v,o); return o[0];
}
// ---- array form for incremental: s = fold over a[j]: s = 3s + a[j]. Change k entries, recompute.
static u64 arr_loop(const u64*a,u64 n){ u64 s=0; for(u64 j=0;j<n;j++) s=s*3+a[j]; return s; }
static u64 *pow3; // pow3[t] = 3^t
static u64 arr_delta(u64 s_old,const uint32_t*idx,const u64*dold,const u64*dnew,u64 k,u64 n){
  u64 s=s_old; for(u64 t=0;t<k;t++) s += (dnew[t]-dold[t])*pow3[n-1-idx[t]]; return s;
}
int main(int argc,char**argv){
  int which = argc>1 ? atoi(argv[1]) : -1;
  sval=malloc((N+1)*8); carr=malloc(N*8); queue=malloc((2*N+4)*4); ops=malloc(N*sizeof(opfn)); pow3=malloc(N*8);
  for(u64 j=0;j<N;j++){ carr[j]=N-j; ops[j]=op_muladd3; }
  ops[0]=op_muladd3; (void)op_add;
  pow3[0]=1; for(u64 t=1;t<N;t++) pow3[t]=pow3[t-1]*3;
  u64 n=opaque(N); double t0; u64 s;
  if(which<0||which==0){ s=0; t0=now_ms(); for(int r=0;r<REPS;r++) s=k1h_loop(opaque(s),n); RES("k1h_loop",now_ms()-t0,REPS,s); }
  if(which<0||which==1){ s=0; t0=now_ms(); for(int r=0;r<REPS;r++) s=spike_run(opaque(s),n,1); RES("k1h_spike_cold",now_ms()-t0,REPS,s); }
  if(which<0||which==2){ s=0; t0=now_ms(); for(int r=0;r<REPS;r++) s=spike_fp_run(opaque(s),n); RES("k1h_spike_fnptr",now_ms()-t0,REPS,s); }
  if(which<0||which==3){ s=0; for(int r=0;r<REPS;r++) s=k1h_closed(s,n); printf("CHECK k1h_closed_%dreps %llu\n",REPS,(unsigned long long)s);
    s=0; t0=now_ms(); for(int r=0;r<REPS*1000;r++) s=k1h_closed(opaque(s),n); RES("k1h_closed_matpow",now_ms()-t0,REPS*1000,s); }
  // incremental: array a = carr (so arr fold == k1h), change k = 1% of entries at random positions
  if(which<0||which==4){
    u64 *a=malloc(N*8); memcpy(a,carr,N*8);
    u64 base=arr_loop(a,N); u64 k=N/100; uint32_t *idx=malloc(k*4); u64 *dold=malloc(k*8),*dnew=malloc(k*8);
    u64 rng=12345; for(u64 t=0;t<k;t++){ rng=rng*6364136223846793005ull+1442695040888963407ull; idx[t]=(rng>>33)%N; dold[t]=a[idx[t]]; dnew[t]=dold[t]+1+(rng>>40)%7; }
    // (duplicates in idx would double-count in delta; dedupe by applying sequentially: dold must be the value right before each change)
    for(u64 t=0;t<k;t++){ dold[t]=a[idx[t]]; a[idx[t]]=dnew[t]; }
    u64 full=arr_loop(a,N);
    t0=now_ms(); u64 x=0; for(int r=0;r<REPS;r++){ x+=arr_loop(a+opaque(0),N); } RES("k1h_arr_full_loop",now_ms()-t0,REPS,x/REPS);
    t0=now_ms(); u64 d=0; for(int r=0;r<REPS*100;r++){ d=arr_delta(base,idx,dold,dnew,k,N); } RES("k1h_arr_delta_1pct",now_ms()-t0,REPS*100,d);
    if(d!=full) printf("MISMATCH delta %llu full %llu\n",(unsigned long long)d,(unsigned long long)full);
    // spike-incremental: warm graph on old a, then apply the k changes to carr and re-spike from each changed node
    memcpy(carr,a,N*8); for(u64 t=0;t<k;t++) carr[idx[t]]=dold[t]; // old values (last write wins = first dold) -- rebuild old exactly:
    for(u64 j=0;j<N;j++) carr[j]=N-j;
    spike_run(0,N,1);                         // warm: graph holds the old chain
    // A FIFO over a chain is QUADRATIC (node j+1 re-fires once per upstream spike), so the honest engine processes
    // nodes in topological order: a dirty bitmap, sweep from the smallest dirty node, early cutoff on unchanged values.
    uint8_t *dirty=calloc(N+1,1); u64 fired=0;
    t0=now_ms(); u64 sp=0; for(int r=0;r<REPS;r++){
      int to_new = (r&1)==0; u64 lo=N;
      for(u64 t=0;t<k;t++){ carr[idx[t]] = to_new? dnew[t]: (N-idx[t]); dirty[idx[t]]=1; if(idx[t]<lo) lo=idx[t]; }
      for(u64 j=lo;j<N;j++){ if(!dirty[j]) continue; dirty[j]=0; fired++; u64 nv=sval[j]*3+carr[j]; if(nv!=sval[j+1]){ sval[j+1]=nv; dirty[j+1]=1; } }
      dirty[N]=0; sp=sval[N];
    } RES("k1h_arr_spike_incr_1pct",now_ms()-t0,REPS,sp);
    printf("FIRED k1h_arr_spike_incr_1pct avg_nodes_fired_per_rerun=%llu of %d\n",(unsigned long long)(fired/REPS),N);
    // after an even number of reps the graph holds the OLD chain; check parity: last rep (r=19) applied old -> sp == base
    if(sp!=base) printf("MISMATCH spike_incr %llu base %llu\n",(unsigned long long)sp,(unsigned long long)base);
    (void)x;
  }
  return 0;
}
