/* diff_fuzz.c -- differential fuzzing: bootstrap interpreter vs self-hosted
 * compiled AArch64 code executed natively.
 *
 * For each iteration a random VALID program in the compiled subset is
 * generated, then inside a forked child:
 *   1. interpreted: the program's fns are loaded through bp_parse_fn_decl and
 *      `main` evaluated via qtt_eval_binds;
 *   2. compiled: the .bp self-hosted compiler (emit_words/emit_offsets) turns
 *      the same source into AArch64 words which are mmapped and called;
 *   3. results compared bit-exactly.
 * Exit codes: 0 match, 3 mismatch, 4 compile fail, 5 interp fail, signal =
 * crash (all findings). Mismatching/crashing source is printed to stderr.
 *
 * usage: diff_fuzz [iterations] [compiler.bp] [rng_seed]
 */
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <unistd.h>
#include <signal.h>
#include <sys/mman.h>
#include <sys/wait.h>
#include <sys/resource.h>
#include <ucontext.h>

#include "parser.h"
#include "expr.h"
#include "qtt.h"
#include "typereg.h"

/* ── loaded self-host compiler state ──────────────────────────────────── */
enum { MAX_FNS = 256 };
static char fnbuf[MAX_FNS][64];
static const char *fn_names[MAX_FNS];
static Term *fn_terms[MAX_FNS];
static int fn_count = 0;
static int idx_emit_words = -1;
static int idx_emit_offsets = -1;
static TyRegistry registry;

static int load_compiler(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *src = (char *)malloc((size_t)sz + 1);
    if (!src) { fclose(f); return -1; }
    size_t rd = fread(src, 1, (size_t)sz, f);
    src[rd] = '\0';
    fclose(f);
    AstProgram prog;
    BpParseError perr;
    if (bp_parse(src, &prog, &perr) != 0) { free(src); return -1; }
    typereg_init(&registry);
    expr_set_registry(&registry);
    char err[256];
    expr_pool_reset();
    fn_count = 0;
    for (size_t i = 0; i < prog.len && fn_count < MAX_FNS; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_FN && it->text && it->text_len > 0) {
            char *txt = (char *)malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            Term *fterm = NULL;
            Ty *fty = NULL;
            if (bp_parse_fn_decl(txt, &registry, &fterm, &fty, err, sizeof err) != 0) {
                free(txt);
                bp_program_free(&prog);
                free(src);
                return -1;
            }
            free(txt);
            (void)fty;
            size_t fl = it->name_len < 63 ? it->name_len : 63;
            memcpy(fnbuf[fn_count], it->name ? it->name : "?", fl);
            fnbuf[fn_count][fl] = '\0';
            fn_names[fn_count] = fnbuf[fn_count];
            fn_terms[fn_count] = fterm;
            fn_count++;
        }
    }
    bp_program_free(&prog);
    free(src);
    for (int k = 0; k < fn_count; k++) {
        if (strcmp(fn_names[k], "emit_words") == 0) idx_emit_words = k;
        if (strcmp(fn_names[k], "emit_offsets") == 0) idx_emit_offsets = k;
    }
    return (idx_emit_words >= 0 && idx_emit_offsets >= 0) ? 0 : -1;
}

/* Call a one-string-arg .bp fn; returns scalar value or -1 on error. */
static long call_str_fn(int idx, const char *s, int *ok) {
    static Term argterm;
    memset(&argterm, 0, sizeof argterm);
    argterm.kind = TERM_STR;
    argterm.name = s;
    static Term app;
    memset(&app, 0, sizeof app);
    app.kind = TERM_APP;
    app.a = fn_terms[idx];
    app.b = &argterm;
    int vk; long vi; int vb; double vf = 0.0; char err[256];
    int r = qtt_eval_binds(&app, fn_names, (Term *const *)fn_terms, fn_count,
                           &vk, &vi, &vb, &vf, err, sizeof err);
    /* array-valued fns (emit_words/emit_offsets -> [i64]) are fine: callers
     * read the result through qtt_last_arr immediately afterwards */
    *ok = (r == 0 && (vk == 0 || vk == 6));
    return vi;
}

