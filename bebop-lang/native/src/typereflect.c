/* Bebop type reflection — implementation. */
#include "typereflect.h"

#include <stdio.h>

size_t type_size(const Ty *t) {
    if (!t) return 0;
    switch (t->kind) {
        case TY_U8:
        case TY_BOOL:
            return 1;
        case TY_U32:
            return 4;
        case TY_I64:
        case TY_U64:
        case TY_F64:
            return 8;
        case TY_VOID:
            return 0;
        case TY_FIELD:
            return 8; /* finite-field element (u64) */
        case TY_HYPERVEC:
            return (size_t)t->n / 8; /* D bits → bytes */
        case TY_VEC:
            return (size_t)t->n * type_size(t->elem);
        case TY_STRUCT: {
            size_t total = 0;
            for (int i = 0; i < t->nfields; i++) {
                total += type_size(t->fields[i].ty);
            }
            return total;
        }
        case TY_ENUM: {
            /* tagged union: u8 tag + max payload size */
            size_t maxp = 0;
            for (int i = 0; i < t->nctors; i++) {
                if (t->ctors[i].payload) {
                    size_t s = type_size(t->ctors[i].payload);
                    if (s > maxp) maxp = s;
                }
            }
            return 1 + maxp;
        }
        case TY_FN:
        case TY_PI:
            return 8; /* function pointer */
        case TY_NAT:
            return 8; /* machine word (unary Peano is erased to u64) */
        case TY_STR:
            return 8; /* borrowed pointer */
        case TY_EQ:
            return 0; /* proof — erased at runtime (QTT 0) */
        case TY_TYPE:
        case TY_VAR:
            return 0;
    }
    return 0;
}

size_t type_align(const Ty *t) {
    if (!t) return 1;
    switch (t->kind) {
        case TY_U8:
        case TY_BOOL:
            return 1;
        case TY_U32:
            return 4;
        case TY_I64:
        case TY_U64:
        case TY_F64:
        case TY_FIELD:
        case TY_FN:
        case TY_PI:
        case TY_NAT:
        case TY_STR:
            return 8;
        case TY_HYPERVEC:
            return 64; /* cache-line aligned (dowiz best practice) */
        case TY_VEC:
            return type_align(t->elem);
        case TY_STRUCT: {
            size_t a = 1;
            for (int i = 0; i < t->nfields; i++) {
                size_t fa = type_align(t->fields[i].ty);
                if (fa > a) a = fa;
            }
            return a;
        }
        case TY_ENUM: {
            size_t a = 1;
            for (int i = 0; i < t->nctors; i++) {
                if (t->ctors[i].payload) {
                    size_t pa = type_align(t->ctors[i].payload);
                    if (pa > a) a = pa;
                }
            }
            return a;
        }
        case TY_VOID:
        case TY_EQ:
        case TY_TYPE:
        case TY_VAR:
            return 1;
    }
    return 1;
}

int typereflect_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    static Ty i64 = {.kind = TY_I64};
    static Ty u8 = {.kind = TY_U8};
    static Ty u32 = {.kind = TY_U32};
    static Ty f64 = {.kind = TY_F64};
    static Ty hv = {.kind = TY_HYPERVEC, .n = 1024};
    static Ty nat = {.kind = TY_NAT};

    A(type_size(&i64) == 8, "sizeof(i64) == 8");
    A(type_size(&u8) == 1, "sizeof(u8) == 1");
    A(type_size(&u32) == 4, "sizeof(u32) == 4");
    A(type_size(&f64) == 8, "sizeof(f64) == 8");
    A(type_size(&hv) == 128, "sizeof(Hypervector<1024>) == 128 bytes");
    A(type_align(&hv) == 64, "alignof(Hypervector<1024>) == 64 (cache line)");
    A(type_size(&nat) == 8, "sizeof(Nat) == 8 (erased to machine word)");

    /* struct: sum of field sizes */
    static TyField f2[2] = {{"x", &i64}, {"y", &u32}};
    static Ty pt = {.kind = TY_STRUCT, .fields = f2, .nfields = 2};
    A(type_size(&pt) == 12, "sizeof(struct{x:i64, y:u32}) == 12 (packed)");
    A(type_align(&pt) == 8, "alignof(struct{x:i64,...}) == 8");

    /* enum: u8 tag + max payload */
    static Ctor cs[2] = {{"None", NULL}, {"Some", &i64}};
    static Ty opt = {.kind = TY_ENUM, .ctors = cs, .nctors = 2};
    A(type_size(&opt) == 9, "sizeof(enum{None, Some(i64)}) == 9 (tag+payload)");

    /* proof types are erased (QTT 0) */
    static Ty eq = {.kind = TY_EQ};
    A(type_size(&eq) == 0, "sizeof(equality proof) == 0 (erased)");

    return all_ok ? 0 : -1;
}
