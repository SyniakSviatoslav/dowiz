/* Bebop native backend — AArch64 machine-code emission + execution. */
#include "native.h"

#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

#include "expr.h"

/* emitter state */
static unsigned int em_code[512];
static size_t em_len;

/* ─── Local-variable frame + register allocation (7B) ───
 * `let`-bound locals are register-resident in x19..x28 (10 callee-saved
 * registers) while any are free; the 11th+ local spills to a frame slot.
 * x15 = frame base (sp + SAVED_REGS), the evaluation stack grows below sp. */
#define FRAME_BYTES 512
#define SAVED_REGS 80 /* 5 stp pairs: x19..x28 */
#define N_LOCAL_REGS 10
static struct {
    const char *name;
    int reg;  /* 19..28 if register-resident, else -1 */
    int slot; /* frame offset from x15 if spilled, else -1 */
} vars[64];
static int nvars = 0;
static int nregs = 0;  /* registers allocated so far (0..N_LOCAL_REGS) */
static int nspill = 0; /* spilled locals so far */

/* Look up a let-bound local (most-recent binding wins). On success exactly one
 * of *reg / *slot is >= 0; both < 0 on a miss. */
static void var_lookup(const char *name, int *reg, int *slot) {
    for (int i = nvars - 1; i >= 0; i--) {
        if (strcmp(vars[i].name, name) == 0) {
            *reg = vars[i].reg;
            *slot = vars[i].slot;
            return;
        }
    }
    *reg = -1;
    *slot = -1;
}

/* stp/ldp xRt,xRt2,[sp,#imm] — imm a multiple of 8. */
static void em(unsigned int ins);
static void emit_stp_sp(int rt, int rt2, int imm) {
    em(0xA9000000u | ((unsigned)(imm >> 3) << 15) | ((unsigned)rt2 << 10) |
       (31u << 5) | (unsigned)rt);
}
static void emit_ldp_sp(int rt, int rt2, int imm) {
    em(0xA9400000u | ((unsigned)(imm >> 3) << 15) | ((unsigned)rt2 << 10) |
       (31u << 5) | (unsigned)rt);
}

static void em(unsigned int ins) {
    if (em_len < sizeof em_code / sizeof em_code[0]) {
        em_code[em_len] = ins;
    }
    em_len++;
}

/* movz/movk: load a 64-bit constant into register rd. */
static void emit_mov64(unsigned long v, int rd) {
    for (int hw = 0; hw < 4; hw++) {
        unsigned imm16 = (unsigned)((v >> (hw * 16)) & 0xFFFF);
        if (hw == 0) {
            em(0xD2800000u | (imm16 << 5) | ((unsigned)hw << 21) | (unsigned)rd);
        } else if (imm16 != 0) {
            em(0xF2800000u | (imm16 << 5) | ((unsigned)hw << 21) | (unsigned)rd);
        }
    }
}

/* push x0: sub sp, sp, #16 ; str x0, [sp] */
static void emit_push(void) {
    em(0xD10043FFu);
    em(0xF90003E0u);
}

/* pop into rd: ldr xrd, [sp] ; add sp, sp, #16 */
static void emit_pop(int rd) {
    em(0xF9400000u | (31u << 5) | (unsigned)rd);
    em(0x910043FFu);
}

/* comparison condition codes (AArch64): EQ=1, NE=0, LT=A, GE=B, LE=C, GT=D */
static int cmp_cond(BinOp op) {
    switch (op) {
        case BOP_EQ: return 0x1;
        case BOP_NE: return 0x0;
        case BOP_LT: return 0xA;
        case BOP_GT: return 0xD;
        case BOP_LE: return 0xC;
        case BOP_GE: return 0xB;
        default:     return -1;
    }
}

static void emit_arith(BinOp op) {
    switch (op) {
        case BOP_ADD: em(0x8B010000u); break;
        case BOP_SUB: em(0xCB010000u); break;
        case BOP_MUL: em(0x9B017C00u); break;
        default:      em(0x8B010000u); break;
    }
}

