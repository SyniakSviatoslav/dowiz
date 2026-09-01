/* Bebop Memory — living-memory primitive (agentic #4): records with trigram-
 * hypervector codes; keyword (substring) + semantic (hypervector similarity)
 * search. Builds on hyper.h. */
#ifndef BEBOP_MEM_H
#define BEBOP_MEM_H

#include <stddef.h>

#include "hyper.h"

#define MEM_MAX 256

typedef struct {
    char name[64];
    char kind[16];
    Hypervector code;
} MemRecord;

typedef struct {
    MemRecord recs[MEM_MAX];
    size_t len;
} Memory;

void mem_init(Memory *m);
int mem_add(Memory *m, const char *name, const char *kind);
size_t mem_search_keyword(const Memory *m, const char *query, size_t *ids, size_t cap);
size_t mem_search_semantic(const Memory *m, const char *query, size_t k, size_t *ids, size_t cap);

int mem_self_test(char *out, size_t cap);

#endif /* BEBOP_MEM_H */
