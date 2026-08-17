/* Bebop codegen — WebAssembly emission (own backend, no LLVM). */
#include "codegen.h"

#include <stdio.h>
#include <string.h>

#include "expr.h"

/* ─── LEB128 ─── */
static size_t uleb(unsigned char *out, unsigned long v) {
    size_t n = 0;
    do {
        unsigned char b = (unsigned char)(v & 0x7f);
        v >>= 7;
        if (v) {
            b |= 0x80;
        }
        out[n++] = b;
    } while (v);
    return n;
}

static size_t sleb(unsigned char *out, long v) {
    size_t n = 0;
    int more = 1;
    while (more) {
        unsigned char b = (unsigned char)(v & 0x7f);
        v >>= 7;
        if ((v == 0 && !(b & 0x40)) || (v == -1 && (b & 0x40))) {
            more = 0;
        } else {
            b |= 0x80;
        }
        out[n++] = b;
    }
    return n;
}

/* Count nested let-bindings (for local declaration). */
static int count_lets(const Term *t) {
    if (!t) {
        return 0;
    }
    int n = (t->kind == TERM_LET) ? 1 : 0;
    n += count_lets(t->a);
    n += count_lets(t->b);
    n += count_lets(t->c);
    return n;
}

typedef struct {
    const char *names[64];
    int idxs[64];
    int n;    /* active bindings */
    int next; /* next local index */
} CodeCtx;

/* Emit instructions for a term. want_i64: 1 if the context expects i64,
 * 0 if it expects i32 (bool). Returns byte count, or -1 on unsupported term. */
static int emit_expr(unsigned char *out, const Term *t, int want_i64,
                     CodeCtx *ctx) {
    switch (t->kind) {
        case TERM_LIT:
            if (t->bval) {
                out[0] = 0x41; /* i32.const */
                return 1 + (int)sleb(out + 1, t->bval ? 1 : 0);
            }
            out[0] = 0x42; /* i64.const */
            return 1 + (int)sleb(out + 1, t->ival);
        case TERM_VAR: {
            for (int i = 0; i < ctx->n; i++) {
                if (strcmp(ctx->names[i], t->name) == 0) {
                    out[0] = 0x20; /* local.get */
                    return 1 + (int)uleb(out + 1, (unsigned long)ctx->idxs[i]);
                }
            }
            return -1;
        }
        case TERM_BIN: {
            int l = emit_expr(out, t->a, 1, ctx);
            if (l < 0) {
                return -1;
            }
            int r = emit_expr(out + l, t->b, 1, ctx);
            if (r < 0) {
                return -1;
            }
            unsigned char op;
            switch (t->op) {
                case BOP_ADD: op = 0x7c; break;
                case BOP_SUB: op = 0x7d; break;
                case BOP_MUL: op = 0x7e; break;
                case BOP_EQ:  op = 0x51; break;
                case BOP_NE:  op = 0x52; break;
                case BOP_LT:  op = 0x53; break;
                case BOP_GT:  op = 0x55; break;
                case BOP_LE:  op = 0x57; break;
                case BOP_GE:  op = 0x59; break;
                default:      op = 0x7c; break;
            }
            out[l + r] = op;
            return l + r + 1;
        }
        case TERM_IF: {
            int c = emit_expr(out, t->a, 0, ctx);
            if (c < 0) {
                return -1;
            }
            out[c] = 0x04;
            out[c + 1] = want_i64 ? 0x7e : 0x7f;
            int th = emit_expr(out + c + 2, t->b, want_i64, ctx);
            if (th < 0) {
                return -1;
            }
            out[c + 2 + th] = 0x05;
            int el = emit_expr(out + c + 3 + th, t->c, want_i64, ctx);
            if (el < 0) {
                return -1;
            }
            out[c + 3 + th + el] = 0x0b;
            return c + 4 + th + el;
        }
        case TERM_LET: {
            int v = emit_expr(out, t->a, 1, ctx);
            if (v < 0) {
                return -1;
            }
            int idx = ctx->next++;
            out[v] = 0x21; /* local.set */
            v += 1 + (int)uleb(out + v + 1, (unsigned long)idx);
            ctx->names[ctx->n] = t->name;
            ctx->idxs[ctx->n] = idx;
            ctx->n++;
            int body = emit_expr(out + v, t->b, want_i64, ctx);
            ctx->n--;
            if (body < 0) {
                return -1;
            }
            return v + body;
        }
        default:
            return -1; /* lambda/app/ann not yet in codegen */
    }
}

