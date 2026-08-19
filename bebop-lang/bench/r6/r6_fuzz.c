/* bench/r6/r6_fuzz.c — front-end crash-resilience fuzzer for Bebop.
 *
 * Generates N random inputs across three categories and feeds each into the
 * real front-end entry points (lexer, parser, type-registry decl parsers,
 * bounded verifier, SMT-LIB generator). Each input runs in a forked child so
 * a fault kills only the child; the parent reaps and classifies:
 *
 *   clean (survived)        -> WIFEXITED
 *   crash  (SIGSEGV/abort)  -> WIFSIGNALED, signal != SIGALRM
 *   hang   (timeout)        -> WIFSIGNALED, signal == SIGALRM
 *
 * Category map (per task spec):
 *   (a) random byte strings           -> gen_bytes
 *   (b) valid-looking token soup      -> gen_tokens
 *   (c) truncated / malformed .bp     -> gen_truncated / gen_mutated / gen_spliced
 *
 * Deterministic xorshift64* PRNG keeps runs reproducible from a seed.
 * Depends on libc + POSIX fork/wait/alarm only.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
#include <signal.h>

#include "lexer.h"
#include "parser.h"
#include "verify.h"
#include "typereg.h"
#include "expr.h"

/* ── deterministic PRNG (xorshift64*) ──────────────────────────────────── */
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
    if (n == 0) return 0;
    return (size_t)(rng_next() % (unsigned long long)n);
}

#define MAXBUF 65536u

/* ── category (c): seed corpus (valid-ish .bp snippets) ───────────────── */
static const char *const SEEDS[] = {
    "module core { }",
    "struct order { id: i64, amount: i64 }",
    "enum color { red, green, blue(i64) }",
    "fn ntt(a: i64) -> i64 { a + 1 }",
    "fn main() -> i64 { 0 }",
    "const VERSION: i64 = 1",
    "use core::math",
    "type Amount = i64",
    "theorem add_comm : 1 + 2 = 3 { refl }",
    "fn f() -> i64 { let s = \"hi\"; s.len }",
    "module core { fn a() -> i64 { 1 } struct s { x: i64 } enum e { a } }",
    "fn g(x: i64) -> i64 { if (x == 0) then 1 else x * g(x - 1) }",
    "fn h() -> i64 { let a = [1, 2, 3]; a[0] }",
    "fn w() -> i64 { while (0) { 1 } }",
    "fn \xCE\xBB(x: i64) -> i64 { x }",
};
#define NSEEDS (sizeof(SEEDS) / sizeof(SEEDS[0]))

/* interesting ASCII for biased mutation */
static const char CHARSET[] =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    "{}()[];,:.->=+-*/%!&|^~<>_'\"\\ \n\t";
#define NCHARS (sizeof(CHARSET) - 1)

/* ── category (b): token pools (identifiers / numbers / glyphs / ops) ─── */
static const char *const IDENTS[] = {
    "fn", "struct", "enum", "module", "use", "const", "type", "theorem",
    "let", "if", "else", "while", "return", "true", "false", "x", "y", "z",
    "foo", "bar", "baz", "ntt", "main", "order", "color", "amount", "id",
    "result", "a", "b", "i64", "u64", "bool", "str", "_", "self", "this",
};
#define NIDENTS (sizeof(IDENTS) / sizeof(IDENTS[0]))

static const char *const NUMS[] = {
    "0", "1", "2", "42", "123", "12345", "123456789", "0x1F", "0b101",
    "3.14", "-1", "1e6", "007", "0o17", "99999999999999999999999",
};
#define NNUMS (sizeof(NUMS) / sizeof(NUMS[0]))

static const char *const GLYPHS[] = {
    "\xCE\xBB",       /* λ */
    "\xCE\xB1",       /* α */
    "\xCE\xB2",       /* β */
    "\xE2\x9A\x9B",   /* ⚛ */
    "\xE2\x88\x80",   /* ∀ */
    "\xE2\x88\x83",   /* ∃ */
    "\xE2\x86\x92",   /* → */
    "\xC3\x97",       /* × */
    "\xE2\x8A\x97",   /* ⊗ */
    "\xE2\x88\x91",   /* ∑ */
    "\xCF\x80",       /* π */
    "\xE2\x84\x95",   /* ℕ */
    "\xE2\x89\xA1",   /* ≡ */
    "\xCE\xA9",       /* Ω */
    "\xC3\xB7",       /* ÷ */
};
#define NGLYPHS (sizeof(GLYPHS) / sizeof(GLYPHS[0]))

