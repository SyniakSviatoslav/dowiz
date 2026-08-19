/* Bebop arena — deterministic memory management (bump allocator).
 * Includes: arena bump allocator, Vec (arena vector), Ring (lock-free SPSC queue). */
#ifndef BEBOP_ARENA_H
#define BEBOP_ARENA_H
#include <stddef.h>
#include <stdint.h>

/* ─── Arena bump allocator ───────────────────────────────────────── */
typedef struct { unsigned char *mem; size_t cap, used; } Arena;
void arena_init(Arena *a, void *mem, size_t cap);
void *arena_alloc(Arena *a, size_t n);  /* 64B aligned */
void arena_reset(Arena *a);

/* ─── Arena vector (T = user-sized element, typed via macro) ──────── */
typedef struct { void *data; size_t len, cap; Arena *arena; } Vec;
void  vec_init(Vec *v, Arena *a, size_t elem_sz, size_t init_cap);
void *vec_push(Vec *v, size_t elem_sz, const void *elem);
#define VEC_PUSH(v, x) vec_push(v, sizeof *(x), x)
#define VEC_GET(v, idx, T) (((T *)(v)->data)[idx])

/* ─── Lock-free SPSC ring buffer (ISR-safe) ──────────────────────── */
typedef struct {
    unsigned char *buf; size_t cap, head, tail;
} Ring;
void ring_init(Ring *r, Arena *a, size_t n, size_t elem_sz);
int  ring_enq(Ring *r, size_t elem_sz, const void *e); /* producer (ISR) */
int  ring_deq(Ring *r, size_t elem_sz, void *e);       /* consumer (main) */
int  ring_empty(const Ring *r);

int arena_self_test(char *out, size_t cap);
#endif
/* Compatibility alias for supervise.h */
typedef Arena BumpArena;
