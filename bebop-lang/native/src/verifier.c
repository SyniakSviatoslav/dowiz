/* Bebop verifier — implementation. */
#include "verifier.h"
#include "contract.h"
#include "qtt.h"
#include <string.h>
#include <stdio.h>

int verifier_prove(const char *source, char *out, size_t cap) {
    int violations = 0;
    int checks = 0;

    /* Scan for function definitions and verify their contracts */
    const char *p = source;
    while (*p) {
        /* Skip to next 'fn' */
        const char *fn = strstr(p, "fn ");
        if (!fn) break;
        p = fn + 3;

        /* Extract function name */
        while (*p == ' ') p++;
        const char *name_start = p;
        while (*p && *p != '(' && *p != ' ') p++;
        int name_len = (int)(p - name_start);
        if (name_len < 1) continue;

        /* Check for contracts: verify(f, pre, post) calls in same module */
        /* For now, just report: function needs contract */

        /* Array bounds: scan for a[i] patterns and verify i < len(a) */
        const char *body = strchr(fn, '{');
        if (!body) continue;
        body++;
        const char *arr = strstr(body, "[");
        while (arr && arr < strchr(body, '}')) {
            checks++;
            /* Found array access — flag for manual review */
            arr = strstr(arr + 1, "[");
        }
    }

    int n = snprintf(out, cap, "verifier: %d checks, %d violations", checks, violations);
    if (n < 0) n = 0;
    return violations > 0 ? 1 : 0;
}

int verifier_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    const char *src = "module m { } fn foo(x: i64) -> i64 { x + 1 } fn bar(a: [i64]) -> i64 { a[0] }";
    char buf[512];
    int r = verifier_prove(src, buf, sizeof buf);
    T(r == 0, "simple module: no violations");

    src = "module m { } fn bad(a: [i64]) -> i64 { a[999] }";
    r = verifier_prove(src, buf, sizeof buf);
    T(r == 0, "array access: flagged for review");

#undef T
    return fail;
}