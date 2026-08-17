/* Bebop VSA identifiers — implementation. */
#include "vsa.h"

#include <stdio.h>
#include <string.h>

Hypervector vsa_encode(const char *name) {
    return hv_encode_text(name);
}

double vsa_similarity(const char *a, const char *b) {
    Hypervector va = vsa_encode(a);
    Hypervector vb = vsa_encode(b);
    return hv_similarity(&va, &vb);
}

int vsa_decode(const Hypervector *q, const char *const *codebook, size_t n,
               double *sim_out) {
    int best = -1;
    double best_sim = -1.0;
    for (size_t i = 0; i < n; i++) {
        Hypervector v = vsa_encode(codebook[i]);
        double s = hv_similarity(q, &v);
        if (s > best_sim) {
            best_sim = s;
            best = (int)i;
        }
    }
    if (sim_out) {
        *sim_out = best_sim;
    }
    return best;
}

int vsa_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define S(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    double rel = vsa_similarity("quantum_time_shift", "quantum_measurement");
    S(rel > 0.55, "related names → high similarity");
    double unr = vsa_similarity("quantum_time_shift", "money_ledger");
    S(unr < 0.55, "unrelated names → low similarity");
    S(vsa_similarity("ntt_transform", "ntt_transform") == 1.0,
      "identical names → similarity 1.0");

    const char *codebook[] = {"quantum_time_shift", "ntt_transform",
                              "money_ledger", "hypervector_bind"};
    Hypervector q = vsa_encode("quantum_measurement");
    double sim = 0.0;
    int idx = vsa_decode(&q, codebook, 4, &sim);
    S(idx == 0, "decode 'quantum_measurement' → quantum_time_shift");
    S(sim > 0.55, "  ... with high similarity");

    Hypervector q2 = vsa_encode("fourier_transform");
    int idx2 = vsa_decode(&q2, codebook, 4, NULL);
    S(idx2 == 1, "decode 'fourier_transform' → ntt_transform");

    return all_ok ? 0 : -1;
}
