/* Bebop type reflection (14C / #13) — compile-time type introspection.
 *
 * Dependent type-reflection: the elaborator can ask a type for its size and
 * alignment, so dimension-manipulating code (packed layouts, SIMD width, cache
 * geometry) is computed statically from the type itself — no hard-coded
 * constants, no runtime RTTI.
 */
#ifndef BEBOP_TYPEREFLECT_H
#define BEBOP_TYPEREFLECT_H

#include <stddef.h>

#include "qtt.h"

/* Size of a type in bytes (compile-time). */
size_t type_size(const Ty *t);
/* Alignment of a type in bytes (compile-time). */
size_t type_align(const Ty *t);

int typereflect_self_test(char *out, size_t cap);

#endif /* BEBOP_TYPEREFLECT_H */
