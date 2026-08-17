/* Bebop type registry — maps a declared type name to its Ty (struct/enum).
 * Gap 2 (elaboration) slice 1: lets .bp source declare struct/enum types that
 * the typechecker resolves by name. */
#ifndef BEBOP_TYPEREG_H
#define BEBOP_TYPEREG_H

#include "qtt.h"

#define TYPEREG_MAX 64

typedef struct {
    const char *name;
    Ty *ty;
} TyRegEntry;

typedef struct {
    TyRegEntry entries[TYPEREG_MAX];
    int len;
} TyRegistry;

void typereg_init(TyRegistry *r);
/* register a type (returns 0, or -1 if full). name is borrowed. */
int typereg_put(TyRegistry *r, const char *name, Ty *ty);
/* lookup by name, returns NULL if absent */
Ty *typereg_get(TyRegistry *r, const char *name);

int typereg_self_test(char *out, size_t cap);

#endif /* BEBOP_TYPEREG_H */
