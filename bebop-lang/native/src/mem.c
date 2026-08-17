/* Bebop Memory — implementation. */
#include "mem.h"

#include <stdio.h>
#include <string.h>

static uint64_t djb2(const char *s) {
    uint64_t h = 5381;
    while (*s) {
        h = ((h << 5) + h) + (unsigned char)*s++;
    }
    return h;
}

/* Encode text as a bundle of its character trigrams (VSA n-gram code). */
static Hypervector hv_encode_text(const char *text) {
    size_t len = strlen(text);
    Hypervector trigrams[256];
    size_t n = 0;
    for (size_t i = 0; i + 3 <= len && n < 256; i++) {
        char tri[4];
        memcpy(tri, text + i, 3);
        tri[3] = '\0';
        trigrams[n++] = hv_code(djb2(tri));
    }
    if (n == 0) {
        return hv_code(djb2(text));
    }
    return hv_bundle(trigrams, n);
}

void mem_init(Memory *m) {
    memset(m, 0, sizeof *m);
}

int mem_add(Memory *m, const char *name, const char *kind) {
    if (m->len >= MEM_MAX) {
        return -1;
    }
    MemRecord *r = &m->recs[m->len++];
    snprintf(r->name, sizeof r->name, "%s", name);
    snprintf(r->kind, sizeof r->kind, "%s", kind);
    r->code = hv_encode_text(name);
    return 0;
}

size_t mem_search_keyword(const Memory *m, const char *query, size_t *ids, size_t cap) {
    size_t n = 0;
    for (size_t i = 0; i < m->len && n < cap; i++) {
        if (strstr(m->recs[i].name, query)) {
            ids[n++] = i;
        }
    }
    return n;
}

size_t mem_search_semantic(const Memory *m, const char *query, size_t k, size_t *ids, size_t cap) {
    Hypervector q = hv_encode_text(query);
    double sims[MEM_MAX];
    for (size_t i = 0; i < m->len; i++) {
        sims[i] = hv_similarity(&q, &m->recs[i].code);
    }
    size_t n = 0;
    for (size_t round = 0; round < k && n < cap && round < m->len; round++) {
        size_t best = (size_t)-1;
        double best_sim = -1.0;
        for (size_t i = 0; i < m->len; i++) {
            if (sims[i] < 0.0) {
                continue;
            }
            if (sims[i] > best_sim) {
                best_sim = sims[i];
                best = i;
            }
        }
        if (best == (size_t)-1) {
            break;
        }
        ids[n++] = best;
        sims[best] = -1.0;
    }
    return n;
}

int mem_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define M(cond, name)                                                    \
    do {                                                                 \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",              \
                         (cond) ? "ok" : "FAIL", name);                  \
        if (r > 0) pos += (size_t)r;                                     \
        if (!(cond)) all_ok = 0;                                         \
    } while (0)

    Memory m;
    mem_init(&m);
    mem_add(&m, "quantum_time_shift", "fn");
    mem_add(&m, "quantum_measurement", "fn");
    mem_add(&m, "ntt_transform", "fn");
    mem_add(&m, "money_ledger", "struct");
    mem_add(&m, "hypervector_bind", "fn");

    M(m.len == 5, "5 records added");

    size_t ids[8];
    size_t n = mem_search_keyword(&m, "quantum", ids, 8);
    M(n == 2, "keyword 'quantum' finds 2");

    n = mem_search_semantic(&m, "quantum", 2, ids, 8);
    M(n == 2 && strstr(m.recs[ids[0]].name, "quantum") &&
      strstr(m.recs[ids[1]].name, "quantum"),
      "semantic 'quantum' ranks quantum_* top-2");

    n = mem_search_semantic(&m, "money", 1, ids, 8);
    M(n >= 1 && strcmp(m.recs[ids[0]].name, "money_ledger") == 0,
      "semantic 'money' → money_ledger");

    n = mem_search_semantic(&m, "transform", 1, ids, 8);
    M(n >= 1 && strcmp(m.recs[ids[0]].name, "ntt_transform") == 0,
      "semantic 'transform' → ntt_transform");

    return all_ok ? 0 : -1;
}
