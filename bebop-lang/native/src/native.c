/* Bebop native backend — AArch64 machine-code emission + execution. */
#include "native.h"

#include <stdio.h>
#include <string.h>
#include <sys/mman.h>

#include "expr.h"
#include "pac.h"

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
        case TERM_APP: {
            /* First-order call: reduce the callee to weak-head-normal form
             * (unwinds curried (λx.λy.…) a b chains to a LAM), then inline:
             * (λx. body) arg ≡ let x = arg in body. */
            Term *callee = t->a;
            while (callee->kind == TERM_APP && callee->a->kind == TERM_LAM) {
                callee = qtt_subst(callee->a->a, callee->a->name, callee->b);
                if (!callee) {
                    return -1;
                }
            }
            if (callee->kind != TERM_LAM) {
                return -1; /* higher-order / unknown callee not yet supported */
            }
            if (emit_expr(t->b) != 0) {
                return -1;
            }
            emit_pop(0); /* x0 = arg */
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
            vars[nvars].name = callee->name;
            nvars++;
            if (emit_expr(callee->a) != 0) {
                return -1;
            }
            return 0;
        }
        case TERM_FIELD: {
            /* base must be a struct literal (its ty carries the field layout);
             * load the field at its byte offset from the struct pointer. */
            int off = -1;
            if (t->a->kind == TERM_STRUCT && t->a->ty) {
                for (int i = 0; i < t->a->ty->nfields; i++) {
                    if (strcmp(t->a->ty->fields[i].name, t->name) == 0) {
                        off = i * 8;
                        break;
                    }
                }
            }
            if (off < 0) {
                return -1; /* need a literal struct to resolve the offset */
            }
            if (emit_expr(t->a) != 0) {
                return -1;
            }
            emit_pop(0); /* x0 = struct pointer */
            em(0xF9400000u | ((unsigned)(off >> 3) << 10) | (0u << 5) | 0u); /* ldr x0,[x0,#off] */
            emit_push();
            return 0;
        }
        case TERM_STRUCT: {
            /* allocate an 8·n block in the struct heap (x14 bump pointer),
             * store each field in TYPE order, result = the block pointer. */
            int n = t->ty ? t->ty->nfields : 0;
            for (int i = 0; i < n; i++) {
                const char *fname = t->ty->fields[i].name;
                const Term *val = NULL;
                for (int j = 0; j < t->nfields; j++) {
                    if (strcmp(t->fields[j].name, fname) == 0) {
                        val = t->fields[j].val;
                        break;
                    }
                }
                if (!val) {
                    return -1;
                }
                if (emit_expr(val) != 0) {
                    return -1;
                }
                emit_pop(0); /* x0 = field value */
                em(0xF9000000u | ((unsigned)i << 10) | (14u << 5) | 0u); /* str x0,[x14,#i*8] */
            }
            em(0xAA0E03E0u); /* mov x0, x14 — the struct pointer */
            if (n) {
                em(0x91000000u | ((unsigned)(n * 8) << 10) | (14u << 5) | 14u); /* add x14,x14,#8n */
            }
            emit_push();
            return 0;
        }
        case TERM_ENUM_CTOR: {
            /* allocate a 16-byte block [tag, payload] in the struct heap */
            int tag = -1;
            if (t->ty && t->ty->kind == TY_ENUM) {
                for (int i = 0; i < t->ty->nctors; i++) {
                    if (strcmp(t->ty->ctors[i].name, t->name) == 0) {
                        tag = i;
                        break;
                    }
                }
            }
            if (tag < 0) {
                return -1;
            }
            emit_mov64((unsigned long)tag, 0); /* x0 = tag */
            em(0xF90001C0u); /* str x0, [x14] */
            if (t->a) {
                if (emit_expr(t->a) != 0) {
                    return -1;
                }
                emit_pop(0); /* x0 = payload */
                em(0xF90005C0u); /* str x0, [x14, #8] */
            }
            em(0xAA0E03E0u); /* mov x0, x14 */
            em(0x910042EEu); /* add x14, x14, #16 */
            emit_push();
            return 0;
        }
        case TERM_MATCH: {
            /* literal enum scrutinee: select the arm at compile time */
            if (t->a->kind != TERM_ENUM_CTOR) {
                return -1; /* runtime tag dispatch needs type propagation */
            }
            const char *ctor = t->a->name;
            for (int j = 0; j < t->narms; j++) {
                if (strcmp(t->arms[j].ctor, ctor) == 0) {
                    if (t->arms[j].var && t->a->a) {
                        if (emit_expr(t->a->a) != 0) {
                            return -1;
                        }
                        emit_pop(0);
                        if (nvars >= 64) {
                            return -1;
                        }
                        if (nregs < N_LOCAL_REGS) {
                            int reg = 19 + nregs;
                            nregs++;
                            vars[nvars].reg = reg;
                            vars[nvars].slot = -1;
                            em(0xAA0003E0u | (unsigned)reg);
                        } else {
                            int slot = nspill * 8;
                            nspill++;
                            vars[nvars].reg = -1;
                            vars[nvars].slot = slot;
                            em(0xF9000000u | ((unsigned)(slot >> 3) << 10) | (15u << 5) | 0u);
                        }
                        vars[nvars].name = t->arms[j].var;
                        nvars++;
                    }
                    return emit_expr(t->arms[j].body);
                }
            }
            return -1;
        }
        case TERM_WHILE: {
            /* Label-based while loop: evaluate cond, cbz to exit, eval body,
             * jump back. Uses 4 labels: start, body_after_cond, end, cbz_patch */
            size_t start = em_len;  /* top of loop */
            if (emit_expr(t->a) != 0) return -1; /* cond → x0 (pushed) */
            emit_pop(0);                          /* x0 = cond */
            /* cbz x0, <patch>: placeholder — will fix up after we know end */
            size_t cbz_at = em_len;
            em(0xB4000000u);                      /* cbz x0, 0 (placeholder, 64-bit) */
            if (emit_expr(t->b) != 0) return -1;  /* body → x0 (pushed) */
            emit_pop(0);                          /* discard body result */
            /* b start */
            int back = (int)(start - em_len);
            em(0x14000000u | ((unsigned)back & 0x3FFFFFFu)); /* b <start> */
            size_t end = em_len;
            /* Patch cbz to jump to end */
            int fwd = (int)(end - cbz_at);
            em_code[cbz_at] = 0xB4000000u | (((unsigned)fwd & 0x7FFFFu) << 5);
            /* while evaluates to void → push 0 */
            em(0xD2800000u);                      /* mov x0, #0 */
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
    qtt_term_pool_reset(); /* fresh kernel pool for compile-time β-reduction */
    if (pac_available()) em(PAC_PACIASP); /* sign LR (ROP mitigation, 16B) */
    em(0xD10803FFu); /* sub sp, sp, #512 — allocate the frame */
    emit_stp_sp(19, 20, 0); /* save x19..x28 (callee-saved locals) */
    emit_stp_sp(21, 22, 16);
    emit_stp_sp(23, 24, 32);
    emit_stp_sp(25, 26, 48);
    emit_stp_sp(27, 28, 64);
    em(0x910143EFu); /* add x15, sp, #80 — frame base for spilled locals */
    em(0x910403EEu); /* add x14, sp, #256 — struct heap bump pointer */
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
    if (pac_available()) em(PAC_AUTIASP); /* authenticate LR before ret */
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
    /* function application (compile-time β-reduction) */
    const char *apps[] = {"(\\x:i64. x + 1)(41)", "(\\x:i64. x * x)(7)",
                          "(\\x:i64. \\y:i64. x + y)(3)(4)"};
    long awants[] = {42, 49, 7};
    for (int i = 0; i < 3; i++) {
        expr_pool_reset();
        Term *t = NULL;
        if (expr_parse(apps[i], &t, err, sizeof err) != 0) {
            N(0, "app parse");
            continue;
        }
        long got = native_eval(t, err, sizeof err);
        char label[64];
        snprintf(label, sizeof label, "native '%s' == %ld", apps[i], awants[i]);
        N(got == awants[i], label);
    }
    /* struct + field access (built directly — expr parser doesn't emit TERM_STRUCT) */
    {
        static TyField pt_fields[2] = {{"x", NULL}, {"y", NULL}};
        static Ty pt_ty = {.kind = TY_STRUCT, .fields = pt_fields, .nfields = 2};
        static Term pool[16];
        int pi = 0;
        Term *lx = &pool[pi++]; memset(lx, 0, sizeof *lx); lx->kind = TERM_LIT; lx->ival = 3;
        Term *ly = &pool[pi++]; memset(ly, 0, sizeof *ly); ly->kind = TERM_LIT; ly->ival = 4;
        static TermField sf[2];
        sf[0].name = "x"; sf[0].val = lx;
        sf[1].name = "y"; sf[1].val = ly;
        Term *st = &pool[pi++]; memset(st, 0, sizeof *st);
        st->kind = TERM_STRUCT; st->ty = &pt_ty; st->fields = sf; st->nfields = 2;
        Term *fx = &pool[pi++]; memset(fx, 0, sizeof *fx); fx->kind = TERM_FIELD; fx->name = "x"; fx->a = st;
        Term *fy = &pool[pi++]; memset(fy, 0, sizeof *fy); fy->kind = TERM_FIELD; fy->name = "y"; fy->a = st;
        Term *adds = &pool[pi++]; memset(adds, 0, sizeof *adds);
        adds->kind = TERM_BIN; adds->op = BOP_ADD; adds->a = fx; adds->b = fy;
        long got = native_eval(adds, err, sizeof err);
        N(got == 7, "native struct{x:3,y:4}.x + .y == 7");
        Term *muls = &pool[pi++]; memset(muls, 0, sizeof *muls);
        muls->kind = TERM_BIN; muls->op = BOP_MUL; muls->a = fx; muls->b = fy;
        long got2 = native_eval(muls, err, sizeof err);
        N(got2 == 12, "native struct{x:3,y:4}.x * .y == 12");
    }
    /* enum + match (built directly) */
    {
        static Ctor opt_ctors[2];
        opt_ctors[0].name = "None"; opt_ctors[0].payload = NULL;
        opt_ctors[1].name = "Some"; opt_ctors[1].payload = NULL;
        static Ty opt_ty = {.kind = TY_ENUM, .ctors = opt_ctors, .nctors = 2};
        static Term pool[16];
        int pi = 0;
        Term *lit42 = &pool[pi++]; memset(lit42, 0, sizeof *lit42); lit42->kind = TERM_LIT; lit42->ival = 42;
        Term *some = &pool[pi++]; memset(some, 0, sizeof *some);
        some->kind = TERM_ENUM_CTOR; some->name = "Some"; some->ty = &opt_ty; some->a = lit42;
        Term *xvar = &pool[pi++]; memset(xvar, 0, sizeof *xvar); xvar->kind = TERM_VAR; xvar->name = "x";
        Term *zero = &pool[pi++]; memset(zero, 0, sizeof *zero); zero->kind = TERM_LIT; zero->ival = 0;
        static MatchArm arms[2];
        arms[0].ctor = "None"; arms[0].var = NULL; arms[0].body = zero;
        arms[1].ctor = "Some"; arms[1].var = "x"; arms[1].body = xvar;
        Term *m = &pool[pi++]; memset(m, 0, sizeof *m);
        m->kind = TERM_MATCH; m->a = some; m->arms = arms; m->narms = 2;
        long got = native_eval(m, err, sizeof err);
        N(got == 42, "native match Some(42) { None=>0, Some(x)=>x } == 42");
        Term *none = &pool[pi++]; memset(none, 0, sizeof *none);
        none->kind = TERM_ENUM_CTOR; none->name = "None"; none->ty = &opt_ty; none->a = NULL;
        Term *n99 = &pool[pi++]; memset(n99, 0, sizeof *n99); n99->kind = TERM_LIT; n99->ival = 99;
        static MatchArm arms2[2];
        arms2[0].ctor = "None"; arms2[0].var = NULL; arms2[0].body = n99;
        arms2[1].ctor = "Some"; arms2[1].var = "x"; arms2[1].body = xvar;
        Term *m2 = &pool[pi++]; memset(m2, 0, sizeof *m2);
        m2->kind = TERM_MATCH; m2->a = none; m2->arms = arms2; m2->narms = 2;
        long got2 = native_eval(m2, err, sizeof err);
        N(got2 == 99, "native match None { None=>99, Some(x)=>x } == 99");
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
    /* native while loop: while (0) { 42 } — condition false, body never runs. */

    {
        static Term wpool[4];
        int pi = 0;
        Term *zero = &wpool[pi++]; memset(zero, 0, sizeof *zero);
        zero->kind = TERM_LIT; zero->ival = 0;
        Term *body42 = &wpool[pi++]; memset(body42, 0, sizeof *body42);
        body42->kind = TERM_LIT; body42->ival = 42;
        Term *wh = &wpool[pi++]; memset(wh, 0, sizeof *wh);
        wh->kind = TERM_WHILE; wh->a = zero; wh->b = body42;
        long got = native_eval(wh, err, sizeof err);
        N(got == 0, "native while (0) { 42 } == 0 (void)");
    }

    return all_ok ? 0 : -1;
}
