/* Bebop expression parser — implementation. */
#include "expr.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TERM_POOL 65536
static Term pool[TERM_POOL];
static int pi = 0;

/* String-literal arena: a flat bump allocator so string literals get unique,
 * stable storage for the whole parse (the old fixed 64×256 ring overflowed at
 * 64 literals and corrupted earlier strings). Sized for the self-host compiler
 * (whose own source, ~48 KB, is one large literal during self-compilation). */
#define STR_ARENA (1 << 20)
static char str_arena[STR_ARENA];
static size_t str_pos = 0;

/* Array-literal field arena: flat bump allocator for TERM_ARRAY's TermField
 * arrays (the old 128×512 ring capped literals at 512 elements and overflowed
 * at 128 literals). Sized for the self-host compiler's large insns buffers. */
#define AF_ARENA (1 << 20)
static TermField af_arena[AF_ARENA];
static size_t af_pos = 0;

/* Registry (set by cmd_check/cmd_run) for struct construction + field access. */
static TyRegistry *g_reg = NULL;
void expr_set_registry(TyRegistry *reg) { g_reg = reg; }

/* find the TY_ENUM that has a constructor `name`; returns NULL if none */
static Ty *enum_ctor_lookup(const char *name) {
    if (!g_reg) return NULL;
    for (int i = 0; i < g_reg->len; i++) {
        Ty *ty = g_reg->entries[i].ty;
        if (ty && ty->kind == TY_ENUM) {
            for (int j = 0; j < ty->nctors; j++) {
                if (strcmp(ty->ctors[j].name, name) == 0) return ty;
            }
        }
    }
    return NULL;
}

