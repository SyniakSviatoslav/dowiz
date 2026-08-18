/* Bebop living_memory — vector-based persistent knowledge store (port of dowiz).
 * Symbol graph with semantic search via hypervector similarity. */
#ifndef BEBOP_LMEM_H
#define BEBOP_LMEM_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#define LMEM_MAX_SYMBOLS 512
#define LMEM_NAME_MAX    64

/* One symbol: name + vector + edges to other symbols. */
typedef struct {
    char     name[LMEM_NAME_MAX];
    uint64_t vec[16];      /* hypervector (1024-bit, 16 x u64) */
    int      edges[8];     /* indices of related symbols */
    int      n_edges;
} LmSymbol;

typedef struct {
    LmSymbol syms[LMEM_MAX_SYMBOLS];
    int      n_syms;
} LmGraph;

/* Init and basic ops. */
void lmem_init(LmGraph *g);

/* Add/update a symbol with its hypervector. Returns index or -1. */
int  lmem_upsert(LmGraph *g, const char *name, const uint64_t *vec);

/* Find symbol by name. Returns index or -1. */
int  lmem_find(const LmGraph *g, const char *name);

/* Hamming distance between two hypervectors. */
int  lmem_hamming_dist(const uint64_t *a, const uint64_t *b, int n_words);

/* Semantic search: find top-k closest symbols to query vector.
 * Returns number of results filled in out_indices. */
int  lmem_search(const LmGraph *g, const uint64_t *query, int k, int *out_idx);

/* Add a bidirectional edge between two symbols. */
void lmem_link(LmGraph *g, int a_idx, int b_idx);

/* Traverse outgoing edges from a symbol, returning neighbors. */
int  lmem_neighbors(const LmGraph *g, int idx, int *out, int max);

/* ─── self-test ─────────────────────────────────────────────────────────── */
int  lmem_self_test(char *out, size_t cap);

#endif