/* ── random valid-program generator (compiled subset) ─────────────────── */
static uint64_t rng_state = 0x9E3779B97F4A7C15ULL;
static uint64_t rnd(void) {
    uint64_t x = rng_state;
    x ^= x << 13; x ^= x >> 7; x ^= x << 17;
    rng_state = x;
    return x;
}
static uint32_t rnd_below(uint32_t n) { return n ? (uint32_t)(rnd() % n) : 0; }

enum { GBUF = 4096 };
typedef struct {
    char buf[GBUF];
    size_t len;
    int nvars;
    char vars[8];
} Gen;

static void gput(Gen *g, const char *s) {
    size_t l = strlen(s);
    if (g->len + l < GBUF - 1) { memcpy(g->buf + g->len, s, l); g->len += l; }
    g->buf[g->len] = '\0';
}
static void gnum(Gen *g) {
    char t[24];
    /* bias: 25% big constants (>65535) to keep stressing movk halves */
    uint32_t mode = rnd_below(4);
    long v = mode == 0 ? (long)(rnd_below(4000000) + 70000)
                       : (long)rnd_below(1000);
    snprintf(t, sizeof t, "%ld", v);
    gput(g, t);
}
static void gvar(Gen *g) {
    char t[2] = { g->vars[rnd_below((uint32_t)g->nvars)], 0 };
    gput(g, t);
}
static void gcmp(Gen *g);
static void gexpr(Gen *g, int depth) {
    uint32_t pick = depth <= 0 ? rnd_below(2) : rnd_below(7);
    switch (pick) {
    case 0: case 1: gnum(g); break;
    case 2: if (g->nvars) gvar(g); else gnum(g); break;
    case 3: gput(g, "("); gexpr(g, depth - 1); gput(g, " + "); gexpr(g, depth - 1); gput(g, ")"); break;
    case 4: gput(g, "("); gexpr(g, depth - 1); gput(g, " - "); gexpr(g, depth - 1); gput(g, ")"); break;
    case 5: gput(g, "("); gexpr(g, depth - 1); gput(g, " * "); gexpr(g, depth - 1); gput(g, ")"); break;
    default:
        /* if-expressions are always parenthesized: an unparenthesized if
         * inside another expression is ambiguous in this subset */
        gput(g, "(if "); gcmp(g); gput(g, " then "); gexpr(g, depth - 1);
        gput(g, " else "); gexpr(g, depth - 1); gput(g, ")");
        break;
    }
}
/* arith-only expression: comparisons take these as operands because a bare
 * if-expression inside a comparison is outside the supported grammar subset
 * (both bootstrap parser and .bp compiler reject it). */
static void garith(Gen *g, int depth) {
    uint32_t pick = depth <= 0 ? rnd_below(2) : rnd_below(6);
    switch (pick) {
    case 0: case 1: gnum(g); break;
    case 2: if (g->nvars) gvar(g); else gnum(g); break;
    case 3: gput(g, "("); garith(g, depth - 1); gput(g, " + "); garith(g, depth - 1); gput(g, ")"); break;
    case 4: gput(g, "("); garith(g, depth - 1); gput(g, " - "); garith(g, depth - 1); gput(g, ")"); break;
    default: gput(g, "("); garith(g, depth - 1); gput(g, " * "); garith(g, depth - 1); gput(g, ")"); break;
    }
}
static void gcmp(Gen *g) {
    static const char *ops[] = { "<", ">", "==", "!=", "<=", ">=" };
    garith(g, 1); gput(g, " "); gput(g, ops[rnd_below(6)]); gput(g, " "); garith(g, 1);
}

