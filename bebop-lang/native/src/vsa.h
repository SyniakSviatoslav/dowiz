/* Bebop VSA identifiers — names as hypervector embeddings (replaces Morse).
 * A name is encoded via trigram bundling (hv_encode_text); similar names have
 * similar vectors, so identity is fuzzy/semantic. decode = nearest codebook. */
#ifndef BEBOP_VSA_H
#define BEBOP_VSA_H

#include <stddef.h>

#include "hyper.h"

/* Encode a name into a hypervector (VSA embedding). */
Hypervector vsa_encode(const char *name);

/* Similarity between two names (via their VSA embeddings). */
double vsa_similarity(const char *a, const char *b);

/* Decode: nearest name in a codebook to the query vector. Returns the index of
 * the nearest entry, or -1 if the codebook is empty; writes the similarity. */
int vsa_decode(const Hypervector *q, const char *const *codebook, size_t n,
               double *sim_out);

int vsa_self_test(char *out, size_t cap);

#endif /* BEBOP_VSA_H */
