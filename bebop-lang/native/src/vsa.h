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

/* ─── Zero-copy VSA binary packet (15B / #42 + 21B / #47) ───
 * A densely-packed struct: header + raw 1024-bit hypervector bytes. No
 * marshalling — the vector is memcpy'd in/out directly, so a packet maps 1:1
 * onto the vector for cache-line-aligned mesh transmission. */
#define VSA_PACKET_MAGIC 0x56534150u /* "VSAP" */
#define VSA_PACKET_HV 128             /* 1024-bit hypervector = 128 bytes */

typedef struct {
    uint32_t magic;
    uint32_t len;
    uint64_t checksum; /* XOR-fold of the 16 payload words */
    unsigned char hv[VSA_PACKET_HV]; /* raw hypervector (zero-copy) */
} __attribute__((packed, aligned(8))) VsaPacket;

/* Encode a hypervector into a packet (memcpy, zero-copy). Returns 0. */
int vsa_packet_encode(const Hypervector *hv, VsaPacket *pkt);
/* Decode a packet back into a hypervector; -1 on bad magic/len/checksum. */
int vsa_packet_decode(const VsaPacket *pkt, Hypervector *hv);

int vsa_self_test(char *out, size_t cap);

#endif /* BEBOP_VSA_H */
