/* Bebop parser — recursive descent → lightweight item AST (Phase 1).
 *
 * Parses the top-level grammar: module / fn / struct / const / use / type.
 * An item is recorded with its kind, name, and full source span (for fmt).
 */
#include "parser.h"
#include "lexer.h"
#include "qtt.h"
#include "expr.h"

#include <stdio.h>
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
        case AST_ITEM_THEOREM:
            return "theorem";
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
        } else if (peek_ident(&p, "theorem")) {
            kind = AST_ITEM_THEOREM;
        } else {
            err->line = cur(&p).line;
            err->msg = "expected an item (module/fn/struct/const/use/type/theorem)";
            bp_program_free(prog);
            return -1;
        }

        const char *text_start = cur(&p).start;
        advance(&p); /* consume the keyword */

        const char *name = NULL;
        size_t name_len = 0;
        if (!at_eof(&p) &&
            (cur(&p).kind == BP_TOK_IDENT || cur(&p).kind == BP_TOK_GLYPH)) {
            name = cur(&p).start;
            name_len = cur(&p).len;
            advance(&p);
        }

        /* for fn items: skip signature (params + return type) until '{' */
        if (kind == AST_ITEM_FN) {
            while (!at_eof(&p) && !peek_punct(&p, '{')) advance(&p);
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
    }

    err->line = 0;
    err->msg = "";
    return 0;
}

void bp_program_free(AstProgram *prog) {
    if (prog->items) {
        free(prog->items);
    }
    memset(prog, 0, sizeof *prog);
}

int bp_parse_struct_decl(const char *src, TyRegistry *reg, char *err,
                         size_t cap) {
    BpToken toks[256];
    int n = bp_lex(src, toks, 256);
    if (n < 0) {
        snprintf(err, cap, "too many tokens");
        return -1;
    }
    /* skip comments / glyphs to first ident */
    int pos = 0;
    while (pos < n && toks[pos].kind != BP_TOK_IDENT) {
        pos++;
    }
    if (pos >= n) {
        snprintf(err, cap, "expected struct name");
        return -1;
    }
    char name[64];
    size_t nl = toks[pos].len < 63 ? toks[pos].len : 63;
    memcpy(name, toks[pos].start, nl);
    name[nl] = '\0';
    pos++;
    /* skip to '{' */
    while (pos < n && !(toks[pos].kind == BP_TOK_PUNCT &&
                        toks[pos].start[0] == '{')) {
        pos++;
    }
    if (pos >= n) {
        snprintf(err, cap, "expected '{'");
        return -1;
    }
    pos++;


    static TyField fields[32];
    static char fnames[32][64];
    int nf = 0;
    while (pos < n && nf < 32) {
        if (toks[pos].kind == BP_TOK_PUNCT &&
            toks[pos].start[0] == '}') {
            break;
        }
        if (toks[pos].kind != BP_TOK_IDENT) {
            snprintf(err, cap, "expected field name");
            return -1;
        }
        size_t fl =
            toks[pos].len < 63 ? toks[pos].len : 63;
        memcpy(fnames[nf], toks[pos].start, fl);
        fnames[nf][fl] = '\0';
        pos++;
        if (pos >= n || toks[pos].kind != BP_TOK_PUNCT ||
            toks[pos].start[0] != ':') {
            snprintf(err, cap, "expected ':'");
            return -1;
        }
        pos++;
        /* type name (built-in or user-defined) */
        Ty *ft = NULL;
        if (toks[pos].kind == BP_TOK_IDENT) {
            char tname[32];
            size_t tl =
                toks[pos].len < 31 ? toks[pos].len : 31;
            memcpy(tname, toks[pos].start, tl);
            tname[tl] = '\0';
            /* user-registered type */
            ft = typereg_get(reg, tname);
            if (!ft) {
                if (strcmp(tname, "i64") == 0) {
                    ft = qtt_i64();
                } else if (strcmp(tname, "bool") == 0) {
                    ft = qtt_bool();
                } else if (strcmp(tname, "Type") == 0) {
                    ft = qtt_type();
                }
            }
            if (!ft) {
                snprintf(err, cap, "unknown type '%s'",
                         tname);
                return -1;
            }
            pos++;
        } else {
            snprintf(err, cap, "expected type name");
            return -1;
        }
        fields[nf].name = fnames[nf];
        fields[nf].ty = ft;
        nf++;
        if (pos < n && toks[pos].kind == BP_TOK_PUNCT &&
            toks[pos].start[0] == ',') {
            pos++;
        }
    }
    if (nf == 0) {
        snprintf(err, cap, "empty struct");
        return -1;
    }
    static Ty st;
    memset(&st, 0, sizeof st);
    st.kind = TY_STRUCT;
    st.fields = fields;
    st.nfields = nf;
    if (typereg_put(reg, name, &st) != 0) {
        snprintf(err, cap, "type '%s' already declared",
                 name);
        return -1;
    }
    return 0;
}

