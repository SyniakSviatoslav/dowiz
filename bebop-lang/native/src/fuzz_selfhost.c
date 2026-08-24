/* src/fuzz_selfhost.c — fuzz the SELF-HOST .bp compiler's parser + codegen.
 *
 * Loads selfhost/expr_compile.bp once (parse + elaborate all 62 functions via
 * bp_parse_fn_decl, exactly like cmd_selfcompile in main.c), then repeatedly
 * feeds MUTATED/generated .bp source strings to its compile_program / compile /
 * compile_fn entry points through qtt_eval_binds. Each input is evaluated in a
 * forked child (so a fault in the interpreter kills only that child, never the
 * harness) with a per-child alarm() to bound hangs; the parent reaps and
 * classifies the exit via waitpid(). The compiler's own state (term pools, type
 * registry) is inherited copy-on-write, so every child starts from a pristine
 * post-load snapshot — a crash can't poison later inputs.
 *
 * A deterministic xorshift64 PRNG keeps runs reproducible from a fixed seed.
 * Before fuzzing, a forked sanity check asserts the compiler reproduces three
 * known-good AArch64 checksums (from expr_compile.bp's own self_check table),
 * proving the harness actually exercises the compiler.
 *
 * Exit 0 (PASS) when zero crashes and zero hangs; 1 otherwise.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
#include <signal.h>
#include <sys/resource.h>
#include <sys/time.h>

#include "parser.h"
#include "expr.h"
#include "qtt.h"
#include "typereg.h"

static double now_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (double)tv.tv_sec * 1000.0 + (double)tv.tv_usec / 1000.0;
}

/* ── deterministic PRNG (xorshift64*) ──────────────────────────────────── */

/* SIGSEGV diagnostics: print last-resort marker so the failing site can be
 * narrowed by bisecting instrumentation. */
static unsigned long long rng_state = 0x9E3779B97F4A7C15ULL;

static unsigned long long rng_next(void) {
    unsigned long long x = rng_state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    rng_state = x;
    return x;
}

static size_t rand_below(size_t n) {
    return (size_t)(rng_next() % (unsigned long long)n);
}

/* ── loaded self-host compiler state (file-scope: lives across forks) ──── */
enum { MAX_FNS = 256 };

static char fnbuf[MAX_FNS][64];
static const char *fn_names[MAX_FNS];
static Term *fn_terms[MAX_FNS];
static int fn_count = 0;
static int idx_program = -1; /* compile_program(s) -> i64 */
static int idx_expr = -1;    /* compile(s)        -> i64 */
static int idx_fn = -1;      /* compile_fn(s)     -> i64 */
static TyRegistry registry;  /* borrowed-name table; empty for expr_compile.bp */

static int load_compiler(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) { return -1; }
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
    if (bp_parse(src, &prog, &perr) != 0) {
        free(src); return -1;
    }
    typereg_init(&registry);
    expr_set_registry(&registry);
    char err[256];
    expr_pool_reset();
    /* struct/enum decls first (none in expr_compile.bp, kept for generality) */
    for (size_t i = 0; i < prog.len; i++) {
        const AstItem *it = &prog.items[i];
        if ((it->kind == AST_ITEM_STRUCT || it->kind == AST_ITEM_ENUM) &&
            it->text && it->text_len > 0) {
            char *txt = (char *)malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            if (it->kind == AST_ITEM_STRUCT) {
                (void)bp_parse_struct_decl(txt, &registry, err, sizeof err);
            } else {
                (void)bp_parse_enum_decl(txt, &registry, err, sizeof err);
            }
            free(txt);
        }
    }
    /* functions */
    fn_count = 0;
    for (size_t i = 0; i < prog.len && fn_count < MAX_FNS; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_FN && it->text && it->text_len > 0) {
            char *txt = (char *)malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            Term *fn_term = NULL;
            Ty *fn_ty = NULL;
            if (bp_parse_fn_decl(txt, &registry, &fn_term, &fn_ty,
                                 err, sizeof err) != 0) {
                free(txt); bp_program_free(&prog); free(src); return -1;
            }
            free(txt);
            (void)fn_ty;
            size_t fl = it->name_len < 63 ? it->name_len : 63;
            memcpy(fnbuf[fn_count], it->name ? it->name : "?", fl);
            fnbuf[fn_count][fl] = '\0';
            fn_names[fn_count] = fnbuf[fn_count];
            fn_terms[fn_count] = fn_term;
            fn_count++;
        }
    }
    bp_program_free(&prog);
    free(src);
    if (fn_count == 0) { return -1; }
    for (int i = 0; i < fn_count; i++) {
        if (strcmp(fn_names[i], "compile_program") == 0) { idx_program = i; }
        if (strcmp(fn_names[i], "compile") == 0) { idx_expr = i; }
        if (strcmp(fn_names[i], "compile_fn") == 0) { idx_fn = i; }
    }
    if (idx_program < 0 || idx_expr < 0 || idx_fn < 0) { return -1; }
    return 0;
}

