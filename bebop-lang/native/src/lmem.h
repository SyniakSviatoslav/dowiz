/* Bebop living_memory v2 — persistent knowledge store for agents.
 * Symbol graph with semantic search via hypervector similarity.
 * Every symbol carries a kind, a note payload and a revision stamp,
 * so states can be compared across time ("living" part).
 * Persistence: binary file (docs/memory.lmem), committed to git. */
#ifndef BEBOP_LMEM_H
#define BEBOP_LMEM_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#define LMEM_MAX_SYMBOLS 512
#define LMEM_NAME_MAX    64
#define LMEM_NOTE_MAX    192
#define LMEM_VEC_WORDS   16     /* 1024-bit hypervectors */

/* Symbol kinds. */
enum {
    LEM_MODULE = 0,   /* code module / file */
    LEM_CONTRACT,     /* ABI / format / register contract */
    LEM_STATE,        /* snapshot of ongoing work */
    LEM_PAT_OK,       /* success pattern */
    LEM_PAT_BAD,      /* negative pattern / postmortem */
    LEM_RULE          /* distilled agent rule */
};

/* One symbol: name + vector + edges + payload. */
typedef struct {
    char     name[LMEM_NAME_MAX];
    uint64_t vec[LMEM_VEC_WORDS];
    int      edges[8];     /* indices of related symbols */
    int      n_edges;
    uint8_t  kind;
    uint64_t stamp;        /* unix seconds of last update */
    char     note[LMEM_NOTE_MAX];
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

/* Deterministic text -> hypervector (FNV-1a over 4-grams). No RNG. */
void lmem_vec_from_text(const char *text, uint64_t *vec);

/* Upsert with note/kind; vector derived from name+note. Returns index. */
int  lmem_remember(LmGraph *g, const char *name, int kind,
                   const char *note, uint64_t stamp);

/* Persistence: binary format magic "LMEM2". 0 = ok. */
int  lmem_save(const LmGraph *g, const char *path);
int  lmem_load(LmGraph *g, const char *path);

/* ─── self-test ─────────────────────────────────────────────────────────── */
int  lmem_self_test(char *out, size_t cap);

#endif