static Term *tnew(void) {
    if (pi >= TERM_POOL) {
        return NULL; /* pool exhausted (bounded parse) */
    }
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
    for (;;) {
        while (p->s[p->pos] == ' ' || p->s[p->pos] == '\t' || p->s[p->pos] == '\n') {
            p->pos++;
        }
        /* Strip // line comments too: fn-body text is sliced from token spans
         * and can include comment bytes between statements, which the raw
         * expression parser otherwise misreads as '/' '/' operators. */
        if (p->s[p->pos] == '/' && p->s[p->pos + 1] == '/') {
            while (p->s[p->pos] && p->s[p->pos] != '\n') p->pos++;
        } else {
            break;
        }
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
static Term *parse_seq(P *p);

static Term *parse_lambda(P *p) {
    /* after consuming '\', parse: NAME [ : [^q] TYPE ] . body */
    int start = p->pos;
    while (is_ident_cont(p->s[p->pos])) {
        p->pos++;
    }
    static char names[32768][32];
    static int ni = 0;
    int len = p->pos - start;
    if (len >= 32) {
        len = 31;
    }
    char *name = names[ni++ % 32768];
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
    if (!t) { err(p, "term pool exhausted"); return NULL; }
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
    if (p->s[p->pos] == '!' && p->s[p->pos + 1] != '=') {
        /* unary not: !e  desugars to  (e == 0) */
        p->pos++;
        Term *inner = parse_primary(p);
        if (!inner) return NULL;
        Term *zero = tnew();
        if (!zero) { err(p, "term pool exhausted"); return NULL; }
        zero->kind = TERM_LIT;
        zero->ival = 0;
        Term *t = tnew();
        if (!t) { err(p, "term pool exhausted"); return NULL; }
        t->kind = TERM_BIN;
        t->op = BOP_EQ;
        t->a = inner;
        t->b = zero;
        atom = t;
    } else if (p->s[p->pos] == '(') {
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
        /* f64 literal if '.' followed by a digit */
        if (p->s[p->pos] == '.' && isdigit((unsigned char)p->s[p->pos + 1])) {
            p->pos++; /* consume '.' */
            double fv = (double)v, scale = 0.1;
            while (isdigit((unsigned char)p->s[p->pos])) {
                fv += (p->s[p->pos] - '0') * scale;
                scale *= 0.1;
                p->pos++;
            }
            Term *t = tnew();
            if (!t) { err(p, "term pool exhausted"); return NULL; }
            t->kind = TERM_FLIT;
            t->fval = fv;
            atom = t;
        } else {
            Term *t = tnew();
            if (!t) { err(p, "term pool exhausted"); return NULL; }
            t->kind = TERM_LIT;
            t->ival = v;
            atom = t;
        }
    } else if (match_kw(p, "true")) {
        Term *t = tnew();
        if (!t) { err(p, "term pool exhausted"); return NULL; }
        t->kind = TERM_LIT;
        t->bval = 1;
        atom = t;
    } else if (match_kw(p, "false")) {
        Term *t = tnew();
        if (!t) { err(p, "term pool exhausted"); return NULL; }
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
        Term *body = parse_seq(p);
        if (!body) return NULL;
        skip_ws(p);
        if (p->s[p->pos] != '}') { err(p, "expected '}'"); return NULL; }
        p->pos++;
        Term *w = tnew();
        if (!w) { err(p, "term pool exhausted"); return NULL; }
        w->kind = TERM_WHILE;
        w->a = cond;
        w->b = body;
        atom = w;
    } else if (p->s[p->pos] == '[') {
        /* array literal [e1, e2, ...] */
        p->pos++;
        size_t af_start = af_pos;
        int na = 0;
        skip_ws(p);
        if (p->s[p->pos] != ']') {
            for (;;) {
                Term *el = parse_expr(p);
                if (!el) return NULL;
                if (af_start + (size_t)na >= AF_ARENA) {
                    err(p, "array literal too large"); return NULL;
                }
                af_arena[af_start + na].name = "";
                af_arena[af_start + na].val = el;
                na++;
                skip_ws(p);
                if (p->s[p->pos] == ',') { p->pos++; skip_ws(p); continue; }
                if (p->s[p->pos] == ']') break;
                err(p, "expected ',' or ']'"); return NULL;
            }
        }
        p->pos++;
        af_pos = af_start + (size_t)na;
        Term *arr = tnew();
        if (!arr) { err(p, "term pool exhausted"); return NULL; }
        arr->kind = TERM_ARRAY;
        arr->fields = &af_arena[af_start];
        arr->nfields = na;
        atom = arr;
    } else if (p->s[p->pos] == '\"') {
        int start_pos = ++p->pos;
        while (p->s[p->pos] && p->s[p->pos] != '"') {
            if (p->s[p->pos] == '\\' && p->s[p->pos + 1]) {
                p->pos += 2; /* escaped char: never terminates the literal */
            } else {
                p->pos++;
            }
        }
        int slen = p->pos - start_pos;
        /* Reject before writing: escaped output is at most `slen` bytes (escapes
         * only shrink), plus one NUL. Pre-check avoids overflowing the arena. */
        if (str_pos + (size_t)slen + 1 > STR_ARENA) {
            err(p, "string literal too long");
            return NULL;
        }
        char *dst = str_arena + str_pos;
        /* process escape sequences: \n \t \\ \" */
        int di = 0;
        for (int si = 0; si < slen && di < 65535; si++) {
            char ch = p->s[start_pos + si];
            if (ch == '\\' && si + 1 < slen) {
                char nx = p->s[start_pos + si + 1];
                if (nx == 'n') { dst[di++] = '\n'; si++; }
                else if (nx == 't') { dst[di++] = '\t'; si++; }
                else if (nx == '\\') { dst[di++] = '\\'; si++; }
                else if (nx == '\"') { dst[di++] = '\"'; si++; }
                else { dst[di++] = ch; }
            } else {
                dst[di++] = ch;
            }
        }
        dst[di] = '\0';
        str_pos += (size_t)di + 1;
        if (p->s[p->pos] == '\"') p->pos++;
        Term *st = tnew();
        if (!st) { err(p, "term pool exhausted"); return NULL; }
        st->kind = TERM_STR;
        st->name = dst;
        atom = st;
    } else if (is_ident_start(p->s[p->pos])) {
        int start = p->pos;
        while (is_ident_cont(p->s[p->pos])) {
            p->pos++;
        }
        /* copy the name into a static buffer (identifiers live for pool lifetime) */
        static char names[32768][32];
        static int ni = 0;
        int len = p->pos - start;
        if (len >= 32) len = 31;
        char *buf = names[ni++ % 32768];
        memcpy(buf, p->s + start, (size_t)len);
        buf[len] = '\0';
        Term *t = tnew();
        if (!t) { err(p, "term pool exhausted"); return NULL; }
        t->kind = TERM_VAR;
        t->name = buf;
        atom = t;
        if (strcmp(buf, "write") == 0 || strcmp(buf, "exit") == 0 ||
            strcmp(buf, "power") == 0) {
            t->kind = TERM_SYSCALL;
            t->ival = strcmp(buf, "write") == 0 ? 64
                    : strcmp(buf, "exit") == 0 ? 93 : 300;
        } else if (strcmp(buf, "char") == 0) {
            t->kind = TERM_STR_CHAR; /* placeholder: two args parsed in postfix */
        } else if (strcmp(buf, "chr") == 0) {
            t->kind = TERM_CHR; /* placeholder: one arg parsed in postfix */
        } else if (strcmp(buf, "str_len") == 0) {
            t->kind = TERM_STR_LEN; /* placeholder: one arg parsed in postfix */
        } else if (strcmp(buf, "zeros") == 0) {
            t->kind = TERM_ZEROS; /* placeholder: one arg parsed in postfix: zeros(n) -> fresh [n]i64 */
        } else if (strcmp(buf, "hvham") == 0) {
            t->kind = TERM_HVHAM; /* three args parsed in postfix: hvham(a,b,n) -> i64 popcount(a^b) over floor(n/8)*8 words */
        } else if (strcmp(buf, "hvham2") == 0) {
            t->kind = TERM_HVHAM2; /* five args: hvham2(a,ao,b,bo,n) -> popcount over a[ao+k]^b[bo+k], floor(n/8)*8 */
        } else if (strcmp(buf, "sys_open") == 0) {
            t->kind = TERM_SYSOPEN; /* (p,n,flags): byte-per-element path buffer */
        } else if (strcmp(buf, "sys_read") == 0) {
            t->kind = TERM_SYSREAD; /* (fd,buf,len) */
        } else if (strcmp(buf, "sys_write") == 0) {
            t->kind = TERM_SYSWRITE; /* (fd,buf,len) */
        } else if (strcmp(buf, "sys_close") == 0) {
            t->kind = TERM_SYSCLOSE; /* (fd) */
        } else if (strcmp(buf, "sys_exit") == 0) {
            t->kind = TERM_SYSEXIT; /* (code) noreturn */
        } else if (strcmp(buf, "clock_ms") == 0) {
            t->kind = TERM_CLOCKMS; /* () monotonic ms */
        } else if (strcmp(buf, "sys_readbuf") == 0) {
            t->kind = TERM_SYSREADBUF; /* (fd,len): read into scratch, return its address */
        } else if (strcmp(buf, "sys_slurp") == 0) {
            t->kind = TERM_SYSREADBUF; /* same shape, arena buffer */
        } else if (strcmp(buf, "sys_ftruncate") == 0) {
            t->kind = TERM_SYSFTRUNCATE; /* (fd,len): file grows to len */
        } else if (strcmp(buf, "sys_munmap") == 0) {
            t->kind = TERM_SYSMUNMAP; /* (addr,len): unmap */
        } else if (strcmp(buf, "sys_mmap") == 0) {
            t->kind = TERM_SYSMMAP; /* (addr,len,prot,flags,fd,off) 6 args in postfix */
        } else if (strcmp(buf, "sys_rename") == 0) {
            t->kind = TERM_SYSRENAME; /* (old,new): renameat(AT_FDCWD,...) */
        } else if (strcmp(buf, "sys_clone") == 0) {
            t->kind = TERM_SYSCLONE; /* (flags,stack_top) */
        } else if (strcmp(buf, "sys_cond_set") == 0) {
            t->kind = TERM_SYSCONDSET; /* (cond,arr,idx,val) */
        } else if (strcmp(buf, "sys_exit_thread_guard") == 0) {
            t->kind = TERM_SYSEXITTHREAD; /* (cond,code) guarded thread exit */
        } else if (strcmp(buf, "sys_futex_wait_guard") == 0) {
            t->kind = TERM_SYSFUTEXWAIT; /* (cond,arr,idx,val) guarded WAIT */
        } else if (strcmp(buf, "sys_futex_wake") == 0) {
            t->kind = TERM_SYSFUTEXWAKE; /* (arr,idx,n) */
        } else if (strcmp(buf, "sys_atomic_add") == 0) {
            t->kind = TERM_SYSATOMICADD; /* (arr,idx,val) */
        } else if (strcmp(buf, "sys_arena_base") == 0) {
            t->kind = TERM_SYSARENABASE; /* () */
        } else if (strcmp(buf, "sys_arena_end") == 0) {
            t->kind = TERM_SYSARENAEND; /* () */
        } else if (strcmp(buf, "exec") == 0) {
            t->kind = TERM_EXEC; /* placeholder: two args parsed in postfix */
        } else if (g_reg && enum_ctor_lookup(buf)) {
            t->kind = TERM_ENUM_CTOR;
            t->ty = enum_ctor_lookup(buf);
        }
        /* struct construction: Name{ f: e, ... } */
        if (g_reg) {
            int save = p->pos;
            skip_ws(p);
            if (p->s[p->pos] == '{') {
                Ty *sty = typereg_get(g_reg, buf);
                if (sty && sty->kind == TY_STRUCT) {
                    p->pos++; /* skip '{' */
                    static TermField sf[32];
                    static char sfname[32][64];
                    int nf = 0;
                    for (;;) {
                        skip_ws(p);
                        if (p->s[p->pos] == '}') { p->pos++; break; }
                        if (!is_ident_start(p->s[p->pos])) { err(p, "expected field name"); return NULL; }
                        int fs = p->pos;
                        while (is_ident_cont(p->s[p->pos])) p->pos++;
                        int fl = p->pos - fs; if (fl >= 64) fl = 63;
                        memcpy(sfname[nf], p->s + fs, (size_t)fl);
                        sfname[nf][fl] = '\0';
                        skip_ws(p);
                        if (p->s[p->pos] != ':') { err(p, "expected ':' in struct"); return NULL; }
                        p->pos++;
                        Term *fv = parse_expr(p);
                        if (!fv) return NULL;
                        sf[nf].name = sfname[nf];
                        sf[nf].val = fv;
                        nf++;
                        skip_ws(p);
                        if (p->s[p->pos] == ',') { p->pos++; continue; }
                    }
                    t->kind = TERM_STRUCT;
                    t->ty = sty;
                    t->fields = sf;
                    t->nfields = nf;
                } else {
                    p->pos = save;
                }
            } else {
                p->pos = save;
            }
        }
    } else {
        err(p, "unexpected token");
        return NULL;
    }
    if (!atom) {
        return NULL;
    }
    /* field access: atom.field */
    while (p->s[p->pos] == '.') {
        p->pos++;
        if (!is_ident_start(p->s[p->pos])) { err(p, "expected field name after '.'"); return NULL; }
        int fs = p->pos;
        while (is_ident_cont(p->s[p->pos])) p->pos++;
        static char fbuf[256][32];
        static int fi = 0;
        int fl = p->pos - fs; if (fl >= 32) fl = 31;
        char *fn = fbuf[fi++ % 256];
        memcpy(fn, p->s + fs, (size_t)fl);
        fn[fl] = '\0';
        Term *fld = tnew();
        if (!fld) { err(p, "term pool exhausted"); return NULL; }
        fld->kind = TERM_FIELD;
        fld->a = atom;
        fld->name = fn;
        atom = fld;
    }
    /* postfix: application atom(arg) and indexing atom[i] */
    while (p->s[p->pos] == '(' || p->s[p->pos] == '[') {
        if (atom->kind == TERM_ZEROS && p->s[p->pos] == '(') {
            p->pos++;
            Term *sa = parse_expr(p);
            if (!sa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = sa;
            continue;
        }
        if (atom->kind == TERM_HVHAM2 && p->s[p->pos] == '(') {
            p->pos++;
            Term *args[5];
            for (int ai = 0; ai < 5; ai++) {
                skip_ws(p);
                if (ai > 0) {
                    if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
                    p->pos++;
                    skip_ws(p);
                }
                args[ai] = parse_expr(p);
                if (!args[ai]) return NULL;
                skip_ws(p);
            }
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            /* symmetric pairs: slot a = ARRAY[a_expr, ao_expr],
             * slot b = ARRAY[b_expr, bo_expr], slot c = n_expr */
            {
                static TermField pf[8][2];
                static int pi2 = 0;
                int sa = pi2++ % 8;
                int sb = pi2++ % 8;
                Term *pa = tnew();
                Term *pb = tnew();
                if (!pa || !pb) { err(p, "term pool exhausted"); return NULL; }
                pf[sa][0].name = "arr";
                pf[sa][0].val = args[0];
                pf[sa][1].name = "off";
                pf[sa][1].val = args[1];
                pf[sb][0].name = "arr";
                pf[sb][0].val = args[2];
                pf[sb][1].name = "off";
                pf[sb][1].val = args[3];
                pa->kind = TERM_ARRAY;
                pa->fields = pf[sa];
                pa->nfields = 2;
                pb->kind = TERM_ARRAY;
                pb->fields = pf[sb];
                pb->nfields = 2;
                atom->a = pa;
                atom->b = pb;
                atom->c = args[4];
            }
            continue;
        }
        if (atom->kind == TERM_HVHAM && p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *an = parse_expr(p);
            if (!an) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            atom->c = an;
            continue;
        }
        if ((atom->kind == TERM_SYSCONDSET || atom->kind == TERM_SYSFUTEXWAIT) &&
            p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ac = parse_expr(p);
            if (!ac) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ad = parse_expr(p);
            if (!ad) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            atom->c = ac;
            atom->d = ad;
            continue;
        }
        if ((atom->kind == TERM_SYSFUTEXWAKE || atom->kind == TERM_SYSATOMICADD) &&
            p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ac = parse_expr(p);
            if (!ac) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            atom->c = ac;
            continue;
        }
        if (atom->kind == TERM_SYSCLONE && p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            continue;
        }
        if (atom->kind == TERM_SYSEXITTHREAD && p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            continue;
        }
        if ((atom->kind == TERM_SYSOPEN || atom->kind == TERM_SYSREAD ||
             atom->kind == TERM_SYSWRITE) && p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *an = parse_expr(p);
            if (!an) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            atom->c = an;
            continue;
        }
        if ((atom->kind == TERM_SYSRENAME || atom->kind == TERM_SYSFTRUNCATE ||
             atom->kind == TERM_SYSMUNMAP) && p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            continue;
        }
        if (atom->kind == TERM_SYSMMAP && p->s[p->pos] == '(') {
            /* sys_mmap(addr, len, prot, flags, fd, off) — 6 slots: a..f */
            p->pos++;
            Term *args[6]; int nargs = 0;
            for (;;) {
                Term *aa = parse_expr(p);
                if (!aa) return NULL;
                args[nargs++] = aa;
                skip_ws(p);
                if (p->s[p->pos] == ',') { p->pos++; skip_ws(p); continue; }
                if (p->s[p->pos] != ')') { err(p, "expected ')' in sys_mmap"); return NULL; }
                p->pos++;
                break;
            }
            if (nargs != 6) { err(p, "sys_mmap expects 6 args"); return NULL; }
            atom->a = args[0]; atom->b = args[1]; atom->c = args[2];
            atom->d = args[3]; atom->e = args[4]; atom->f = args[5];
            continue;
        }
        if (atom->kind == TERM_SYSREADBUF && p->s[p->pos] == '(') {
            p->pos++;
            Term *aa = parse_expr(p);
            if (!aa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ','"); return NULL; }
            p->pos++;
            Term *ab = parse_expr(p);
            if (!ab) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = aa;
            atom->b = ab;
            continue;
        }
        if ((atom->kind == TERM_SYSCLOSE || atom->kind == TERM_SYSEXIT) && p->s[p->pos] == '(') {
            p->pos++;
            Term *sa = parse_expr(p);
            if (!sa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = sa;
            continue;
        }
        if ((atom->kind == TERM_CLOCKMS || atom->kind == TERM_SYSARENABASE ||
             atom->kind == TERM_SYSARENAEND) && p->s[p->pos] == '(') {
            p->pos++;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            continue;
        }
        if (atom->kind == TERM_STR_LEN && p->s[p->pos] == '(') {
            p->pos++;
            Term *sa = parse_expr(p);
            if (!sa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = sa;
            continue;
        }
        if (atom->kind == TERM_CHR && p->s[p->pos] == '(') {
            p->pos++;
            Term *sa = parse_expr(p);
            if (!sa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = sa;
            continue;
        }
        if (atom->kind == TERM_STR_CHAR && p->s[p->pos] == '(') {
            p->pos++;
            Term *sa = parse_expr(p);
            if (!sa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ',' in char"); return NULL; }
            p->pos++;
            Term *sb = parse_expr(p);
            if (!sb) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = sa;
            atom->b = sb;
            continue;
        }
        if (atom->kind == TERM_EXEC && p->s[p->pos] == '(') {
            p->pos++;
            Term *sa = parse_expr(p);
            if (!sa) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ',' in exec"); return NULL; }
            p->pos++;
            Term *sb = parse_expr(p);
            if (!sb) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ',') { err(p, "expected ',' in exec"); return NULL; }
            p->pos++;
            Term *sc = parse_expr(p);
            if (!sc) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = sa;
            atom->b = sb;
            atom->c = sc;
            continue;
        }
        if (atom->kind == TERM_ENUM_CTOR && p->s[p->pos] == '(') {
            p->pos++;
            Term *pl = parse_expr(p);
            if (!pl) return NULL;
            skip_ws(p);
            if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
            p->pos++;
            atom->a = pl;
            continue;
        }
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
            skip_ws(p);
            if (p->s[p->pos] == '=' && p->s[p->pos+1] != '=') {
                /* array mutation: arr[i] = v */
                p->pos++;
                Term *val = parse_expr(p);
                if (!val) return NULL;
                Term *set = tnew();
                if (!set) { err(p, "term pool exhausted"); return NULL; }
                set->kind = TERM_ARRAY_SET;
                set->a = atom;
                set->b = idx;
                set->c = val;
                atom = set;
                continue;
            }
            Term *get = tnew();
            if (!get) { err(p, "term pool exhausted"); return NULL; }
            get->kind = TERM_ARRAY_GET;
            get->a = atom;
            get->b = idx;
            atom = get;
            continue;
        }
        p->pos++;
        for (;;) {
            skip_ws(p);
            /* zero-arg call: f() — wrap in APP(f, unit) so the call applies */
            if (p->s[p->pos] == ')') {
                p->pos++;
                Term *unit = tnew();
                if (!unit) { err(p, "term pool exhausted"); return NULL; }
                unit->kind = TERM_LIT; unit->ival = 0; unit->bval = 0;
                Term *app = tnew();
                if (!app) { err(p, "term pool exhausted"); return NULL; }
                app->kind = TERM_APP; app->a = atom; app->b = unit;
                atom = app;
                break;
            }
            Term *arg = parse_expr(p);
            if (!arg) return NULL;
            skip_ws(p);
            Term *app = tnew();
            if (!app) { err(p, "term pool exhausted"); return NULL; }
            app->kind = TERM_APP;
            app->a = atom;
            app->b = arg;
            atom = app;
            if (p->s[p->pos] == ',') {
                p->pos++;
                continue;
            }
            if (p->s[p->pos] != ')') {
                err(p, "expected ')'");
                return NULL;
            }
            p->pos++;
            break;
        }
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
    if (c == '<' && d == '<') { *op = BOP_SHL; *adv = 2; return 4; }
    if (c == '>' && d == '>') { *op = BOP_SHR; *adv = 2; return 4; }
    if (c == '>') { *op = BOP_GT; return 1; }
    if (c == '<') { *op = BOP_LT; return 1; }
    if (c == '&') { *op = BOP_BAND; return 5; }
    if (c == '^') { *op = BOP_BXOR; return 6; }
    if (c == '|') { *op = BOP_BOR; return 7; }
    if (c == '+' && d == '+') { *op = BOP_CAT; *adv = 2; return 2; }
    if (c == '+') { *op = BOP_ADD; return 2; }
    if (c == '-') { *op = BOP_SUB; return 2; }
    if (c == '*') { *op = BOP_MUL; return 3; }
    if (c == '/' && d != '/') { *op = BOP_DIV; return 3; }
    if (c == '%') { *op = BOP_MOD; return 3; }
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
        if (!b) { err(p, "term pool exhausted"); return NULL; }
        if (op == BOP_CAT) {
            b->kind = TERM_STR_CAT;
        } else {
            b->kind = TERM_BIN;
            b->op = op;
        }
        b->a = lhs;
        b->b = rhs;
        lhs = b;
    }
    return lhs;
}

/* Sequence: expr (';' expr)*  — recursive: LET(_sN, e1, rest) chains the
 * environment forward so every later item sees earlier bindings.
 * Compound statements IDENT (+|-|*|/|%)= expr lower to
 *   LET(IDENT, IDENT OP expr, rest)
 * whose env node persists through the rest of the sequence (mutation). */
static int seq_compound_peek(P *p, char *name, size_t cap, BinOp *op, long *after_op) {
    long save = p->pos;
    skip_ws(p);
    if (!(isalpha((unsigned char)p->s[p->pos]) || p->s[p->pos] == '_')) return 0;
    long q = p->pos;
    while (is_ident_cont(p->s[q])) q++;
    long len = q - p->pos;
    if (len <= 0 || (size_t)len >= cap) { p->pos = save; return 0; }
    long q2 = q;
    while (p->s[q2] == ' ' || p->s[q2] == '\t') q2++;
    char c0 = p->s[q2], c1 = p->s[q2 + 1];
    if (c1 != '=') { p->pos = save; return 0; }
    switch (c0) {
        case '+': *op = BOP_ADD; break;
        case '-': *op = BOP_SUB; break;
        case '*': *op = BOP_MUL; break;
        case '/': *op = BOP_DIV; break;
        case '%': *op = BOP_MOD; break;
        default: p->pos = save; return 0;
    }
    for (long i = 0; i < len; i++) name[i] = p->s[p->pos + i];
    name[len] = '\0';
    /* reject '==' style accidents: op char cannot be '=' */
    *after_op = q2 + 2;
    return 1;
}

static Term *parse_seq(P *p) {
    {
        char cname[64];
        BinOp cop;
        long after_op = 0;
        if (seq_compound_peek(p, cname, sizeof cname, &cop, &after_op)) {
            p->pos = after_op;
            Term *rhs = parse_expr(p);
            if (!rhs) return NULL;
            static char cn0[8][64];
            static int ci0 = 0;
            snprintf(cn0[ci0], sizeof cn0[0], "%s", cname);
            Term *vx = tnew();
            Term *b = tnew();
            if (!vx || !b) { err(p, "term pool exhausted"); return NULL; }
            vx->kind = TERM_VAR;
            vx->name = cn0[ci0];
            b->kind = TERM_BIN;
            b->op = cop;
            b->a = vx;
            b->b = rhs;
            skip_ws(p);
            Term *rest;
            int final1 = 0;
            if (p->s[p->pos] == ';') {
                p->pos++;
                skip_ws(p);
                if (p->s[p->pos] == '}' || p->s[p->pos] == '\0') final1 = 1;
                else rest = parse_seq(p);
            } else {
                /* no separator: end of body */
                final1 = 1;
            }
            if (final1) {
                Term *selfr = tnew();
                if (!selfr) { err(p, "term pool exhausted"); return NULL; }
                selfr->kind = TERM_VAR;
                selfr->name = cname;
                rest = selfr;
            }
            int ci = ci0;
            char (*cn)[64] = cn0;
            Term *l = tnew();
            if (!l) { err(p, "term pool exhausted"); return NULL; }
            l->kind = TERM_LET;
            l->name = cn[ci];
            l->a = b;
            l->b = rest;
            ci = (ci + 1) & 7;
            return l;
        }
    }
    Term *first = parse_expr(p);
    if (!first) return NULL;
    skip_ws(p);
    if (p->s[p->pos] == ';') {
        p->pos++;
        Term *rest = parse_seq(p);
        if (!rest) return NULL;
        static char tmp[24];
        static int ti = 0;
        Term *l = tnew();
        if (!l) { err(p, "term pool exhausted"); return NULL; }
        snprintf(tmp, sizeof tmp, "_s%d", ti++);
        l->kind = TERM_LET;
        l->name = tmp;
        l->a = first;
        l->b = rest;
        first = l;
    }
    return first;
}

static Term *parse_expr(P *p) {
    skip_ws(p);
    if (match_kw(p, "let")) {
        /* let NAME = expr in expr */
        int start = p->pos;
        while (is_ident_cont(p->s[p->pos])) p->pos++;
        static char names[32768][32];
        static int ni = 0;
        int len = p->pos - start;
        if (len >= 32) len = 31;
        char *name = names[ni++ % 32768];
        memcpy(name, p->s + start, (size_t)len);
        name[len] = '\0';
        skip_ws(p);
        if (p->s[p->pos] != '=') { err(p, "expected '=' in let"); return NULL; }
        p->pos++;
        /* chained-discard-assign: `let _ = IDENT = expr [;|in] rest` lowers to
         * LET(IDENT, expr, rest) — the '_' slot is discarded. The .bp emitter
         * (emit_let_stmt is_chain) accepts this surface for compiled programs;
         * accepting it here reconciles the C tier with the .bp tier (one
         * spec, two engines). Fall through to the normal path otherwise. */
        Term *val = NULL;
        if (name[0] == '_' && name[1] == '\0') {
            long save2 = p->pos;
            skip_ws(p);
            if ((isalpha((unsigned char)p->s[p->pos]) || p->s[p->pos] == '_')) {
                long ns = p->pos;
                while (is_ident_cont(p->s[p->pos])) p->pos++;
                long nl = p->pos - ns;
                skip_ws(p);
                if (nl > 0 && p->s[p->pos] == '=' && p->s[p->pos + 1] != '=') {
                    p->pos++;
                    val = parse_expr(p);
                    if (!val) return NULL;
                    static char iname[64];
                    if (nl >= 64) nl = 63;
                    memcpy(iname, p->s + ns, (size_t)nl);
                    iname[nl] = '\0';
                    skip_ws(p);
                    skip_ws(p);
                    Term *ibody = NULL;
                    if (match_kw(p, "in")) {
                        ibody = parse_seq(p);
                    } else if (p->s[p->pos] == ';') {
                        p->pos++;
                        ibody = parse_seq(p);
                    } else {
                        err(p, "expected 'in' or ';' after let");
                        return NULL;
                    }
                    if (!ibody) return NULL;
                    Term *it = tnew();
                    if (!it) { err(p, "term pool exhausted"); return NULL; }
                    it->kind = TERM_LET;
                    it->name = iname;
                    it->a = val;
                    it->b = ibody;
                    return it;
                }
            }
            p->pos = save2;
        }
        val = parse_expr(p);
        if (!val) return NULL;
        skip_ws(p);
        skip_ws(p);
        Term *body = NULL;
        if (match_kw(p, "in")) {
            body = parse_seq(p);
        } else if (p->s[p->pos] == ';') {
            p->pos++;
            body = parse_seq(p);
        } else {
            err(p, "expected 'in' or ';' after let");
            return NULL;
        }
        if (!body) return NULL;
        Term *t = tnew();
        if (!t) { err(p, "term pool exhausted"); return NULL; }
        t->kind = TERM_LET;
        t->name = name;
        t->a = val;
        t->b = body;
        return t;
    }
    if (match_kw(p, "match")) {
        Term *scrut = parse_expr(p);
        if (!scrut) return NULL;
        skip_ws(p);
        if (p->s[p->pos] != '{') { err(p, "expected '{' after match"); return NULL; }
        p->pos++;
        static MatchArm arms[32];
        static char acname[32][64];
        static char avname[32][64];
        int na = 0;
        for (;;) {
            skip_ws(p);
            if (p->s[p->pos] == '}') { p->pos++; break; }
            if (!is_ident_start(p->s[p->pos])) { err(p, "expected ctor in match arm"); return NULL; }
            int cs = p->pos;
            while (is_ident_cont(p->s[p->pos])) p->pos++;
            int cl = p->pos - cs; if (cl >= 64) cl = 63;
            memcpy(acname[na], p->s + cs, (size_t)cl); acname[na][cl] = '\0';
            arms[na].ctor = acname[na];
            arms[na].var = NULL;
            skip_ws(p);
            if (p->s[p->pos] == '(') {
                p->pos++;
                if (!is_ident_start(p->s[p->pos])) { err(p, "expected var name"); return NULL; }
                int vs = p->pos;
                while (is_ident_cont(p->s[p->pos])) p->pos++;
                int vl = p->pos - vs; if (vl >= 64) vl = 63;
                memcpy(avname[na], p->s + vs, (size_t)vl); avname[na][vl] = '\0';
                arms[na].var = avname[na];
                skip_ws(p);
                if (p->s[p->pos] != ')') { err(p, "expected ')'"); return NULL; }
                p->pos++;
            }
            skip_ws(p);
            if (p->s[p->pos] != '=' || p->s[p->pos + 1] != '>') { err(p, "expected '=>'"); return NULL; }
            p->pos += 2;
            Term *body = parse_expr(p);
            if (!body) return NULL;
            arms[na].body = body;
            na++;
            skip_ws(p);
            if (p->s[p->pos] == ',') { p->pos++; continue; }
        }
        Term *m = tnew();
        if (!m) { err(p, "term pool exhausted"); return NULL; }
        m->kind = TERM_MATCH;
        m->a = scrut;
        m->arms = arms;
        m->narms = na;
        return m;
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
        if (!t) { err(p, "term pool exhausted"); return NULL; }
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
    str_pos = 0;
    af_pos = 0;
}

int expr_parse(const char *s, Term **term, char *errbuf, size_t cap) {
    P p = {s, 0, errbuf, cap};
    Term *t = parse_seq(&p);
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
