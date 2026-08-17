/* Bebop verification — implementation. */
#include "verify.h"

#include <stdio.h>
#include <string.h>

#include "expr.h"
#include "qtt.h"

static int truth_of(int kind, long i, int b) {
    return (kind == 1) ? b : (i != 0);
}

int verify_bounded(const char *body, const char *requires, const char *ensures,
                   long lo, long hi, char *out, size_t cap) {
    expr_pool_reset();
    Term *body_t = NULL, *pre_t = NULL, *post_t = NULL;
    char err[256];
    if (expr_parse(body, &body_t, err, sizeof err) != 0) {
        snprintf(out, cap, "parse body: %s", err);
        return -1;
    }
    if (expr_parse(requires, &pre_t, err, sizeof err) != 0) {
        snprintf(out, cap, "parse requires: %s", err);
        return -1;
    }
    if (expr_parse(ensures, &post_t, err, sizeof err) != 0) {
        snprintf(out, cap, "parse ensures: %s", err);
        return -1;
    }

    long checked = 0;
    for (long x = lo; x <= hi; x++) {
        int k;
        long i;
        int b;
        QttBind bx[1] = {{"x", x}};
        if (qtt_eval_bound(pre_t, bx, 1, &k, &i, &b, err, sizeof err) != 0) {
            snprintf(out, cap, "eval requires: %s", err);
            return -1;
        }
        if (!truth_of(k, i, b)) {
            continue; /* precondition false — vacuous */
        }
        if (qtt_eval_bound(body_t, bx, 1, &k, &i, &b, err, sizeof err) != 0) {
            snprintf(out, cap, "eval body: %s", err);
            return -1;
        }
        long result = truth_of(k, i, b) ? (long)(b ? 1 : i) : i;
        QttBind bxr[2] = {{"x", x}, {"result", result}};
        if (qtt_eval_bound(post_t, bxr, 2, &k, &i, &b, err, sizeof err) != 0) {
            snprintf(out, cap, "eval ensures: %s", err);
            return -1;
        }
        checked++;
        if (!truth_of(k, i, b)) {
            snprintf(out, cap,
                     "counterexample: x=%ld, result=%ld (violates ensures)", x,
                     result);
            return -1;
        }
    }
    snprintf(out, cap, "verified (%ld cases)", checked);
    return 0;
}

int verify_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char buf[256];
#define V(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    V(verify_bounded("x * 2", "x > 0", "result > x", 0, 10, buf, sizeof buf) == 0,
      "verify: x>0 ⟹ 2x>x (10 cases)");
    V(strstr(buf, "10") != NULL, "  ... reports 10 cases");

    V(verify_bounded("x - 1", "x >= 0", "result >= 0", 0, 5, buf, sizeof buf) == -1,
      "verify: x>=0 ⟹ x-1>=0 fails (counterexample)");
    V(strstr(buf, "counterexample") != NULL && strstr(buf, "x=0") != NULL,
      "  ... counterexample x=0");

    V(verify_bounded("x * x", "true", "result >= 0", -5, 5, buf, sizeof buf) == 0,
      "verify: x*x >= 0 always (11 cases)");

    V(verify_bounded("x + 1", "true", "result > x", 0, 100, buf, sizeof buf) == 0,
      "verify: x+1 > x always (101 cases)");

    return all_ok ? 0 : -1;
}
