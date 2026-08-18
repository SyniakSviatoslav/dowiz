/* bebopc — native C bootstrap CLI (Phase 1). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "glyph.h"
#include "lexer.h"
#include "parser.h"
#include "morse.h"
#include "qtt.h"
#include "ntt.h"
#include "hyper.h"
#include "mem.h"
#include "expr.h"
#include "verify.h"
#include "vsa.h"
#include "codegen.h"
#include "native.h"
#include "money.h"
#include "fft.h"
#include "arena.h"
#include "event.h"
#include "modular.h"
#include "sort.h"
#include "token_bucket.h"
#include "checksum.h"
#include "hex_util.h"
#include "trig.h"
#include "rng.h"
#include "stats.h"
#include "pid.h"
#include "markov.h"
#include "autonomic.h"
#include "noether.h"
#include "typereg.h"
#include "atomic.h"
#include "bench_all.h"
#include "vir.h"
#include "theorem.h"
#include "pac.h"
#include "effect.h"
#include "jittable.h"
#include "supervise.h"
#include "session.h"
#include "syscall.h"
#include "typereflect.h"
#include "smt.h"
#include "termination.h"
#include "contract.h"
#include "comptime.h"
#include "fmt.h"
#include "power.h"
#include "x86_64.h"
#include "gt.h"

static void usage(void) {
    fprintf(stderr,
            "usage: bebopc version | glyphs | glyph NAME | tokens FILE\n");
}

static void cmd_glyphs(void) {
    for (size_t i = 0; i < bp_glyph_count(); i++) {
        const BpGlyph *g = bp_glyph_at(i);
        char buf[512];
        int n = bp_glyph_render_braille(g, buf, sizeof buf);
        printf("%s (%s), %zu deltas:\n%s\n", g->name, g->placeholder,
               g->outline_len, (n < 0 ? "(overflow)" : buf));
    }
}

static void cmd_glyph(const char *name) {
    const BpGlyph *g = bp_glyph_by_name(name);
    if (!g) {
        fprintf(stderr, "unknown glyph: %s\n", name);
        exit(1);
    }
    char buf[512];
    int n = bp_glyph_render_braille(g, buf, sizeof buf);
    printf("%s\n", (n < 0 ? "(overflow)" : buf));
}

static const char *kind_name(BpTokKind k) {
    switch (k) {
        case BP_TOK_IDENT:
            return "ident";
        case BP_TOK_GLYPH:
            return "glyph";
        case BP_TOK_NUMBER:
            return "number";
        case BP_TOK_PUNCT:
            return "punct";
        case BP_TOK_EOF:
            return "eof";
    }
    return "?";
}

static void cmd_tokens(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "cannot open %s\n", path);
        exit(1);
    }
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *src = malloc((size_t)sz + 1);
    if (!src) {
        fclose(f);
        fprintf(stderr, "oom\n");
        exit(1);
    }
    size_t rd = fread(src, 1, (size_t)sz, f);
    src[rd] = '\0';
    fclose(f);

    BpToken toks[4096];
    int n = bp_lex(src, toks, 4096);
    if (n < 0) {
        fprintf(stderr, "too many tokens\n");
        free(src);
        exit(1);
    }
    for (int i = 0; i < n; i++) {
        printf("%3u  %-6s  '%.*s'\n", toks[i].line, kind_name(toks[i].kind),
               (int)toks[i].len, toks[i].start);
    }
    free(src);
}

static void cmd_parse(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "cannot open %s\n", path);
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

    AstProgram prog;
    BpParseError err;
    if (bp_parse(src, &prog, &err) != 0) {
        fprintf(stderr, "parse error at line %u: %s\n", err.line, err.msg);
        free(src);
        exit(1);
    }
    printf("parsed %zu items:\n", prog.len);
    for (size_t i = 0; i < prog.len; i++) {
        const AstItem *it = &prog.items[i];
        const char *name = it->name ? it->name : "";
        printf("  %-6s '%.*s'\n", ast_item_kind_name(it->kind), (int)it->name_len,
               name);
    }
    bp_program_free(&prog);
    free(src);
}

static void cmd_morse(const char *text) {
    char buf[1024];
    if (bp_morse_encode(text, buf, sizeof buf) != 0) {
        fprintf(stderr, "cannot encode (unsupported char)\n");
        exit(1);
    }
    printf("%s\n", buf);
}

static void cmd_unmorse(const char *morse) {
    char buf[1024];
    if (bp_morse_decode(morse, buf, sizeof buf) != 0) {
        fprintf(stderr, "cannot decode (unknown code)\n");
        exit(1);
    }
    printf("%s\n", buf);
}

static void cmd_fmt(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "cannot open %s\n", path);
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

    AstProgram prog;
    BpParseError err;
    if (bp_parse(src, &prog, &err) != 0) {
        fprintf(stderr, "parse error at line %u: %s\n", err.line, err.msg);
        free(src);
        exit(1);
    }
    /* Canonical form: trim each item's span, one blank line between items. */
    for (size_t i = 0; i < prog.len; i++) {
        const AstItem *it = &prog.items[i];
        const char *s = it->text;
        const char *e = it->text + it->text_len;
        while (s < e && (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r')) {
            s++;
        }
        while (e > s && (e[-1] == ' ' || e[-1] == '\t' || e[-1] == '\n' || e[-1] == '\r')) {
            e--;
        }
        printf("%.*s\n\n", (int)(e - s), s);
    }
    bp_program_free(&prog);
    free(src);
}

