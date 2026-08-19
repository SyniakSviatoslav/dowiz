/* Bebop VIR — vector IR lowering to hand-encoded AArch64 NEON (6B).
 *
 * The Bebop native backend emits machine code DIRECTLY — no arm_neon.h, no
 * LLVM, no assembler at runtime. The NEON encodings below were derived once
 * from the AArch64 ISA (verified against the GNU assembler) and are emitted
 * as raw 32-bit instructions. This is the vector half of closing the
 * NTT/FFT/sort gap vs Rust's LLVM auto-vectorization.
 */
#include "vir.h"

#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

/* ─── Hand-encoded NEON (Advanced SIMD, 128-bit) ───
 * Three-register "three-same" form:  enc = base | (Rm<<16) | (Rn<<5) | Rd.
 * Load/store single-structure form:  enc = base | (Rn<<5) | Rt.            */
#define NEON_ADD_2D  0x4EE08400u
#define NEON_SUB_2D  0x6EE08400u
#define NEON_ADD_4S  0x4EA08400u
#define NEON_SUB_4S  0x6EA08400u
#define NEON_MUL_4S  0x4EA09C00u
#define NEON_FADD_2D 0x4E60D400u
#define NEON_FSUB_2D 0x4EE0D400u
#define NEON_FMUL_2D 0x6E60DC00u
#define NEON_LD1_2D  0x4C407C00u
#define NEON_ST1_2D  0x4C007C00u
#define NEON_LD1_4S  0x4C407800u
#define NEON_ST1_4S  0x4C007800u

#define MAX_INS 64
static unsigned int code[MAX_INS];
static size_t clen;

static void e(unsigned int ins) {
    if (clen < MAX_INS) {
        code[clen] = ins;
    }
    clen++;
}

int vir_binop(VirOp op, Vir128 a, Vir128 b, Vir128 *out, char *err,
              size_t cap_err) {
    (void)a;
    (void)b;
    unsigned int three = 0, ld = 0, st = 0;
    int is_4s = 0;
    switch (op) {
        case VIR_ADD_2D:  three = NEON_ADD_2D;  break;
        case VIR_SUB_2D:  three = NEON_SUB_2D;  break;
        case VIR_FADD_2D: three = NEON_FADD_2D; break;
        case VIR_FSUB_2D: three = NEON_FSUB_2D; break;
        case VIR_FMUL_2D: three = NEON_FMUL_2D; break;
        case VIR_ADD_4S:  three = NEON_ADD_4S;  is_4s = 1; break;
        case VIR_SUB_4S:  three = NEON_SUB_4S;  is_4s = 1; break;
        case VIR_MUL_4S:  three = NEON_MUL_4S;  is_4s = 1; break;
        default:
            snprintf(err, cap_err, "vir: unknown op %d", (int)op);
            return -1;
    }
    ld = is_4s ? NEON_LD1_4S : NEON_LD1_2D;
    st = is_4s ? NEON_ST1_4S : NEON_ST1_2D;

    clen = 0;
    e(ld | (0u << 5) | 1u);  /* ld1 {v1}, [x0] — a */
    e(ld | (1u << 5) | 2u);  /* ld1 {v2}, [x1] — b */
    e(three | (2u << 16) | (1u << 5) | 0u); /* v0 = v1 op v2 */
    e(st | (2u << 5) | 0u);  /* st1 {v0}, [x2] — out */
    e(0xD65F03C0u);          /* ret */

    size_t sz = clen * sizeof(unsigned int);
    /* W^X: writeable → emit → executable (never W+X). */
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE,
                     MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (mem == MAP_FAILED) {
        snprintf(err, cap_err, "vir: mmap failed");
        return -1;
    }
    memcpy(mem, code, sz);
    __builtin___clear_cache((char *)mem, (char *)mem + sz);
    if (mprotect(mem, sz, PROT_READ | PROT_EXEC) != 0) {
        munmap(mem, sz);
        snprintf(err, cap_err, "vir: mprotect W^X failed");
        return -1;
    }
    void (*fn)(const uint64_t *, const uint64_t *, uint64_t *);
    memcpy(&fn, &mem, sizeof(fn));
    fn((const uint64_t *)&a, (const uint64_t *)&b, (uint64_t *)out);
    munmap(mem, sz);
    return 0;
}