/* ── input corpus (valid-ish .bp source for each entry point) ─────────── */
static const char *const PROG_SEEDS[] = {
    "fn main() { 42 }",
    "fn main() { helper(41) } fn helper(x) { x + 1 }",
    "fn main() { add(3,4) } fn add(a,b) { a + b }",
    "fn fact(n) { if n <= 1 then 1 else n * fact(n-1) }",
    "fn main() { let i = 0; let acc = 0; while i < 5 { let acc = acc + i; let i = i + 1; 0 }; acc }",
    "fn main() { let p = pt { x: 1, y: 2 }; p.x + p.y } struct pt { x: i64, y: i64 }",
    "fn main() { pt { x: 1, y: 2 } } struct pt { x: i64, y: i64 }",
    "fn main(p) { p.x + p.y } struct pt { x: i64, y: i64 }",
    "fn main() { some(5) } enum opt { none, some }",
    "fn main() { none } enum opt { none, some }",
    "fn main() { match some(5) { none => 0, some(x) => x + 1 } } enum opt { none, some }",
    "fn main() { match none { none => 0, some(x) => x + 1 } } enum opt { none, some }",
    "fn main() { let a = [1, 2, 3]; a[0] }",
    "fn main() { let s = \"hi\"; 1 }",
};
#define NPROG_SEEDS (sizeof(PROG_SEEDS) / sizeof(PROG_SEEDS[0]))

static const char *const EXPR_SEEDS[] = {
    "42",
    "1+2",
    "1+2*3",
    "2*(3+4)",
    "8/2-1",
    "if 1 then 10 else 20",
    "if 0 then 10 else 20",
    "1 + if 2 then 3 else 4",
    "if 1 then 1+2 else 3*4",
    "1<2",
    "3==3",
    "2>3",
    "1+2<4",
    "5==2+3",
    "12345678901234567890",
    "((((((1))))))",
    "-1",
    "1 2 3",
};
#define NEXPR_SEEDS (sizeof(EXPR_SEEDS) / sizeof(EXPR_SEEDS[0]))

static const char *const FN_SEEDS[] = {
    "fn f(a) { a + 1 }",
    "fn f(a,b) { a + b }",
    "fn f() { let a = 1; a + 2 }",
    "fn f(a) { let b = a + 1; b * 2 }",
    "fn main() { f(41) } fn f(a) { a + 1 }",
    "fn main() { s(6) } fn s(a) { a * a }",
    "fn main() { g(3,4) } fn g(a,b) { a + b }",
    "fn main() { foo(41) } fn foo(x) { x + 1 }",
    "fn main() { let acc = 10; let delta = 5; acc + delta }",
    "fn fact(n) { if n <= 1 then 1 else n * fact(n-1) }",
    "fn main() { let i = 0; let acc = 0; while i < 5 { let acc = acc + i; let i = i + 1; 0 }; acc }",
    "fn main() { let p = pt { x: 1, y: 2 }; p.x + p.y } struct pt { x: i64, y: i64 }",
    "fn main() { match some(5) { none => 0, some(x) => x + 1 } } enum opt { none, some }",
};
#define NFN_SEEDS (sizeof(FN_SEEDS) / sizeof(FN_SEEDS[0]))

/* interesting ASCII for the biased random generator */
static const char CHARSET[] =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    "{}()[];,:.->=+-*/%!&|^~<>_'\"\\ \n\t";
#define NCHARS (sizeof(CHARSET) - 1)

