// K3H: a = 3a + 2x + 3y, nested 300x300 (wrapping). loop | closed (4x4 affine composition: inner^300 then outer^300).
#include "common.h"
#define NN 300
#define REPS 200
static u64 k3h_loop(u64 a,u64 n){ for(u64 x=n;x>0;x--) for(u64 y=n;y>0;y--) a=a*3+x*2+y*3; return a; }
static u64 k3h_closed(u64 a0,u64 n){
  // state (a, x, y, 1). inner step: a' = 3a + 2x + 3y; y' = y-1. inner^n maps y: n -> 0.
  Mat in={4,{{3,2,3,0},{0,1,0,0},{0,0,1,(u64)-1},{0,0,0,1}}}; Mat inp=mat_pow(in,n);
  // reset: x' = x-1, y' = n (from the 1-column), a,1 unchanged. outer = reset * inner^n
  Mat rs={4,{{1,0,0,0},{0,1,0,(u64)-1},{0,0,0,n},{0,0,0,1}}}; Mat outer=mat_mul(&rs,&inp); Mat op=mat_pow(outer,n);
  u64 v[4]={a0,n,n,1},o[4]; mat_apply(&op,v,o); return o[0];
}
int main(int argc,char**argv){
  int which=argc>1?atoi(argv[1]):-1; u64 n=opaque(NN); double t0; u64 a;
  if(which<0||which==0){ a=0; t0=now_ms(); for(int r=0;r<REPS;r++) a=k3h_loop(opaque(a),n); RES("k3h_loop",now_ms()-t0,REPS,a); }
  if(which<0||which==1){ a=0; for(int r=0;r<REPS;r++) a=k3h_closed(a,n); printf("CHECK k3h_closed_%dreps %llu\n",REPS,(unsigned long long)a);
    a=0; t0=now_ms(); for(int r=0;r<REPS*100;r++) a=k3h_closed(opaque(a),n); RES("k3h_closed_matpow",now_ms()-t0,REPS*100,a); }
  return 0;
}
