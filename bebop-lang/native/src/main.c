/* bebopc — native C bootstrap CLI (Phase 1). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "glyph.h"
#include "lexer.h"
#include "parser.h"
#include "morse.h"

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
        const char *name = prog.items[i].name ? prog.items[i].name : "";
        printf("  %-6s '%.*s'\n", ast_item_kind_name(prog.items[i].kind),
               (int)prog.items[i].name_len, name);
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
