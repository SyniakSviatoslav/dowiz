// K8H: LCG x, bit = (x>>60)&1 (~50% coin flip), acc = bit ? acc+x : acc-i; N=20000, REPS=3000.
// Versions: branch (forced b.cond) | csel (compiler's if-conversion) | mask (explicit bitmask select) |
// spike (event queue over the unrolled chain) | predictable-branch control ((i>>4)&1) | closed: NOT APPLICABLE.
#include "common.h"
#define N 20000
#define REPS 3000
#define A 6364136223846793005ull
#define C 1442695040888963407ull
__attribute__((noinline)) static u64 k8_branch(u64 *px,u64 acc,u64 n){
  u64 x=*px;
  for(u64 i=n;i>0;i--){ x=x*A+C; u64 bit=(x>>60)&1;
    if(bit){ acc+=x; __asm__ volatile("" ::: "memory"); } else { acc-=i; __asm__ volatile("" ::: "memory"); } }  // barriers forbid if-conversion -> real branch
  *px=x; return acc;
}
__attribute__((noinline)) static u64 k8_csel(u64 *px,u64 acc,u64 n){
  u64 x=*px; for(u64 i=n;i>0;i--){ x=x*A+C; u64 bit=(x>>60)&1; acc = bit ? acc+x : acc-i; } *px=x; return acc;
}
__attribute__((noinline)) static u64 k8_mask(u64 *px,u64 acc,u64 n){
  u64 x=*px; for(u64 i=n;i>0;i--){ x=x*A+C; u64 m=(u64)0-((x>>60)&1); acc += (x & m) | ((0-i) & ~m); } *px=x; return acc;
}
__attribute__((noinline)) static u64 k8_pred_branch(u64 *px,u64 acc,u64 n){   // predictable control: bit = (i>>4)&1
  u64 x=*px; for(u64 i=n;i>0;i--){ x=x*A+C; u64 bit=(i>>4)&1;
    if(bit){ acc+=x; __asm__ volatile("" ::: "memory"); } else { acc-=i; __asm__ volatile("" ::: "memory"); } } *px=x; return acc;
}
static u64 *xs,*accs; static uint32_t *queue;
static u64 k8_spike(u64 *px,u64 acc0,u64 n){
  // two chains (x and acc), node j consumes x[j], acc[j]; produces x[j+1], acc[j+1]
  for(u64 j=0;j<=n;j++){ xs[j]=~0ull; accs[j]=~0ull; } xs[0]=*px; accs[0]=acc0;
  uint32_t qh=0,qt=0; queue[qt++]=0;
  while(qh!=qt){ uint32_t j=queue[qh++]; if(j>=n) continue; u64 i=n-j; u64 x=xs[j]*A+C; u64 bit=(x>>60)&1; u64 a= bit? accs[j]+x : accs[j]-i;
    if(x!=xs[j+1]||a!=accs[j+1]){ xs[j+1]=x; accs[j+1]=a; queue[qt++]=j+1; } }
  *px=xs[n]; return accs[n];
}
int main(int argc,char**argv){
  int which=argc>1?atoi(argv[1]):-1; xs=malloc((N+1)*8); accs=malloc((N+1)*8); queue=malloc((N+2)*4);
  u64 n=opaque(N); double t0; u64 acc,x;
#define RUN(name,fn) { acc=0; x=1; t0=now_ms(); for(int r=0;r<REPS;r++){ acc=fn(&x,opaque(acc),n); x=opaque(x);} RES(name,now_ms()-t0,REPS,acc); }
  if(which<0||which==0) RUN("k8h_branch",k8_branch)
  if(which<0||which==1) RUN("k8h_csel",k8_csel)
  if(which<0||which==2) RUN("k8h_mask",k8_mask)
  if(which<0||which==3) RUN("k8h_spike_cold",k8_spike)
  if(which<0||which==4) RUN("k8h_predictable_branch_CONTROL",k8_pred_branch)
  return 0;
}
