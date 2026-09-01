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

/* ─── v2: text→vector, remember, persistence ───────────────────────────── */

void lmem_vec_from_text(const char *text, uint64_t *vec) {
    /* FNV-1a over every 4-gram; byte i lands in word (i & 15) after a
     * rotate — spreads correlated texts across the 1024-bit space while
     * staying fully deterministic. */
    for (int w = 0; w < LMEM_VEC_WORDS; w++) vec[w] = 0;
    size_t n = strlen(text);
    uint64_t h = 14695981039346656037ULL;
    for (size_t i = 0; i + 4 <= n; i++) {
        for (int b = 0; b < 4; b++) {
            h ^= (unsigned char)text[i + b];
            h *= 1099511628211ULL;
        }
        int w = (int)(i % LMEM_VEC_WORDS);
        vec[w] ^= h;
        vec[w] = (vec[w] << 7) | (vec[w] >> 57); /* rotate to decorrelate */
    }
    if (n == 0) vec[0] = h;
}

int lmem_remember(LmGraph *g, const char *name, int kind,
                  const char *note, uint64_t stamp) {
    char buf[LMEM_NAME_MAX + LMEM_NOTE_MAX];
    snprintf(buf, sizeof buf, "%s|%s", name, note ? note : "");
    uint64_t vec[LMEM_VEC_WORDS];
    lmem_vec_from_text(buf, vec);
    int idx = lmem_upsert(g, name, vec);
    if (idx < 0) return -1;
    g->syms[idx].kind = (uint8_t)kind;
    g->syms[idx].stamp = stamp;
    if (note) {
        size_t nl = strlen(note);
        if (nl >= LMEM_NOTE_MAX) nl = LMEM_NOTE_MAX - 1;
        memcpy(g->syms[idx].note, note, nl);
        g->syms[idx].note[nl] = 0;
    }
    return idx;
}

#define LMEM_MAGIC 0x324D454CUL /* "LEML" little-endian: LMEM2 */

int lmem_save(const LmGraph *g, const char *path) {
    FILE *f = fopen(path, "wb");
    if (!f) return -1;
    uint32_t magic = LMEM_MAGIC, ver = 2, cnt = (uint32_t)g->n_syms;
    fwrite(&magic, 4, 1, f);
    fwrite(&ver, 4, 1, f);
    fwrite(&cnt, 4, 1, f);
    for (int i = 0; i < g->n_syms; i++) {
        const LmSymbol *s = &g->syms[i];
        fwrite(s->name, 1, LMEM_NAME_MAX, f);
        fwrite(s->vec, 8, LMEM_VEC_WORDS, f);
        fwrite(s->edges, sizeof(int), 8, f);
        fwrite(&s->n_edges, sizeof(int), 1, f);
        fwrite(&s->kind, 1, 1, f);
        fwrite(&s->stamp, 8, 1, f);
        fwrite(s->note, 1, LMEM_NOTE_MAX, f);
    }
    fclose(f);
    return 0;
}

int lmem_load(LmGraph *g, const char *path) {
    FILE *f = fopen(path, "rb");
    if (!f) return -1;
    uint32_t magic = 0, ver = 0, cnt = 0;
    if (fread(&magic, 4, 1, f) != 1 || magic != LMEM_MAGIC ||
        fread(&ver, 4, 1, f) != 1 || ver != 2 ||
        fread(&cnt, 4, 1, f) != 1 || cnt > LMEM_MAX_SYMBOLS) {
        fclose(f); return -1;
    }
    lmem_init(g);
    g->n_syms = (int)cnt;
    for (int i = 0; i < (int)cnt; i++) {
        LmSymbol *s = &g->syms[i];
        if (fread(s->name, 1, LMEM_NAME_MAX, f) != LMEM_NAME_MAX ||
            fread(s->vec, 8, LMEM_VEC_WORDS, f) != LMEM_VEC_WORDS ||
            fread(s->edges, sizeof(int), 8, f) != 8 ||
            fread(&s->n_edges, sizeof(int), 1, f) != 1 ||
            fread(&s->kind, 1, 1, f) != 1 ||
            fread(&s->stamp, 8, 1, f) != 1 ||
            fread(s->note, 1, LMEM_NOTE_MAX, f) != LMEM_NOTE_MAX) {
            fclose(f); lmem_init(g); return -1;
        }
        s->name[LMEM_NAME_MAX-1] = 0;
        s->note[LMEM_NOTE_MAX-1] = 0;
        if (s->n_edges < 0 || s->n_edges > 8) s->n_edges = 0;
    }
    fclose(f);
    return 0;
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

    /* v2: deterministic text vectors */
    uint64_t t1[16], t2[16], t3[16];
    lmem_vec_from_text("seed loader contract", t1);
    lmem_vec_from_text("seed loader contract", t2);
    lmem_vec_from_text("hydra PID control", t3);
    T(memcmp(t1, t2, sizeof t1) == 0, "vec_from_text deterministic");
    T(lmem_hamming_dist(t1, t3, 16) > 200, "unrelated texts far apart");

    /* v2: remember payload */
    LmGraph g2; lmem_init(&g2);
    int ri = lmem_remember(&g2, "state/m3", LEM_STATE,
                           "beboSelf entry=0 path works", 1700000000);
    T(ri == 0 && g2.n_syms == 1, "remember inserts one symbol");
    T(g2.syms[0].kind == LEM_STATE && g2.syms[0].stamp == 1700000000,
      "kind and stamp stored");
    T(strlen(g2.syms[0].note) > 0, "note stored");

    /* v2: persistence round-trip */
    const char *tmp_path = "/tmp/opencode/lmem_roundtrip.bin";
    T(lmem_save(&g2, tmp_path) == 0, "save ok");
    LmGraph g3; lmem_init(&g3);
    T(lmem_load(&g3, tmp_path) == 0, "load ok");
    T(g3.n_syms == 1 && strcmp(g3.syms[0].name, "state/m3") == 0,
      "roundtrip name");
    T(memcmp(g3.syms[0].vec, g2.syms[0].vec, sizeof t1) == 0,
      "roundtrip vector");
    T(strcmp(g3.syms[0].note, "beboSelf entry=0 path works") == 0,
      "roundtrip note");
    T(g3.syms[0].kind == LEM_STATE && g3.syms[0].stamp == 1700000000,
      "roundtrip kind+stamp");

#undef T
    return fail;
}