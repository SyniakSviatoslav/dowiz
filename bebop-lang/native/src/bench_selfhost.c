/* bench_selfhost.c — B2-7: honest self-host compiler throughput measurement.
 *
 * Loads a .bp compiler module (default selfhost/full_compiler.bp) with the same
 * loader path as cmd_run/cmd_selfcompile, then times each phase with
 * CLOCK_MONOTONIC, median over R runs (wcet.c pattern). Results are consumed
 * into a checksum so no phase can be dead-code-eliminated.
 *
 * Phases:
 *   (a) typecheck suite   — typecheck.bp self_check (loaded in a second pass)
 *   (b) self_check        — compiler module's own suite (compile+exec goldens)
 *   (c) compile_program   — full lex->items->per-item compile of ~1KB source
 *   (d) words/sec         — source bytes / median compile_program time
 *
 * innovate: .bp has no monotonic-clock primitive (only write/exit syscalls are
 * wired), so bench_compile.bp reports WORK counters, not seconds; wall time is
 * measured here in C. Upgrade trigger: a clock builtin in the bootstrap.
 * Zero dependencies beyond libc. */
#define _POSIX_C_SOURCE 200809L
#include "bench_selfhost.h"
#include "parser.h"
#include "expr.h"
#include "qtt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define BH_MAX_FNS 256
#define BH_RUNS 10
#define BH_ERR_CAP 256

typedef struct {
    const char *names[BH_MAX_FNS];
    Term *terms[BH_MAX_FNS];
    int count;
} BhModule;

static char *bh_slurp(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "bench: cannot open %s\n", path);
        exit(1);
    }
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *src = malloc((size_t)sz + 1);
    if (!src) {
        fclose(f);
        exit(1);
    }
    size_t rd = fread(src, 1, (size_t)sz, f);
    src[rd] = '\0';
    fclose(f);
    return src;
}

static void bh_load(const char *path, BhModule *m) {
    char *src = bh_slurp(path);

    AstProgram prog;
    BpParseError perr;
    if (bp_parse(src, &prog, &perr) != 0) {
        fprintf(stderr, "bench: parse error at %u: %s\n", perr.line, perr.msg);
        exit(1);
    }
    TyRegistry reg;
    typereg_init(&reg);
    expr_set_registry(&reg);
    char err[BH_ERR_CAP];
    expr_pool_reset();

    for (size_t i = 0; i < prog.len; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_STRUCT && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            if (bp_parse_struct_decl(txt, &reg, err, sizeof err) != 0) {
                fprintf(stderr, "bench: struct parse error: %s\n", err);
                exit(1);
            }
            free(txt);
        }
        if (it->kind == AST_ITEM_ENUM && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            if (bp_parse_enum_decl(txt, &reg, err, sizeof err) != 0) {
                fprintf(stderr, "bench: enum parse error: %s\n", err);
                exit(1);
            }
            free(txt);
        }
    }
    m->count = 0;
    for (size_t i = 0; i < prog.len && m->count < BH_MAX_FNS; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_FN && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            Term *fn_term = NULL;
            Ty *fn_ty = NULL;
            if (bp_parse_fn_decl(txt, &reg, &fn_term, &fn_ty, err, sizeof err) != 0) {
                fprintf(stderr, "bench: fn parse error [%s]: %s\n",
                        it->name ? it->name : "?", err);
                exit(1);
            }
            free(txt);
            static char fnbuf[BH_MAX_FNS][64];
            size_t fl = it->name_len < 63 ? it->name_len : 63;
            memcpy(fnbuf[m->count], it->name ? it->name : "?", fl);
            fnbuf[m->count][fl] = '\0';
            m->names[m->count] = fnbuf[m->count];
            m->terms[m->count] = fn_term;
            m->count++;
        }
    }
    /* NOTE: src, prog, reg intentionally stay alive for the process lifetime;
     * term names borrow from them. */
}

static int bh_find(const BhModule *m, const char *name) {
    for (int i = 0; i < m->count; i++) {
        if (strcmp(m->names[i], name) == 0) return i;
    }
    return -1;
}

/* call f(str_arg); returns the i64 result or -1 on eval error */
static long bh_call_str(BhModule *m, int fi, const char *arg) {
    static Term argterm;
    memset(&argterm, 0, sizeof argterm);
    argterm.kind = TERM_STR;
    argterm.name = arg;
    static Term app;
    memset(&app, 0, sizeof app);
    app.kind = TERM_APP;
    app.a = m->terms[fi];
    app.b = &argterm;
    int vk;
    long vi = -1;
    int vb;
    double vf;
    char err[BH_ERR_CAP];
    if (qtt_eval_binds(&app, m->names, (Term *const *)m->terms, m->count,
                       &vk, &vi, &vb, &vf, err, sizeof err) != 0) {
        fprintf(stderr, "bench: eval error in %s: %s\n", m->names[fi], err);
        exit(1);
    }
    return vi;
}

static double bh_now_s(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec + (double)ts.tv_nsec * 1e-9;
}

static int bh_cmp_double(const void *a, const void *b) {
    double x = *(const double *)a;
    double y = *(const double *)b;
    return (x > y) - (x < y);
}

