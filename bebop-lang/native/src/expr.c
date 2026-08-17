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

static Term *parse_lambda(P *p) {
    /* after consuming '\', parse: NAME [ : [^q] TYPE ] . body */
    int start = p->pos;
    while (is_ident_cont(p->s[p->pos])) {
        p->pos++;
    }
    static char names[256][32];
    static int ni = 0;
    int len = p->pos - start;
    if (len >= 32) {
        len = 31;
    }
    char *name = names[ni++ % 256];
    memcpy(name, p->s + start, (size_t)len);
    name[len] = '\0';
    skip_ws(p);
    Quantity q = Q_ONE;
    Ty *dom = qtt_i64();
    if (p->s[p->pos] == ':') {
        p->pos++;
        skip_ws(p);
        if (p->s[p->pos] == '^') {
            p->pos++;
            if (p->s[p->pos] == '0') {
                q = Q_ZERO;
                p->pos++;
            } else if (p->s[p->pos] == '1') {
                q = Q_ONE;
                p->pos++;
            } else if (p->s[p->pos] == 'w') {
                q = Q_MANY;
                p->pos++;
            }
            skip_ws(p);
        }
        if (match_kw(p, "i64")) {
            dom = qtt_i64();
        } else if (match_kw(p, "bool")) {
            dom = qtt_bool();
        } else {
            err(p, "expected type (i64/bool)");
            return NULL;
        }
    }
    skip_ws(p);
    if (p->s[p->pos] != '.') {
        err(p, "expected '.' in lambda");
        return NULL;
    }
    p->pos++;
    Term *body = parse_expr(p);
    if (!body) {
        return NULL;
    }
    Term *t = tnew();
    t->kind = TERM_LAM;
    t->name = name;
    t->q = q;
    t->ty = dom;
    t->a = body;
    return t;
}

