// K2H: fib(25) = 75025. recursive (one real call per node, as bebop) | dag_shared (memoised DAG: 26 nodes,
// bottom-up) | doubling (closed form O(log n)) | spike over the shared DAG (event queue, topological).
#include "common.h"
#define REPS 500
__attribute__((noinline)) static u64 fib(u64 n){ return n<2? n : fib(n-1)+fib(n-2); }
static u64 fib_dag(u64 n){ u64 a=0,b=1; for(u64 i=0;i<n;i++){ u64 c=a+b; a=b; b=c; } return a; }
static void fib_dbl(u64 n,u64*f,u64*f1){ if(n==0){*f=0;*f1=1;return;} u64 a,b; fib_dbl(n>>1,&a,&b); u64 c=a*(2*b-a); u64 d=a*a+b*b; if(n&1){*f=d;*f1=c+d;} else {*f=c;*f1=d;} }
static u64 fib_doubling(u64 n){ u64 f,f1; fib_dbl(n,&f,&f1); return f; }
static u64 fib_spike(u64 n){ // shared DAG, node i depends on i-1,i-2; a node fires when both inputs arrived (counts)
  u64 val[64]; uint8_t cnt[64]; uint32_t q[64]; int qh=0,qt=0;
  for(u64 i=0;i<=n;i++) cnt[i]=0;
  val[0]=0; val[1]=1; q[qt++]=0; q[qt++]=1;
  while(qh!=qt){ uint32_t i=q[qh++]; // deliver to consumers i+1 and i+2
    for(int d=1;d<=2;d++){ u64 c=i+d; if(c<2||c>n) continue; if(++cnt[c]==2){ val[c]=val[c-1]+val[c-2]; q[qt++]=c; } } }
  return val[n];
}
int main(int argc,char**argv){
  int which=argc>1?atoi(argv[1]):-1; u64 n=opaque(25); double t0; u64 r;
  if(which<0||which==0){ r=0; t0=now_ms(); for(int i=0;i<REPS;i++) r+=fib(opaque(n)); RES("k2h_recursive",now_ms()-t0,REPS,r/REPS); }
  if(which<0||which==1){ r=0; t0=now_ms(); for(int i=0;i<REPS*10000;i++) r+=fib_dag(opaque(n)); RES("k2h_dag_shared_memo",now_ms()-t0,REPS*10000,r/(REPS*10000)); }
  if(which<0||which==2){ r=0; t0=now_ms(); for(int i=0;i<REPS*10000;i++) r+=fib_doubling(opaque(n)); RES("k2h_closed_doubling",now_ms()-t0,REPS*10000,r/(REPS*10000)); }
  if(which<0||which==3){ r=0; t0=now_ms(); for(int i=0;i<REPS*10000;i++) r+=fib_spike(opaque(n)); RES("k2h_spike_shared_dag",now_ms()-t0,REPS*10000,r/(REPS*10000)); }
  return 0;
}
