/* Bebop lexer — tokenizes a .bp source into glyph/ident/number/punct tokens. */
#ifndef BEBOP_LEXER_H
#define BEBOP_LEXER_H

#include <stddef.h>

typedef enum {
    BP_TOK_EOF,
    BP_TOK_IDENT, /* ASCII fallback identifier (fn, struct, …) */
    BP_TOK_GLYPH, /* glyph (non-ASCII UTF-8 sequence) */
    BP_TOK_NUMBER,
    BP_TOK_PUNCT, /* ASCII operator / delimiter */
} BpTokKind;

typedef struct {
    BpTokKind kind;
    const char *start; /* pointer into the source buffer */
    size_t len;        /* byte length */
    unsigned line;
} BpToken;

/* Tokenize `src` into `out` (max `cap` entries, including a trailing EOF).
 * Returns the number of tokens written (including EOF), or -1 on overflow. */
int bp_lex(const char *src, BpToken *out, size_t cap);

#endif /* BEBOP_LEXER_H */
