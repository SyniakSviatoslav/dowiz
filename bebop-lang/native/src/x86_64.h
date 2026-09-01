/* Bebop x86_64 backend — encoder + minimal codegen (byte-verified, no exec). */
#ifndef BEBOP_X86_64_H
#define BEBOP_X86_64_H

#include <stddef.h>

#include "qtt.h"

/* Encode one instruction (advanced for byte-verification tests). */
void x64_mov_rax_imm64(long v);
void x64_push_rax(void);
void x64_pop_rax(void);
void x64_ret(void);
void x64_syscall(void);
void x64_add_rbx_rax(void);
void x64_sub_rbx_rax(void);
void x64_mul_rbx_rax(void);
void x64_cmp_rbx_rax(void);
void x64_cmove_rax_rbx(void);
void x64_cmovne_rax_rbx(void);

/* Compile a term to x86_64 machine code. Returns bytes written or -1. */
int x86_64_compile(const Term *t, unsigned char *out, size_t cap, char *err, size_t ecap);

/* Run the x86_64 encoder self-test. Returns 0 on success. */
int x86_64_self_test(char *out, size_t cap);

#endif /* BEBOP_X86_64_H */