static const char *const OPS[] = {
    "+", "-", "*", "/", "%", "==", "!=", "<", ">", "<=", ">=", "->",
    "=>", "=", "{", "}", "(", ")", "[", "]", ";", ":", ",", ".", "&",
    "|", "^", "~", "!", "::", "_", "=>", "|>", "<|", "@", "$", "#",
};
#define NOPS (sizeof(OPS) / sizeof(OPS[0]))

/* ── generators ────────────────────────────────────────────────────────── */

/* (a) random byte strings (arbitrary 0x00..0xFF) */
static size_t gen_bytes(char *buf, size_t cap) {
    size_t len = rand_below(cap);
    for (size_t i = 0; i < len; i++) {
        buf[i] = (char)(rng_next() & 0xFFu);
    }
    return len;
}

/* (b) valid-looking tokens concatenated (identifiers / numbers / glyphs / ops) */
static size_t gen_tokens(char *buf, size_t cap) {
    size_t off = 0;
    int ntokens = 1 + (int)(rng_next() % 64u);
    for (int i = 0; i < ntokens; i++) {
        if ((rng_next() & 3u) == 0 && off + 1 < cap) {
            static const char ws[] = " \n\t";
            buf[off++] = ws[rand_below(3)];
        }
        const char *t;
        size_t tlen;
        switch (rng_next() % 4u) {
            case 0: t = IDENTS[rand_below(NIDENTS)]; break;
            case 1: t = NUMS[rand_below(NNUMS)]; break;
            case 2: t = GLYPHS[rand_below(NGLYPHS)]; break;
            default: t = OPS[rand_below(NOPS)]; break;
        }
        tlen = strlen(t);
        if (off + tlen >= cap) break;
        memcpy(buf + off, t, tlen);
        off += tlen;
    }
    return off;
}

static size_t copy_seed(char *buf, size_t cap, size_t idx) {
    const char *s = SEEDS[idx];
    size_t len = strlen(s);
    if (len >= cap) len = cap - 1;
    memcpy(buf, s, len);
    return len;
}

/* (c) truncate a seed at a random (non-zero) length */
static size_t gen_truncated(char *buf, size_t cap) {
    size_t idx = rand_below(NSEEDS);
    size_t len = copy_seed(buf, cap, idx);
    if (len > 1) len = 1 + rand_below(len);
    return len;
}

static char mut_byte(void) {
    if ((rng_next() & 3u) != 0u) return CHARSET[rand_below(NCHARS)];
    return (char)(rng_next() & 0xFFu);
}

static void flip_byte(char *buf, size_t *len) {
    if (*len == 0) return;
    buf[rand_below(*len)] = mut_byte();
}

static void insert_byte(char *buf, size_t *len, size_t cap) {
    if (*len + 1 >= cap) return;
    size_t pos = rand_below(*len + 1);
    memmove(buf + pos + 1, buf + pos, *len - pos);
    buf[pos] = mut_byte();
    (*len)++;
}

static void delete_byte(char *buf, size_t *len) {
    if (*len == 0) return;
    size_t pos = rand_below(*len);
    memmove(buf + pos, buf + pos + 1, *len - pos - 1);
    (*len)--;
}

static void overwrite_span(char *buf, size_t *len) {
    if (*len == 0) return;
    size_t pos = rand_below(*len);
    size_t n = 1 + rand_below(*len - pos);
    for (size_t i = 0; i < n; i++) buf[pos + i] = mut_byte();
}

static void duplicate_span(char *buf, size_t *len, size_t cap) {
    if (*len < 2) return;
    size_t a = rand_below(*len);
    size_t b = a + 1 + rand_below(*len - a);
    size_t span = b - a;
    if (*len + span >= cap) return;
    size_t c = rand_below(*len + 1);
    memmove(buf + c + span, buf + c, *len - c);
    memcpy(buf + c, buf + a, span);
    *len += span;
}