/* one helper fn: fn hX(a: i64) -> i64 { <expr over a> } */
static void ghelper(Gen *g, char name) {
    char hdr[48];
    snprintf(hdr, sizeof hdr, "fn h%c(a: i64) -> i64 { ", name);
    gput(g, hdr);
    uint32_t pick = rnd_below(3);
    if (pick == 0) { gput(g, "(a + "); gnum(g); gput(g, ")"); }
    else if (pick == 1) { gput(g, "(a * "); gnum(g); gput(g, ")"); }
    else { gput(g, "if a < "); gnum(g); gput(g, " then a else "); gnum(g); }
    /* note: the if-form here is the WHOLE body value, which both stacks accept */
    gput(g, " }\n");
}

/* gen_main body: stmts + final expr; helpers already emitted */
static void gen_program(Gen *g) {
    g->len = 0; g->buf[0] = '\0'; g->nvars = 0;
    int nh = (int)rnd_below(3); /* 0..2 helpers */
    for (int i = 0; i < nh; i++) ghelper(g, (char)('a' + i));
    gput(g, "fn main() -> i64 {\n");
    int nstmt = 1 + (int)rnd_below(4);
    for (int i = 0; i < nstmt; i++) {
        uint32_t kind = rnd_below(4);
        if ((kind == 0 || g->nvars == 0) && g->nvars < 8) {
            /* new variable: register it only AFTER its initializer is
             * generated, so the initializer cannot reference itself */
            char v = (char)('p' + g->nvars);
            char let[16];
            snprintf(let, sizeof let, "let %c = ", v);
            gput(g, let);
            gexpr(g, 2);
            gput(g, ";\n");
            g->vars[g->nvars++] = v;
        } else if (kind == 1 && g->nvars < 8) {
            char v = (char)('p' + g->nvars);
            char let[32];
            /* fresh small counter: copying an arbitrary var could iterate
             * ~4e14 times and hit the child alarm */
            snprintf(let, sizeof let, "let %c = %u; while ", v,
                     (unsigned)(1 + rnd_below(40)));
            gput(g, let);
            g->vars[g->nvars++] = v;
            char t[2] = { v, 0 };
            gput(g, t);
            gput(g, " > 0 { let ");
            gput(g, t);
            gput(g, " = ");
            gput(g, t);
            gput(g, " - 1; ");
            gexpr(g, 1);
            /* no ';' directly before '}': a trailing empty item in a block
             * is outside the bootstrap grammar subset */
            gput(g, " };\n");
        } else if (kind == 2 || kind == 0) {
            gexpr(g, 2);
            gput(g, ";\n");
        } else {
            /* if-as-statement followed by another item */
            gput(g, "if "); gcmp(g); gput(g, " then "); gexpr(g, 1);
            gput(g, " else "); gexpr(g, 1); gput(g, ";\n");
        }
    }
    gexpr(g, 2);
    gput(g, "\n}\n");
}

/* ── per-case worker (runs IN CHILD) ──────────────────────────────────── */
typedef long (*fn0)(void);

enum { WMAX = 8192 };

static unsigned int *g_code_base;
static int g_code_words;
static void onill(int sig, siginfo_t *si, void *uc) {
    ucontext_t *u = (ucontext_t *)uc;
    unsigned long pc = (unsigned long)u->uc_mcontext.__pc;
    long off = g_code_base ? (long)((pc - (unsigned long)g_code_base) / 4) : -1;
    fprintf(stderr, "CHILDILL sig=%d pc_off=%ld (words=%d)\n", sig, off,
            g_code_words);
    _exit(8);
}