/* ═══ Atomic machine-code ops (⚛) — hand-encoded LSE, W^X ═══ */

uint64_t vir_atomic_add(uint64_t *ptr, uint64_t delta) {
    unsigned int code[2];
    code[0] = 0xF8E10000u; /* ldaddal x1, x0, [x0]: *x0 += x1, x0 = old */
    code[1] = 0xD65F03C0u; /* ret */
    size_t sz = sizeof code;
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (mem == MAP_FAILED) return 0;
    memcpy(mem, code, sz);
    __builtin___clear_cache((char *)mem, (char *)mem + sz);
    mprotect(mem, sz, PROT_READ | PROT_EXEC);
    uint64_t (*fn)(uint64_t *, uint64_t);
    memcpy(&fn, &mem, sizeof(fn));
    uint64_t old = fn(ptr, delta);
    munmap(mem, sz);
    return old;
}

uint64_t vir_atomic_cas(uint64_t *ptr, uint64_t expected, uint64_t desired) {
    unsigned int code[3];
    code[0] = 0xC8E1FC02u; /* casal x1, x2, [x0]: if *x0==x1 then *x0=x2; x1=old */
    code[1] = 0xAA0103E0u; /* mov x0, x1 — return old */
    code[2] = 0xD65F03C0u; /* ret */
    size_t sz = sizeof code;
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (mem == MAP_FAILED) return 0;
    memcpy(mem, code, sz);
    __builtin___clear_cache((char *)mem, (char *)mem + sz);
    mprotect(mem, sz, PROT_READ | PROT_EXEC);
    uint64_t (*fn)(uint64_t *, uint64_t, uint64_t);
    memcpy(&fn, &mem, sizeof(fn));
    uint64_t old = fn(ptr, expected, desired);
    munmap(mem, sz);
    return old;
}

int vir_atomic_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define W(cond, name)                                                      \
    do {                                                                   \
        int c_ = (int)(cond);                                              \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",               \
                          c_ ? "ok" : "FAIL", name);                       \
        if (r_ > 0) pos += (size_t)r_;                                     \
        if (!c_) all_ok = 0;                                               \
    } while (0)

    /* atomic fetch-add: 10 + 5 = 15, returns old 10 */
    {
        static uint64_t cell = 10;
        uint64_t old = vir_atomic_add(&cell, 5);
        W(old == 10 && cell == 15, "atomic_add 10+5 → old=10, cell=15");
    }
    /* CAS success: cell==15 → 99 */
    {
        static uint64_t cell = 15;
        uint64_t old = vir_atomic_cas(&cell, 15, 99);
        W(old == 15 && cell == 99, "cas(15→99) success → old=15, cell=99");
    }
    /* CAS failure: expected mismatch leaves cell untouched */
    {
        static uint64_t cell = 99;
        uint64_t old = vir_atomic_cas(&cell, 7, 1);
        W(old == 99 && cell == 99, "cas(7→1) fails → old=99, cell=99");
    }

    return all_ok ? 0 : -1;
}

