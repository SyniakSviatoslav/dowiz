/* Bebop chain — implementation. */
#include "chain.h"
#include <string.h>
#include <stdio.h>

void chain_init(Chain *c) { memset(c, 0, sizeof *c); }

int chain_add(Chain *c, const char *name, ChainStepFn fn) {
    if (c->n_steps >= CHAIN_MAX_STEPS) return -1;
    c->steps[c->n_steps].name = name;
    c->steps[c->n_steps].fn = fn;
    return c->n_steps++;
}

int chain_run(Chain *c, const char *input, char *output, size_t cap, void *ctx) {
    if (c->n_steps == 0) { if (cap) output[0] = 0; return 0; }
    char buf_a[CHAIN_BUF_SIZE], buf_b[CHAIN_BUF_SIZE];
    const char *cur = input;
    for (int i = 0; i < c->n_steps; i++) {
        char *dst = (i & 1) ? buf_b : buf_a;
        int n = c->steps[i].fn(cur, dst, CHAIN_BUF_SIZE, ctx);
        if (n < 0) return -1;
        dst[n] = 0;
        cur = dst;
    }
    size_t len = strlen(cur);
    if (len >= cap) len = cap - 1;
    memcpy(output, cur, len);
    output[len] = 0;
    return (int)len;
}

int chain_step_uppercase(const char *in, char *out, size_t cap, void *ctx) {
    (void)ctx;
    size_t i;
    for (i = 0; in[i] && i + 1 < cap; i++)
        out[i] = (in[i] >= 'a' && in[i] <= 'z') ? (char)(in[i] - 32) : in[i];
    out[i] = 0;
    return (int)i;
}

int chain_step_reverse(const char *in, char *out, size_t cap, void *ctx) {
    (void)ctx;
    size_t len = strlen(in);
    if (len >= cap) len = cap - 1;
    for (size_t i = 0; i < len; i++) out[i] = in[len - 1 - i];
    out[len] = 0;
    return (int)len;
}

int chain_step_echo(const char *in, char *out, size_t cap, void *ctx) {
    (void)ctx;
    size_t len = strlen(in);
    if (len >= cap) len = cap - 1;
    memcpy(out, in, len);
    out[len] = 0;
    return (int)len;
}

int chain_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    Chain c;
    chain_init(&c);
    chain_add(&c, "upper", chain_step_uppercase);
    chain_add(&c, "reverse", chain_step_reverse);
    char r[256];
    chain_run(&c, "hello", r, sizeof r, NULL);
    T(strcmp(r, "OLLEH") == 0, "chain: hello -> upper -> reverse = OLLEH");

    chain_init(&c);
    chain_add(&c, "echo", chain_step_echo);
    chain_run(&c, "test", r, sizeof r, NULL);
    T(strcmp(r, "test") == 0, "chain: single echo returns input");

#undef T
    return fail;
}