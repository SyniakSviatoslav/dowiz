/* Bebop native backend — AArch64 machine-code emission + execution. */
#include "native.h"

#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include <sys/mman.h>

#include "expr.h"
#include "pac.h"

/* emitter state */
static unsigned int em_code[512];
static size_t em_len;

/* Trailing string data region (see TERM_STR). String bytes are emitted here
 * (NOT into em_code) and placed AFTER the code words in the single mmap'd
 * buffer, so adr (PC-relative) can load their address. Each string is aligned
 * to a 4-byte boundary (adr encodes offsets in units of 4 bytes). */
static unsigned char em_str[2048];
static size_t em_strlen;       /* bytes used in em_str */
static size_t em_adr_at[128];  /* word index of each adr instruction to patch */
static size_t em_adr_off[128]; /* em_str byte offset each adr loads */
static int em_adr_n;           /* number of pending adr fixups */

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
/* While-loop nesting during compilation. A let inside a loop must REUSE the
 * existing binding (interpreter in_while semantics): allocating a fresh
 * register/slot per textual occurrence makes the loop condition read the
 * original binding forever - the emitted loop never terminates. */
static int in_while = 0;
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
        case BOP_DIV: em(0x9AC10C00u); break; /* sdiv x0,x0,x1 (÷0 traps) */
        case BOP_MOD:
            em(0xAA0003E2u);        /* mov x2,x0: keep dividend */
            em(0x9AC10C00u);        /* sdiv x0,x0,x1 -> quotient */
            em(0x9B018800u);        /* msub x0,x0,x1,x2 -> a - q*b */
            break;
        case BOP_BAND: em(0x8A010000u); break; /* and  x0,x0,x1 */
        case BOP_BOR: em(0xAA010000u); break; /* orr  x0,x0,x1 */
        case BOP_BXOR: em(0xCA010000u); break; /* eor  x0,x0,x1 */
        case BOP_SHL: em(0x9AC12000u); break; /* lslv x0,x0,x1 (&63 on host) */
        case BOP_SHR: em(0x9AC12400u); break; /* lsrv x0,x0,x1 */
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
            int reg0 = -1, slot0 = -1;
            if (in_while) {
                var_lookup(t->name, &reg0, &slot0);
            }
            if (reg0 >= 0) {
                /* loop-carried update: write through to the existing register
                 * ORR x<reg>, xzr, x0 -> rd=reg0 in bits[4:0] */
                em(0xAA0003E0u | (unsigned)reg0);
                vars[nvars].name = t->name;
                vars[nvars].reg = reg0;
                vars[nvars].slot = -1;
                nvars++;
            } else if (slot0 >= 0) {
                /* loop-carried update: store through to the existing spill slot */
                em(0xF9000000u | ((unsigned)(slot0 >> 3) << 10) | (15u << 5) | 0u);
                vars[nvars].name = t->name;
                vars[nvars].reg = -1;
                vars[nvars].slot = slot0;
                nvars++;
            } else if (nregs < N_LOCAL_REGS) {
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
        case TERM_ARRAY:
            /* allocate n*8 on heap bump (x14), store elements, return ptr, bump */
            {
                int n = t->nfields;
                for (int j = 0; j < n; j++) {
                    if (emit_expr(t->fields[j].val) != 0) return -1;
                    emit_pop(0);
                    em(0xF9000000u | ((unsigned)(j) << 10) | (14u << 5) | 0u);
                }
                em(0xAA0E03E0u);
                if (n > 0) em(0x91000000u | ((unsigned)(n * 8) << 10) | (14u << 5) | 14u);
                emit_push();
                return 0;
            }
        case TERM_ARRAY_GET:
            /* a = array, b = index. eval array → x0 (push), eval index → x0;
             * pop index→x1, pop base→x2; ldr x0,[x2,x1,lsl 3]; push.
             * Register-offset load: Rm=index bits[20:16], Rn=base bits[9:5],
             * Rt=dest bits[4:0]. */
            if (emit_expr(t->a) != 0) return -1;
            if (emit_expr(t->b) != 0) return -1;
            emit_pop(1); /* index → x1 */
            emit_pop(2); /* base  → x2 */
            em(0xF8607800u | (1u << 16) | (2u << 5) | 0u); /* ldr x0,[x2,x1,lsl 3] */
            emit_push();
            return 0;
        case TERM_ARRAY_SET:
            /* a = array, b = index, c = value. eval all three (each → x0, push);
             * pop value→x0, pop index→x1, pop base→x2;
             * str x0,[x2,x1,lsl 3] (Rm=index, Rn=base, Rt=value); result 0. */
            if (emit_expr(t->a) != 0) return -1;
            if (emit_expr(t->b) != 0) return -1;
            if (emit_expr(t->c) != 0) return -1;
            emit_pop(0); /* value → x0 */
            emit_pop(1); /* index → x1 */
            emit_pop(2); /* base  → x2 */
            em(0xF8207800u | (1u << 16) | (2u << 5) | 0u); /* str x0,[x2,x1,lsl 3] */
            em(0xD2800000u); /* mov x0, #0 — array set evaluates to void */
            emit_push();
            return 0;
        case TERM_STR:
            /* String literal: emit the bytes into the trailing data region and
             * load their address with adr (PC-relative). Layout of the single
             * mmap'd region (base = mmap return, mem):
             *   [0, em_len*4)            = code words (em_code)
             *   [em_len*4, em_len*4+em_strlen) = string bytes (em_str)
             * The adr instruction is at word index `at` (PC = mem + at*4); the
             * string data sits at mem + em_len*4 + off. The offset it encodes
             * is (em_len*4 + off) - at*4, always a multiple of 4, patched in
             * native_eval once em_len is final. */
            {
                const char *s = t->name ? t->name : "";
                size_t len = strlen(s);
                while ((em_strlen & 3u) != 0u) em_str[em_strlen++] = 0u; /* 4B align */
                size_t off = em_strlen;
                for (size_t k = 0; k <= len && em_strlen < sizeof em_str; k++) {
                    em_str[em_strlen++] = (unsigned char)s[k];
                }
                if (em_adr_n < (int)(sizeof em_adr_at / sizeof em_adr_at[0])) {
                    em_adr_at[em_adr_n] = em_len;
                    em_adr_off[em_adr_n] = off;
                    em_adr_n++;
                }
                em(0x10000000u | 0u); /* adr x0, #imm (patched in native_eval) */
                emit_push();
                return 0;
            }
        case TERM_STR_LEN: {
            /* a = string. eval → x0 (push); pop → x0 = ptr; x2 = count; scan:
             * ldrb w1,[x0,x2]; cbz w1,end; add x2,#1; b loop; end: mov x0,x2. */
            if (emit_expr(t->a) != 0) return -1;
            emit_pop(0);
            em(0xD2800002u); /* mov x2, #0 — count */
            size_t loop = em_len;
            /* ldrb w1,[x0,x2]: Rm=count bits[20:16], Rn=ptr bits[9:5], Rt=w1. */
            em(0x38606800u | (2u << 16) | (0u << 5) | 1u);
            size_t cbz_at = em_len;
            em(0x34000000u | 1u); /* cbz w1, <end> (patched) */
            em(0x91000442u);       /* add x2, x2, #1 */
            int back = (int)(loop - em_len);
            em(0x14000000u | ((unsigned)back & 0x3FFFFFFu)); /* b loop */
            size_t end = em_len;
            int fwd = (int)(end - cbz_at);
            em_code[cbz_at] = 0x34000000u | (((unsigned)fwd & 0x7FFFFu) << 5) | 1u;
            em(0xAA0203E0u); /* mov x0, x2 — return count */
            emit_push();
            return 0;
        }
        case TERM_STR_CHAR:
            /* a = string, b = index. eval string → x0 (push), eval index → x0;
             * pop index→x1, pop base→x2; ldrb w0,[x2,x1] (byte, zero-extended
             * into x0); push. */
            if (emit_expr(t->a) != 0) return -1;
            if (emit_expr(t->b) != 0) return -1;
            emit_pop(1); /* index → x1 */
            emit_pop(2); /* base  → x2 */
            /* ldrb w0,[x2,x1]: Rm=index bits[20:16], Rn=base bits[9:5], Rt=w0. */
            em(0x38606800u | (1u << 16) | (2u << 5) | 0u);
            emit_push();
            return 0;
        case TERM_SYSCALL:
            /* raw AArch64 svc #0: t->ival = syscall nr, t->a = optional first arg */
            if (t->a) {
                if (emit_expr(t->a) != 0) return -1;
                emit_pop(0);
            } else {
                em(0xD2800000u);  /* mov x0, #0 — no arg */
            }
            em(0xD2800000u | ((unsigned)(t->ival & 0xFFFF) << 5) | 8u); /* mov x8, #nr */
            em(0xD4000001u);                      /* svc #0 */
            emit_push();
            return 0;
        case TERM_WHILE: {
            /* Label-based while loop: evaluate cond, cbz to exit, eval body,
             * jump back. Uses 4 labels: start, body_after_cond, end, cbz_patch */
            in_while++;
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
            in_while--;
            return 0;
        }
        default:
            return -1;
    }
}