int codegen_wasm(const Term *t, unsigned char *out, size_t cap, char *err,
                 size_t cap_err) {
    char ty[64];
    if (qtt_check_closed(t, ty, sizeof ty, err, cap_err) != 0) {
        return -1;
    }
    int want_i64 = (strcmp(ty, "i64") == 0);

    size_t n = 0;
    out[n++] = 0x00; out[n++] = 0x61; out[n++] = 0x73; out[n++] = 0x6d; /* magic */
    out[n++] = 0x01; out[n++] = 0x00; out[n++] = 0x00; out[n++] = 0x00; /* version */

    /* type section: 1 func type () -> (i64|i32) */
    unsigned char ts[] = {0x01, 0x60, 0x00, 0x01, 0x7e};
    ts[4] = want_i64 ? 0x7e : 0x7f;
    out[n++] = 0x01;
    n += uleb(out + n, sizeof ts);
    memcpy(out + n, ts, sizeof ts);
    n += sizeof ts;

    /* function section: 1 func, type 0 */
    unsigned char fs[] = {0x01, 0x00};
    out[n++] = 0x03;
    n += uleb(out + n, sizeof fs);
    memcpy(out + n, fs, sizeof fs);
    n += sizeof fs;

    /* export section: export "main" (func 0) */
    unsigned char es[] = {0x01, 0x04, 'm', 'a', 'i', 'n', 0x00, 0x00};
    out[n++] = 0x07;
    n += uleb(out + n, sizeof es);
    memcpy(out + n, es, sizeof es);
    n += sizeof es;

    /* code section: 1 body */
    int nlets = count_lets(t);
    unsigned char body[1024];
    size_t bn = 0;
    body[bn++] = nlets > 0 ? 1 : 0; /* local decl group count */
    if (nlets > 0) {
        bn += uleb(body + bn, (unsigned long)nlets);
        body[bn++] = 0x7e; /* i64 */
    }
    CodeCtx ctx;
    memset(&ctx, 0, sizeof ctx);
    int en = emit_expr(body + bn, t, want_i64, &ctx);
    if (en < 0) {
        snprintf(err, cap_err, "codegen: term not yet supported (lambda/app)");
        return -1;
    }
    bn += (size_t)en;
    body[bn++] = 0x0b; /* end */

    unsigned char cs[1024];
    size_t cn = 0;
    cs[cn++] = 0x01; /* function count = 1 */
    cn += uleb(cs + cn, bn);
    memcpy(cs + cn, body, bn);
    cn += bn;

    out[n++] = 0x0a;
    n += uleb(out + n, cn);
    if (n + cn > cap) {
        snprintf(err, cap_err, "codegen buffer overflow");
        return -1;
    }
    memcpy(out + n, cs, cn);
    n += cn;

    return (int)n;
}

int codegen_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[128];
    unsigned char buf[2048];
#define C(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    /* build a simple term 1 + 2 via the expr parser */
    Term *t = NULL;
    expr_pool_reset();
    if (expr_parse("1 + 2", &t, err, sizeof err) != 0) {
        C(0, "parse '1 + 2'");
        return -1;
    }
    int n = codegen_wasm(t, buf, sizeof buf, err, sizeof err);
    C(n > 0, "emit WASM module");
    C(n >= 8 && buf[0] == 0x00 && buf[1] == 0x61 && buf[2] == 0x73 &&
          buf[3] == 0x6d,
      "WASM magic \\0asm");
    C(n >= 8 && buf[4] == 0x01 && buf[5] == 0x00 && buf[6] == 0x00 &&
          buf[7] == 0x00,
      "WASM version 1");
    /* sections present: type(1), function(3), export(7), code(10) */
    int has_type = 0, has_func = 0, has_export = 0, has_code = 0;
    for (int i = 8; i < n;) {
        unsigned char id = buf[i];
        int k = i + 1;
        unsigned long sz = 0, shift = 0;
        while (k < n) {
            unsigned char b = buf[k++];
            sz |= (b & 0x7f) << shift;
            if (!(b & 0x80)) break;
            shift += 7;
        }
        if (id == 0x01) has_type = 1;
        if (id == 0x03) has_func = 1;
        if (id == 0x07) has_export = 1;
        if (id == 0x0a) has_code = 1;
        i = k + (int)sz;
    }
    C(has_type && has_func && has_export && has_code,
      "all 4 sections present");

    /* bool expression compiles too */
    Term *b = NULL;
    expr_pool_reset();
    if (expr_parse("5 > 3", &b, err, sizeof err) == 0) {
        int m = codegen_wasm(b, buf, sizeof buf, err, sizeof err);
        C(m > 0, "emit WASM for bool expression");
    }

    /* let expression compiles */
    Term *lt = NULL;
    expr_pool_reset();
    if (expr_parse("let x = 5 in (x + 3) * 2", &lt, err, sizeof err) == 0) {
        int m = codegen_wasm(lt, buf, sizeof buf, err, sizeof err);
        C(m > 0, "emit WASM for let expression");
    }

    return all_ok ? 0 : -1;
}

