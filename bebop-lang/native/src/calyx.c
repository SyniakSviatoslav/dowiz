/* Bebop calyx — implementation. Emits Calyx IR for FPGA/ASIC synthesis. */
#include "calyx.h"
#include <stdio.h>
#include <stdarg.h>
#include <string.h>

static int emit(char *out, size_t cap, size_t *pos, const char *fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(out + *pos, cap - *pos, fmt, ap);
    va_end(ap);
    if (n < 0 || (size_t)n >= cap - *pos) return -1;
    *pos += (size_t)n;
    return 0;
}

/* Modular multiply-accumulate: acc = (acc + (a*b mod p)) mod p */
int calyx_emit_mac(const char *comp_name, unsigned bw, char *out, size_t cap) {
    size_t pos = 0;
    if (emit(out, cap, &pos, "import \"primitives/core.futil\";\n\n")) return -1;
    if (emit(out, cap, &pos, "component %s(@go go: 1, @clk clk: 1, @reset reset: 1) -> (@done done: 1) {\n", comp_name)) return -1;
    if (emit(out, cap, &pos, "  cells {\n    a = std_reg(%u);\n    b = std_reg(%u);\n    acc = std_reg(%u);\n    mul = std_mult_pipe(%u);\n    add = std_add(%u);\n    mod = std_mod_pipe(%u);\n  }\n", bw, bw, bw, bw, bw, bw)) return -1;
    if (emit(out, cap, &pos, "  wires {\n    group do_mac {\n      mul.left = a.out;\n      mul.right = b.out;\n      mul.go = !mul.done ? 1'd1;\n      add.left = mul.out;\n      add.right = acc.out;\n      mod.left = add.out;\n      mod.right = %u'd998244353;\n      acc.in = mod.out;\n      acc.write_en = mul.done ? 1'd1;\n      do_mac[done] = mul.done ? 1'd1;\n    }\n  }\n  control { do_mac; }\n}\n", bw)) return -1;
    return (int)pos;
}

/* NTT butterfly: (u+v mod p, u-v mod p) with twiddle multiply. */
int calyx_emit_butterfly(const char *comp_name, unsigned bw, char *out, size_t cap) {
    size_t pos = 0;
    if (emit(out, cap, &pos, "component %s(@go go: 1, @clk clk: 1, @reset reset: 1) -> (@done done: 1) {\n", comp_name)) return -1;
    if (emit(out, cap, &pos, "  cells {\n    u = std_reg(%u);\n    v = std_reg(%u);\n    w = std_reg(%u);\n    t = std_mult_pipe(%u);\n    tmod = std_mod_pipe(%u);\n    s = std_add(%u);\n    smod = std_mod_pipe(%u);\n    d = std_sub(%u);\n    dmod = std_mod_pipe(%u);\n  }\n  wires {\n    group butterfly {\n      t.left = v.out;\n      t.right = w.out;\n      tmod.left = t.out;\n      tmod.right = %u'd998244353;\n      s.left = u.out;\n      s.right = tmod.out;\n      smod.left = s.out;\n      smod.right = %u'd998244353;\n      d.left = u.out;\n      d.right = tmod.out;\n      dmod.left = d.out;\n      dmod.right = %u'd998244353;\n      butterfly[done] = 1'd1;\n    }\n  }\n  control { butterfly; }\n}\n", bw, bw, bw, bw, bw, bw, bw, bw, bw, bw)) return -1;
    return (int)pos;
}

int calyx_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define C(cond, name) do { \
        int c_ = (int)(cond); \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n", c_ ? "ok" : "FAIL", name); \
        if (r_ > 0) pos += (size_t)r_; \
        if (!c_) all_ok = 0; \
    } while (0)

    char buf[4096];
    int n = calyx_emit_mac("mac_mod", 32, buf, sizeof buf);
    C(n > 0 && strstr(buf, "component mac_mod"), "calyx mac component emitted");
    C(n > 0 && strstr(buf, "std_mod_pipe"), "calyx mac has mod pipe");
    C(n > 0 && strstr(buf, "control"), "calyx mac has control block");

    n = calyx_emit_butterfly("ntt_bfly", 32, buf, sizeof buf);
    C(n > 0 && strstr(buf, "component ntt_bfly"), "calyx butterfly component emitted");
    C(n > 0 && strstr(buf, "std_sub"), "calyx butterfly has subtract");
    C(n > 0 && strstr(buf, "998244353"), "calyx uses NTT prime 998244353");
#undef C
    return all_ok ? 0 : 1;
}