/* Emit a stack-machine evaluation. Returns 0 on success, -1 on unsupported. */
static int emit_expr(const Term *t) {
    switch (t->kind) {
        case TERM_LIT:
            emit_mov64((unsigned long)t->ival, 0);
            emit_push();
            return 0;
        case TERM_VAR: {
            /* load a let-bound local: register-resident (mov x0, xr) or spilled
             * (ldr x0, [x15, #slot]); push as the result */
            int reg, slot;
            var_lookup(t->name, &reg, &slot);
            if (reg >= 0) {
                em(0xAA0003E0u | ((unsigned)reg << 16)); /* mov x0, x<reg> */
            } else if (slot >= 0) {
                em(0xF9400000u | ((unsigned)(slot >> 3) << 10) | (15u << 5) | 0u); /* ldr x0,[x15,#slot] */
            } else {
                return -1; /* unbound variable: not a closed i64 term */
            }
            emit_push();
            return 0;
        }
        case TERM_LET: {
            /* bind t->name := value (in x0). Register-allocate while x19..x28
             * are free; the 11th+ local spills to the frame. Then eval body. */
            if (emit_expr(t->a) != 0) {
                return -1;
            }
            emit_pop(0); /* x0 = value */
            if (nvars >= 64) {
                return -1;
            }
            if (nregs < N_LOCAL_REGS) {
                int reg = 19 + nregs;
                nregs++;
                vars[nvars].reg = reg;
                vars[nvars].slot = -1;
                em(0xAA0003E0u | (unsigned)reg); /* mov x<reg>, x0 */
            } else {
                int slot = nspill * 8;
                nspill++;
                vars[nvars].reg = -1;
                vars[nvars].slot = slot;
                em(0xF9000000u | ((unsigned)(slot >> 3) << 10) | (15u << 5) | 0u); /* str x0,[x15,#slot] */
            }
            vars[nvars].name = t->name;
            nvars++;
            if (emit_expr(t->b) != 0) {
                return -1;
            }
            return 0;
        }
        case TERM_BIN:
            if (emit_expr(t->a) != 0) return -1;
            if (emit_expr(t->b) != 0) return -1;
            emit_pop(1);
            emit_pop(0);
            if (cmp_cond(t->op) >= 0) {
                em(0xEB01001Fu);
                em(0x9A9F07E0u | ((unsigned)cmp_cond(t->op) << 12));
            } else {
                emit_arith(t->op);
            }
            emit_push();
            return 0;
        case TERM_IF: {
            /* Branchless ⤫ (#30, spec ⤫): evaluate cond + BOTH branches, then
             * csel picks the result. All native terms are pure (no IO), so
             * speculating both branches is always sound and removes a
             * mispredictable cbz/b branch. */
            if (emit_expr(t->a) != 0) return -1; /* cond pushed */
            if (emit_expr(t->b) != 0) return -1; /* then pushed */
            if (emit_expr(t->c) != 0) return -1; /* else pushed */
            emit_pop(2);   /* else -> x2 */
            emit_pop(1);   /* then -> x1 */
            emit_pop(0);   /* cond -> x0 */
            em(0xF100001Fu); /* cmp x0, #0 */
            em(0x9A821020u); /* csel x0, x1, x2, ne — x0 = (x0!=0) ? x1 : x2 */
            emit_push();
            return 0;
        }
        default:
            return -1;
    }
}

long native_eval(const Term *t, char *err, size_t cap) {
    em_len = 0;
    nvars = 0;
    nregs = 0;
    nspill = 0;
    em(0xD10803FFu); /* sub sp, sp, #512 — allocate the frame */
    emit_stp_sp(19, 20, 0); /* save x19..x28 (callee-saved locals) */
    emit_stp_sp(21, 22, 16);
    emit_stp_sp(23, 24, 32);
    emit_stp_sp(25, 26, 48);
    emit_stp_sp(27, 28, 64);
    em(0x910143EFu); /* add x15, sp, #80 — frame base for spilled locals */
    if (emit_expr(t) != 0) {
        snprintf(err, cap, "native: unsupported term");
        return 0;
    }
    emit_pop(0);
    emit_ldp_sp(19, 20, 0); /* restore x19..x28 */
    emit_ldp_sp(21, 22, 16);
    emit_ldp_sp(23, 24, 32);
    emit_ldp_sp(25, 26, 48);
    emit_ldp_sp(27, 28, 64);
    em(0x910803FFu); /* add sp, sp, #512 — free the frame */
    em(0xD65F03C0u); /* ret */

    size_t sz = em_len * sizeof(unsigned int);
    /* W^X (#21 / 2B): two-step — map writeable (NOT executable), emit the
     * code, then flip to executable (NOT writeable). Never W+X at once, so a
     * code-injection write cannot land in an executable page. */
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS,
                     -1, 0);
    if (mem == MAP_FAILED) {
        snprintf(err, cap, "mmap failed");
        return 0;
    }
    memcpy(mem, em_code, sz);
    __builtin___clear_cache((char *)mem, (char *)mem + sz);
    if (mprotect(mem, sz, PROT_READ | PROT_EXEC) != 0) {
        munmap(mem, sz);
        snprintf(err, cap, "mprotect W^X failed");
        return 0;
    }
    long (*fn)(void);
    memcpy(&fn, &mem, sizeof(fn));
    long result = fn();
    munmap(mem, sz);
    return result;
}