int codegen_wasm_fn(const Term *lam, unsigned char *out, size_t cap, char *err,
                    size_t cap_err) {
    if (!lam || lam->kind != TERM_LAM) {
        snprintf(err, cap_err, "codegen: expected lambda");
        return -1;
    }
    char ty[64];
    if (qtt_check_closed(lam, ty, sizeof ty, err, cap_err) != 0) {
        return -1;
    }
    const char *arrow = strrchr(ty, '>');
    int result_i64 = 1;
    if (arrow && strstr(arrow, "bool") != NULL) {
        result_i64 = 0;
    }
    int param_i64 = (lam->ty && lam->ty->kind == TY_I64) ? 1 : 0;

    size_t n = 0;
    out[n++] = 0x00; out[n++] = 0x61; out[n++] = 0x73; out[n++] = 0x6d;
    out[n++] = 0x01; out[n++] = 0x00; out[n++] = 0x00; out[n++] = 0x00;

    unsigned char ts[8];
    size_t tn = 0;
    ts[tn++] = 0x01;
    ts[tn++] = 0x60;
    ts[tn++] = 0x01;
    ts[tn++] = (unsigned char)(param_i64 ? 0x7e : 0x7f);
    ts[tn++] = 0x01;
    ts[tn++] = (unsigned char)(result_i64 ? 0x7e : 0x7f);
    out[n++] = 0x01;
    n += uleb(out + n, tn);
    memcpy(out + n, ts, tn);
    n += tn;

    unsigned char fs[] = {0x01, 0x00};
    out[n++] = 0x03;
    n += uleb(out + n, sizeof fs);
    memcpy(out + n, fs, sizeof fs);
    n += sizeof fs;

    unsigned char es[] = {0x01, 0x04, 'm', 'a', 'i', 'n', 0x00, 0x00};
    out[n++] = 0x07;
    n += uleb(out + n, sizeof es);
    memcpy(out + n, es, sizeof es);
    n += sizeof es;

    int nlets = count_lets(lam->a);
    unsigned char body[1024];
    size_t bn = 0;
    body[bn++] = (unsigned char)(nlets > 0 ? 1 : 0);
    if (nlets > 0) {
        bn += uleb(body + bn, (unsigned long)nlets);
        body[bn++] = 0x7e;
    }
    CodeCtx ctx;
    memset(&ctx, 0, sizeof ctx);
    ctx.names[0] = lam->name;
    ctx.idxs[0] = 0;
    ctx.n = 1;
    ctx.next = 1;
    int en = emit_expr(body + bn, lam->a, result_i64, &ctx);
    if (en < 0) {
        snprintf(err, cap_err, "codegen: unsupported term in fn body");
        return -1;
    }
    bn += (size_t)en;
    body[bn++] = 0x0b;

    unsigned char cs[1024];
    size_t cn = 0;
    cs[cn++] = 0x01;
    cn += uleb(cs + cn, bn);
    memcpy(cs + cn, body, bn);
    cn += bn;

    out[n++] = 0x0a;
    n += uleb(out + n, cn);
    if (n + cn > cap) {
        snprintf(err, cap_err, "codegen buffer overflow");
        return -1;
    }
    memcpy(out + n, cs, cn);
    n += cn;
    return (int)n;
}