/* Smaller than fuzz.c's 64K: the self-host interpreter runs ~15us/byte, so a
 * 64K input would take ~1s per eval and make a 100k run take hours. 512 bytes
 * keeps the worst-case bounded (~8ms/eval) while still stressing deep nesting,
 * token bombs, and arbitrary-byte scanning. */
#define MAXBUF 512u

/* ── corpus descriptor + input generators ─────────────────────────────── */
typedef struct {
    const char *const *seeds;
    size_t nseeds;
} Corpus;

static const Corpus CORPUS_PROG = { PROG_SEEDS, NPROG_SEEDS };
static const Corpus CORPUS_EXPR = { EXPR_SEEDS, NEXPR_SEEDS };
static const Corpus CORPUS_FN = { FN_SEEDS, NFN_SEEDS };

static size_t gen_ascii(char *buf, size_t cap) {
    size_t len = rand_below(cap);
    for (size_t i = 0; i < len; i++) {
        buf[i] = CHARSET[rand_below(NCHARS)];
    }
    return len;
}

static size_t gen_bytes(char *buf, size_t cap) {
    size_t len = rand_below(cap);
    for (size_t i = 0; i < len; i++) {
        buf[i] = (char)(rng_next() & 0xFFu);
    }
    return len;
}

static size_t copy_seed(const Corpus *c, char *buf, size_t cap, size_t idx) {
    const char *s = c->seeds[idx];
    size_t len = strlen(s);
    if (len >= cap) { len = cap - 1; }
    memcpy(buf, s, len);
    return len;
}

static size_t gen_truncated(const Corpus *c, char *buf, size_t cap) {
    size_t idx = rand_below(c->nseeds);
    size_t len = copy_seed(c, buf, cap, idx);
    if (len > 1) { len = 1 + rand_below(len); }
    return len;
}

static char mut_byte(void) {
    if ((rng_next() & 3u) != 0u) { return CHARSET[rand_below(NCHARS)]; }
    return (char)(rng_next() & 0xFFu);
}

static void flip_byte(char *buf, size_t *len) {
    if (*len == 0) { return; }
    buf[rand_below(*len)] = mut_byte();
}

static void insert_byte(char *buf, size_t *len, size_t cap) {
    if (*len + 1 >= cap) { return; }
    size_t pos = rand_below(*len + 1);
    memmove(buf + pos + 1, buf + pos, *len - pos);
    buf[pos] = mut_byte();
    (*len)++;
}

static void delete_byte(char *buf, size_t *len) {
    if (*len == 0) { return; }
    size_t pos = rand_below(*len);
    memmove(buf + pos, buf + pos + 1, *len - pos - 1);
    (*len)--;
}

static void overwrite_span(char *buf, size_t *len) {
    if (*len == 0) { return; }
    size_t pos = rand_below(*len);
    size_t n = 1 + rand_below(*len - pos);
    for (size_t i = 0; i < n; i++) { buf[pos + i] = mut_byte(); }
}

static void duplicate_span(char *buf, size_t *len, size_t cap) {
    if (*len < 2) { return; }
    size_t a = rand_below(*len);
    size_t b = a + 1 + rand_below(*len - a);
    size_t span = b - a;
    if (*len + span >= cap) { return; }
    size_t cpos = rand_below(*len + 1);
    memmove(buf + cpos + span, buf + cpos, *len - cpos);
    memcpy(buf + cpos, buf + a, span);
    *len += span;
}

static size_t gen_mutated(const Corpus *c, char *buf, size_t cap) {
    size_t idx = rand_below(c->nseeds);
    size_t len = copy_seed(c, buf, cap, idx);
    int rounds = 1 + (int)(rng_next() % 8u);
    for (int i = 0; i < rounds; i++) {
        switch (rng_next() % 5u) {
            case 0: flip_byte(buf, &len); break;
            case 1: insert_byte(buf, &len, cap); break;
            case 2: delete_byte(buf, &len); break;
            case 3: overwrite_span(buf, &len); break;
            case 4: duplicate_span(buf, &len, cap); break;
        }
    }
    return len;
}

