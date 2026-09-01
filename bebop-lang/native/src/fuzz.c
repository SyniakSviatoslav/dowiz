/* src/fuzz.c — parser fuzz harness for the Bebop C bootstrap parser.
 *
 * Feeds random / truncated / mutated inputs to bp_parse() and counts how
 * many crash or hang. Each input is parsed in a forked child so a fault in
 * the parser kills only that child, never the harness; the parent reaps the
 * child and classifies the exit via waitpid(). A per-child alarm() bounds
 * hangs. A deterministic xorshift64 PRNG (no rand()) keeps runs reproducible
 * from a fixed seed. Depends only on libc stdio/stdlib/string plus the POSIX
 * fork/wait/alarm primitives.
 *
 * Exit 0 (PASS) when there are zero crashes and zero hangs; 1 otherwise.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/wait.h>
#include <signal.h>

#include "parser.h"
#include "expr.h"
#include "qtt.h"

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
    return (size_t)(rng_next() % (unsigned long long)n);
}

/* ── input corpus (valid-ish .bp snippets) ─────────────────────────────── */
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
    /* a glyph surface (UTF-8 λ = 0xCE 0xBB) to exercise non-ASCII paths */
    "fn \xCE\xBB(x: i64) -> i64 { x }",
};
#define NSEEDS (sizeof(SEEDS) / sizeof(SEEDS[0]))

/* interesting ASCII for the biased random generator */
static const char CHARSET[] =
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    "{}()[];,:.->=+-*/%!&|^~<>_'\"\\ \n\t";
#define NCHARS (sizeof(CHARSET) - 1)

#define MAXBUF 65536u

/* ── input generators (return length, buffer left NUL-free) ────────────── */

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

static size_t copy_seed(char *buf, size_t cap, size_t idx) {
    const char *s = SEEDS[idx];
    size_t len = strlen(s);
    if (len >= cap) {
        len = cap - 1;
    }
    memcpy(buf, s, len);
    return len;
}

/* truncate a seed at a random (non-zero) length */
static size_t gen_truncated(char *buf, size_t cap) {
    size_t idx = rand_below(NSEEDS);
    size_t len = copy_seed(buf, cap, idx);
    if (len > 1) {
        len = 1 + rand_below(len); /* 1..len */
    }
    return len;
}

/* random byte for mutations: biased toward interesting ASCII */
static char mut_byte(void) {
    if ((rng_next() & 3u) != 0u) {
        return CHARSET[rand_below(NCHARS)];
    }
    return (char)(rng_next() & 0xFFu);
}

static void flip_byte(char *buf, size_t *len) {
    if (*len == 0) {
        return;
    }
    buf[rand_below(*len)] = mut_byte();
}

static void insert_byte(char *buf, size_t *len, size_t cap) {
    if (*len + 1 >= cap) {
        return;
    }
    size_t pos = rand_below(*len + 1);
    memmove(buf + pos + 1, buf + pos, *len - pos);
    buf[pos] = mut_byte();
    (*len)++;
}

static void delete_byte(char *buf, size_t *len) {
    if (*len == 0) {
        return;
    }
    size_t pos = rand_below(*len);
    memmove(buf + pos, buf + pos + 1, *len - pos - 1);
    (*len)--;
}

static void overwrite_span(char *buf, size_t *len) {
    if (*len == 0) {
        return;
    }
    size_t pos = rand_below(*len);
    size_t n = 1 + rand_below(*len - pos);
    for (size_t i = 0; i < n; i++) {
        buf[pos + i] = mut_byte();
    }
}

static void duplicate_span(char *buf, size_t *len, size_t cap) {
    if (*len < 2) {
        return;
    }
    size_t a = rand_below(*len);
    size_t b = a + 1 + rand_below(*len - a);
    size_t span = b - a;
    if (*len + span >= cap) {
        return;
    }
    size_t c = rand_below(*len + 1);
    memmove(buf + c + span, buf + c, *len - c);
    memcpy(buf + c, buf + a, span);
    *len += span;
}

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

/* splice two seeds together with random glue, then maybe truncate */
static size_t gen_spliced(char *buf, size_t cap) {
    size_t i = rand_below(NSEEDS);
    size_t j = rand_below(NSEEDS);
    size_t la = copy_seed(buf, cap, i);
    size_t off = la;
    static const char glue[] = " { }\n; , : -> ";
    size_t gl = strlen(glue);
    size_t gn = rand_below(gl + 1);
    if (gn > 0 && off + gn < cap) {
        memcpy(buf + off, glue, gn);
        off += gn;
    }
    const char *s = SEEDS[j];
    size_t lb = strlen(s);
    if (off + lb >= cap) {
        lb = cap - off - 1;
    }
    memcpy(buf + off, s, lb);
    off += lb;
    if (off > 1 && (rng_next() & 1u)) {
        off = 1 + rand_below(off);
    }
    return off;
}

