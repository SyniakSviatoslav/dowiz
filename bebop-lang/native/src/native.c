/* Bebop native backend — AArch64 machine-code emission + execution. */
#include "native.h"

#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

#include "expr.h"

/* movz/movk: load a 64-bit constant into register rd. */
static size_t emit_mov64(unsigned int *out, unsigned long v, int rd) {
    size_t n = 0;
    for (int hw = 0; hw < 4; hw++) {
        unsigned imm16 = (unsigned)((v >> (hw * 16)) & 0xFFFF);
        if (hw == 0) {
            out[n++] = 0xD2800000u | (imm16 << 5) | ((unsigned)hw << 21) | (unsigned)rd;
        } else if (imm16 != 0) {
            out[n++] = 0xF2800000u | (imm16 << 5) | ((unsigned)hw << 21) | (unsigned)rd;
        }
    }
    return n;
}

/* push x0: sub sp, sp, #16 ; str x0, [sp] */
static size_t emit_push(unsigned int *out) {
    out[0] = 0xD10043FFu; /* sub sp, sp, #16 */
    out[1] = 0xF90003E0u; /* str x0, [sp] */
    return 2;
}

/* pop into rd: ldr xrd, [sp] ; add sp, sp, #16 */
static size_t emit_pop(unsigned int *out, int rd) {
    out[0] = 0xF9400000u | (31u << 5) | (unsigned)rd; /* ldr xrd, [sp] */
    out[1] = 0x910043FFu; /* add sp, sp, #16 */
    return 2;
}

static unsigned op_enc(BinOp op) {
    switch (op) {
        case BOP_ADD: return 0x8B010000u; /* add x0, x0, x1 */
        case BOP_SUB: return 0xCB010000u; /* sub x0, x0, x1 */
        case BOP_MUL: return 0x9B017C00u; /* mul x0, x0, x1 (Rm=x1) */
        default:      return 0x8B010000u;
    }
}

/* Emit a stack-machine evaluation of a term (i64 arithmetic only). Returns the
 * instruction count, or 0 if the term is unsupported. */
static size_t emit_expr(unsigned int *out, const Term *t) {
    switch (t->kind) {
        case TERM_LIT: {
            size_t n = emit_mov64(out, (unsigned long)t->ival, 0);
            n += emit_push(out + n);
            return n;
        }
        case TERM_BIN: {
            size_t n = emit_expr(out, t->a);
            if (n == 0) {
                return 0;
            }
            size_t m = emit_expr(out + n, t->b);
            if (m == 0) {
                return 0;
            }
            n += m;
            n += emit_pop(out + n, 1); /* x1 = rhs */
            n += emit_pop(out + n, 0); /* x0 = lhs */
            out[n++] = op_enc(t->op);
            n += emit_push(out + n);
            return n;
        }
        default:
            return 0;
    }
}

long native_eval(const Term *t, char *err, size_t cap) {
    unsigned int code[256];
    size_t n = emit_expr(code, t);
    if (n == 0) {
        snprintf(err, cap, "native: unsupported term (only i64 arithmetic)");
        return 0;
    }
    n += emit_pop(code + n, 0); /* result -> x0 */
    code[n++] = 0xD65F03C0u;    /* ret */

    size_t sz = n * sizeof(unsigned int);
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE | PROT_EXEC,
                     MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (mem == MAP_FAILED) {
        snprintf(err, cap, "mmap failed");
        return 0;
    }
    memcpy(mem, code, sz);
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
        char ty[64];
        if (qtt_check_closed(t, ty, sizeof ty, err, sizeof err) != 0) {
            N(0, "typecheck");
            continue;
        }
        long got = native_eval(t, err, sizeof err);
        char label[64];
        snprintf(label, sizeof label, "native '%s' == %ld", exprs[i], wants[i]);
        N(got == wants[i], label);
    }
    return all_ok ? 0 : -1;
}
