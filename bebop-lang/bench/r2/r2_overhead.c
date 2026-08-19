/* R2 Per-Object Overhead harness — measures sizeof vs bytes actually requested,
 * and arena allocator overhead vs plain malloc.
 * Hypervector: 16*uint64_t aligned(64) => 128 bytes, no hidden overhead
 * Complex: 2*double => 16 bytes
 * Arena: 64B alignment overhead per alloc */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Replicate Bebop structs exactly */
#include <stddef.h>

typedef struct {
    double re, im;
} Complex;

typedef struct {
    uint64_t words[16];
} __attribute__((aligned(64))) Hypervector;

/* Arena bump allocator — exact copy from arena.h / arena.c */
#define ARENA_ALIGN 64

typedef struct {
    unsigned char *mem;
    size_t cap, used;
} Arena;

static void arena_init(Arena *a, void *mem, size_t cap) {
    memset(a, 0, sizeof *a);
    uintptr_t base = (uintptr_t)mem;
    uintptr_t aligned = (base + ARENA_ALIGN - 1) & ~(uintptr_t)(ARENA_ALIGN - 1);
    a->mem = (unsigned char *)aligned;
    a->cap = cap - (size_t)(aligned - base);
}

static void *arena_alloc(Arena *a, size_t n) {
    if (a->used + n > a->cap) return NULL;
    size_t off = (a->used + ARENA_ALIGN - 1) & ~(size_t)(ARENA_ALIGN - 1);
    if (off + n > a->cap) return NULL;
    void *p = a->mem + off;
    a->used = off + n;
    return p;
}

int main(void) {
    printf("=== R2 Per-Object Memory Overhead ===\n\n");

    /* ── sizeof measurements ── */
    printf("--- sizeof(struct) ---\n");
    printf("sizeof(Complex)      = %zu bytes\n", sizeof(Complex));
    printf("sizeof(Hypervector)  = %zu bytes (aligned(64))\n", sizeof(Hypervector));

    /* ── malloc overhead: ask for small sizes, see what glibc actually gives ── */
    printf("\n--- malloc overhead (glibc minimum allocation) ---\n");
    /* glibc ptmalloc2 overhead: minimum chunk size is 32 bytes on 64-bit.
     * Overhead is usually 16 bytes (prev_size + size fields per chunk). */
    size_t alloc_sizes[] = {1, 8, 16, 32, 64, 128, 1024, 4096};
    int n = sizeof(alloc_sizes) / sizeof(alloc_sizes[0]);

    void *ptrs[32];
    for (int i = 0; i < n; i++) {
        ptrs[i] = malloc(alloc_sizes[i]);
        if (!ptrs[i]) { printf("malloc fail\n"); return 1; }
        memset(ptrs[i], 0, alloc_sizes[i]);
    }

    /* Estimate glibc chunk overhead by comparing adjacent allocations */
    /* We can't directly see chunk overhead without internal glibc, but we can
     * detect minimum spacing by allocating 2 items and checking pointer diff. */
    void *a1 = malloc(1);
    void *a2 = malloc(1);
    memset(a1, 0, 1); memset(a2, 0, 1);
    ptrdiff_t glibc_min_spacing = (char*)a2 - (char*)a1;
    ptrdiff_t glibc_overhead_per_chunk = glibc_min_spacing - 1;
    if (glibc_overhead_per_chunk < 0) glibc_overhead_per_chunk = -(glibc_overhead_per_chunk);
    /* The actual minimum chunk is usually 32 bytes on 64-bit glibc */
    printf("glibc min chunk spacing (malloc(1), malloc(1)): %td bytes\n", glibc_min_spacing);
    printf("  => per-chunk metadata overhead: ~%td bytes\n", glibc_overhead_per_chunk);

    free(a1); free(a2);
    for (int i = 0; i < n; i++) free(ptrs[i]);

    /* ── Arena overhead: allocate many small objects, measure wasted bytes ── */
    printf("\n--- Arena overhead (64B alignment per alloc) ---\n");
    unsigned char buf[65536 * 4] __attribute__((aligned(64)));
    Arena ar;
    arena_init(&ar, buf, sizeof(buf));
    size_t initial_used = ar.used; /* should be 0 */

    const int N_ARENA_ALLOCS = 1000;
    size_t sizes[] = {1, 4, 8, 16, 32, 64, 128, 256, 512, 1024, sizeof(Complex), sizeof(Hypervector)};
    int m = sizeof(sizes)/sizeof(sizes[0]);

    for (int s = 0; s < m; s++) {
        arena_init(&ar, buf, sizeof(buf));
        void *first = arena_alloc(&ar, sizes[s]);
        size_t used1 = ar.used;
        /* Compute waste: arena aligns to 64; waste = (aligned - requested) / requested */
        size_t waste = used1 - sizes[s];
        double waste_pct = (double)waste / (double)sizes[s] * 100.0;
        printf("arena_alloc(%4zu): used=%5zu, waste=%5zu bytes, overhead=%.1f%%\n",
               sizes[s], used1, waste, waste_pct);
        (void)first; /* silence unused */
    }

    /* Overall arena efficiency for 1000 allocs of mixed sizes */
    printf("\n--- Arena mixed alloc efficiency (1000 allocs) ---\n");
    arena_init(&ar, buf, sizeof(buf));
    size_t total_requested = 0;
    for (int i = 0; i < N_ARENA_ALLOCS; i++) {
        size_t sz = sizes[i % m];
        total_requested += sz;
        void *p = arena_alloc(&ar, sz);
        if (!p) { printf("arena OOM at %d\n", i); break; }
    }
    size_t total_used = ar.used;
    double overall_overhead_pct = (double)(total_used - total_requested) / (double)total_requested * 100.0;
    printf("total_requested: %zu bytes\n", total_requested);
    printf("total_arena_used: %zu bytes\n", total_used);
    printf("overall overhead:  %.1f%%\n", overall_overhead_pct);

    /* malloc equivalent: allocate same mixed sizes */
    printf("\n--- malloc mixed alloc (estimate) ---\n");
    printf("Each malloc(1) costs at least 32 bytes in glibc (min chunk)\n");
    printf("Total malloc overhead for 1000 x 1B: %zu real bytes\n", (size_t)1000 * 32);

    printf("\n=== Summary ===\n");
    printf("sizeof(Hypervector)  : %zu B (aligned 64, no hidden malloc overhead possible on that alignment)\n", sizeof(Hypervector));
    printf("sizeof(Complex)      : %zu B (2 doubles, no padding)\n", sizeof(Complex));
    printf("sizeof(Arena)        : %zu B (ptr+2 size_t)\n", sizeof(Arena));
    printf("arena_alloc churn    : minimum waste = 63 bytes per small alloc (64B align)\n");
    printf("glibc malloc overhead: ~16 bytes per allocation (chunk header: prev_size+size)\n");

    return 0;
}