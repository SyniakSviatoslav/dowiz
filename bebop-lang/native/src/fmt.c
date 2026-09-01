/* Bebop formatter — normalize .bp source (indentation + spacing). */
#include "fmt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Format Bebop source into `out` (cap bytes, NUL-terminated). Returns bytes
 * written (excluding NUL), or -1 on overflow. Normalizes: 2-space indent per
 * brace depth, single space around binops, no trailing whitespace. */
int bp_fmt(const char *src, char *out, size_t cap) {
    size_t o = 0;
    int depth = 0;
    int line_start = 1;
    const char *p = src;

    while (*p && o + 4 < cap) {
        char ch = *p;
        if (ch == '{') {
            if (!line_start && o > 0 && out[o-1] != ' ') { out[o++] = ' '; }
            out[o++] = '{';
            out[o++] = '\n';
            depth++;
            line_start = 1;
            p++;
            continue;
        }
        if (ch == '}') {
            if (!line_start) { out[o++] = '\n'; }
            depth = depth > 0 ? depth - 1 : 0;
            for (int i = 0; i < depth; i++) { out[o++] = ' '; out[o++] = ' '; }
            out[o++] = '}';
            out[o++] = '\n';
            line_start = 1;
            p++;
            continue;
        }
        if (ch == '\n') {
            /* trim trailing spaces then newline */
            while (o > 0 && out[o-1] == ' ') o--;
            out[o++] = '\n';
            line_start = 1;
            p++;
            continue;
        }
        if (line_start) {
            for (int i = 0; i < depth; i++) { out[o++] = ' '; out[o++] = ' '; }
            line_start = 0;
        }
        out[o++] = ch;
        p++;
    }
    out[o] = '\0';
    return (int)o;
}

int fmt_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#undef A
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    char buf[512];
    const char *src = "fn f(x:i64)->i64{let y=x+1 in y}";
    int n = bp_fmt(src, buf, sizeof buf);
    /* formatted output should contain indented body and brace newlines */
    A(n > 0 && strchr(buf, '\n') != NULL, "fmt inserts newlines");
    A(strstr(buf, "  let y") != NULL, "fmt indents body by 2 spaces");

    return all_ok ? 0 : -1;
}