static Term *parse_primary(P *p) {
    skip_ws(p);
    Term *atom = NULL;
    if (p->s[p->pos] == '(') {
        p->pos++;
        atom = parse_expr(p);
        skip_ws(p);
        if (p->s[p->pos] != ')') {
            err(p, "expected ')'");
            return NULL;
        }
        p->pos++;
    } else if (p->s[p->pos] == '\\') {
        p->pos++;
        atom = parse_lambda(p);
    } else if (isdigit((unsigned char)p->s[p->pos])) {
        long v = 0;
        while (isdigit((unsigned char)p->s[p->pos])) {
            v = v * 10 + (p->s[p->pos] - '0');
            p->pos++;
        }
        Term *t = tnew();
        t->kind = TERM_LIT;
        t->ival = v;
        atom = t;
    } else if (match_kw(p, "true")) {
        Term *t = tnew();
        t->kind = TERM_LIT;
        t->bval = 1;
        atom = t;
    } else if (match_kw(p, "false")) {
        Term *t = tnew();
        t->kind = TERM_LIT;
        t->bval = 0;
        atom = t;
    } else if (match_kw(p, "while")) {
        /* while cond { body } */
        Term *cond = parse_expr(p);
        if (!cond) return NULL;
        skip_ws(p);
        if (p->s[p->pos] != '{') { err(p, "expected '{'"); return NULL; }
        p->pos++;
        Term *body = parse_expr(p);
        if (!body) return NULL;
        skip_ws(p);
        if (p->s[p->pos] != '}') { err(p, "expected '}'"); return NULL; }
        p->pos++;
        Term *w = tnew();
        w->kind = TERM_WHILE;
        w->a = cond;
        w->b = body;
        atom = w;
    } else if (p->s[p->pos] == '[') {
        /* array literal [e1, e2, ...] */
        p->pos++;
        static TermField af[64];
        int na = 0;
        skip_ws(p);
        if (p->s[p->pos] != ']') {
            for (;;) {
                Term *el = parse_expr(p);
                if (!el) return NULL;
                af[na].name = "";
                af[na].val = el;
                na++;
                skip_ws(p);
                if (p->s[p->pos] == ',') { p->pos++; skip_ws(p); continue; }
                if (p->s[p->pos] == ']') break;
                err(p, "expected ',' or ']'"); return NULL;
            }
        }
        p->pos++;
        Term *arr = tnew();
        arr->kind = TERM_ARRAY;
        arr->fields = af;
        arr->nfields = na;
        atom = arr;
    } else if (p->s[p->pos] == '"') {
        int start_pos = ++p->pos;
        while (p->s[p->pos] && p->s[p->pos] != '"') p->pos++;
        int slen = p->pos - start_pos;
        static char sbuf[256];
        memcpy(sbuf, p->s + start_pos, (size_t)slen);
        sbuf[slen] = '\0';
        if (p->s[p->pos] == '"') p->pos++;
        Term *st = tnew();
        st->kind = TERM_STR;
        st->name = sbuf;
        atom = st;
    } else if (is_ident_start(p->s[p->pos])) {
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
        atom = t;
        if (strcmp(buf, "write") == 0 || strcmp(buf, "exit") == 0) {
            t->kind = TERM_SYSCALL;
            t->ival = strcmp(buf, "write") == 0 ? 64 : 93;
        }
    } else {
        err(p, "unexpected token");
        return NULL;
    }
    if (!atom) {
        return NULL;
    }
    /* postfix: application atom(arg) and indexing atom[i] */
    while (p->s[p->pos] == '(' || p->s[p->pos] == '[') {
        if (atom->kind == TERM_SYSCALL && p->s[p->pos] == '(') {
            p->pos++;
            Term *sa = parse_expr(p);
            if (!sa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = sa;
            break;
        }
        if (p->s[p->pos] == '[') {
            p->pos++;
            Term *idx = parse_expr(p);
            if (!idx) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ']') { err(p, "expected ']'"); return NULL; }
            p->pos++;
            Term *get = tnew();
            get->kind = TERM_ARRAY_GET;
            get->a = atom;
            get->b = idx;
            atom = get;
            continue;
        }
        p->pos++;
        Term *arg = parse_expr(p);
        skip_ws(p);
        if (p->s[p->pos] != ')') {
            err(p, "expected ')'");
            return NULL;
        }
        p->pos++;
        Term *app = tnew();
        app->kind = TERM_APP;
        app->a = atom;
        app->b = arg;
        atom = app;
    }
    return atom;
}

/* detect a binary operator at p; returns precedence, or -1 if none. */
static int binop(P *p, BinOp *op, int *adv) {
    char c = p->s[p->pos];
    char d = p->s[p->pos + 1];
    *adv = 1;
    if (c == '=' && d == '=') { *op = BOP_EQ; *adv = 2; return 1; }
    if (c == '!' && d == '=') { *op = BOP_NE; *adv = 2; return 1; }
    if (c == '>' && d == '=') { *op = BOP_GE; *adv = 2; return 1; }
    if (c == '<' && d == '=') { *op = BOP_LE; *adv = 2; return 1; }
    if (c == '>') { *op = BOP_GT; return 1; }
    if (c == '<') { *op = BOP_LT; return 1; }
    if (c == '+') { *op = BOP_ADD; return 2; }
    if (c == '-') { *op = BOP_SUB; return 2; }
    if (c == '*') { *op = BOP_MUL; return 3; }
    return -1;
}

static Term *parse_bin(P *p, int min_prec) {
    Term *lhs = parse_primary(p);
    if (!lhs) {
        return NULL;
    }
    for (;;) {
        skip_ws(p);
        BinOp op;
        int adv;
        int pr = binop(p, &op, &adv);
        if (pr < 0 || pr < min_prec) {
            break;
        }
        p->pos += adv;
        Term *rhs = parse_bin(p, pr + 1);
        if (!rhs) {
            return NULL;
        }
        Term *b = tnew();
        b->kind = TERM_BIN;
        b->op = op;
        b->a = lhs;
        b->b = rhs;
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

void expr_pool_reset(void) {
    pi = 0;
}

int expr_parse(const char *s, Term **term, char *errbuf, size_t cap) {
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