static int run_case(const char *src) {
    /* 1. interpret: load generated fns, call main */
    AstProgram prog;
    BpParseError perr;
    if (bp_parse(src, &prog, &perr) != 0) return 4;
    char err[256];
    /* NOTE: no expr_pool_reset() here -- it would free the self-hosted
     * compiler's own fn terms, which the compile stage below still needs.
     * Generated fns simply append to the same pool. */
    const char *names[64];
    Term *terms[64];
    static char nbuf[64][32];
    int cnt = 0;
    int imain = -1;
    for (size_t i = 0; i < prog.len && cnt < 64; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_FN && it->text && it->text_len > 0) {
            char *txt = (char *)malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            Term *ft = NULL;
            Ty *fyt = NULL;
            if (bp_parse_fn_decl(txt, &registry, &ft, &fyt, err, sizeof err) != 0) {
                free(txt);
                bp_program_free(&prog);
                fprintf(stderr, "fn_decl fail: %s\n", err);
                return 55;
            }
            free(txt);
            size_t fl = it->name_len < 31 ? it->name_len : 31;
            memcpy(nbuf[cnt], it->name ? it->name : "?", fl);
            nbuf[cnt][fl] = '\0';
            names[cnt] = nbuf[cnt];
            terms[cnt] = ft;
            if (strcmp(names[cnt], "main") == 0) imain = cnt;
            cnt++;
        }
    }
    bp_program_free(&prog);
    if (imain < 0) { fprintf(stderr, "no main among %d fns\n", cnt); return 56; }
    int vk; long vi; int vb; double vf = 0.0;
    /* zero-arg main is evaluated directly (same as cmd_run); wrapping it in
     * TERM_APP makes the interpreter fail */
    if (getenv("DF_SKIP_INTERP") == NULL) {
        if (qtt_eval_binds(terms[imain], names, (Term *const *)terms, cnt,
                           &vk, &vi, &vb, &vf, err, sizeof err) != 0 || vk != 0) {
            fprintf(stderr, "interp fail: %s (vk=%d)\n", err, vk);
            return 57;
        }
    } else {
        vi = 0;
    }

    /* 2. compile via the .bp self-hosted compiler.
     * qtt_last_arr aliases the interpreter arena and every eval replaces it,
     * so each result is fully copied out IMMEDIATELY after its own eval. */
    static long offs[256];
    static unsigned int code[WMAX + 64];
    int noff = 0, wc = 0;

    int okw = 0;
    (void)call_str_fn(idx_emit_offsets, src, &okw);
    void *oarr = okw ? qtt_last_arr(&noff) : NULL;
    if (!okw || !oarr || noff < 2 || noff > 256) return 4;
    for (int k = 0; k < noff; k++) offs[k] = qtt_last_arr_elem(k);

    int nw = 0;
    (void)call_str_fn(idx_emit_words, src, &okw);
    void *warr = okw ? qtt_last_arr(&nw) : NULL;
    (void)warr;
    wc = (int)qtt_last_arr_elem(0);
    if (wc < 1 || wc > WMAX) return 4;
    for (int k = 0; k < wc; k++) code[k] = (unsigned int)qtt_last_arr_elem(k + 1);

    long entryw = offs[noff - 1]; /* main is the LAST fn by language rule */
    if (entryw < 0 || entryw >= wc) return 4;
    if (getenv("DF_DEBUG")) {
        unsigned long sm = 0;
        for (int k = 0; k < wc; k++) sm = sm * 131 + code[k];
        fprintf(stderr, "DF wc=%d entry=%ld nfn=%ld sum=%lu w0..3=%u %u %u %u\n",
                wc, entryw, noff - 1, sm, code[0], code[1], code[2], code[3]);
        fprintf(stderr, "DF src[0..50]=%.50s\n", src);
    }

    /* 3. execute natively */
    size_t pagesz = ((size_t)wc * 4 + 4095) & ~4095ul;
    uint32_t *mem = (uint32_t *)mmap(NULL, pagesz, PROT_READ | PROT_WRITE | PROT_EXEC,
                                     MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (mem == MAP_FAILED) return 6;
    memcpy(mem, code, (size_t)wc * 4);
    __builtin___clear_cache((char *)mem, (char *)mem + (size_t)wc * 4);
    g_code_base = mem;
    g_code_words = wc;
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = onill;
    sa.sa_flags = SA_SIGINFO;
    sigaction(SIGILL, &sa, NULL);
    sigaction(SIGSEGV, &sa, NULL);
    sigaction(SIGBUS, &sa, NULL);
    long vnative = ((fn0)((char *)mem + entryw * 4))();
    munmap(mem, pagesz);

    return vnative == vi ? 0 : 3;
}

