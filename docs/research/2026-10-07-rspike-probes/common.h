// R-SPIKE probes: common timing + output. Every probe prints lines
//   RES <name> <ms_total> <reps> <result_as_u64>
// and the driver takes medians over R runs and checks <result> against the oracle.
#ifndef RSPIKE_COMMON_H
#define RSPIKE_COMMON_H
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
typedef uint64_t u64; typedef int64_t i64;
static inline double now_ms(void){ struct timespec t; clock_gettime(CLOCK_MONOTONIC,&t); return t.tv_sec*1e3 + t.tv_nsec/1e6; }
#define RES(name, ms, reps, val) printf("RES %s %.4f %d %llu\n", (name), (ms), (int)(reps), (unsigned long long)(val))
// opaque: a value the compiler cannot see through (like Rust's black_box)
static inline u64 opaque(u64 x){ __asm__ volatile("" : "+r"(x)); return x; }
// 3x3 / 4x4 affine-state matrix power over Z/2^64 (wrapping): the "closed form" of an affine recurrence
#define MAXD 4
typedef struct { int d; u64 m[MAXD][MAXD]; } Mat;
static Mat mat_mul(const Mat*a,const Mat*b){ Mat r; r.d=a->d; for(int i=0;i<r.d;i++)for(int j=0;j<r.d;j++){u64 s=0; for(int k=0;k<r.d;k++) s+=a->m[i][k]*b->m[k][j]; r.m[i][j]=s;} return r; }
static Mat mat_pow(Mat a, u64 e){ Mat r; r.d=a.d; memset(r.m,0,sizeof r.m); for(int i=0;i<r.d;i++) r.m[i][i]=1; while(e){ if(e&1) r=mat_mul(&r,&a); a=mat_mul(&a,&a); e>>=1; } return r; }
static void mat_apply(const Mat*a,const u64*v,u64*out){ for(int i=0;i<a->d;i++){u64 s=0; for(int k=0;k<a->d;k++) s+=a->m[i][k]*v[k]; out[i]=s;} }
#endif
