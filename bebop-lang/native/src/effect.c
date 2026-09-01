/* Bebop effect registry — implementation. */
#include "effect.h"

#include <stdio.h>
#include <string.h>

#define EFF_MAX 64
static struct {
    const char *name;
    Effect e;
} effs[EFF_MAX];
static int nff = 0;

void effect_init(void) { nff = 0; }

int effect_declare(const char *name, Effect e) {
    if (nff >= EFF_MAX) return -1;
    effs[nff].name = name;
    effs[nff].e = e;
    nff++;
    return 0;
}

int effect_is_io(const char *name) {
    for (int i = 0; i < nff; i++) {
        if (strcmp(effs[i].name, name) == 0) {
            return effs[i].e == EFF_IO;
        }
    }
    return 0;
}

/* does any subterm apply a registered-io function? (transitive through the
 * term tree, mirroring qtt_term_has_io's walk) */
static int term_calls_io(const Term *t) {
    if (!t) return 0;
    if (t->kind == TERM_VAR) {
        return effect_is_io(t->name);
    }
    if (t->kind == TERM_APP) {
        /* the callee (t->a) being an io fn marks the call io */
        if (t->a && t->a->kind == TERM_VAR && effect_is_io(t->a->name)) {
            return 1;
        }
    }
    if (term_calls_io(t->a) || term_calls_io(t->b) || term_calls_io(t->c)) {
        return 1;
    }
    for (int i = 0; i < t->nfields; i++) {
        if (term_calls_io(t->fields[i].val)) return 1;
    }
    for (int i = 0; i < t->narms; i++) {
        if (term_calls_io(t->arms[i].body)) return 1;
    }
    return 0;
}

int effect_has_io(const Term *t) {
    return qtt_term_has_io(t) || term_calls_io(t);
}

int effect_parse_decl(const char *decl, char *err, size_t cap) {
    const char *p = decl;
    while (*p == ' ' || *p == '\t' || *p == '\n') p++;
    if (strncmp(p, "extern", 6) != 0) {
        snprintf(err, cap, "expected 'extern'");
        return -1;
    }
    p += 6;
    while (*p == ' ' || *p == '\t') p++;
    Effect e;
    if (strncmp(p, "pure", 4) == 0) {
        e = EFF_PURE;
        p += 4;
    } else if (strncmp(p, "io", 2) == 0) {
        e = EFF_IO;
        p += 2;
    } else {
        snprintf(err, cap, "expected 'pure' or 'io'");
        return -1;
    }
    while (*p == ' ' || *p == '\t') p++;
    if (strncmp(p, "fn", 2) != 0) {
        snprintf(err, cap, "expected 'fn'");
        return -1;
    }
    p += 2;
    while (*p == ' ' || *p == '\t') p++;
    /* name: up to '(' or whitespace */
    char name[64];
    size_t nl = 0;
    while (*p && *p != '(' && *p != ' ' && *p != '\t' && nl < 63) {
        name[nl++] = *p++;
    }
    if (nl == 0) {
        snprintf(err, cap, "expected function name");
        return -1;
    }
    name[nl] = '\0';
    /* borrow the name — copy into a static arena so it outlives the decl */
    static char names[EFF_MAX][64];
    static int ni = 0;
    strcpy(names[ni % EFF_MAX], name);
    return effect_declare(names[ni++ % EFF_MAX], e);
}

int effect_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    effect_init();
    char err[128];
    A(effect_parse_decl("extern pure fn hash(x:i64) -> i64", err, sizeof err) == 0,
      "parse extern pure");
    A(effect_parse_decl("extern io fn send(x:i64) -> i64", err, sizeof err) == 0,
      "parse extern io");
    A(!effect_is_io("hash"), "hash is pure");
    A(effect_is_io("send"), "send is io");
    A(!effect_is_io("unknown"), "unknown fn is pure-by-default");

    /* a call to the io fn is io; a call to the pure fn is not */
    static Term pool[16];
    static int pi = 0;
    pi = 0;
    Term *h = &pool[pi++]; memset(h, 0, sizeof *h); h->kind = TERM_VAR; h->name = "hash";
    Term *s = &pool[pi++]; memset(s, 0, sizeof *s); s->kind = TERM_VAR; s->name = "send";
    Term *arg = &pool[pi++]; memset(arg, 0, sizeof *arg); arg->kind = TERM_LIT; arg->ival = 1;
    Term *callh = &pool[pi++]; memset(callh, 0, sizeof *callh);
    callh->kind = TERM_APP; callh->a = h; callh->b = arg;
    Term *calls = &pool[pi++]; memset(calls, 0, sizeof *calls);
    calls->kind = TERM_APP; calls->a = s; calls->b = arg;
    A(!effect_has_io(callh), "call to pure fn is pure");
    A(effect_has_io(calls), "call to io fn is io");

    /* transitive: pure term wrapping an io call is io */
    Term *wrap = &pool[pi++]; memset(wrap, 0, sizeof *wrap);
    wrap->kind = TERM_BIN; wrap->op = BOP_ADD; wrap->a = arg; wrap->b = calls;
    A(effect_has_io(wrap), "io propagates through enclosing pure term");

    return all_ok ? 0 : -1;
}