/* Compile a closed term to AArch64 words WITHOUT mmap/run (benchmarking and
 * parity tooling). Returns word count or -1. */
int native_compile_words(const Term *t, unsigned int *out, size_t cap,
                         char *err, size_t cerr) {
    em_len = 0;
    nvars = 0;
    nregs = 0;
    nspill = 0;
    em_strlen = 0;
    em_adr_n = 0;
    qtt_term_pool_reset();
    if (pac_available()) em(PAC_PACIASP);
    em(0xD10803FFu);
    emit_stp_sp(19, 20, 0);
    emit_stp_sp(21, 22, 16);
    emit_stp_sp(23, 24, 32);
    emit_stp_sp(25, 26, 48);
    emit_stp_sp(27, 28, 64);
    em(0x910143EFu);
    em(0x910403EEu);
    if (emit_expr(t) != 0) {
        snprintf(err, cerr, "native: unsupported term");
        return -1;
    }
    emit_pop(0);
    emit_ldp_sp(19, 20, 0);
    emit_ldp_sp(21, 22, 16);
    emit_ldp_sp(23, 24, 32);
    emit_ldp_sp(25, 26, 48);
    emit_ldp_sp(27, 28, 64);
    em(0x910803FFu);
    if (pac_available()) em(PAC_AUTIASP);
    em(0xD65F03C0u);
    if (em_len > cap) {
        snprintf(err, cerr, "native: code too large");
        return -1;
    }
    memcpy(out, em_code, em_len * sizeof(unsigned int));
    return (int)em_len;
}

