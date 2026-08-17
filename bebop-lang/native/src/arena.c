/* Bebop Arena — implementation (port of dowiz arena.rs). */
#include "arena.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

void arena_init(BumpArena *a, unsigned char *buf, size_t cap) {
    a->buf = buf;
    a->cap = cap;
    a->offset = 0;
    a->peak = 0;
}

void arena_reset(BumpArena *a) {
    a->offset = 0; /* O(1) — the "rollback" of a transactional snapshot */
}

static size_t align_up(size_t n, size_t align) {
    return (n + align - 1) & ~(align - 1);
}

void *arena_alloc(BumpArena *a, size_t n) {
    size_t align = _Alignof(max_align_t);
    size_t off = align_up(a->offset, align);
    if (off + n < off || off + n > a->cap) {
        return NULL; /* degrade-closed: exhaustion / overflow */
    }
    void *p = a->buf + off;
    a->offset = off + n;
    if (a->offset > a->peak) {
        a->peak = a->offset;
    }
    return p;
}

void *arena_alloc_zero(BumpArena *a, size_t n) {
    void *p = arena_alloc(a, n);
    if (p) {
        memset(p, 0, n);
    }
    return p;
}

size_t arena_used(const BumpArena *a) {
    return a->offset;
}
size_t arena_peak(const BumpArena *a) {
    return a->peak;
}

ArenaSnapshot arena_snapshot_take(BumpArena *a) {
    ArenaSnapshot s;
    s.arena = a;
    s.offset = a->offset;
    return s;
}

void arena_snapshot_restore(ArenaSnapshot s) {
    s.arena->offset = s.offset; /* O(1) rollback */
}

void cowlog_init(CowLog *l) {
    l->len = 0;
}

int cowlog_append(CowLog *l, const BumpArena *a) {
    if (l->len >= (int)(sizeof l->offsets / sizeof l->offsets[0])) {
        return -1;
    }
    l->offsets[l->len++] = a->offset;
    return 0;
}

int cowlog_replay(const CowLog *l, int index, BumpArena *a) {
    if (index < 0 || index >= l->len) {
        return -1;
    }
    a->offset = l->offsets[index]; /* O(1) jump to the recorded state */
    return 0;
}

int arena_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    unsigned char buf[4096];
    BumpArena a;
    arena_init(&a, buf, sizeof buf);

    long *p = arena_alloc(&a, sizeof(long));
    A(p != NULL, "alloc returns pointer");
    *p = 42;
    A(*p == 42, "read-back 42");

    void *q = arena_alloc(&a, 100);
    A((unsigned char *)q > (unsigned char *)p, "bump pointer advances");

    arena_reset(&a);
    A(arena_used(&a) == 0, "reset -> offset 0");
    void *p2 = arena_alloc(&a, sizeof(long));
    A(p2 == (void *)buf, "reset reuses same region");

    /* exhaustion is degrade-closed (never panics, never grows) */
    BumpArena small;
    unsigned char sbuf[64];
    arena_init(&small, sbuf, sizeof sbuf);
    int exhausted = 0;
    for (int i = 0; i < 100; i++) {
        if (arena_alloc(&small, 16) == NULL) {
            exhausted = 1;
            break;
        }
    }
    A(exhausted, "exhaustion -> NULL (degrade-closed)");
    A(arena_peak(&small) == 64, "peak == 64 (4 x 16-aligned)");

    /* transactional snapshot: rollback frees everything after the snapshot */
    {
        BumpArena t;
        unsigned char tbuf[4096];
        arena_init(&t, tbuf, sizeof tbuf);
        arena_alloc(&t, 32);
        ArenaSnapshot snap = arena_snapshot_take(&t);
        arena_alloc(&t, 32);
        arena_alloc(&t, 32);
        A(arena_used(&t) == 96, "snapshot: allocs after snapshot advance offset");
        arena_snapshot_restore(snap);
        A(arena_used(&t) == 32, "snapshot: rollback returns to snapshot offset");
        void *r2 = arena_alloc(&t, 32);
        A((unsigned char *)r2 == tbuf + 32, "snapshot: rollback reuses freed region");
    }

    /* append-only CoW log (24B): record a history, replay to any state */
    {
        BumpArena t;
        unsigned char tbuf[4096];
        arena_init(&t, tbuf, sizeof tbuf);
        CowLog log;
        cowlog_init(&log);
        cowlog_append(&log, &t); /* state 0: offset 0 */
        arena_alloc(&t, 32);
        cowlog_append(&log, &t); /* state 1: offset 32 */
        arena_alloc(&t, 32);
        cowlog_append(&log, &t); /* state 2: offset 64 */
        A(log.len == 3, "cowlog records 3 snapshots");
        A(cowlog_replay(&log, 1, &t) == 0 && arena_used(&t) == 32,
          "cowlog replay to state 1 == 32");
        A(cowlog_replay(&log, 0, &t) == 0 && arena_used(&t) == 0,
          "cowlog replay to state 0 == 0");
        A(cowlog_replay(&log, 2, &t) == 0 && arena_used(&t) == 64,
          "cowlog replay forward to state 2 == 64");
        A(cowlog_replay(&log, 3, &t) != 0, "cowlog out-of-range replay rejected");
        /* immutability: replaying never mutates the recorded offsets */
        A(log.offsets[0] == 0 && log.offsets[1] == 32 && log.offsets[2] == 64,
          "cowlog history is immutable across replays");
    }

    return all_ok ? 0 : -1;
}
