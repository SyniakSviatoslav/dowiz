/* Bebop native backend — AArch64 machine-code emission + execution. */
#include "native.h"

#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

#include "expr.h"

/* emitter state */
static unsigned int em_code[512];
static size_t em_len;

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
            if (emit_expr(t->a) != 0) return -1;
            emit_pop(0); /* pop condition value into x0 (cbz reads x0) */
            size_t cbz_pos = em_len;
            em(0xB4000000u);
            if (emit_expr(t->b) != 0) return -1;
            size_t b_pos = em_len;
            em(0x14000000u);
            size_t else_pos = em_len;
            if (emit_expr(t->c) != 0) return -1;
            size_t end_pos = em_len;
            em_code[cbz_pos] = 0xB4000000u | ((unsigned)(else_pos - cbz_pos) << 5);
            em_code[b_pos] = 0x14000000u | ((unsigned)(end_pos - b_pos) & 0x3FFFFFFu);
            return 0;
        }
        default:
            return -1;
    }
}

long native_eval(const Term *t, char *err, size_t cap) {
    em_len = 0;
    if (emit_expr(t) != 0) {
        snprintf(err, cap, "native: unsupported term");
        return 0;
    }
    emit_pop(0);
    em(0xD65F03C0u);

    size_t sz = em_len * sizeof(unsigned int);
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE | PROT_EXEC,
                     MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (mem == MAP_FAILED) {
        snprintf(err, cap, "mmap failed");
        return 0;
    }
    memcpy(mem, em_code, sz);
    __builtin___clear_cache((char *)mem, (char *)mem + sz);
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
    const char *ifs[] = {"if (1 == 1) then 10 else 20", "if (3 > 5) then 1 else 2"};
    long iwants[] = {10, 2};
    for (int i = 0; i < 2; i++) {
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
    return all_ok ? 0 : -1;
}