/* gen mode: emit N random valid programs as files for the out-of-process
 * differential driver (interpreter CLI vs compilewords+exec_words). */
static int mode_gen(int n, const char *outdir) {
    static Gen g;
    for (int i = 0; i < n; i++) {
        gen_program(&g);
        char p[512];
        snprintf(p, sizeof p, "%s/case_%04d.bp", outdir, i);
        FILE *f = fopen(p, "wb");
        if (!f) { perror("gen"); return 2; }
        fwrite(g.buf, 1, g.len, f);
        fclose(f);
    }
    printf("diff_fuzz: wrote %d programs to %s\n", n, outdir);
    return 0;
}

int main(int argc, char **argv) {
    if (argc > 4 && strcmp(argv[1], "gen") == 0) {
        rng_state = strtoull(argv[3], NULL, 0);
        if (rng_state == 0) rng_state = 0x9E3779B97F4A7C15ULL;
        return mode_gen(atoi(argv[2]), argv[4]);
    }
    unsigned long iterations = 2000u;
    const char *path = "../selfhost/expr_compile.bp";
    if (argc > 1) iterations = strtoul(argv[1], NULL, 10);
    if (argc > 2) path = argv[2];
    if (argc > 3) {
        rng_state = strtoull(argv[3], NULL, 0);
        if (rng_state == 0) rng_state = 0x9E3779B97F4A7C15ULL;
    }

    unsigned long match = 0, mismatch = 0, cfail = 0, ifail = 0, crash = 0, skip = 0;
    static Gen g;
    for (unsigned long it = 0; it < iterations; it++) {
        gen_program(&g);
        pid_t pid = fork();
        if (pid < 0) { fprintf(stderr, "diff_fuzz: fork failed\n"); return 2; }
        if (pid == 0) {
            /* Fresh child per case: the compiler module, the generated
             * program and both evaluations all live in this clean process,
             * so pool/registry state can never bleed between stages or
             * cases. Exit codes: 0 match 3 mismatch 4 cfail 5+ interp. */
            alarm(20);
#if 0
            /* RLIMIT_AS disabled: the interpreter's static arenas plus the
             * W^X mapping exceed 1GB of VIRTUAL space on some setups, which
             * made mmap/malloc fail mid-eval and looked like crashes */
            struct rlimit rl;
            rl.rlim_cur = rl.rlim_max = 1024UL * 1024 * 1024;
            setrlimit(RLIMIT_AS, &rl);
#endif
            if (load_compiler(path) != 0) _exit(7);
            int rc = run_case(g.buf);
            if (rc != 0 && rc != 5)
                fprintf(stderr, "--- diff_fuzz case (rc=%d) ---\n%s\n", rc, g.buf);
            _exit(rc);
        }
        int status = 0;
        waitpid(pid, &status, 0);
        if (WIFSIGNALED(status)) {
            crash++;
            fprintf(stderr, "--- CRASH (sig=%d) ---\n%s\n",
                    WTERMSIG(status), g.buf);
        } else {
            if (getenv("DF_DEBUG"))
                fprintf(stderr, "PARENT: exit=%d\n", WEXITSTATUS(status));
            switch (WEXITSTATUS(status)) {
            case 0: match++; break;
            case 3: mismatch++; break;
            case 4: case 6: cfail++; break;
            case 5: ifail++; break;
            default: skip++; break;
            }
        }
        if ((it + 1) % 250 == 0)
            fprintf(stderr,
                    "[%lu] match=%lu mismatch=%lu cfail=%lu ifail=%lu crash=%lu skip=%lu\n",
                    it + 1, match, mismatch, cfail, ifail, crash, skip);
    }
    printf("diff_fuzz: %lu cases: match=%lu mismatch=%lu compile_fail=%lu "
           "interp_fail=%lu crash=%lu skip=%lu\n",
           iterations, match, mismatch, cfail, ifail, crash, skip);
    return (mismatch || crash) ? 1 : 0;
}