static void cmd_qtt(void) {
    char buf[8192];
    int ok1 = qtt_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("QTT semiring: %s\n", ok1 == 0 ? "PASS" : "FAIL");
    int ok2 = qtt_check_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("QTT typechecker: %s\n", ok2 == 0 ? "PASS" : "FAIL");
    int ok3 = qtt_eval_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("QTT evaluator: %s\n", ok3 == 0 ? "PASS" : "FAIL");
    int ok4 = qtt_struct_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("QTT structs: %s\n", ok4 == 0 ? "PASS" : "FAIL");
    int ok5 = qtt_enum_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("QTT enums: %s\n", ok5 == 0 ? "PASS" : "FAIL");
    int ok6 = qtt_dep_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("QTT dependent: %s\n", ok6 == 0 ? "PASS" : "FAIL");
    int ok7 = qtt_effect_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("QTT effects: %s\n", ok7 == 0 ? "PASS" : "FAIL");
    exit((ok1 == 0 && ok2 == 0 && ok3 == 0 && ok4 == 0 && ok5 == 0 &&
          ok6 == 0 && ok7 == 0) ? 0 : 1);
}

static void cmd_ntt(void) {
    char buf[4096];
    int ok = ntt_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("NTT self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_hyper(void) {
    char buf[4096];
    int ok = hyper_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Hypervector self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_mem(void) {
    char buf[4096];
    int ok = mem_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Memory self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_expr(const char *text) {
    Term *t = NULL;
    char err[256];
    expr_pool_reset();
    if (expr_parse(text, &t, err, sizeof err) != 0) {
        fprintf(stderr, "parse error: %s\n", err);
        exit(1);
    }
    char ty[128];
    if (qtt_check_closed(t, ty, sizeof ty, err, sizeof err) != 0) {
        fprintf(stderr, "type error: %s\n", err);
        exit(1);
    }
    int kind;
    long i;
    int b;
    if (qtt_eval(t, &kind, &i, &b, err, sizeof err) != 0) {
        fprintf(stderr, "eval error: %s\n", err);
        exit(1);
    }
    if (kind == 0) {
        printf("%s = %ld\n", ty, i);
    } else {
        printf("%s = %s\n", ty, b ? "true" : "false");
    }
}

static void cmd_verify(void) {
    char buf[4096];
    int ok = verify_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Verification self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_vsa(void) {
    char buf[4096];
    int ok = vsa_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("VSA self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_codegen(const char *text, const char *outfile) {
    Term *t = NULL;
    char err[256];
    expr_pool_reset();
    if (expr_parse(text, &t, err, sizeof err) != 0) {
        fprintf(stderr, "parse error: %s\n", err);
        exit(1);
    }
    unsigned char buf[2048];
    int n = codegen_wasm(t, buf, sizeof buf, err, sizeof err);
    if (n < 0) {
        fprintf(stderr, "codegen error: %s\n", err);
        exit(1);
    }
    if (outfile) {
        FILE *f = fopen(outfile, "wb");
        if (!f) {
            fprintf(stderr, "cannot open %s\n", outfile);
            exit(1);
        }
        fwrite(buf, 1, (size_t)n, f);
        fclose(f);
        printf("wrote %d bytes to %s\n", n, outfile);
    } else {
        for (int i = 0; i < n; i++) {
            printf("%02x", buf[i]);
        }
        printf("\n");
    }
}

static void cmd_codegen_test(void) {
    char buf[4096];
    int ok = codegen_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Codegen self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_jit(const char *text) {
    Term *t = NULL;
    char err[256];
    expr_pool_reset();
    if (expr_parse(text, &t, err, sizeof err) != 0) {
        fprintf(stderr, "parse error: %s\n", err);
        exit(1);
    }
    char ty[128];
    if (qtt_check_closed(t, ty, sizeof ty, err, sizeof err) != 0) {
        fprintf(stderr, "type error: %s\n", err);
        exit(1);
    }
    err[0] = '\0';
    long result = native_eval(t, err, sizeof err);
    if (err[0]) {
        fprintf(stderr, "native error: %s\n", err);
        exit(1);
    }
    printf("%s = %ld\n", ty, result);
}

static void cmd_native_test(void) {
    char buf[4096];
    int ok = native_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Native self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_money(void) {
    char buf[4096];
    int ok = money_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Money self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_fft(void) {
    char buf[4096];
    int ok = fft_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("FFT self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_arena(void) {
    char buf[4096];
    int ok = arena_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Arena self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_event(void) {
    char buf[4096];
    int ok = event_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Event self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_modular(void) {
    char buf[4096];
    int ok = modular_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Modular self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_sort(void) {
    char buf[4096];
    int ok = sort_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Sort self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_token_bucket(void) {
    char buf[4096];
    int ok = token_bucket_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("TokenBucket self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

/* run: execute a .bp function with a string argument (interpreter). */
static void cmd_run(const char *path, const char *arg) {
    FILE *f = fopen(path, "rb");
    if (!f) { fprintf(stderr, "cannot open %s\n", path); exit(1); }
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *src = malloc((size_t)sz + 1);
    if (!src) { fclose(f); exit(1); }
    size_t rd = fread(src, 1, (size_t)sz, f);
    src[rd] = '\0';
    fclose(f);

    AstProgram prog;
    BpParseError perr;
    if (bp_parse(src, &prog, &perr) != 0) {
        fprintf(stderr, "parse error at %u: %s\n", perr.line, perr.msg);
        free(src); exit(1);
    }
    TyRegistry reg;
    typereg_init(&reg);
    expr_set_registry(&reg);
    char err[256];
    expr_pool_reset();
    /* Parse struct declarations into the registry first. */
    for (size_t i = 0; i < prog.len; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_STRUCT && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            if (bp_parse_struct_decl(txt, &reg, err, sizeof err) != 0) {
                fprintf(stderr, "struct parse error: %s\n", err);
                free(txt); bp_program_free(&prog); free(src); exit(1);
            }
            free(txt);
        }
        if (it->kind == AST_ITEM_ENUM && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            if (bp_parse_enum_decl(txt, &reg, err, sizeof err) != 0) {
                fprintf(stderr, "enum parse error: %s\n", err);
                free(txt); bp_program_free(&prog); free(src); exit(1);
            }
            free(txt);
        }
    }
    /* Collect all fns so later fns can call earlier ones (closures). */
    enum { MAX_FNS = 64 };
    const char *fn_names[MAX_FNS];
    const Ty *fn_tys[MAX_FNS];
    Term *fn_terms[MAX_FNS];
    int fn_count = 0;
    for (size_t i = 0; i < prog.len && fn_count < MAX_FNS; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_FN && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            Term *fn_term = NULL;
            Ty *fn_ty = NULL;
            if (bp_parse_fn_decl(txt, &reg, &fn_term, &fn_ty, err, sizeof err) != 0) {
                fprintf(stderr, "fn parse error: %s\n", err);
                free(txt); bp_program_free(&prog); free(src); exit(1);
            }
            free(txt);
            static char fnbuf[MAX_FNS][64];
            size_t fl = it->name_len < 63 ? it->name_len : 63;
            memcpy(fnbuf[fn_count], it->name ? it->name : "?", fl);
            fnbuf[fn_count][fl] = '\0';
            fn_names[fn_count] = fnbuf[fn_count];
            fn_tys[fn_count] = fn_ty;
            fn_terms[fn_count] = fn_term;
            fn_count++;
        }
    }
    if (fn_count == 0) {
        fprintf(stderr, "no function found\n");
        bp_program_free(&prog);
        free(src);
        exit(1);
    }
    /* Entry point = last fn defined. */
    int ei = fn_count - 1;
    Term *target = fn_terms[ei];
    const Ty *tt = fn_tys[ei];
    static Term argterm;
    memset(&argterm, 0, sizeof argterm);
    if (tt && tt->kind == TY_PI && tt->dom && tt->dom->kind == TY_I64) {
        argterm.kind = TERM_LIT;
        argterm.ival = atoll(arg);
    } else {
        argterm.kind = TERM_STR;
        argterm.name = arg;
    }
    static Term app;
    memset(&app, 0, sizeof app);
    app.kind = TERM_APP;
    app.a = target;
    app.b = &argterm;
    int vk; long vi; int vb;
    if (qtt_eval_binds(&app, fn_names, fn_terms, fn_count,
                       &vk, &vi, &vb, err, sizeof err) != 0) {
        fprintf(stderr, "eval error: %s\n", err);
        bp_program_free(&prog); free(src); exit(1);
    }
    printf("%ld\n", vi);
    bp_program_free(&prog);
    free(src);
}

static void cmd_checksum(void) {
    char buf[4096];
    int ok = checksum_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Checksum self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_hex(void) {
    char buf[4096];
    int ok = hex_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("HexUtil self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_trig(void) {
    char buf[8192];
    int ok = trig_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Trig self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_rng(void) {
    char buf[8192];
    int ok = rng_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("RNG self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_stats(void) {
    char buf[8192];
    int ok = stats_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Stats self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_pid(void) {
    char buf[8192];
    int ok = pid_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("PID self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_markov(void) {
    char buf[8192];
    int ok = markov_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Markov self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_autonomic(void) {
    char buf[8192];
    int ok = autonomic_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Autonomic self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_noether(void) {
    char buf[8192];
    int ok = noether_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Noether self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_check(const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) {
        fprintf(stderr, "cannot open %s\n", path);
        exit(1);
    }
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *src = malloc((size_t)sz + 1);
    if (!src) { fclose(f); exit(1); }
    size_t rd = fread(src, 1, (size_t)sz, f);
    src[rd] = '\0';
    fclose(f);

    AstProgram prog;
    BpParseError perr;
    if (bp_parse(src, &prog, &perr) != 0) {
        fprintf(stderr, "parse error at %u: %s\n", perr.line, perr.msg);
        free(src);
        exit(1);
    }

    TyRegistry reg;
    typereg_init(&reg);
    expr_set_registry(&reg);
    char err[256];
    int structs = 0;
    for (size_t i = 0; i < prog.len; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_STRUCT && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            if (bp_parse_struct_decl(txt, &reg, err, sizeof err) != 0) {
                fprintf(stderr, "struct parse error '%s': %s\n",
                        it->name ? it->name : "?", err);
                free(txt);
                bp_program_free(&prog);
                free(src);
                exit(1);
            }
            free(txt);
            structs++;
        }
        if (it->kind == AST_ITEM_ENUM && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            if (bp_parse_enum_decl(txt, &reg, err, sizeof err) != 0) {
                fprintf(stderr, "enum parse error '%s': %s\n",
                        it->name ? it->name : "?", err);
                free(txt);
                bp_program_free(&prog);
                free(src);
                exit(1);
            }
            free(txt);
            structs++;
        }
    }
    int fns = 0;
    expr_pool_reset();
    /* First pass: parse all fns and collect their types for cross-fn binding. */
    enum { MAX_FNS = 64 };
    const char *fn_names[MAX_FNS];
    const Ty *fn_tys[MAX_FNS];
    Term *fn_terms[MAX_FNS];
    int fn_count = 0;
    for (size_t i = 0; i < prog.len && fn_count < MAX_FNS; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_FN && it->text && it->text_len > 0) {
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            Term *fn_term = NULL;
            Ty *fn_ty = NULL;
            if (bp_parse_fn_decl(txt, &reg, &fn_term, &fn_ty, err, sizeof err) != 0) {
                fprintf(stderr, "fn parse error: %s\n", err);
                free(txt);
                bp_program_free(&prog);
                free(src);
                exit(1);
            }
            free(txt);
            static char fnbuf[MAX_FNS][64];
            size_t fl = it->name_len < 63 ? it->name_len : 63;
            memcpy(fnbuf[fn_count], it->name ? it->name : "?", fl);
            fnbuf[fn_count][fl] = '\0';
            fn_names[fn_count] = fnbuf[fn_count];
            fn_tys[fn_count] = fn_ty;
            fn_terms[fn_count] = fn_term;
            fn_count++;
        }
    }
    /* Protect the fn types from later type-pool allocations. */
    qtt_ty_checkpoint();
    /* Second pass: typecheck each fn with all earlier fns bound. */
    for (int k = 0; k < fn_count; k++) {
        const char *fname = fn_names[k] ? fn_names[k] : "?";
        Term *fn_term = fn_terms[k];
        char ty[128];
        /* Bind all fns (including self) so cross-references resolve. */
        if (qtt_check_binds(fn_term, fn_names, fn_tys, fn_count,
                            ty, sizeof ty, err, sizeof err) != 0) {
            fprintf(stderr, "fn '%s' type error: %s\n", fname, err);
            bp_program_free(&prog);
            free(src);
            exit(1);
        }
        unsigned char wasm[2048];
        int wl = codegen_wasm_fn(fn_term, wasm, sizeof wasm, err, sizeof err);
        if (wl > 0) {
            char wpath[256];
            snprintf(wpath, sizeof wpath, "/tmp/%s.wasm", fname);
            FILE *wf = fopen(wpath, "wb");
            if (wf) {
                fwrite(wasm, 1, (size_t)wl, wf);
                fclose(wf);
                printf("  compiled %s (%d bytes)\n", wpath, wl);
            }
        }
        printf("fn '%s' : %s  ok\n", fname, ty);
        fns++;
    }
    printf("parsed %d struct declarations (%d types in registry), %d functions\n",
           structs, reg.len, fns);
    int thms = 0;
    for (size_t i = 0; i < prog.len; i++) {
        const AstItem *it = &prog.items[i];
        if (it->kind == AST_ITEM_THEOREM && it->text && it->text_len > 0) {
            char tname[64];
            if (it->name && it->name_len > 0) {
                size_t tl = it->name_len < 63 ? it->name_len : 63;
                memcpy(tname, it->name, tl);
                tname[tl] = '\0';
            } else {
                strcpy(tname, "?");
            }
            char *txt = malloc(it->text_len + 1);
            memcpy(txt, it->text, it->text_len);
            txt[it->text_len] = '\0';
            char out[128];
            if (theorem_prove(txt, out, sizeof out, err, sizeof err) != 0) {
                fprintf(stderr, "theorem '%s' proof error: %s\n", tname, err);
                free(txt);
                bp_program_free(&prog);
                free(src);
                exit(1);
            }
            free(txt);
            printf("theorem '%s' : %s  proven\n", tname, out);
            thms++;
        }
    }
    if (thms > 0) {
        printf("%d theorem(s) verified\n", thms);
    }
    bp_program_free(&prog);
    free(src);
}

static void cmd_atomic(void) {
    char buf[4096];
    int ok = atomic_self_test(buf, sizeof buf);
    fputs(buf, stdout);
    printf("Atomic self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
    exit(ok == 0 ? 0 : 1);
}

static void cmd_bench(void) {
    char buf[1024];
    hv_benchmark(buf, sizeof buf);
    fputs(buf, stdout);
    bench_all_run();
}

int main(int argc, char **argv) {
    if (argc < 2) {
        usage();
        return 2;
    }
    if (strcmp(argv[1], "version") == 0) {
        printf("bebopc 0.1.0 (native C bootstrap)\n");
        return 0;
    }
    if (strcmp(argv[1], "glyphs") == 0) {
        cmd_glyphs();
        return 0;
    }
    if (strcmp(argv[1], "glyph") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_glyph(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "tokens") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_tokens(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "parse") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_parse(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "fmt") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_fmt(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "qtt") == 0) {
        cmd_qtt();
        return 0;
    }
    if (strcmp(argv[1], "ntt") == 0) {
        cmd_ntt();
        return 0;
    }
    if (strcmp(argv[1], "hyper") == 0) {
        cmd_hyper();
        return 0;
    }
    if (strcmp(argv[1], "mem") == 0) {
        cmd_mem();
        return 0;
    }
    if (strcmp(argv[1], "expr") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_expr(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "verify") == 0) {
        cmd_verify();
        return 0;
    }
    if (strcmp(argv[1], "vsa") == 0) {
        cmd_vsa();
        return 0;
    }
    if (strcmp(argv[1], "compile") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_codegen(argv[2], argc >= 4 ? argv[3] : NULL);
        return 0;
    }
    if (strcmp(argv[1], "codegen") == 0) {
        cmd_codegen_test();
        return 0;
    }
    if (strcmp(argv[1], "jit") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_jit(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "native") == 0) {
        cmd_native_test();
        return 0;
    }
    if (strcmp(argv[1], "money") == 0) {
        cmd_money();
        return 0;
    }
    if (strcmp(argv[1], "fft") == 0) {
        cmd_fft();
        return 0;
    }
    if (strcmp(argv[1], "arena") == 0) {
        cmd_arena();
        return 0;
    }
    if (strcmp(argv[1], "event") == 0) {
        cmd_event();
        return 0;
    }
    if (strcmp(argv[1], "modular") == 0) {
        cmd_modular();
        return 0;
    }
    if (strcmp(argv[1], "sort") == 0) {
        cmd_sort();
        return 0;
    }
    if (strcmp(argv[1], "token") == 0) {
        cmd_token_bucket();
        return 0;
    }
    if (strcmp(argv[1], "checksum") == 0) {
        cmd_checksum();
        return 0;
    }
    if (strcmp(argv[1], "hex") == 0) {
        cmd_hex();
        return 0;
    }
    if (strcmp(argv[1], "trig") == 0) {
        cmd_trig();
        return 0;
    }
    if (strcmp(argv[1], "rng") == 0) {
        cmd_rng();
        return 0;
    }
    if (strcmp(argv[1], "stats") == 0) {
        cmd_stats();
        return 0;
    }
    if (strcmp(argv[1], "pid") == 0) {
        cmd_pid();
        return 0;
    }
    if (strcmp(argv[1], "markov") == 0) {
        cmd_markov();
        return 0;
    }
    if (strcmp(argv[1], "autonomic") == 0) {
        cmd_autonomic();
        return 0;
    }
    if (strcmp(argv[1], "noether") == 0) {
        cmd_noether();
        return 0;
    }
    if (strcmp(argv[1], "check") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_check(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "atomic") == 0) {
        cmd_atomic();
        return 0;
    }
    if (strcmp(argv[1], "conv") == 0) {
        char buf[1024];
        int ok = qtt_conv_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Proof kernel (conversion) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "proof") == 0) {
        char buf[1024];
        int ok = qtt_proof_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Proof kernel (equality) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "nat") == 0) {
        char buf[1024];
        int ok = qtt_nat_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Proof kernel (Nat/recursor) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "str") == 0) {
        char buf[4096];
        int ok = qtt_str_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("QTT strings (check/conv/prove) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "run") == 0) {
        if (argc < 4) { usage(); return 2; }
        cmd_run(argv[2], argv[3]);
        return 0;
    }
    if (strcmp(argv[1], "x86_64") == 0) {
        char buf[4096];
        int ok = x86_64_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("x86_64 (encoder) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "power") == 0) {
        char buf[4096];
        int ok = power_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Power (WFI/WFE + PMU) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "fmttest") == 0) {
        char buf[4096];
        int ok = fmt_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Formatter (bp_fmt) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "comptime") == 0) {
        char buf[4096];
        int ok = comptime_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Comptime (const eval) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "contract") == 0) {
        char buf[4096];
        int ok = contract_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Contracts (requires/ensures -> SMT) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "termination") == 0) {
        char buf[4096];
        int ok = qtt_termination_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Termination (structural) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "universe") == 0) {
        char buf[4096];
        int ok = qtt_universe_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Universes (cumulative) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "array") == 0) {
        char buf[4096];
        int ok = qtt_array_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Arrays (literal/index) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "vir") == 0) {
        char buf[2048];
        int ok = vir_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("VIR (NEON lowering) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "pac") == 0) {
        char buf[1024];
        int ok = pac_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("PAC (pointer auth) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "effect") == 0) {
        char buf[1024];
        int ok = effect_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Effect (pure/io) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "jittable") == 0) {
        char buf[1024];
        int ok = jittable_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("JIT table (atomic swap) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "supervise") == 0) {
        char buf[1024];
        int ok = supervise_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Supervision tree (CoW rollback) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "session") == 0) {
        char buf[1024];
        int ok = session_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Session types (duality) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "syscall") == 0) {
        char buf[1024];
        int ok = syscall_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Raw syscall (no libc) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "typereflect") == 0) {
        char buf[1024];
        int ok = typereflect_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Type reflection (sizeof/alignof) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "atomicjit") == 0) {
        char buf[1024];
        int ok = vir_atomic_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Atomic machine-code (⚛) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "smt") == 0) {
        char buf[2048];
        int ok = smt_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("SMT (DPLL) self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "gt") == 0) {
        char buf[1024];
        int ok = gt_self_test(buf, sizeof buf);
        fputs(buf, stdout);
        printf("Green threads self-test: %s\n", ok == 0 ? "PASS" : "FAIL");
        return ok == 0 ? 0 : 1;
    }
    if (strcmp(argv[1], "bench") == 0) {
        cmd_bench();
        return 0;
    }
    if (strcmp(argv[1], "morse") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_morse(argv[2]);
        return 0;
    }
    if (strcmp(argv[1], "unmorse") == 0) {
        if (argc < 3) {
            usage();
            return 2;
        }
        cmd_unmorse(argv[2]);
        return 0;
    }
    usage();
    return 2;
}