/* a token bomb: enough single-char tokens to trip the lexer's token cap */
static size_t gen_token_bomb(char *buf, size_t cap) {
    size_t len = 33000u; /* > 32768 token cap, but cheap to lex */
    if (len > cap) {
        len = cap - 1;
    }
    memset(buf, ';', len);
    return len;
}

static size_t make_input(char *buf, size_t cap) {
    switch (rng_next() % 6u) {
        case 0: return gen_ascii(buf, cap);
        case 1: return gen_bytes(buf, cap);
        case 2: return gen_truncated(buf, cap);
        case 3: return gen_mutated(buf, cap);
        case 4: return gen_spliced(buf, cap);
        default: return gen_token_bomb(buf, cap);
    }
}

/* dump a bounded slice of a crashing input for repro */
static void dump_input(const char *buf, size_t len) {
    fprintf(stderr, "input (%zu bytes):\n", len);
    size_t shown = len < 256u ? len : 256u;
    for (size_t i = 0; i < shown; i++) {
        unsigned char c = (unsigned char)buf[i];
        if (c >= 0x20u && c < 0x7Fu) {
            fputc((int)c, stderr);
        } else {
            fprintf(stderr, "\\x%02x", c);
        }
    }
    if (len > shown) {
        fprintf(stderr, "...");
    }
    fputc('\n', stderr);
}

int main(int argc, char **argv) {
    unsigned long iterations = 100000u;
    int hang_timeout = 1; /* seconds */

    if (argc > 1) {
        iterations = (unsigned long)strtoul(argv[1], NULL, 10);
        if (iterations == 0) {
            iterations = 1;
        }
    }
    if (argc > 2) {
        rng_state = strtoull(argv[2], NULL, 0);
        if (rng_state == 0) {
            rng_state = 0x9E3779B97F4A7C15ULL;
        }
    }

    char *buf = (char *)malloc(MAXBUF);
    if (!buf) {
        fprintf(stderr, "fuzz: out of memory\n");
        return 2;
    }

    unsigned long ok = 0, parse_err = 0, crashes = 0, hangs = 0;
    int last_sig = 0;
    size_t last_len = 0;

    for (unsigned long it = 0; it < iterations; it++) {
        size_t len = make_input(buf, MAXBUF - 1);
        buf[len] = '\0';

        pid_t pid = fork();
        if (pid < 0) {
            fprintf(stderr, "fuzz: fork failed at iteration %lu\n", it);
            free(buf);
            return 2;
        }
        if (pid == 0) {
            /* child: bound runtime, parse, then report status via _exit.
             * B3-4 deepening: on a clean item-parse, ALSO fn-decl-parse every
             * function body (expr-parser level), so the fuzzer exercises past
             * the item scanner into declaration parsing. */
            alarm(hang_timeout);
            AstProgram prog;
            BpParseError err;
            int r = bp_parse(buf, &prog, &err);
            if (r == 0) {
                static TyRegistry fz_reg;
                typereg_init(&fz_reg);
                expr_set_registry(&fz_reg);
                char ferr[256];
                for (size_t qi = 0; qi < prog.len && r == 0; qi++) {
                    const AstItem *fnitem = &prog.items[qi];
                    if (fnitem->kind == AST_ITEM_FN && fnitem->text && fnitem->text_len > 0) {
                        char *txt = (char *)malloc(fnitem->text_len + 1);
                        if (!txt) { _exit(3); }
                        memcpy(txt, fnitem->text, fnitem->text_len);
                        txt[fnitem->text_len] = '\0';
                        Term *ft = NULL;
                        Ty *fty = NULL;
                        if (bp_parse_fn_decl(txt, &fz_reg, &ft, &fty,
                                             ferr, sizeof ferr) != 0) {
                            r = 2; /* decl-level failure: counted as parse_err */
                        }
                        free(txt);
                    }
                }
            }
            bp_program_free(&prog);
            _exit(r == 0 ? 0 : 1);
        }

        int status = 0;
        waitpid(pid, &status, 0);

        if (WIFEXITED(status)) {
            if (WEXITSTATUS(status) == 0) {
                ok++;
            } else {
                parse_err++;
            }
        } else if (WIFSIGNALED(status)) {
            int sig = WTERMSIG(status);
            if (sig == SIGALRM) {
                hangs++;
            } else {
                crashes++;
                last_sig = sig;
                last_len = len;
                if (crashes <= 5) {
                    fprintf(stderr, "CRASH: signal %d at iteration %lu\n", sig, it);
                    dump_input(buf, len);
                }
            }
        }
    }

    free(buf);

    printf("fuzz: %lu inputs | ok=%lu parse_err=%lu crashes=%lu hangs=%lu\n",
           iterations, ok, parse_err, crashes, hangs);
    if (crashes == 0 && hangs == 0) {
        printf("PASS: no crashes or hangs\n");
        return 0;
    }
    printf("FAIL: %lu crash(es), %lu hang(s) (last signal %d, input len %zu)\n",
           crashes, hangs, last_sig, last_len);
    return 1;
}
