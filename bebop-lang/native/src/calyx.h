/* Bebop calyx — FPGA/ASIC backend: emits Calyx IR (for Calyx/CIRCT toolchain).
 * Generates a hardware component from a Bebop NTT kernel specification.
 * Pure C11, zero deps — produces textual Calyx, consumed by `fud`/CIRCT. */
#ifndef BEBOP_CALYX_H
#define BEBOP_CALYX_H

#include <stddef.h>

/* Emit a Calyx component implementing modular multiply-accumulate.
 * out: buffer, cap: capacity. Returns bytes written or -1 on overflow. */
int calyx_emit_mac(const char *comp_name, unsigned bitwidth, char *out, size_t cap);

/* Emit a Calyx component implementing an NTT butterfly (mod add/sub). */
int calyx_emit_butterfly(const char *comp_name, unsigned bitwidth, char *out, size_t cap);

int calyx_self_test(char *out, size_t cap);

#endif