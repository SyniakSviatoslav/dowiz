/* Bebop formatter. */
#ifndef BEBOP_FMT_H
#define BEBOP_FMT_H

#include <stddef.h>

/* Format Bebop source into `out`. Returns bytes written, or -1 on overflow. */
int bp_fmt(const char *src, char *out, size_t cap);

/* Run the formatter self-test. Returns 0 on success. */
int fmt_self_test(char *out, size_t cap);

#endif /* BEBOP_FMT_H */
