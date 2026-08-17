/* Bebop Arena — deterministic bump allocator (port of dowiz arena.rs).
 * Foundation of the transactional-snapshot primitive (agentic #1): allocate a
 * pass's scratch in one contiguous region, free it all with an O(1) reset.
 * Degrade-closed: exhaustion returns NULL, never grows, never panics. */
#ifndef BEBOP_ARENA_H
#define BEBOP_ARENA_H

#include <stddef.h>

typedef struct {
    unsigned char *buf;
    size_t cap;
    size_t offset;
    size_t peak;
} BumpArena;

void arena_init(BumpArena *a, unsigned char *buf, size_t cap);
void arena_reset(BumpArena *a); /* O(1): no frees, no walk */

/* Bump-allocate n bytes (max_align_t-aligned). NULL on exhaustion/overflow. */
void *arena_alloc(BumpArena *a, size_t n);
void *arena_alloc_zero(BumpArena *a, size_t n);

size_t arena_used(const BumpArena *a);
size_t arena_peak(const BumpArena *a);

int arena_self_test(char *out, size_t cap);

#endif /* BEBOP_ARENA_H */
