// K4: v = (v + i*7)*3 - 11, i = N..1 (wrapping), N = 2,000,000. loop | spike | closed (matrix power).
#include "common.h"
#define N 2000000
#define REPS 10
static u64 k4_loop(u64 v,u64 n){ for(u64 i=n;i>0;i--) v=(v+i*7)*3-11; return v; }
static u64 *sval; static uint32_t *queue;
static u64 k4_spike(u64 v0,u64 n){
  for(u64 j=0;j<=n;j++) sval[j]=~0ull; sval[0]=v0;
  uint32_t qh=0,qt=0; queue[qt++]=0;
  while(qh!=qt){ uint32_t j=queue[qh++]; if(j>=n) continue; u64 i=n-j; u64 nv=(sval[j]+i*7)*3-11; if(nv!=sval[j+1]){ sval[j+1]=nv; queue[qt++]=j+1; } }
  return sval[n];
}
// state (v, i, 1): v' = 3v + 21 i - 11 ; i' = i - 1
static u64 k4_closed(u64 v0,u64 n){ Mat m={3,{{3,21,(u64)-11},{0,1,(u64)-1},{0,0,1}}}; Mat p=mat_pow(m,n); u64 v[3]={v0,n,1},o[3]; mat_apply(&p,v,o); return o[0]; }
int main(int argc,char**argv){
  int which=argc>1?atoi(argv[1]):-1; sval=malloc((N+1)*8); queue=malloc((N+2)*4);
  u64 n=opaque(N); double t0; u64 v;
  if(which<0||which==0){ v=1; t0=now_ms(); for(int r=0;r<REPS;r++) v=k4_loop(opaque(v),n); RES("k4_loop",now_ms()-t0,REPS,v); }
  if(which<0||which==1){ v=1; t0=now_ms(); for(int r=0;r<REPS;r++) v=k4_spike(opaque(v),n); RES("k4_spike_cold",now_ms()-t0,REPS,v); }
  if(which<0||which==2){ v=1; for(int r=0;r<REPS;r++) v=k4_closed(v,n); printf("CHECK k4_closed_%dreps %llu\n",REPS,(unsigned long long)v);
    v=1; t0=now_ms(); for(int r=0;r<REPS*1000;r++) v=k4_closed(opaque(v),n); RES("k4_closed_matpow",now_ms()-t0,REPS*1000,v); }
  return 0;
}
