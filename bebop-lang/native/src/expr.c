/* Bebop expression parser — implementation. */
#include "expr.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static Term pool[256];
static int pi = 0;

static Term *tnew(void) {
    memset(&pool[pi], 0, sizeof(Term));
    return &pool[pi++];
}

typedef struct {
    const char *s;
    int pos;
    char *err;
    size_t cap;
} P;

static void skip_ws(P *p) {
    while (p->s[p->pos] == ' ' || p->s[p->pos] == '\t' || p->s[p->pos] == '\n') {
        p->pos++;
    }
}

static int err(P *p, const char *msg) {
    snprintf(p->err, p->cap, "%s (at %d)", msg, p->pos);
    return -1;
}

static int is_ident_start(char c) {
    return isalpha((unsigned char)c) || c == '_';
}
static int is_ident_cont(char c) {
    return isalnum((unsigned char)c) || c == '_';
}

static int match_kw(P *p, const char *kw) {
    size_t n = strlen(kw);
    if (strncmp(p->s + p->pos, kw, n) == 0 &&
        !is_ident_cont(p->s[p->pos + n])) {
        p->pos += (int)n;
        skip_ws(p);
        return 1;
    }
    return 0;
}

static Term *parse_expr(P *p);

static Term *parse_primary(P *p) {
    skip_ws(p);
    if (p->s[p->pos] == '(') {
        p->pos++;
        Term *e = parse_expr(p);
        skip_ws(p);
        if (p->s[p->pos] != ')') {
            err(p, "expected ')'");
            return NULL;
        }
        p->pos++;
        return e;
    }
    if (isdigit((unsigned char)p->s[p->pos])) {
        long v = 0;
        while (isdigit((unsigned char)p->s[p->pos])) {
            v = v * 10 + (p->s[p->pos] - '0');
            p->pos++;
        }
        Term *t = tnew();
        t->kind = TERM_LIT;
        t->ival = v;
        return t;
    }
    if (match_kw(p, "true")) {
        Term *t = tnew();
        t->kind = TERM_LIT;
        t->bval = 1;
        return t;
    }
    if (match_kw(p, "false")) {
        Term *t = tnew();
        t->kind = TERM_LIT;
        t->bval = 0;
        return t;
    }
    if (is_ident_start(p->s[p->pos])) {
        int start = p->pos;
        while (is_ident_cont(p->s[p->pos])) {
            p->pos++;
        }
        /* copy the name into a static buffer (identifiers live for pool lifetime) */
        static char names[256][32];
        static int ni = 0;
        int len = p->pos - start;
        if (len >= 32) len = 31;
        char *buf = names[ni++ % 256];
        memcpy(buf, p->s + start, (size_t)len);
        buf[len] = '\0';
        Term *t = tnew();
        t->kind = TERM_VAR;
        t->name = buf;
        return t;
    }
    err(p, "unexpected token");
    return NULL;
}

/* precedence: lowest first */
static int prec(char c) {
    switch (c) {
        case '=': case '!': case '<': case '>': return 1;
        case '+': case '-': return 2;
        case '*': case '/': case '%': return 3;
    }
    return -1;
}

static Term *parse_bin(P *p, int min_prec) {
    Term *lhs = parse_primary(p);
    if (!lhs) return NULL;
    for (;;) {
        skip_ws(p);
        char c = p->s[p->pos];
        int pr = prec(c);
        if (pr < min_prec || pr < 0) break;
        /* handle == and != (two-char) */
        BinOp op;
        int adv = 1;
        if (c == '=') { op = BOP_EQ; p->pos++; if (p->s[p->pos] == '=') p->pos++; }
        else if (c == '!') { op = BOP_EQ; p->pos++; if (p->s[p->pos] == '=') { op = BOP_EQ; p->pos++; } else { err(p, "expected '='"); return NULL; } }
        else if (c == '<') { op = BOP_LT; p->pos++; }
        else if (c == '>') { op = BOP_LT; p->pos++; } /* > treated as LT with swapped operands */
        else if (c == '+') { op = BOP_ADD; p->pos++; }
        else if (c == '-') { op = BOP_SUB; p->pos++; }
        else if (c == '*') { op = BOP_MUL; p->pos++; }
        else { break; }
        (void)adv;
        Term *rhs = parse_bin(p, pr + 1);
        if (!rhs) return NULL;
        Term *b = tnew();
        b->kind = TERM_BIN;
        b->op = op;
        if (op == BOP_LT && c == '>') {
            /* a > b == b < a */
            b->a = rhs;
            b->b = lhs;
        } else {
            b->a = lhs;
            b->b = rhs;
        }
        lhs = b;
    }
    return lhs;
}

static Term *parse_expr(P *p) {
    skip_ws(p);
    if (match_kw(p, "let")) {
        /* let NAME = expr in expr */
        int start = p->pos;
        while (is_ident_cont(p->s[p->pos])) p->pos++;
        static char names[256][32];
        static int ni = 0;
        int len = p->pos - start;
        if (len >= 32) len = 31;
        char *name = names[ni++ % 256];
        memcpy(name, p->s + start, (size_t)len);
        name[len] = '\0';
        skip_ws(p);
        if (p->s[p->pos] != '=') { err(p, "expected '=' in let"); return NULL; }
        p->pos++;
        Term *val = parse_expr(p);
        if (!val) return NULL;
        skip_ws(p);
        if (!match_kw(p, "in")) { err(p, "expected 'in'"); return NULL; }
        Term *body = parse_expr(p);
        if (!body) return NULL;
        Term *t = tnew();
        t->kind = TERM_LET;
        t->name = name;
        t->a = val;
        t->b = body;
        return t;
    }
    if (match_kw(p, "if")) {
        Term *cond = parse_expr(p);
        skip_ws(p);
        if (!match_kw(p, "then")) { err(p, "expected 'then'"); return NULL; }
        Term *th = parse_expr(p);
        skip_ws(p);
        if (!match_kw(p, "else")) { err(p, "expected 'else'"); return NULL; }
        Term *el = parse_expr(p);
        Term *t = tnew();
        t->kind = TERM_IF;
        t->a = cond;
        t->b = th;
        t->c = el;
        return t;
    }
    return parse_bin(p, 0);
}

int expr_parse(const char *s, Term **term, char *errbuf, size_t cap) {
    pi = 0;
    P p = {s, 0, errbuf, cap};
    Term *t = parse_expr(&p);
    if (!t) {
        return -1;
    }
    skip_ws(&p);
    if (p.s[p.pos] != '\0') {
        return err(&p, "trailing input");
    }
    *term = t;
    return 0;
}