long native_eval(const Term *t, char *err, size_t cap) {
    em_len = 0;
    nvars = 0;
    nregs = 0;
    nspill = 0;
    em_strlen = 0;
    em_adr_n = 0;
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

    /* Patch the adr fixups for trailing string data. The code occupies
     * [0, em_len*4); each string lives at em_len*4 + em_adr_off[i] within the
     * same buffer. adr loads PC + (imm21<<2), so the offset from the adr
     * instruction (word index em_adr_at[i]) to the string byte is
     * (em_len*4 + off) - at*4 — always a multiple of 4. */
    size_t code_bytes = em_len * sizeof(unsigned int);
    for (int i = 0; i < em_adr_n; i++) {
        long off = (long)(code_bytes + em_adr_off[i]) -
                   (long)(em_adr_at[i] * sizeof(unsigned int));
        unsigned imm21 = (unsigned)(off >> 2) & 0x1FFFFFu;
        em_code[em_adr_at[i]] = 0x10000000u | (imm21 << 5) | 0u; /* adr x0, #off */
    }

    size_t sz = code_bytes + em_strlen;
    /* W^X (#21 / 2B): two-step — map writeable (NOT executable), emit the
     * code, then flip to executable (NOT writeable). Never W+X at once, so a
     * code-injection write cannot land in an executable page. */
    void *mem = mmap(NULL, sz, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS,
                     -1, 0);
    if (mem == MAP_FAILED) {
        snprintf(err, cap, "mmap failed");
        return 0;
    }
    memcpy(mem, em_code, code_bytes);
    if (em_strlen) memcpy((char *)mem + code_bytes, em_str, em_strlen);
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
    /* native syscall: getpid() (nr=172, argless) */
    {
        static Term sc;
        memset(&sc, 0, sizeof sc);
        sc.kind = TERM_SYSCALL; sc.ival = 172; sc.a = NULL;
        long pid = native_eval(&sc, err, sizeof err);
        N(pid > 0, "native getpid() > 0 (syscall svc #0)");
    }

    /* native array alloc */
    {
        static Term t; static TermField f; static Term e;
        memset(&t, 0, sizeof t); memset(&e, 0, sizeof e);
        t.kind = TERM_ARRAY; t.nfields = 1;
        e.kind = TERM_LIT; e.ival = 99; f.val = &e; t.fields = &f;
        long ar = native_eval(&t, err, sizeof err);
        N(ar != 0 && ar != 99, "native alloc [99] returns ptr");
    }
    /* native array indexing (register offset) */
    {
        static Term ap[6]; static TermField af[2];
        Term *e1 = &ap[0]; memset(e1, 0, 64); e1->kind = TERM_LIT; e1->ival = 10;
        Term *e2 = &ap[1]; memset(e2, 0, 64); e2->kind = TERM_LIT; e2->ival = 20;
        af[0].val = e1; af[1].val = e2;
        Term *arr = &ap[2]; memset(arr, 0, 64); arr->kind = TERM_ARRAY; arr->fields = af; arr->nfields = 2;
        Term *i1 = &ap[3]; memset(i1, 0, 64); i1->kind = TERM_LIT; i1->ival = 1;
        Term *get = &ap[4]; memset(get, 0, 64); get->kind = TERM_ARRAY_GET; get->a = arr; get->b = i1;
        long g = native_eval(get, err, sizeof err);
        N(g == 20, "native [10,20][1] == 20");
    }
    /* native array set + get (mutation, via let-bound array) */
    {
        static Term pool[16]; static TermField fields[3];
        int pi = 0;
        Term *e1 = &pool[pi++]; memset(e1, 0, sizeof *e1); e1->kind = TERM_LIT; e1->ival = 10;
        Term *e2 = &pool[pi++]; memset(e2, 0, sizeof *e2); e2->kind = TERM_LIT; e2->ival = 20;
        Term *e3 = &pool[pi++]; memset(e3, 0, sizeof *e3); e3->kind = TERM_LIT; e3->ival = 30;
        fields[0].name = "0"; fields[0].val = e1;
        fields[1].name = "1"; fields[1].val = e2;
        fields[2].name = "2"; fields[2].val = e3;
        Term *arr = &pool[pi++]; memset(arr, 0, sizeof *arr);
        arr->kind = TERM_ARRAY; arr->fields = fields; arr->nfields = 3;
        Term *avar = &pool[pi++]; memset(avar, 0, sizeof *avar); avar->kind = TERM_VAR; avar->name = "a";
        Term *idx = &pool[pi++]; memset(idx, 0, sizeof *idx); idx->kind = TERM_LIT; idx->ival = 1;
        Term *val = &pool[pi++]; memset(val, 0, sizeof *val); val->kind = TERM_LIT; val->ival = 99;
        Term *set = &pool[pi++]; memset(set, 0, sizeof *set);
        set->kind = TERM_ARRAY_SET; set->a = avar; set->b = idx; set->c = val;
        Term *get = &pool[pi++]; memset(get, 0, sizeof *get);
        get->kind = TERM_ARRAY_GET; get->a = avar; get->b = idx;
        Term *add = &pool[pi++]; memset(add, 0, sizeof *add);
        add->kind = TERM_BIN; add->op = BOP_ADD; add->a = set; add->b = get;
        Term *let = &pool[pi++]; memset(let, 0, sizeof *let);
        let->kind = TERM_LET; let->name = "a"; let->a = arr; let->b = add;
        long got = native_eval(let, err, sizeof err);
        N(got == 99, "native let a=[10,20,30] in (a[1]=99)+a[1] == 99");
    }
    /* native string literals: str_len / str_char */
    {
        static Term pool[12];
        int pi = 0;
        Term *hello = &pool[pi++]; memset(hello, 0, sizeof *hello);
        hello->kind = TERM_STR; hello->name = "hello";
        Term *slen = &pool[pi++]; memset(slen, 0, sizeof *slen);
        slen->kind = TERM_STR_LEN; slen->a = hello;
        long n = native_eval(slen, err, sizeof err);
        N(n == 5, "native str_len(\"hello\") == 5");

        Term *empty = &pool[pi++]; memset(empty, 0, sizeof *empty);
        empty->kind = TERM_STR; empty->name = "";
        Term *elen = &pool[pi++]; memset(elen, 0, sizeof *elen);
        elen->kind = TERM_STR_LEN; elen->a = empty;
        long e = native_eval(elen, err, sizeof err);
        N(e == 0, "native str_len(\"\") == 0");

        Term *idx0 = &pool[pi++]; memset(idx0, 0, sizeof *idx0); idx0->kind = TERM_LIT; idx0->ival = 0;
        Term *idx1 = &pool[pi++]; memset(idx1, 0, sizeof *idx1); idx1->kind = TERM_LIT; idx1->ival = 1;
        Term *sc0 = &pool[pi++]; memset(sc0, 0, sizeof *sc0);
        sc0->kind = TERM_STR_CHAR; sc0->a = hello; sc0->b = idx0;
        long c0 = native_eval(sc0, err, sizeof err);
        N(c0 == 'h', "native char(\"hello\",0) == 'h' (104)");
        Term *sc1 = &pool[pi++]; memset(sc1, 0, sizeof *sc1);
        sc1->kind = TERM_STR_CHAR; sc1->a = hello; sc1->b = idx1;
        long c1 = native_eval(sc1, err, sizeof err);
        N(c1 == 'e', "native char(\"hello\",1) == 'e' (101)");
    }
    return all_ok ? 0 : -1;
}
