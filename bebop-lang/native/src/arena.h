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

/* ─── Transactional snapshot (agentic #1) ───
 * take: record the current offset. restore: roll the offset back (O(1)), so
 * everything allocated after the snapshot is freed — nanosecond rollback. */
typedef struct {
    BumpArena *arena;
    size_t offset;
} ArenaSnapshot;

ArenaSnapshot arena_snapshot_take(BumpArena *a);
void arena_snapshot_restore(ArenaSnapshot s);

int arena_self_test(char *out, size_t cap);

#endif /* BEBOP_ARENA_H */
