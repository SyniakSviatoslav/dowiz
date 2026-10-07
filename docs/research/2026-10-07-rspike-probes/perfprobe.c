// perf_event_open probe: branch-misses / branches / instructions for K8H branch vs csel vs predictable control,
// and for the K1H counted loop. Prints PERF lines or PERF_UNAVAILABLE <errno>.
#include "common.h"
#include <linux/perf_event.h>
#include <sys/syscall.h>
#include <sys/ioctl.h>
#include <unistd.h>
#include <errno.h>
#define A 6364136223846793005ull
#define C 1442695040888963407ull
static int pe_open(u64 type,u64 cfg){ struct perf_event_attr a; memset(&a,0,sizeof a); a.type=type; a.size=sizeof a; a.config=cfg; a.disabled=1; a.exclude_kernel=1; a.exclude_hv=1; return (int)syscall(__NR_perf_event_open,&a,0,-1,-1,0); }
__attribute__((noinline)) static u64 k8_branch(u64 x,u64 acc,u64 n){ for(u64 i=n;i>0;i--){ x=x*A+C; u64 bit=(x>>60)&1; if(bit){acc+=x; __asm__ volatile("":::"memory");} else {acc-=i; __asm__ volatile("":::"memory");} } return acc+x; }
__attribute__((noinline)) static u64 k8_csel(u64 x,u64 acc,u64 n){ for(u64 i=n;i>0;i--){ x=x*A+C; u64 bit=(x>>60)&1; acc = bit? acc+x: acc-i; } return acc+x; }
__attribute__((noinline)) static u64 k8_pred(u64 x,u64 acc,u64 n){ for(u64 i=n;i>0;i--){ x=x*A+C; u64 bit=(i>>4)&1; if(bit){acc+=x; __asm__ volatile("":::"memory");} else {acc-=i; __asm__ volatile("":::"memory");} } return acc+x; }
__attribute__((noinline)) static u64 k1h(u64 s,u64 n){ for(u64 i=n;i>0;i--) s=s*3+i; return s; }
int main(void){
  int fb=pe_open(PERF_TYPE_HARDWARE,PERF_COUNT_HW_BRANCH_INSTRUCTIONS), fm=pe_open(PERF_TYPE_HARDWARE,PERF_COUNT_HW_BRANCH_MISSES), fi=pe_open(PERF_TYPE_HARDWARE,PERF_COUNT_HW_INSTRUCTIONS), fc=pe_open(PERF_TYPE_HARDWARE,PERF_COUNT_HW_CPU_CYCLES);
  if(fb<0||fm<0||fi<0||fc<0){ printf("PERF_UNAVAILABLE errno=%d (%s)\n",errno,strerror(errno)); return 0; }
  u64 n=opaque(2000000);
#define MEASURE(name,expr) { int fds[4]={fb,fm,fi,fc}; for(int k=0;k<4;k++){ioctl(fds[k],PERF_EVENT_IOC_RESET,0); ioctl(fds[k],PERF_EVENT_IOC_ENABLE,0);} u64 r=(expr); for(int k=0;k<4;k++) ioctl(fds[k],PERF_EVENT_IOC_DISABLE,0); u64 v[4]; for(int k=0;k<4;k++) if(read(fds[k],&v[k],8)!=8) v[k]=0; printf("PERF %s iters=%llu branches=%llu misses=%llu miss_per_iter=%.4f instr_per_iter=%.3f cycles_per_iter=%.3f res=%llu\n",name,(unsigned long long)n,(unsigned long long)v[0],(unsigned long long)v[1],(double)v[1]/n,(double)v[2]/n,(double)v[3]/n,(unsigned long long)r); }
  MEASURE("k8h_branch",k8_branch(1,0,n)); MEASURE("k8h_csel",k8_csel(1,0,n)); MEASURE("k8h_predictable_branch",k8_pred(1,0,n)); MEASURE("k1h_counted_loop",k1h(0,n));
  return 0;
}