int bp_parse_fn_decl(const char *src, TyRegistry *reg, Term **out,
                     Ty **out_ty, char *err, size_t cap) {
    BpToken toks[256];
    int n = bp_lex(src, toks, 256);
    if (n < 0) { snprintf(err, cap, "too many tokens"); return -1; }
    int pos = 0;
    while (pos < n && toks[pos].kind != BP_TOK_IDENT) pos++;
    if (pos >= n) { snprintf(err, cap, "expected fn name"); return -1; }
    pos++; /* skip name */
    while (pos < n && !(toks[pos].kind == BP_TOK_PUNCT && toks[pos].start[0] == '(')) pos++;
    if (pos >= n) { snprintf(err, cap, "expected '('"); return -1; }
    pos++;
    if (pos >= n || toks[pos].kind != BP_TOK_IDENT) { snprintf(err, cap, "expected param name"); return -1; }
    static char pname_pool[64][64];
    static int pname_i = 0;
    char *pname = pname_pool[pname_i++ % 64];
    size_t pl = toks[pos].len < 63 ? toks[pos].len : 63;
    memcpy(pname, toks[pos].start, pl); pname[pl] = '\0'; pos++;
    if (pos >= n || toks[pos].kind != BP_TOK_PUNCT || toks[pos].start[0] != ':') { snprintf(err, cap, "expected ':'"); return -1; }
    pos++;
    Ty *pty = NULL;
    if (toks[pos].kind == BP_TOK_IDENT) {
        char tn[32]; size_t tl = toks[pos].len < 31 ? toks[pos].len : 31;
        memcpy(tn, toks[pos].start, tl); tn[tl] = '\0';
        pty = typereg_get(reg, tn);
        if (!pty) {
            if (strcmp(tn, "i64") == 0) pty = qtt_i64();
            else if (strcmp(tn, "bool") == 0) pty = qtt_bool();
            else if (strcmp(tn, "str") == 0) pty = qtt_str();
        }
    }
    if (!pty) { snprintf(err, cap, "unknown param type"); return -1; }
    pos++;
    while (pos < n && !(toks[pos].kind == BP_TOK_PUNCT && toks[pos].start[0] == ')')) pos++;
    if (pos >= n) { snprintf(err, cap, "expected ')'"); return -1; }
    pos++;
    while (pos < n && !(toks[pos].kind == BP_TOK_PUNCT && toks[pos].start[0] == '-')) pos++;
    if (pos + 1 >= n || toks[pos + 1].kind != BP_TOK_PUNCT || toks[pos + 1].start[0] != '>') { snprintf(err, cap, "expected '->'"); return -1; }
    pos += 2;
    Ty *rty = NULL;
    if (toks[pos].kind == BP_TOK_IDENT) {
        char tn[32]; size_t tl = toks[pos].len < 31 ? toks[pos].len : 31;
        memcpy(tn, toks[pos].start, tl); tn[tl] = '\0';
        rty = typereg_get(reg, tn);
        if (!rty) {
            if (strcmp(tn, "i64") == 0) rty = qtt_i64();
            else if (strcmp(tn, "bool") == 0) rty = qtt_bool();
            else if (strcmp(tn, "str") == 0) rty = qtt_str();
        }
    }
    if (!rty) { snprintf(err, cap, "unknown return type"); return -1; }
    pos++;
    while (pos < n && !(toks[pos].kind == BP_TOK_PUNCT && toks[pos].start[0] == '{')) pos++;
    if (pos >= n) { snprintf(err, cap, "expected '{'"); return -1; }
    int bstart = pos + 1, depth = 1; pos++;
    while (pos < n && depth > 0) {
        if (toks[pos].kind == BP_TOK_PUNCT) {
            if (toks[pos].start[0] == '{') depth++;
            else if (toks[pos].start[0] == '}') depth--;
        }
        pos++;
    }
    int bend = pos - 1;
    if (bend <= bstart) { snprintf(err, cap, "empty body"); return -1; }
    const char *bt = toks[bstart].start;
    size_t bl = (size_t)(toks[bend].start - bt);
    char *bs = malloc(bl + 1); memcpy(bs, bt, bl); bs[bl] = '\0';
    /* NOTE: do NOT reset the expr pool here — the caller parses multiple fns
     * into the same growing pool so their bodies stay distinct. */
    Term *body = NULL;
    if (expr_parse(bs, &body, err, cap) != 0) { free(bs); return -1; }
    free(bs);
    static Term lam_pool[64];
    static int lam_i = 0;
    Term *lam = &lam_pool[lam_i++ % 64];
    memset(lam, 0, sizeof *lam);
    lam->kind = TERM_LAM; lam->name = pname; lam->q = Q_MANY; lam->ty = pty; lam->a = body;
    *out = lam;
    static Ty pi_pool[64];
    static int pi_i = 0;
    Ty *pi = &pi_pool[pi_i++ % 64];
    memset(pi, 0, sizeof *pi);
    pi->kind = TY_PI; pi->q = Q_MANY; pi->x = pname; pi->dom = pty; pi->cod = rty;
    *out_ty = pi;
    return 0;
}
