// Threaded code vs switch vs counted loop vs straight-line.
// (a) A tiny VM: 8 opcodes, program of 4096 instrs, run 2000 times (8.2M dispatches):
//     switch dispatch | computed-goto threaded dispatch (goto *tbl[op]) | the same program compiled to native straight-line C (what a compiler emits).
// (b) Loop-free chain: K1H's body as 16 blocks linked by `goto *next` (indirect br, static target) vs the counted loop.
#include "common.h"
#define PLEN 4096
#define VREPS 2000
enum { OP_ADD, OP_MUL3, OP_XOR, OP_SUB, OP_SHR, OP_ADDI, OP_ROT, OP_NEG, NOPS };
static uint8_t prog[PLEN]; static u64 imm[PLEN];
__attribute__((noinline)) static u64 vm_switch(u64 acc){
  for(int pc=0;pc<PLEN;pc++){ u64 k=imm[pc]; switch(prog[pc]){
    case OP_ADD: acc+=k; break; case OP_MUL3: acc=acc*3; break; case OP_XOR: acc^=k; break; case OP_SUB: acc-=k; break;
    case OP_SHR: acc>>=1; break; case OP_ADDI: acc+=1; break; case OP_ROT: acc=(acc<<7)|(acc>>57); break; default: acc=0-acc; } }
  return acc;
}
__attribute__((noinline)) static u64 vm_threaded(u64 acc){
  static void* tbl[NOPS]={&&L_ADD,&&L_MUL3,&&L_XOR,&&L_SUB,&&L_SHR,&&L_ADDI,&&L_ROT,&&L_NEG};
  int pc=0; u64 k;
#define NEXT if(++pc>=PLEN) return acc; k=imm[pc]; goto *tbl[prog[pc]];
  k=imm[0]; goto *tbl[prog[0]];
  L_ADD: acc+=k; NEXT
  L_MUL3: acc=acc*3; NEXT
  L_XOR: acc^=k; NEXT
  L_SUB: acc-=k; NEXT
  L_SHR: acc>>=1; NEXT
  L_ADDI: acc+=1; NEXT
  L_ROT: acc=(acc<<7)|(acc>>57); NEXT
  L_NEG: acc=0-acc; NEXT
}
#include "vmprog.h"   // generated: vm_native(acc) = the same 4096 instrs as straight-line C
// (b) loop-free chain for K1H's body
#define N 1000000
__attribute__((noinline)) static u64 k1h_counted(u64 s,u64 n){ for(u64 i=n;i>0;i--) s=s*3+i; return s; }
__attribute__((noinline)) static u64 k1h_threaded_blocks(u64 s,u64 n){
  // 16 blocks; block b: s = s*3 + i; i--; then br to next[b] (an indirect branch with a static target, like `br xN`)
  static void* next[16]={&&B1,&&B2,&&B3,&&B4,&&B5,&&B6,&&B7,&&B8,&&B9,&&B10,&&B11,&&B12,&&B13,&&B14,&&B15,&&B0};
  u64 i=n; void **nx=opaque((u64)next)==(u64)next? next : next;
  #define BLK(L,K) L: s=s*3+i; i--; if(i==0) return s; goto *nx[K];
  BLK(B0,0) BLK(B1,1) BLK(B2,2) BLK(B3,3) BLK(B4,4) BLK(B5,5) BLK(B6,6) BLK(B7,7)
  BLK(B8,8) BLK(B9,9) BLK(B10,10) BLK(B11,11) BLK(B12,12) BLK(B13,13) BLK(B14,14) BLK(B15,15)
}
int main(int argc,char**argv){
  int which=argc>1?atoi(argv[1]):-1; double t0; u64 a;
  u64 rng=7; for(int p=0;p<PLEN;p++){ rng=rng*6364136223846793005ull+1442695040888963407ull; prog[p]=(rng>>40)%NOPS; imm[p]=rng>>13; }
  // vmprog.h was generated with the SAME generator; verify agreement
  if(which<0||which==0){ a=1; t0=now_ms(); for(int r=0;r<VREPS;r++) a=vm_switch(opaque(a)); RES("vm_switch",now_ms()-t0,VREPS,a); }
  if(which<0||which==1){ a=1; t0=now_ms(); for(int r=0;r<VREPS;r++) a=vm_threaded(opaque(a)); RES("vm_threaded_goto",now_ms()-t0,VREPS,a); }
  if(which<0||which==2){ a=1; t0=now_ms(); for(int r=0;r<VREPS;r++) a=vm_native(opaque(a)); RES("vm_native_straightline",now_ms()-t0,VREPS,a); }
  if(which<0||which==3){ a=0; t0=now_ms(); for(int r=0;r<20;r++) a=k1h_counted(opaque(a),opaque(N)); RES("k1h_counted_loop",now_ms()-t0,20,a); }
  if(which<0||which==4){ a=0; t0=now_ms(); for(int r=0;r<20;r++) a=k1h_threaded_blocks(opaque(a),opaque(N)); RES("k1h_threaded_blocks_br",now_ms()-t0,20,a); }
  return 0;
}
