/* Bebop lexer — implementation. Zero dependencies (libc only). */
#include "lexer.h"

static int is_ident_start(unsigned char c) {
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || c == '_';
}

static int is_ident_cont(unsigned char c) {
    return is_ident_start(c) || (c >= '0' && c <= '9');
}

static int is_digit(unsigned char c) {
    return c >= '0' && c <= '9';
}

/* UTF-8 sequence length for a lead byte (1 if not a lead byte). */
static int utf8_len(unsigned char c) {
    if (c < 0x80) {
        return 1;
    }
    if ((c & 0xE0) == 0xC0) {
        return 2;
    }
    if ((c & 0xF0) == 0xE0) {
        return 3;
    }
    if ((c & 0xF8) == 0xF0) {
        return 4;
    }
    return 1;
}

int bp_lex(const char *src, BpToken *out, size_t cap) {
    size_t n = 0;
    const char *p = src;
    unsigned line = 1;

    while (*p) {
        unsigned char c = (unsigned char)*p;

        if (c == '\n') {
            line++;
            p++;
            continue;
        }
        if (c == ' ' || c == '\t' || c == '\r') {
            p++;
            continue;
        }
        if (c == '/' && p[1] == '/') {
            while (*p && *p != '\n') {
                p++;
            }
            continue;
        }
        if (is_ident_start(c)) {
            const char *s = p;
            while (is_ident_cont((unsigned char)*p)) {
                p++;
            }
            if (n >= cap) {
                return -1;
            }
            out[n].kind = BP_TOK_IDENT;
            out[n].start = s;
            out[n].len = (size_t)(p - s);
            out[n].line = line;
            n++;
            continue;
        }
        if (is_digit(c)) {
            const char *s = p;
            while (is_digit((unsigned char)*p)) {
                p++;
            }
            if (n >= cap) {
                return -1;
            }
            out[n].kind = BP_TOK_NUMBER;
            out[n].start = s;
            out[n].len = (size_t)(p - s);
            out[n].line = line;
            n++;
            continue;
        }
        if (c >= 0x80) {
            int l = utf8_len(c);
            if (n >= cap) {
                return -1;
            }
            out[n].kind = BP_TOK_GLYPH;
            out[n].start = p;
            out[n].len = (size_t)l;
            out[n].line = line;
            n++;
            p += l;
            continue;
        }
        /* ASCII punctuation / operator */
        if (n >= cap) {
            return -1;
        }
        out[n].kind = BP_TOK_PUNCT;
        out[n].start = p;
        out[n].len = 1;
        out[n].line = line;
        n++;
        p++;
    }

    if (n >= cap) {
        return -1;
    }
    out[n].kind = BP_TOK_EOF;
    out[n].start = p;
    out[n].len = 0;
    out[n].line = line;
    n++;
    return (int)n;
}
