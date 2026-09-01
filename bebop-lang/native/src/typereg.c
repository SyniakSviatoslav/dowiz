/* Bebop type registry — implementation. */
#include "typereg.h"

#include <stdio.h>
#include <string.h>

void typereg_init(TyRegistry *r) {
    r->len = 0;
}

int typereg_put(TyRegistry *r, const char *name, Ty *ty) {
    if (r->len >= TYPEREG_MAX) {
        return -1;
    }
    r->entries[r->len].name = name;
    r->entries[r->len].ty = ty;
    r->len++;
    return 0;
}

Ty *typereg_get(TyRegistry *r, const char *name) {
    for (int i = 0; i < r->len; i++) {
        if (strcmp(r->entries[i].name, name) == 0) {
            return r->entries[i].ty;
        }
    }
    return NULL;
}

int typereg_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) {                                                          \
            pos += (size_t)r_;                                                 \
        }                                                                      \
        if (!c_) {                                                             \
            all_ok = 0;                                                        \
        }                                                                      \
    } while (0)

    TyRegistry r;
    typereg_init(&r);
    static Ty i64 = {.kind = TY_I64};
    static Ty boolt = {.kind = TY_BOOL};

    A(typereg_put(&r, "Point", &i64) == 0, "put");
    A(typereg_get(&r, "Point") == &i64, "get existing");
    A(typereg_get(&r, "Missing") == NULL, "get missing -> NULL");
    A(typereg_put(&r, "Flag", &boolt) == 0, "put second");
    A(typereg_get(&r, "Flag") == &boolt, "get second");

    return all_ok ? 0 : -1;
}