int vir_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[128];
#define V(cond, name)                                                      \
    do {                                                                   \
        int c_ = (int)(cond);                                              \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",               \
                          c_ ? "ok" : "FAIL", name);                       \
        if (r_ > 0) pos += (size_t)r_;                                     \
        if (!c_) all_ok = 0;                                               \
    } while (0)

    /* 2×u64 element-wise add: (1,2) + (10,20) = (11,22) */
    {
        Vir128 a = {1, 2}, b = {10, 20}, r;
        V(vir_binop(VIR_ADD_2D, a, b, &r, err, sizeof err) == 0 &&
              r.lo == 11 && r.hi == 22,
          "VIR add.2d (1,2)+(10,20)=(11,22)");
    }
    /* 2×u64 sub: (100,50) - (30,20) = (70,30) */
    {
        Vir128 a = {100, 50}, b = {30, 20}, r;
        V(vir_binop(VIR_SUB_2D, a, b, &r, err, sizeof err) == 0 &&
              r.lo == 70 && r.hi == 30,
          "VIR sub.2d (100,50)-(30,20)=(70,30)");
    }
    /* 4×u32 add: (1,2,3,4) + (10,20,30,40) = (11,22,33,44) */
    {
        Vir128 a = {0x0000000200000001ULL, 0x0000000400000003ULL};
        Vir128 b = {0x000000140000000aULL, 0x000000280000001eULL};
        Vir128 r;
        V(vir_binop(VIR_ADD_4S, a, b, &r, err, sizeof err) == 0 &&
              r.lo == 0x000000160000000bULL && r.hi == 0x0000002c00000021ULL,
          "VIR add.4s (1,2,3,4)+(10,20,30,40)");
    }
    /* 4×u32 mul: (2,3,4,5) * (10,10,10,10) = (20,30,40,50) */
    {
        Vir128 a = {0x0000000300000002ULL, 0x0000000500000004ULL};
        Vir128 b = {0x0000000a0000000aULL, 0x0000000a0000000aULL};
        Vir128 r;
        V(vir_binop(VIR_MUL_4S, a, b, &r, err, sizeof err) == 0 &&
              r.lo == 0x0000001e00000014ULL && r.hi == 0x0000003200000028ULL,
          "VIR mul.4s (2,3,4,5)*(10,...)=(20,30,40,50)");
    }
    /* 2×f64 add: 1.5+2.5, 3.0+4.0 → 4.0, 7.0 */
    {
        union { double f; uint64_t u; } fa = {1.5}, fb = {2.5}, fc = {3.0}, fd = {4.0};
        Vir128 a = {fa.u, fc.u}, b = {fb.u, fd.u}, r;
        V(vir_binop(VIR_FADD_2D, a, b, &r, err, sizeof err) == 0,
          "VIR fadd.2d executes (no fault)");
        union { double f; uint64_t u; } rl = {0}, rh = {0};
        rl.u = r.lo; rh.u = r.hi;
        V(rl.f == 4.0 && rh.f == 7.0, "VIR fadd.2d (1.5,3.0)+(2.5,4.0)=(4.0,7.0)");
    }
    /* 2×f64 mul: 3.0*4.0, 0.5*8.0 → 12.0, 4.0 */
    {
        union { double f; uint64_t u; } a0 = {3.0}, a1 = {0.5}, b0 = {4.0}, b1 = {8.0};
        Vir128 a = {a0.u, a1.u}, b = {b0.u, b1.u}, r;
        union { double f; uint64_t u; } rl = {0}, rh = {0};
        V(vir_binop(VIR_FMUL_2D, a, b, &r, err, sizeof err) == 0,
          "VIR fmul.2d executes");
        rl.u = r.lo; rh.u = r.hi;
        V(rl.f == 12.0 && rh.f == 4.0, "VIR fmul.2d (3.0,0.5)*(4.0,8.0)=(12.0,4.0)");
    }
    /* synthesized 2×64 umulh: hi64(2^63 · 2^63) = 2^62 per lane */
    {
        Vir128 a = {0x8000000000000000ULL, 0x8000000000000000ULL};
        Vir128 b = {0x8000000000000000ULL, 0x8000000000000000ULL};
        Vir128 r;
        V(vir_umulh2(a, b, &r, err, sizeof err) == 0, "VIR umulh2 (vector) executes");
        V(r.lo == 0x4000000000000000ULL && r.hi == 0x4000000000000000ULL,
          "VIR umulh2 2^63·2^63 hi = 2^62");
        /* small: umulh(2^32, 2^32) = hi64(2^64) = 1 */
        Vir128 c = {0x100000000ULL, 0x5}, d = {0x100000000ULL, 0x7}, r2;
        V(vir_umulh2(c, d, &r2, err, sizeof err) == 0, "VIR umulh2 small executes");
        V(r2.lo == 1 && r2.hi == 0, "VIR umulh2 (2^32·2^32)=1, (5·7)=0");
    }


    
return all_ok ? 0 : -1;
}