int native_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[128];
#define N(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    const char *exprs[] = {"1 + 2", "1 + 2 * 3", "(3 + 4) * 2", "100 - 50", "7 * 6"};
    long wants[] = {3, 7, 14, 50, 42};
    for (int i = 0; i < 5; i++) {
        expr_pool_reset();
        Term *t = NULL;
        if (expr_parse(exprs[i], &t, err, sizeof err) != 0) {
            N(0, "parse");
            continue;
        }
        long got = native_eval(t, err, sizeof err);
        char label[64];
        snprintf(label, sizeof label, "native '%s' == %ld", exprs[i], wants[i]);
        N(got == wants[i], label);
    }
    const char *cmps[] = {"5 > 3", "1 == 1", "3 < 2", "4 <= 4", "5 != 5"};
    long cwants[] = {1, 1, 0, 1, 0};
    for (int i = 0; i < 5; i++) {
        expr_pool_reset();
        Term *t = NULL;
        if (expr_parse(cmps[i], &t, err, sizeof err) != 0) {
            N(0, "cmp parse");
            continue;
        }
        long got = native_eval(t, err, sizeof err);
        char label[64];
        snprintf(label, sizeof label, "native '%s' == %ld", cmps[i], cwants[i]);
        N(got == cwants[i], label);
    }
    const char *ifs[] = {"if (1 == 1) then 10 else 20", "if (3 > 5) then 1 else 2",
                         "if (1 == 1) then (let x = 5 in x * 2) else 99",
                         "if (2 > 3) then 99 else (let y = 3 in y * 3)"};
    long iwants[] = {10, 2, 10, 9};
    for (int i = 0; i < 4; i++) {
        expr_pool_reset();
        Term *t = NULL;
        if (expr_parse(ifs[i], &t, err, sizeof err) != 0) {
            N(0, "if parse");
            continue;
        }
        long got = native_eval(t, err, sizeof err);
        char label[64];
        snprintf(label, sizeof label, "native '%s' == %ld", ifs[i], iwants[i]);
        N(got == iwants[i], label);
    }
    const char *lets[] = {"let x = 5 in x + 1", "let x = 5 in let y = 3 in x * y + 1",
                          "let x = 10 in x - 4", "let x = 1 in let x = 5 in x"};
    long lwants[] = {6, 16, 6, 5};
    for (int i = 0; i < 4; i++) {
        expr_pool_reset();
        Term *t = NULL;
        if (expr_parse(lets[i], &t, err, sizeof err) != 0) {
            N(0, "let parse");
            continue;
        }
        long got = native_eval(t, err, sizeof err);
        char label[64];
        snprintf(label, sizeof label, "native '%s' == %ld", lets[i], lwants[i]);
        N(got == lwants[i], label);
    }
    /* register-pressure spill: 12 nested lets (11th+ spill to the frame).
     * sum 1..12 == 78. */
    {
        char spill[512];
        size_t sp_ = 0;
        for (int v = 1; v <= 12; v++) {
            sp_ += (size_t)snprintf(spill + sp_, sizeof spill - sp_, "let v%d = %d in ", v, v);
        }
        sp_ += (size_t)snprintf(spill + sp_, sizeof spill - sp_, "v1+v2+v3+v4+v5+v6+v7+v8+v9+v10+v11+v12");
        expr_pool_reset();
        Term *t = NULL;
        if (expr_parse(spill, &t, err, sizeof err) != 0) {
            N(0, "spill parse");
        } else {
            long got = native_eval(t, err, sizeof err);
            N(got == 78, "native 12-let spill sum == 78");
        }
    }
    return all_ok ? 0 : -1;
}
