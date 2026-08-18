/* Bebop living_memory — implementation. */
#include "lmem.h"
#include <string.h>
#include <stdio.h>

void lmem_init(LmGraph *g) { memset(g, 0, sizeof *g); }

int lmem_upsert(LmGraph *g, const char *name, const uint64_t *vec) {
    /* Find existing */
    int idx = lmem_find(g, name);
    if (idx >= 0) {
        memcpy(g->syms[idx].vec, vec, 16 * sizeof(uint64_t));
        return idx;
    }
    if (g->n_syms >= LMEM_MAX_SYMBOLS) return -1;
    idx = g->n_syms++;
    size_t nlen = strlen(name);
    if (nlen >= LMEM_NAME_MAX) nlen = LMEM_NAME_MAX - 1;
    memcpy(g->syms[idx].name, name, nlen);
    g->syms[idx].name[nlen] = 0;
    memcpy(g->syms[idx].vec, vec, 16 * sizeof(uint64_t));
    return idx;
}

int lmem_find(const LmGraph *g, const char *name) {
    for (int i = 0; i < g->n_syms; i++)
        if (strcmp(g->syms[i].name, name) == 0) return i;
    return -1;
}

int lmem_hamming_dist(const uint64_t *a, const uint64_t *b, int n_words) {
    int d = 0;
    for (int i = 0; i < n_words; i++) {
        uint64_t x = a[i] ^ b[i];
        while (x) { d++; x &= x - 1; }
    }
    return d;
}

int lmem_search(const LmGraph *g, const uint64_t *query, int k, int *out) {
    typedef struct { int idx; int dist; } Pair;
    Pair best[16];
    int nbest = 0;
    for (int i = 0; i < g->n_syms; i++) {
        int d = lmem_hamming_dist(query, g->syms[i].vec, 16);
        if (nbest < k) {
            best[nbest].idx = i; best[nbest].dist = d; nbest++;
        } else {
            int worst = 0;
            for (int j = 1; j < nbest; j++) if (best[j].dist > best[worst].dist) worst = j;
            if (d < best[worst].dist) { best[worst].idx = i; best[worst].dist = d; }
        }
    }
    for (int i = 0; i < nbest; i++) out[i] = best[i].idx;
    return nbest;
}

void lmem_link(LmGraph *g, int a, int b) {
    if (a >= g->n_syms || b >= g->n_syms) return;
    LmSymbol *sa = &g->syms[a], *sb = &g->syms[b];
    int has = 0;
    for (int i = 0; i < sa->n_edges; i++) if (sa->edges[i] == b) has = 1;
    if (!has && sa->n_edges < 8) sa->edges[sa->n_edges++] = b;
    has = 0;
    for (int i = 0; i < sb->n_edges; i++) if (sb->edges[i] == a) has = 1;
    if (!has && sb->n_edges < 8) sb->edges[sb->n_edges++] = a;
}

int lmem_neighbors(const LmGraph *g, int idx, int *out, int max) {
    if (idx >= g->n_syms) return 0;
    int n = g->syms[idx].n_edges;
    if (n > max) n = max;
    for (int i = 0; i < n; i++) out[i] = g->syms[idx].edges[i];
    return n;
}

int lmem_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    LmGraph g; lmem_init(&g);
    uint64_t va[16] = {1,0}, vb[16] = {3,0}, vc[16] = {0xFFFF,0};
    lmem_upsert(&g, "alpha", va);
    lmem_upsert(&g, "beta",  vb);
    lmem_upsert(&g, "gamma", vc);
    T(g.n_syms == 3, "3 symbols inserted");

    lmem_link(&g, 0, 1);
    int nb[8];
    T(lmem_neighbors(&g, 0, nb, 8) == 1, "alpha has 1 neighbor");
    T(nb[0] == 1, "neighbor is beta");

    uint64_t q[16] = {2,0};
    int results[3];
    int nr = lmem_search(&g, q, 2, results);
    T(nr == 2, "search returns 2 results");
    T(results[0] == 0 || results[0] == 1, "closest to query is alpha or beta");

    T(lmem_find(&g, "gamma") == 2, "find gamma at index 2");
    T(lmem_find(&g, "delta") == -1, "delta not found");

#undef T
    return fail;
}