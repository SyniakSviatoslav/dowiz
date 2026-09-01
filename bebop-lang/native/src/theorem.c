/* Bebop theorem surface — implementation.
 *
 * The proof fragment is parsed into proof-kernel terms. Currently supported
 * proof grammar (a focused, correct subset; extended as the kernel grows):
 *
 *   proof    ::= "refl" | "refl" "(" expr ")"
 *   prop     ::= expr "=" expr          (i64 definitional equality)
 *   expr     ::= the i64 expression grammar (expr.c: lit/binary/let/if/lam/app)
 *
 * `refl` proves `l = r` exactly when the kernel's conversion check accepts
 * l ≡ r up to β+δ — the same judgement Lean uses for `rfl`.
 */
#include "theorem.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "expr.h"
#include "qtt.h"

static const char *skip_ws(const char *p) {
    while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r') {
        p++;
    }
    return p;
}

/* find a single '=' (not '==') at paren depth 0 in [s, end) */
static const char *find_eq(const char *s, const char *end) {
    int depth = 0;
    for (const char *q = s; q < end; q++) {
        if (*q == '(') {
            depth++;
        } else if (*q == ')') {
            depth--;
        } else if (*q == '=' && depth == 0 && (q + 1 >= end || q[1] != '=')) {
            return q;
        }
    }
    return NULL;
}

/* copy [s, e) into `buf` and trim surrounding whitespace. Returns the trimmed
 * length, or -1 if it does not fit. */
static int copy_trim(const char *s, const char *e, char *buf, size_t cap) {
    while (s < e && (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r')) s++;
    while (e > s && (e[-1] == ' ' || e[-1] == '\t' || e[-1] == '\n' || e[-1] == '\r')) e--;
    size_t n = (size_t)(e - s);
    if (n >= cap) return -1;
    memcpy(buf, s, n);
    buf[n] = '\0';
    return (int)n;
}

int theorem_prove(const char *decl, char *out, size_t cap, char *err,
                  size_t cap_err) {
    const char *p = skip_ws(decl);
    if (strncmp(p, "theorem", 7) == 0 && (p[7] == ' ' || p[7] == '\t')) {
        p = skip_ws(p + 7);
    }
    const char *colon = strchr(p, ':');
    if (!colon) {
        snprintf(err, cap_err, "theorem: expected ':'");
        return -1;
    }
    const char *eq = strstr(colon, ":=");
    if (!eq) {
        snprintf(err, cap_err, "theorem: expected ':='");
        return -1;
    }
    const char *prop = colon + 1;
    const char *prop_end = eq;
    const char *proof = skip_ws(eq + 2);

    const char *eqpos = find_eq(prop, prop_end);
    if (!eqpos) {
        snprintf(err, cap_err, "theorem: proposition must be 'a = b'");
        return -1;
    }

    char lbuf[256], rbuf[256];
    if (copy_trim(prop, eqpos, lbuf, sizeof lbuf) < 0 ||
        copy_trim(eqpos + 1, prop_end, rbuf, sizeof rbuf) < 0) {
        snprintf(err, cap_err, "theorem: side expression too long");
        return -1;
    }
    if (lbuf[0] == '\0' || rbuf[0] == '\0') {
        snprintf(err, cap_err, "theorem: empty side of equality");
        return -1;
    }

    /* proof must be `refl` (or `refl(...)` — the wrapped form is accepted and
     * checked the same way, since refl l proves l = r iff l ≡ r). */
    char pbuf[256];
    size_t plen = strlen(proof);
    while (plen > 0 && (proof[plen - 1] == ' ' || proof[plen - 1] == '\n' ||
                        proof[plen - 1] == '\t' || proof[plen - 1] == '\r')) {
        plen--;
    }
    if (plen >= sizeof pbuf) {
        snprintf(err, cap_err, "theorem: proof too long");
        return -1;
    }
    memcpy(pbuf, proof, plen);
    pbuf[plen] = '\0';
    if (strcmp(pbuf, "refl") != 0 &&
        strncmp(pbuf, "induction", 9) != 0) {
        snprintf(err, cap_err, "theorem: only 'refl' or 'induction' proofs are supported");
        return -1;
    }

    /* parse both sides into the SAME term pool — do NOT reset between them,
     * or the second parse clobbers the first term. */
    expr_pool_reset();
    Term *l = NULL, *r = NULL;
    if (expr_parse(lbuf, &l, err, cap_err) != 0) {
        return -1;
    }
    if (expr_parse(rbuf, &r, err, cap_err) != 0) {
        return -1;
    }

    if (strcmp(pbuf, "refl") == 0) {
        return qtt_prove_refl(l, r, out, cap, err, cap_err);
    }
    if (strncmp(pbuf, "induction", 9) == 0) {
        return qtt_prove_induction(pbuf + 9, l, r, out, cap, err, cap_err);
    }
    snprintf(err, cap_err, "theorem: unsupported proof");
    return -1;
}
