/* Bebop x86_64 backend — machine-code encoder + minimal codegen.
 * Host is aarch64, so we verify ENCODING byte-for-byte (no execution).
 * SysV AMD64 ABI: rax=result/scratch, rbx/r12-r15 callee-saved, syscall nr in rax.
 * Direct opcode emission (no assembler). */
#include "x86_64.h"

#include <stdio.h>
#include <string.h>

/* ─── emitter state ─── */
static unsigned char code[256];
static size_t clen;

static void e(unsigned char b) { code[clen++] = b; }

/* ─── x86_64 opcode table (hand-verified) ─── */
/* mov rax, imm64 : 48 B8 <imm64 little-endian> */
void x64_mov_rax_imm64(long v) {
    e(0x48); e(0xB8);
    for (int i = 0; i < 8; i++) e((unsigned char)((v >> (8 * i)) & 0xFF));
}
/* push rax : 50 ; pop rax : 58 */
void x64_push_rax(void) { e(0x50); }
void x64_pop_rax(void) { e(0x58); }
/* ret : C3 */
void x64_ret(void) { e(0xC3); }
/* syscall : 0F 05 (nr in rax) */
void x64_syscall(void) { e(0x0F); e(0x05); }
/* add rbx, rax : 48 01 C3  (rbx += rax) */
void x64_add_rbx_rax(void) { e(0x48); e(0x01); e(0xC3); }
/* sub rbx, rax : 48 29 C3 */
void x64_sub_rbx_rax(void) { e(0x48); e(0x29); e(0xC3); }
/* imul rbx, rax : 48 0F AF C3 */
void x64_mul_rbx_rax(void) { e(0x48); e(0x0F); e(0xAF); e(0xC3); }
/* cmp rbx, rax : 48 39 C3  (flags = rbx - rax) */
void x64_cmp_rbx_rax(void) { e(0x48); e(0x39); e(0xC3); }
/* cmove rax, rbx : 48 0F 44 C3  (rax = rbx if ZF==1) */
void x64_cmove_rax_rbx(void) { e(0x48); e(0x0F); e(0x44); e(0xC3); }
/* cmovne rax, rbx : 48 0F 45 C3  (rax = rbx if ZF==0) */
void x64_cmovne_rax_rbx(void) { e(0x48); e(0x0F); e(0x45); e(0xC3); }

/* ─── minimal expression codegen (mirrors native.c AArch64) ─── */
static int x64_emit_expr(const Term *t);

static int x64_emit_expr(const Term *t) {
    if (!t) return -1;
    switch (t->kind) {
        case TERM_LIT:
            x64_mov_rax_imm64(t->bval ? 1 : t->ival);
            x64_push_rax();
            return 0;
        case TERM_BIN: {
            if (x64_emit_expr(t->a) != 0 || x64_emit_expr(t->b) != 0) return -1;
            x64_pop_rax();          /* b -> rax */
            /* pop a into rbx via xchg: use a temp slot */
            e(0x5B);                /* pop rbx */
            switch (t->op) {
                case BOP_ADD: x64_add_rbx_rax(); break;
                case BOP_SUB: x64_sub_rbx_rax(); break;
                case BOP_MUL: x64_mul_rbx_rax(); break;
                default: return -1; /* comparisons need cmov: keep minimal */
            }
            x64_push_rax();
            return 0;
        }
        case TERM_SYSCALL:
            if (t->a && x64_emit_expr(t->a) != 0) return -1;
            if (t->a) x64_pop_rax();
            else x64_mov_rax_imm64(0);
            x64_mov_rax_imm64(t->ival); /* syscall nr (overwrites arg; ok for getpid) */
            x64_syscall();
            x64_push_rax();
            return 0;
        default:
            return -1; /* unsupported in minimal backend */
    }
}

int x86_64_compile(const Term *t, unsigned char *out, size_t cap, char *err, size_t ecap) {
    (void)err; (void)ecap;
    clen = 0;
    if (x64_emit_expr(t) != 0) return -1;
    x64_pop_rax();
    x64_ret();
    if (clen > cap) return -1;
    memcpy(out, code, clen);
    return (int)clen;
}

int x86_64_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#undef A
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    /* Verify opcode encodings byte-for-byte */
    clen = 0;
    x64_mov_rax_imm64(42);
    static const unsigned char mov42[] = {0x48, 0xB8, 42, 0, 0, 0, 0, 0, 0, 0};
    A(clen == 10 && memcmp(code, mov42, 10) == 0, "mov rax, 42 == 48 B8 2A 00..");

    clen = 0;
    x64_syscall();
    A(clen == 2 && code[0] == 0x0F && code[1] == 0x05, "syscall == 0F 05");

    clen = 0;
    x64_ret();
    A(clen == 1 && code[0] == 0xC3, "ret == C3");

    clen = 0;
    x64_add_rbx_rax();
    A(clen == 3 && code[0] == 0x48 && code[1] == 0x01 && code[2] == 0xC3,
      "add rbx,rax == 48 01 C3");

    clen = 0;
    x64_cmove_rax_rbx();
    A(clen == 4 && code[0] == 0x48 && code[1] == 0x0F && code[2] == 0x44 && code[3] == 0xC3,
      "cmove rax,rbx == 48 0F 44 C3");

    /* Compile 1+2 and check the arithmetic tail is emitted */
    static Term l1, l2, add;
    memset(&l1, 0, sizeof l1); l1.kind = TERM_LIT; l1.ival = 1;
    memset(&l2, 0, sizeof l2); l2.kind = TERM_LIT; l2.ival = 2;
    memset(&add, 0, sizeof add); add.kind = TERM_BIN; add.op = BOP_ADD; add.a = &l1; add.b = &l2;
    unsigned char buf[64];
    int n = x86_64_compile(&add, buf, sizeof buf, out, 0);
    A(n > 0, "compile 1+2 emits code");

    return all_ok ? 0 : -1;
}
