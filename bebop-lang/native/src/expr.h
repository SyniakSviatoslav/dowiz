/* Bebop expression parser — text → core Term (bridge front-end ↔ typechecker).
 * Handles let/if/binary-ops/unary/literals/idents/parens. No lambdas yet. */
#ifndef BEBOP_EXPR_H
#define BEBOP_EXPR_H

#include "qtt.h"
#include "typereg.h"

/* Parse an expression string into a core Term (from a static pool). Returns 0
 * on success (*term filled), -1 on error (err filled). */
int expr_parse(const char *s, Term **term, char *err, size_t cap);

/* Reset the static term pool (call before a fresh parse batch). */
void expr_pool_reset(void);

/* Set the type registry used for struct construction / field access. */
void expr_set_registry(TyRegistry *reg);

#endif /* BEBOP_EXPR_H */