static size_t gen_spliced(const Corpus *c, char *buf, size_t cap) {
    size_t i = rand_below(c->nseeds);
    size_t j = rand_below(c->nseeds);
    size_t off = copy_seed(c, buf, cap, i);
    static const char glue[] = " { }\n; , : -> ";
    size_t gl = strlen(glue);
    size_t gn = rand_below(gl + 1);
    if (gn > 0 && off + gn < cap) {
        memcpy(buf + off, glue, gn);
        off += gn;
    }
    const char *s = c->seeds[j];
    size_t lb = strlen(s);
    if (off + lb >= cap) { lb = cap - off - 1; }
    memcpy(buf + off, s, lb);
    off += lb;
    if (off > 1 && (rng_next() & 1u)) { off = 1 + rand_below(off); }
    return off;
}

static size_t gen_token_bomb(char *buf, size_t cap) {
    size_t len = cap - 1;
    memset(buf, ';', len);
    return len;
}

static size_t make_input(const Corpus *c, char *buf, size_t cap) {
    switch (rng_next() % 6u) {
        case 0: return gen_ascii(buf, cap);
        case 1: return gen_bytes(buf, cap);
        case 2: return gen_truncated(c, buf, cap);
        case 3: return gen_mutated(c, buf, cap);
        case 4: return gen_spliced(c, buf, cap);
        default: return gen_token_bomb(buf, cap);
    }
}

static void dump_input(const char *buf, size_t len) {
    fprintf(stderr, "input (%zu bytes):\n", len);
    size_t shown = len < 256u ? len : 256u;
    for (size_t i = 0; i < shown; i++) {
        unsigned char c = (unsigned char)buf[i];
        if (c >= 0x20u && c < 0x7Fu) { fputc((int)c, stderr); }
        else { fprintf(stderr, "\\x%02x", c); }
    }
    if (len > shown) { fprintf(stderr, "..."); }
    fputc('\n', stderr);
}

/* Evaluate `entry` on `input` in a forked child; return the checksum (or -1 on
 * eval error) and set *ok to whether the eval succeeded without fault/hang. */
static long eval_checksum(int entry, const char *input, int timeout, int *ok) {
    int fds[2];
    if (pipe(fds) != 0) { *ok = 0; return 0; }
    pid_t pid = fork();
    if (pid < 0) { close(fds[0]); close(fds[1]); *ok = 0; return 0; }
    if (pid == 0) {
        close(fds[0]);
        alarm(timeout);
        Term argterm;
        memset(&argterm, 0, sizeof argterm);
        argterm.kind = TERM_STR;
        argterm.name = input;
        Term app;
        memset(&app, 0, sizeof app);
        app.kind = TERM_APP;
        app.a = fn_terms[entry];
        app.b = &argterm;
        int vk; long vi; int vb; double vf = 0.0; char err[256];
        int r = qtt_eval_binds(&app, fn_names, fn_terms, fn_count,
                               &vk, &vi, &vb, &vf, err, sizeof err);
        long result = (r == 0) ? vi : -1;
        ssize_t wr = write(fds[1], &result, sizeof result);
        (void)wr;
        close(fds[1]);
        _exit(r == 0 ? 0 : 1);
    }
    close(fds[1]);
    long got = -2;
    ssize_t rr = read(fds[0], &got, sizeof got);
    (void)rr;
    close(fds[0]);
    int status = 0;
    waitpid(pid, &status, 0);
    *ok = WIFEXITED(status) && WEXITSTATUS(status) == 0;
    return got;
}