/* median of R timings of a str->i64 fn; each sample = one call */
static double bh_median(BhModule *m, int fi, const char *arg,
                        long *sink) {
    double t[BH_RUNS];
    for (int r = 0; r < BH_RUNS; r++) {
        double t0 = bh_now_s();
        long v = bh_call_str(m, fi, arg);
        double t1 = bh_now_s();
        if (v < 0) {
            fprintf(stderr, "bench: %s returned %ld\n", m->names[fi], v);
            exit(1);
        }
        *sink += v; /* consume: no DCE */
        t[r] = t1 - t0;
    }
    qsort(t, BH_RUNS, sizeof(double), bh_cmp_double);
    return (BH_RUNS % 2 == 1) ? t[BH_RUNS / 2]
                              : 0.5 * (t[BH_RUNS / 2 - 1] + t[BH_RUNS / 2]);
}

/* same, for zero-arg -> i64 fns */
static long bh_call_null(BhModule *m, int fi) {
    int vk;
    long vi = -1;
    int vb;
    double vf;
    char err[BH_ERR_CAP];
    if (qtt_eval_binds(m->terms[fi], m->names, (Term *const *)m->terms, m->count,
                       &vk, &vi, &vb, &vf, err, sizeof err) != 0) {
        fprintf(stderr, "bench: eval error in %s: %s\n", m->names[fi], err);
        exit(1);
    }
    return vi;
}

static double bh_median_null(BhModule *m, int fi, long *sink) {
    double t[BH_RUNS];
    for (int r = 0; r < BH_RUNS; r++) {
        double t0 = bh_now_s();
        long v = bh_call_null(m, fi);
        double t1 = bh_now_s();
        if (v < 0) {
            fprintf(stderr, "bench: %s returned %ld\n", m->names[fi], v);
            exit(1);
        }
        *sink += v;
        t[r] = t1 - t0;
    }
    qsort(t, BH_RUNS, sizeof(double), bh_cmp_double);
    return (BH_RUNS % 2 == 1) ? t[BH_RUNS / 2]
                              : 0.5 * (t[BH_RUNS / 2 - 1] + t[BH_RUNS / 2]);
}

/* deterministic ~n-byte arithmetic source ("1 + 2 * 3 - 4 + ..." pattern) */
static char *bh_gen_source(size_t n) {
    char *s = malloc(n + 1);
    size_t pos = 0;
    long k = 1;
    while (pos + 24 < n) {
        int w = snprintf(s + pos, 24, "%ld + 2 * %ld - ", k, k + 1);
        if (w <= 0) break;
        pos += (size_t)w;
        k += 2;
    }
    strcpy(s + pos, "7");
    return s;
}

int cmd_selfhost_bench(int argc, char **argv) {
    /* usage: bebopc selfhost-bench [compiler.bp] [typecheck.bp] */
    const char *compiler_path =
        (argc > 2) ? argv[2] : "selfhost/full_compiler.bp";
    const char *typecheck_path =
        (argc > 3) ? argv[3] : "selfhost/typecheck.bp";

    printf("selfhost-bench: R=%d median, CLOCK_MONOTONIC\n", BH_RUNS);

    long sink = 0;

    /* --- pass 1: the compiler module --- */
    BhModule comp;
    memset(&comp, 0, sizeof comp);
    bh_load(compiler_path, &comp);

    int sc = bh_find(&comp, "self_check");
    int cp = bh_find(&comp, "compile_program");
    if (sc < 0 || cp < 0) {
        fprintf(stderr, "bench: %s lacks self_check/compile_program\n",
                compiler_path);
        return 2;
    }

    double t_sc = bh_median(&comp, sc, "", &sink);
    printf("  %-34s %10.3f ms   (checksum sink %ld)\n",
           "self_check (full suite)", t_sc * 1e3, sink);

    char *input = bh_gen_source(1024);
    size_t in_len = strlen(input);

    double t_cp = bh_median(&comp, cp, input, &sink);
    printf("  %-34s %10.3f ms   (%zu bytes in)\n",
           "compile_program ~1KB", t_cp * 1e3, in_len);
    if (t_cp > 0) {
        printf("  %-34s %10.1f B/s\n",
               "throughput (bytes/sec)", (double)in_len / t_cp);
    }

    /* --- pass 2: typecheck module (pool reset inside next load) --- */
    BhModule tc;
    memset(&tc, 0, sizeof tc);
    bh_load(typecheck_path, &tc);
    int tcs = bh_find(&tc, "self_check");
    int is_null = 0;
    if (tcs < 0) {
        tcs = bh_find(&tc, "selftest"); /* zero-arg naming convention */
        is_null = 1;
    }
    if (tcs >= 0) {
        double t_tc = is_null ? bh_median_null(&tc, tcs, &sink)
                              : bh_median(&tc, tcs, "", &sink);
        printf("  %-34s %10.3f ms   (typecheck.bp test suite)\n",
               "typecheck suite", t_tc * 1e3);
    } else {
        printf("  %-34s %10s\n", "typecheck suite", "n/a");
    }

    printf("  sink(total consumed results) = %ld\n", sink);
    free(input);
    return 0;
}
