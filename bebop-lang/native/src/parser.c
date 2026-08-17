/* Bebop parser — recursive descent → lightweight item AST (Phase 1).
 *
 * Parses the top-level grammar: module / fn / struct / const / use / type.
 * An item is recorded with its kind, name, and full source span (for fmt).
 */
#include "parser.h"
#include "lexer.h"

#include <stdlib.h>
#include <string.h>

const char *ast_item_kind_name(AstItemKind k) {
    switch (k) {
        case AST_ITEM_MODULE:
            return "module";
        case AST_ITEM_FN:
            return "fn";
        case AST_ITEM_STRUCT:
            return "struct";
        case AST_ITEM_CONST:
            return "const";
        case AST_ITEM_USE:
            return "use";
        case AST_ITEM_TYPE:
            return "type";
        case AST_ITEM_UNKNOWN:
            return "unknown";
    }
    return "?";
}

typedef struct {
    const BpToken *toks;
    size_t n;
    size_t pos;
} Parser;

static BpToken cur(const Parser *p) {
    return p->toks[p->pos < p->n ? p->pos : p->n - 1];
}

static int at_eof(const Parser *p) {
    return p->pos >= p->n || p->toks[p->pos].kind == BP_TOK_EOF;
}

static int peek_ident(const Parser *p, const char *kw) {
    if (p->pos >= p->n) {
        return 0;
    }
    const BpToken *t = &p->toks[p->pos];
    return t->kind == BP_TOK_IDENT && t->len == strlen(kw) &&
           strncmp(t->start, kw, t->len) == 0;
}

static int peek_punct(const Parser *p, char c) {
    if (p->pos >= p->n) {
        return 0;
    }
    const BpToken *t = &p->toks[p->pos];
    return t->kind == BP_TOK_PUNCT && t->len == 1 && t->start[0] == c;
}

static void advance(Parser *p) {
    if (p->pos < p->n) {
        p->pos++;
    }
}

/* Skip a balanced pair of ASCII delimiters (open..close), consuming both.
 * Returns 0 on success, -1 on unbalanced. */
static int skip_balanced(Parser *p, char open, char close) {
    int depth = 0;
    while (!at_eof(p)) {
        if (peek_punct(p, open)) {
            depth++;
        } else if (peek_punct(p, close)) {
            depth--;
            if (depth == 0) {
                advance(p);
                return 0;
            }
        }
        advance(p);
    }
    return -1;
}

/* Skip to the end of a brace-less statement (newline or ';'). */
static void skip_statement(Parser *p) {
    unsigned start_line = cur(p).line;
    while (!at_eof(p)) {
        BpToken t = cur(p);
        if (t.kind == BP_TOK_PUNCT && t.len == 1 && t.start[0] == ';') {
            advance(p);
            return;
        }
        advance(p);
        if (!at_eof(p) && cur(p).line != start_line) {
            return;
        }
    }
}

static int push_item(AstProgram *prog, AstItemKind kind, const char *name,
                     size_t name_len, const char *text, size_t text_len) {
    if (prog->len == prog->cap) {
        size_t ncap = prog->cap ? prog->cap * 2 : 16;
        AstItem *ni = realloc(prog->items, ncap * sizeof(AstItem));
        if (!ni) {
            return -1;
        }
        prog->items = ni;
        prog->cap = ncap;
    }
    AstItem *it = &prog->items[prog->len++];
    it->kind = kind;
    it->name = name;
    it->name_len = name_len;
    it->text = text;
    it->text_len = text_len;
    return 0;
}

int bp_parse(const char *src, AstProgram *prog, BpParseError *err) {
    memset(prog, 0, sizeof *prog);

    BpToken toks[4096];
    int n = bp_lex(src, toks, 4096);
    if (n < 0) {
        err->line = 0;
        err->msg = "too many tokens";
        return -1;
    }

    Parser p = {toks, (size_t)n, 0};

    while (!at_eof(&p)) {
        /* skip stray delimiters / punctuation that don't start an item */
        if (peek_punct(&p, '}') || peek_punct(&p, ')') || peek_punct(&p, ']')) {
            advance(&p);
            continue;
        }

        AstItemKind kind;
        if (peek_ident(&p, "module")) {
            kind = AST_ITEM_MODULE;
        } else if (peek_ident(&p, "fn")) {
            kind = AST_ITEM_FN;
        } else if (peek_ident(&p, "struct")) {
            kind = AST_ITEM_STRUCT;
        } else if (peek_ident(&p, "const")) {
            kind = AST_ITEM_CONST;
        } else if (peek_ident(&p, "use")) {
            kind = AST_ITEM_USE;
        } else if (peek_ident(&p, "type")) {
            kind = AST_ITEM_TYPE;
        } else {
            err->line = cur(&p).line;
            err->msg = "expected an item (module/fn/struct/const/use/type)";
            bp_program_free(prog);
            return -1;
        }

        const char *text_start = cur(&p).start;
        advance(&p); /* consume the keyword */

        const char *name = NULL;
        size_t name_len = 0;
        int name_morse = 0;
        if (!at_eof(&p) &&
            (cur(&p).kind == BP_TOK_IDENT || cur(&p).kind == BP_TOK_GLYPH ||
             cur(&p).kind == BP_TOK_MORSE)) {
            name_morse = (cur(&p).kind == BP_TOK_MORSE);
            name = cur(&p).start;
            name_len = cur(&p).len;
            advance(&p);
        }

        /* body: brace block for module/fn/struct, else statement */
        int ok = 0;
        if (peek_punct(&p, '{')) {
            ok = (skip_balanced(&p, '{', '}') == 0);
        } else {
            skip_statement(&p);
            ok = 1;
        }
        if (!ok) {
            err->line = cur(&p).line;
            err->msg = "unbalanced braces";
            bp_program_free(prog);
            return -1;
        }

        const char *text_end = p.pos > 0 ? (p.toks[p.pos - 1].start + p.toks[p.pos - 1].len) : text_start;
        if (push_item(prog, kind, name, name_len, text_start,
                      (size_t)(text_end - text_start)) != 0) {
            err->line = cur(&p).line;
            err->msg = "out of memory";
            bp_program_free(prog);
            return -1;
        }
        prog->items[prog->len - 1].name_morse = name_morse;
    }

    err->line = 0;
    err->msg = "";
    return 0;
}

void bp_program_free(AstProgram *prog) {
    free(prog->items);
    prog->items = NULL;
    prog->len = 0;
    prog->cap = 0;
}