int main(int argc, char **argv) {
    unsigned long iterations = 100000u;
    int hang_timeout = 2; /* seconds */
    const char *path = "../selfhost/expr_compile.bp";

    if (argc > 1) {
        iterations = (unsigned long)strtoul(argv[1], NULL, 10);
        if (iterations == 0) { iterations = 1; }
    }
    if (argc > 2) { path = argv[2]; }
    if (argc > 3) {
        rng_state = strtoull(argv[3], NULL, 0);
        if (rng_state == 0) { rng_state = 0x9E3779B97F4A7C15ULL; }
    }

    if (load_compiler(path) != 0) {
        fprintf(stderr, "fuzz_selfhost: failed to load compiler %s\n", path);
        return 2;
    }
    fprintf(stderr, "loaded %s: %d functions (program=%d expr=%d fn=%d)\n",
            path, fn_count, idx_program, idx_expr, idx_fn);

    /* sanity: reproduce three known-good checksums from expr_compile.bp's
     * self_check table to prove the harness exercises the compiler. */
    {
        struct { int entry; const char *src; long want; const char *name; }
        checks[3] = {
            { idx_program, "fn main() { helper(41) } fn helper(x) { x + 1 }",
              146919800484L, "compile_program" },
            { idx_expr, "42", 21426575550L, "compile" },
            { idx_fn, "fn f(a) { a + 1 }", 59036607243L, "compile_fn" },
        };
        for (int i = 0; i < 3; i++) {
            int ok = 0;
            long got = eval_checksum(checks[i].entry, checks[i].src, 10, &ok);
            if (!ok || got != checks[i].want) {
                fprintf(stderr, "fuzz_selfhost: SANITY FAIL %s: ok=%d got=%ld want=%ld\n",
                        checks[i].name, ok, got, checks[i].want);
                return 2;
            }
            fprintf(stderr, "sanity ok: %s -> %ld\n", checks[i].name, got);
        }
    }

    char *buf = (char *)malloc(MAXBUF);
    if (!buf) { fprintf(stderr, "fuzz_selfhost: out of memory\n"); return 2; }

    /* single-input mode: FZ_ONE_FILE=<path> runs each target once, timed */
    if (getenv("FZ_ONE_FILE")) {
        FILE *ff = fopen(getenv("FZ_ONE_FILE"), "rb");
        if (!ff) { fprintf(stderr, "cannot open one-file\n"); return 2; }
        size_t flen = fread(buf, 1, MAXBUF - 1, ff);
        buf[flen] = '\0';
        fclose(ff);
        fprintf(stderr, "loaded %zu bytes\n", flen);
        const char *names[3] = { "compile_program", "compile", "compile_fn" };
        int entries[3] = { idx_program, idx_expr, idx_fn };
        for (int t = 0; t < 3; t++) {
            double t0 = now_ms();
            long v = eval_checksum(entries[t], buf, 10, &(int){0});
            double dt = now_ms() - t0;
            printf("%s -> %ld (%.1f ms)\n", names[t], v, dt);
        }
        free(buf);
        return 0;
    }

    unsigned long ok = 0, eval_err = 0, crashes = 0, hangs = 0;
    int last_sig = 0;
    size_t last_len = 0;
    const char *last_target = "?";
    double t_total = 0.0, t_max = 0.0;
    unsigned long hist[6] = {0, 0, 0, 0, 0, 0}; /* <1,<5,<10,<50,<200,>=200 ms */

    for (unsigned long it = 0; it < iterations; it++) {
        int entry;
        const Corpus *corpus;
        const char *target;
        switch (rng_next() % 3u) {
            case 0: entry = idx_program; corpus = &CORPUS_PROG; target = "compile_program"; break;
            case 1: entry = idx_expr;    corpus = &CORPUS_EXPR;  target = "compile";        break;
            default: entry = idx_fn;     corpus = &CORPUS_FN;    target = "compile_fn";     break;
        }

        size_t len = make_input(corpus, buf, MAXBUF - 1);
        buf[len] = '\0';

        double t0 = now_ms();
        /* FZ_CRASH_IT=<n>: run iteration n in-process (no fork) so a debugger
         * sees the faulting state directly. */
        if (getenv("FZ_CRASH_IT") &&
            strtoul(getenv("FZ_CRASH_IT"), 0, 10) == it) {
            Term argterm;
            memset(&argterm, 0, sizeof argterm);
            argterm.kind = TERM_STR;
            argterm.name = buf;
            Term app;
            memset(&app, 0, sizeof app);
            app.kind = TERM_APP;
            app.a = fn_terms[entry];
            app.b = &argterm;
            int vk; long vi; int vb; double vf = 0.0; char err[256];
            int r = qtt_eval_binds(&app, fn_names, fn_terms, fn_count,
                                   &vk, &vi, &vb, &vf, err, sizeof err);
            printf("in-process it=%lu r=%d v=%ld\n", it, r, vi);
            continue;
        }
        pid_t pid = fork();
        if (pid < 0) { fprintf(stderr, "fuzz_selfhost: fork failed at %lu\n", it); free(buf); return 2; }
        if (pid == 0) {
            /* Deep-but-bounded interpreter recursion needs headroom: give the
             * child a large stack budget so the eval-depth cap (graceful
             * failure) always fires before the machine stack does. */
            struct rlimit rl;
            if (getrlimit(RLIMIT_STACK, &rl) == 0) {
                rl.rlim_cur = 512 * 1024 * 1024;
                setrlimit(RLIMIT_STACK, &rl);
            }
            alarm(hang_timeout);
            Term argterm;
            memset(&argterm, 0, sizeof argterm);
            argterm.kind = TERM_STR;
            argterm.name = buf;
            Term app;
            memset(&app, 0, sizeof app);
            app.kind = TERM_APP;
            app.a = fn_terms[entry];
            app.b = &argterm;
            int vk; long vi; int vb; double vf = 0.0; char err[256];
            int r = qtt_eval_binds(&app, fn_names, fn_terms, fn_count,
                                   &vk, &vi, &vb, &vf, err, sizeof err);
            _exit(r == 0 ? 0 : 1);
        }

        int status = 0;
        waitpid(pid, &status, 0);
        double dt = now_ms() - t0;
        t_total += dt;
        if (dt > t_max) { t_max = dt; }
        if (dt < 1.0) hist[0]++;
        else if (dt < 5.0) hist[1]++;
        else if (dt < 10.0) hist[2]++;
        else if (dt < 50.0) hist[3]++;
        else if (dt < 200.0) hist[4]++;
        else hist[5]++;
        if (dt >= 200.0) {
            static int slow_saved = 0;
            if (getenv("FZ_SLOW_DIR") && slow_saved < 10) {
                char pth[256];
                snprintf(pth, sizeof pth, "%s/slow_%03lu.bin",
                         getenv("FZ_SLOW_DIR"), it);
                FILE *sf = fopen(pth, "wb");
                if (sf) { fwrite(buf, 1, len, sf); fclose(sf); }
                slow_saved++;
            }
            static int slow_shown = 0;
            if (slow_shown < 8) {
                fprintf(stderr, "SLOW %.0fms it=%lu target=%s ", dt, it, target);
                dump_input(buf, len);
                slow_shown++;
            }
        }

        if (WIFEXITED(status)) {
            if (WEXITSTATUS(status) == 0) { ok++; }
            else { eval_err++; }
        } else if (WIFSIGNALED(status)) {
            int sig = WTERMSIG(status);
            if (sig == SIGALRM) {
                hangs++;
            } else {
                crashes++;
                last_sig = sig;
                last_len = len;
                last_target = target;
                if (getenv("FZ_SLOW_DIR")) {
                    char pth[256];
                    snprintf(pth, sizeof pth, "%s/crash_%04lu.sig%d.bin",
                             getenv("FZ_SLOW_DIR"), it, sig);
                    FILE *cf = fopen(pth, "wb");
                    if (cf) { fwrite(buf, 1, len, cf); fclose(cf); }
                }
                if (crashes <= 5) {
                    fprintf(stderr, "CRASH: signal %d at iteration %lu (%s)\n",
                            sig, it, target);
                    dump_input(buf, len);
                }
            }
        }
    }

    free(buf);

    printf("fuzz_selfhost: %lu inputs | ok=%lu eval_err=%lu crashes=%lu hangs=%lu\n",
           iterations, ok, eval_err, crashes, hangs);
    printf("timing: %.1f ms/input avg, %.1f ms max | <1:%lu <5:%lu <10:%lu <50:%lu <200:%lu >=200:%lu\n",
           t_total / (double)iterations, t_max, hist[0], hist[1], hist[2], hist[3],
           hist[4], hist[5]);
    if (crashes == 0 && hangs == 0) {
        printf("PASS: no crashes or hangs\n");
        return 0;
    }
    printf("FAIL: %lu crash(es), %lu hang(s) (last signal %d, target %s, len %zu)\n",
           crashes, hangs, last_sig, last_target, last_len);
    return 1;
}