/* (c) mutate a seed */
static size_t gen_mutated(char *buf, size_t cap) {
    size_t idx = rand_below(NSEEDS);
    size_t len = copy_seed(buf, cap, idx);
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

/* (c) splice two seeds together, then maybe truncate */
static size_t gen_spliced(char *buf, size_t cap) {
    size_t i = rand_below(NSEEDS);
    size_t j = rand_below(NSEEDS);
    size_t off = copy_seed(buf, cap, i);
    static const char glue[] = " { }\n; , : -> ";
    size_t gn = rand_below(sizeof(glue));
    if (gn > 0 && off + gn < cap) {
        memcpy(buf + off, glue, gn);
        off += gn;
    }
    const char *s = SEEDS[j];
    size_t lb = strlen(s);
    if (off + lb >= cap) lb = cap - off - 1;
    memcpy(buf + off, s, lb);
    off += lb;
    if (off > 1 && (rng_next() & 1u)) off = 1 + rand_below(off);
    return off;
}

/* pick a generator; returns the category index (0=a,1=b,2=c) */
static int make_input(char *buf, size_t cap, size_t *len_out) {
    size_t len = 0;
    int cat;
    switch (rng_next() % 3u) {
        case 0: len = gen_bytes(buf, cap); cat = 0; break;
        case 1: len = gen_tokens(buf, cap); cat = 1; break;
        default:
            switch (rng_next() % 3u) {
                case 0: len = gen_truncated(buf, cap); break;
                case 1: len = gen_mutated(buf, cap); break;
                default: len = gen_spliced(buf, cap); break;
            }
            cat = 2;
            break;
    }
    *len_out = len;
    return cat;
}

/* ── the front-end surface exercised per input (runs in the child) ────── */
static void run_frontend(const char *buf) {
    /* Stage 1: lexer */
    BpToken toks[4096];
    (void)bp_lex(buf, toks, 4096);

    /* Stage 2: parser -> AST -> decl parsers (struct/enum/fn) */
    AstProgram prog;
    BpParseError perr;
    int pr = bp_parse(buf, &prog, &perr);
    if (pr == 0 && prog.len > 0) {
        TyRegistry reg;
        typereg_init(&reg);
        expr_set_registry(&reg);
        expr_pool_reset();
        size_t limit = prog.len < 64 ? prog.len : 64;
        char derr[256];
        for (size_t i = 0; i < limit; i++) {
            const AstItem *it = &prog.items[i];
            if (!it->text || it->text_len == 0) continue;
            char *txt = (char *)malloc(it->text_len + 1);
            if (!txt) _exit(2);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            switch (it->kind) {
                case AST_ITEM_STRUCT:
                    (void)bp_parse_struct_decl(txt, &reg, derr, sizeof derr);
                    break;
                case AST_ITEM_ENUM:
                    (void)bp_parse_enum_decl(txt, &reg, derr, sizeof derr);
                    break;
                case AST_ITEM_FN: {
                    Term *t = NULL;
                    Ty *ty = NULL;
                    (void)bp_parse_fn_decl(txt, &reg, &t, &ty, derr, sizeof derr);
                    break;
                }
                default:
                    break;
            }
            free(txt);
        }
    }
    bp_program_free(&prog);

    /* Stage 3: verifier (bounded contract check + SMT-LIB VC generation) */
    char out[512];
    (void)verify_bounded(buf, "true", "result >= 0", 0, 8, out, sizeof out);
    (void)verify_smtlib(buf, "true", "result >= 0", out, sizeof out);

    _exit(0);
}

/* dump a bounded slice of a crashing input for repro (escaped) */
static void dump_input(FILE *f, const char *buf, size_t len) {
    fprintf(f, "input (%zu bytes):\n", len);
    size_t shown = len < 256u ? len : 256u;
    for (size_t i = 0; i < shown; i++) {
        unsigned char c = (unsigned char)buf[i];
        if (c >= 0x20u && c < 0x7Fu) {
            fputc((int)c, f);
        } else {
            fprintf(f, "\\x%02x", c);
        }
    }
    if (len > shown) fprintf(f, "...");
    fputc('\n', f);
}

int main(int argc, char **argv) {
    unsigned long iterations = 100000u;
    int hang_timeout = 2; /* seconds */

    if (argc > 1) {
        iterations = (unsigned long)strtoul(argv[1], NULL, 10);
        if (iterations == 0) iterations = 1;
    }
    if (argc > 2) {
        rng_state = strtoull(argv[2], NULL, 0);
        if (rng_state == 0) rng_state = 0x9E3779B97F4A7C15ULL;
    }
    if (argc > 3) {
        hang_timeout = (int)strtol(argv[3], NULL, 10);
        if (hang_timeout < 1) hang_timeout = 1;
    }

    char *buf = (char *)malloc(MAXBUF);
    if (!buf) {
        fprintf(stderr, "fuzz: out of memory\n");
        return 2;
    }

    unsigned long ok = 0, rejected = 0, crashes = 0, hangs = 0;
    unsigned long crash_by_cat[3] = {0, 0, 0};
    int first_crash_saved = 0;
    int first_sig = 0;
    unsigned long first_it = 0;
    int first_cat = -1;
    size_t first_len = 0;
    char first_bytes[MAXBUF];

    for (unsigned long it = 0; it < iterations; it++) {
        size_t len = 0;
        int cat = make_input(buf, MAXBUF - 1, &len);
        buf[len] = '\0';

        pid_t pid = fork();
        if (pid < 0) {
            fprintf(stderr, "fuzz: fork failed at iteration %lu\n", it);
            free(buf);
            return 2;
        }
        if (pid == 0) {
            alarm((unsigned)hang_timeout);
            run_frontend(buf);
            _exit(0); /* unreachable, but belt-and-suspenders */
        }

        int status = 0;
        waitpid(pid, &status, 0);

        if (WIFEXITED(status)) {
            int code = WEXITSTATUS(status);
            if (code == 0) ok++;
            else rejected++; /* clean error report (or OOM exit 2) */
        } else if (WIFSIGNALED(status)) {
            int sig = WTERMSIG(status);
            if (sig == SIGALRM) {
                hangs++;
            } else {
                crashes++;
                crash_by_cat[cat]++;
                if (!first_crash_saved) {
                    first_crash_saved = 1;
                    first_sig = sig;
                    first_it = it;
                    first_cat = cat;
                    first_len = len;
                    memcpy(first_bytes, buf, len < MAXBUF ? len : MAXBUF - 1);
                    first_bytes[len] = '\0';
                }
                if (crashes <= 5) {
                    fprintf(stderr, "CRASH: signal %d (cat %d) at iteration %lu\n",
                            sig, cat, it);
                    dump_input(stderr, buf, len);
                }
            }
        }
    }

    /* save the first crashing input's bytes for repro */
    if (first_crash_saved) {
        FILE *f = fopen("crash_first.bin", "wb");
        if (f) {
            fwrite(first_bytes, 1, first_len, f);
            fclose(f);
        }
        FILE *g = fopen("crash_first.txt", "w");
        if (g) {
            fprintf(g, "signal=%d category=%d iteration=%lu len=%zu\n",
                    first_sig, first_cat, first_it, first_len);
            dump_input(g, first_bytes, first_len);
            fclose(g);
        }
    }

    free(buf);

    unsigned long clean = iterations - crashes - hangs;
    double clean_pct = 100.0 * (double)clean / (double)iterations;
    double crash_pct = 100.0 * (double)crashes / (double)iterations;

    printf("=== r6 fuzz summary ===\n");
    printf("seed=%llu timeout=%ds\n", rng_state, hang_timeout);
    printf("total_inputs=%lu\n", iterations);
    printf("crashes=%lu (%.4f%%)\n", crashes, crash_pct);
    printf("hangs=%lu\n", hangs);
    printf("ok(accepted)=%lu rejected(clean_error)=%lu\n", ok, rejected);
    printf("clean_rate_pct=%.4f\n", clean_pct);
    printf("crashes_by_cat a=%lu b=%lu c=%lu\n",
           crash_by_cat[0], crash_by_cat[1], crash_by_cat[2]);
    if (first_crash_saved) {
        printf("first_crash: signal=%d cat=%d iter=%lu len=%zu (saved to crash_first.bin)\n",
               first_sig, first_cat, first_it, first_len);
    } else {
        printf("first_crash: none\n");
    }
    printf("=== end ===\n");

    return 0;
}
