/* Bebop effect registry — extern pure/io declarations (22B / #15).
 *
 * Function signatures carry an effect annotation: `extern pure fn …` (no side
 * effects, freely CSE-able / reorderable) vs `extern io fn …` (side-effecting,
 * ordered, never elided). The analysis layers explicit TERM_IO with calls to
 * extern-io functions, so a pure function that calls an io function is itself
 * io — effect is tracked transitively through call sites.
 */
#ifndef BEBOP_EFFECT_H
#define BEBOP_EFFECT_H

#include <stddef.h>

#include "qtt.h"

typedef enum { EFF_PURE = 0, EFF_IO = 1 } Effect;

void effect_init(void);
/* Declare an extern function's effect (name borrowed). 0 on success. */
int effect_declare(const char *name, Effect e);
/* 1 if `name` is a registered extern-io function, else 0. */
int effect_is_io(const char *name);

/* Full effect analysis: explicit TERM_IO, or a call to an extern-io function
 * (a TERM_APP whose callee names a registered io fn), transitively. */
int effect_has_io(const Term *t);

/* Parse + register an extern decl: "extern pure fn name(...) -> ..." or
 * "extern io fn name(...)". Returns 0 on success. */
int effect_parse_decl(const char *decl, char *err, size_t cap);

int effect_self_test(char *out, size_t cap);

#endif /* BEBOP_EFFECT_H */
