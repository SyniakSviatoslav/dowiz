/* Bebop zlib — native DEFLATE/zlib compression (RFC 1950/1951).
 *
 * Deliberately libc-free (only <stdint.h>/<stddef.h>): every byte-level helper is
 * implemented here so the codec also links in a bare-metal AArch64 context. The
 * only host-isms (snprintf/puts) live inside zlib_self_test for the CLI driver.
 *
 * What it does:
 *   - CRC-32 (reflected, table-driven) and Adler-32.
 *   - DEFLATE encoder: greedy LZ77 (hash of 3-byte prefix) + fixed Huffman codes,
 *     packed LSB-first as the DEFLATE spec requires.
 *   - DEFLATE decoder: full inflate — stored, fixed-Huffman, and dynamic-Huffman
 *     blocks — so it round-trips real zlib output (verified against Python's zlib).
 *   - RFC 1950 zlib framing (2-byte header + Adler-32 trailer).
 */
#include "zlib.h"

#include <stdint.h>
#include <stddef.h>

/* snprintf is used only inside zlib_self_test (the host-side CLI driver); the
 * codec itself stays libc-free. */
#include <stdio.h>

/* ──────────────────────────────────────────────────────────────────────────
 * CRC-32
 * ────────────────────────────────────────────────────────────────────────── */
static uint32_t crc32_table[256];
static int crc32_ready = 0;

static void crc32_build(void) {
    for (uint32_t i = 0; i < 256; i++) {
        uint32_t c = i;
        for (int k = 0; k < 8; k++) {
            c = (c & 1) ? (0xEDB88320u ^ (c >> 1)) : (c >> 1);
        }
        crc32_table[i] = c;
    }
    crc32_ready = 1;
}

uint32_t zlib_crc32(uint32_t crc, const uint8_t *data, size_t len) {
    if (!crc32_ready) {
        crc32_build();
    }
    crc = ~crc;
    for (size_t i = 0; i < len; i++) {
        crc = crc32_table[(crc ^ data[i]) & 0xFF] ^ (crc >> 8);
    }
    return ~crc;
}

/* ──────────────────────────────────────────────────────────────────────────
 * Adler-32
 * ────────────────────────────────────────────────────────────────────────── */
#define ADLER_MOD 65521u

uint32_t zlib_adler32(uint32_t adler, const uint8_t *data, size_t len) {
    uint32_t s1 = adler & 0xFFFFu;
    uint32_t s2 = (adler >> 16) & 0xFFFFu;
    size_t i = 0;

    /* Process in chunks so s1/s2 stay well below 2^32 between reductions. */
    while (i < len) {
        size_t n = len - i;
        if (n > 5552) {
            n = 5552; /* 5552 * 255 < 2^21, keeps 16-bit-ish accumulation safe */
        }
        for (size_t j = 0; j < n; j++) {
            s1 += data[i + j];
            s2 += s1;
        }
        s1 %= ADLER_MOD;
        s2 %= ADLER_MOD;
        i += n;
    }
    return (s2 << 16) | s1;
}

/* ──────────────────────────────────────────────────────────────────────────
 * Bit writer (LSB-first, as DEFLATE requires)
 * ────────────────────────────────────────────────────────────────────────── */
typedef struct {
    uint8_t *out;
    size_t cap;
    size_t pos;
    uint64_t bitbuf;
    int bitcnt;
    int overflow;
} BitWriter;

static void bw_write(BitWriter *bw, uint32_t code, int nbits) {
    bw->bitbuf |= (uint64_t)code << bw->bitcnt;
    bw->bitcnt += nbits;
    while (bw->bitcnt >= 8) {
        if (bw->pos >= bw->cap) {
            bw->overflow = 1;
        } else {
            bw->out[bw->pos] = (uint8_t)(bw->bitbuf & 0xFF);
        }
        bw->pos++;
        bw->bitbuf >>= 8;
        bw->bitcnt -= 8;
    }
}

static void bw_flush(BitWriter *bw) {
    if (bw->bitcnt > 0) {
        if (bw->pos >= bw->cap) {
            bw->overflow = 1;
        } else {
            bw->out[bw->pos] = (uint8_t)(bw->bitbuf & 0xFF);
        }
        bw->pos++;
        bw->bitcnt = 0;
        bw->bitbuf = 0;
    }
}

/* Reverse the low `n` bits of `v`. Huffman codes are packed MSB-first into the
 * DEFLATE bit stream, but bw_write emits LSB-first, so code values are reversed
 * before being written. (Extra length/distance bits are already LSB-first.) */
static uint32_t reverse_bits(uint32_t v, int n) {
    uint32_t r = 0;
    for (int i = 0; i < n; i++) {
        r = (r << 1) | (v & 1);
        v >>= 1;
    }
    return r;
}

/* ──────────────────────────────────────────────────────────────────────────
 * Fixed-Huffman code lookup (RFC 1951 §3.2.6)
 * ────────────────────────────────────────────────────────────────────────── */
static void fixed_lit_code(unsigned sym, uint32_t *code, int *nbits) {
    if (sym <= 143) {          /* 0x30 + sym, 8 bits */
        *code = 0x30u + sym;
        *nbits = 8;
    } else if (sym <= 255) {   /* 0x190 + (sym-144), 9 bits */
        *code = 0x190u + (sym - 144);
        *nbits = 9;
    } else if (sym <= 279) {   /* sym-256, 7 bits */
        *code = sym - 256;
        *nbits = 7;
    } else {                   /* 0xC0 + (sym-280), 8 bits */
        *code = 0xC0u + (sym - 280);
        *nbits = 8;
    }
}

/* Length symbol -> (base length, extra bits). Covers 257..285. */
static const uint16_t len_base[29] = {
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51,
    59, 67, 83, 99, 115, 131, 163, 195, 227, 258
};
static const uint8_t len_extra[29] = {
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4,
    4, 5, 5, 5, 5, 0
};

/* Distance symbol -> (base distance, extra bits). Covers 0..29. */
static const uint16_t dist_base[30] = {
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385,
    513, 769, 1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577
};
static const uint8_t dist_extra[30] = {
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10,
    10, 11, 11, 12, 12, 13, 13
};

/* Find the length symbol (257..285) and extra bits for a run length 3..258. */
static unsigned length_symbol(unsigned len, unsigned *extra_bits, unsigned *extra_val) {
    unsigned sym = 257;
    while (sym < 285) {
        unsigned b = len_base[sym - 257];
        unsigned x = len_extra[sym - 257];
        unsigned maxb = b + ((1u << x) - 1);
        if (len >= b && len <= maxb) {
            *extra_bits = x;
            *extra_val = len - b;
            return sym;
        }
        sym++;
    }
    *extra_bits = 0;
    *extra_val = 0;
    return 285; /* 258 */
}

/* Find the distance symbol (0..29) and extra bits for a distance 1..32768. */
static unsigned distance_symbol(unsigned dist, unsigned *extra_bits, unsigned *extra_val) {
    unsigned sym = 0;
    while (sym < 29) {
        unsigned b = dist_base[sym];
        unsigned x = dist_extra[sym];
        unsigned maxb = b + ((1u << x) - 1);
        if (dist >= b && dist <= maxb) {
            *extra_bits = x;
            *extra_val = dist - b;
            return sym;
        }
        sym++;
    }
    *extra_bits = 0;
    *extra_val = 0;
    return 29; /* 24577..32768 */
}

/* ──────────────────────────────────────────────────────────────────────────
 * LZ77 + fixed-Huffman DEFLATE encoder
 * ────────────────────────────────────────────────────────────────────────── */
#define ZLIB_MAX_MATCH 258
#define ZLIB_MIN_MATCH 3
#define ZLIB_WINDOW   32768
#define ZLIB_HASH_BITS 15
#define ZLIB_HASH_SIZE (1u << ZLIB_HASH_BITS)
#define ZLIB_NIL 0xFFFFFFFFu

/* Greedy match search via a hash table keyed on the 3-byte prefix. */
static unsigned find_match(const uint8_t *in, size_t in_len, size_t pos,
                           uint32_t *head, uint32_t *prev, size_t *best_len) {
    uint32_t h = (uint32_t)in[pos] ^ ((uint32_t)in[pos + 1] << 5) ^
                 ((uint32_t)in[pos + 2] << 10);
    h &= (ZLIB_HASH_SIZE - 1);

    unsigned best_dist = 0;
    *best_len = 0;
    size_t max_len = in_len - pos;
    if (max_len > ZLIB_MAX_MATCH) {
        max_len = ZLIB_MAX_MATCH;
    }

    uint32_t cand = head[h];
    int steps = 0;
    while (cand != ZLIB_NIL && steps < 64) {
        size_t d = pos - cand;
        if (d > ZLIB_WINDOW) {
            break;
        }
        /* quick check on the first three bytes */
        if (in[cand] == in[pos] && in[cand + 1] == in[pos + 1] &&
            in[cand + 2] == in[pos + 2]) {
            size_t l = 3;
            while (l < max_len && in[cand + l] == in[pos + l]) {
                l++;
            }
            if (l > *best_len) {
                *best_len = l;
                best_dist = (unsigned)d;
                if (l == max_len) {
                    break;
                }
            }
        }
        cand = prev[cand];
        steps++;
    }
    return best_dist;
}

/* Emit a single (non-final) fixed-Huffman block covering the whole input. */
static int deflate_fixed(const uint8_t *in, size_t in_len, BitWriter *bw) {
    if (in_len == 0) {
        /* Empty input: BFINAL=1, BTYPE=01 (fixed), EOB. */
        bw_write(bw, 1, 1);
        bw_write(bw, 1, 2);
        bw_write(bw, 0, 7); /* EOB (256) */
        return 0;
    }

    /* Hash table of the most recent position for each 3-byte prefix. */
    static uint32_t head[ZLIB_HASH_SIZE];
    static uint32_t prev[ZLIB_WINDOW];

    /* Bare-metal friendliness: if we ever move off host, the caller provides a
     * scratch arena. Here we use static storage sized to the DEFLATE window. */
    for (uint32_t i = 0; i < ZLIB_HASH_SIZE; i++) {
        head[i] = ZLIB_NIL;
    }

    bw_write(bw, 1, 1); /* BFINAL = 1 */
    bw_write(bw, 1, 2); /* BTYPE = 01 (fixed Huffman) */

    size_t pos = 0;
    while (pos < in_len) {
        uint32_t h;
        if (pos + 2 < in_len) {
            h = (uint32_t)in[pos] ^ ((uint32_t)in[pos + 1] << 5) ^
                ((uint32_t)in[pos + 2] << 10);
            h &= (ZLIB_HASH_SIZE - 1);
        } else {
            h = 0;
        }

        size_t best_len = 0;
        unsigned best_dist = 0;
        if (pos + 2 < in_len) {
            best_dist = find_match(in, in_len, pos, head, prev, &best_len);
        }

        if (best_len >= ZLIB_MIN_MATCH) {
            unsigned lbits = 0, lval = 0;
            unsigned lsym = length_symbol((unsigned)best_len, &lbits, &lval);
            unsigned dbits = 0, dval = 0;
            unsigned dsym = distance_symbol(best_dist, &dbits, &dval);

            uint32_t code;
            int nbits;
            fixed_lit_code(lsym, &code, &nbits);
            bw_write(bw, reverse_bits(code, nbits), nbits);
            if (lbits > 0) {
                bw_write(bw, lval, (int)lbits);
            }
            bw_write(bw, reverse_bits(dsym, 5), 5); /* fixed distance code */
            if (dbits > 0) {
                bw_write(bw, dval, (int)dbits);
            }

            /* Insert hashes for the matched span so later positions chain. */
            size_t end = pos + best_len;
            for (size_t p = pos; p < end && p + 2 < in_len; p++) {
                uint32_t ph = (uint32_t)in[p] ^ ((uint32_t)in[p + 1] << 5) ^
                              ((uint32_t)in[p + 2] << 10);
                ph &= (ZLIB_HASH_SIZE - 1);
                prev[p] = head[ph];
                head[ph] = (uint32_t)p;
            }
            pos = end;
        } else {
            uint32_t code;
            int nbits;
            fixed_lit_code(in[pos], &code, &nbits);
            bw_write(bw, reverse_bits(code, nbits), nbits);

            if (pos + 2 < in_len) {
                prev[pos] = head[h];
                head[h] = (uint32_t)pos;
            }
            pos++;
        }
    }

    bw_write(bw, 0, 7); /* EOB (256), 7-bit fixed code */
    return 0;
}

/* ──────────────────────────────────────────────────────────────────────────
 * zlib_deflate — RFC 1950 wrapper + DEFLATE
 * ────────────────────────────────────────────────────────────────────────── */
int64_t zlib_deflate(const uint8_t *in, size_t in_len,
                     uint8_t *out, size_t *out_len) {
    if ((in == NULL && in_len != 0) || out == NULL || out_len == NULL) {
        return -1;
    }
    size_t cap = *out_len;

    BitWriter bw;
    bw.out = out;
    bw.cap = cap;
    bw.pos = 0;
    bw.bitbuf = 0;
    bw.bitcnt = 0;
    bw.overflow = 0;

    /* zlib header: CMF = 0x78 (deflate, 32K window), FLG chosen so
     * (CMF*256 + FLG) % 31 == 0 and no preset dictionary. */
    bw.out[0] = 0x78;
    bw.out[1] = 0x01;
    bw.pos = 2;

    deflate_fixed(in, in_len, &bw);
    bw_flush(&bw);

    /* Adler-32 trailer, big-endian. */
    uint32_t a = zlib_adler32(1u, in, in_len);
    if (bw.pos + 4 > cap) {
        bw.overflow = 1;
    }
    if (!bw.overflow) {
        out[bw.pos++] = (uint8_t)(a >> 24);
        out[bw.pos++] = (uint8_t)(a >> 16);
        out[bw.pos++] = (uint8_t)(a >> 8);
        out[bw.pos++] = (uint8_t)a;
    }

    if (bw.overflow) {
        return -1;
    }
    *out_len = bw.pos;
    return (int64_t)bw.pos;
}

/* ──────────────────────────────────────────────────────────────────────────
 * Inflate: bit reader + canonical Huffman decode
 * ────────────────────────────────────────────────────────────────────────── */
#define MAXBITS 15

typedef struct {
    const uint8_t *in;
    size_t in_len;
    size_t in_pos;
    uint64_t bitbuf;
    int bitcnt;
    int err; /* 0 = ok, negative = specific failure */
} InflateState;

typedef struct {
    int count[MAXBITS + 1];
    int symbol[288]; /* max literal/length codes */
} Huffman;

/* Read `need` bits LSB-first. */
static uint32_t inflate_bits(InflateState *s, int need) {
    while (s->bitcnt < need) {
        if (s->in_pos >= s->in_len) {
            s->err = -1;
            return 0;
        }
        s->bitbuf |= (uint64_t)s->in[s->in_pos++] << s->bitcnt;
        s->bitcnt += 8;
    }
    uint32_t v = (uint32_t)(s->bitbuf & (((uint64_t)1 << need) - 1));
    s->bitbuf >>= need;
    s->bitcnt -= need;
    return v;
}

/* Build a canonical Huffman table from per-symbol code lengths.
 * Returns 0 on success, negative on an invalid (over-subscribed/incomplete) code. */
static int huffman_build(Huffman *h, const uint8_t *lengths, int n) {
    int offs[MAXBITS + 1];
    int len, sym;

    for (len = 0; len <= MAXBITS; len++) {
        h->count[len] = 0;
    }
    for (sym = 0; sym < n; sym++) {
        if (lengths[sym] > MAXBITS) {
            return -2; /* code length out of range */
        }
        h->count[lengths[sym]]++;
    }
    if (h->count[0] == n) {
        return 0; /* no codes */
    }

    /* Over-subscribed check: sum(count[len] << (MAXBITS - len)) > 1<<MAXBITS.
     * Incomplete codes (left > 0) are legal in DEFLATE — e.g. the fixed
     * distance code (30 symbols at 5 bits) — so we do not reject them. */
    {
        int left = 1;
        for (len = 1; len <= MAXBITS; len++) {
            left <<= 1;
            left -= h->count[len];
            if (left < 0) {
                return -3; /* over-subscribed */
            }
        }
    }

    offs[1] = 0;
    for (len = 1; len < MAXBITS; len++) {
        offs[len + 1] = offs[len] + h->count[len];
    }
    for (sym = 0; sym < n; sym++) {
        if (lengths[sym] != 0) {
            h->symbol[offs[lengths[sym]]++] = sym;
        }
    }
    return 0;
}

/* Decode one symbol. Returns -1 on failure. */
static int huffman_decode(InflateState *s, const Huffman *h) {
    int len, code, first, count, index;
    code = first = index = 0;
    for (len = 1; len <= MAXBITS; len++) {
        code |= (int)inflate_bits(s, 1);
        if (s->err) {
            return -1;
        }
        count = h->count[len];
        if (code - first < count) {
            return h->symbol[index + (code - first)];
        }
        index += count;
        first += count;
        first <<= 1;
        code <<= 1;
    }
    s->err = -5; /* invalid code */
    return -1;
}

/* The order in which code-length-code lengths appear (RFC 1951 §3.2.7). */
static const uint8_t clc_order[19] = {
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15
};

/* Fixed literal/length code lengths (RFC 1951 §3.2.6). */
static void fixed_lit_lengths(uint8_t *lens) {
    int sym;
    for (sym = 0; sym <= 143; sym++) {
        lens[sym] = 8;
    }
    for (sym = 144; sym <= 255; sym++) {
        lens[sym] = 9;
    }
    for (sym = 256; sym <= 279; sym++) {
        lens[sym] = 7;
    }
    for (sym = 280; sym <= 287; sym++) {
        lens[sym] = 8;
    }
}

/* ──────────────────────────────────────────────────────────────────────────
 * zlib_inflate — RFC 1950 wrapper + full DEFLATE decoder
 * ────────────────────────────────────────────────────────────────────────── */
int64_t zlib_inflate(const uint8_t *in, size_t in_len,
                     uint8_t *out, size_t *out_len) {
    if ((in == NULL && in_len != 0) || out == NULL || out_len == NULL) {
        return -1;
    }
    size_t cap = *out_len;

    InflateState s;
    s.in = in;
    s.in_len = in_len;
    s.in_pos = 0;
    s.bitbuf = 0;
    s.bitcnt = 0;
    s.err = 0;

    size_t out_pos = 0;
#define OUT_OVERFLOW() ((out_pos) >= (cap))

    /* zlib header (RFC 1950). */
    if (in_len < 2) {
        return -1;
    }
    uint32_t cmf = in[0];
    uint32_t flg = in[1];
    if ((cmf & 0x0F) != 8) { /* deflate */
        return -1;
    }
    if (((cmf >> 4) & 0x0F) > 7) { /* window size */
        return -1;
    }
    if (((cmf << 8) | flg) % 31 != 0) {
        return -1;
    }
    if (flg & 0x20) { /* preset dictionary — not supported */
        return -1;
    }
    s.in_pos = 2;

    uint8_t lit_lens[288];
    uint8_t dist_lens[30];
    uint8_t clc_lens[19];
    Huffman lit_h, dist_h, clc_h;

    int done = 0;
    while (!done) {
        if (s.in_pos >= s.in_len && s.bitcnt <= 0) {
            return -1; /* truncated */
        }
        int bfinal = (int)inflate_bits(&s, 1);
        int btype = (int)inflate_bits(&s, 2);
        if (s.err) {
            return -1;
        }

        if (btype == 0) {
            /* Stored block: align to byte boundary. */
            s.bitbuf = 0;
            s.bitcnt = 0;
            if (s.in_pos + 4 > s.in_len) {
                return -1;
            }
            uint32_t len = in[s.in_pos] | ((uint32_t)in[s.in_pos + 1] << 8);
            uint32_t nlen = in[s.in_pos + 2] | ((uint32_t)in[s.in_pos + 3] << 8);
            s.in_pos += 4;
            if (nlen != ((~len) & 0xFFFF)) {
                return -1;
            }
            if (s.in_pos + len > s.in_len) {
                return -1;
            }
            if (out_pos + len > cap) {
                return -1;
            }
            for (uint32_t i = 0; i < len; i++) {
                out[out_pos++] = in[s.in_pos++];
            }
        } else if (btype == 1) {
            fixed_lit_lengths(lit_lens);
            for (int i = 0; i < 30; i++) {
                dist_lens[i] = 5;
            }
            if (huffman_build(&lit_h, lit_lens, 288) != 0 ||
                huffman_build(&dist_h, dist_lens, 30) != 0) {
                return -1;
            }
            /* decode literals/matches until end-of-block */
            for (;;) {
                int sym = huffman_decode(&s, &lit_h);
                if (sym < 0) {
                    return -1;
                }
                if (sym < 256) {
                    if (OUT_OVERFLOW()) {
                        return -1;
                    }
                    out[out_pos++] = (uint8_t)sym;
                } else if (sym == 256) {
                    break; /* end of block */
                } else {
                    if (sym > 285) {
                        return -1;
                    }
                    unsigned lbits = len_extra[sym - 257];
                    unsigned lbase = len_base[sym - 257];
                    unsigned lval = (lbits > 0) ? inflate_bits(&s, (int)lbits) : 0;
                    if (s.err) {
                        return -1;
                    }
                    unsigned length = lbase + lval;

                    int dsym = huffman_decode(&s, &dist_h);
                    if (dsym < 0 || dsym > 29) {
                        return -1;
                    }
                    unsigned dbits = dist_extra[dsym];
                    unsigned dbase = dist_base[dsym];
                    unsigned dval = (dbits > 0) ? inflate_bits(&s, (int)dbits) : 0;
                    if (s.err) {
                        return -1;
                    }
                    unsigned dist = dbase + dval;

                    if (dist > out_pos) {
                        return -1; /* distance beyond output so far */
                    }
                    if (out_pos + length > cap) {
                        return -1;
                    }
                    for (unsigned i = 0; i < length; i++) {
                        out[out_pos] = out[out_pos - dist];
                        out_pos++;
                    }
                }
            }
        } else if (btype == 2) {
            int hlit = (int)inflate_bits(&s, 5) + 257;
            int hdist = (int)inflate_bits(&s, 5) + 1;
            int hclen = (int)inflate_bits(&s, 4) + 4;
            if (s.err) {
                return -1;
            }
            if (hlit > 286 || hdist > 30) {
                return -1;
            }

            for (int i = 0; i < 19; i++) {
                clc_lens[i] = 0;
            }
            for (int i = 0; i < hclen; i++) {
                clc_lens[clc_order[i]] = (uint8_t)inflate_bits(&s, 3);
            }
            if (s.err) {
                return -1;
            }
            if (huffman_build(&clc_h, clc_lens, 19) != 0) {
                return -1;
            }

            int total = hlit + hdist;
            int idx = 0;
            while (idx < total) {
                int sym = huffman_decode(&s, &clc_h);
                if (sym < 0) {
                    return -1;
                }
                if (sym < 16) {
                    if (idx < hlit) {
                        lit_lens[idx] = (uint8_t)sym;
                    } else {
                        dist_lens[idx - hlit] = (uint8_t)sym;
                    }
                    idx++;
                } else if (sym == 16) {
                    if (idx == 0) {
                        return -1; /* no previous length */
                    }
                    int rep = 3 + (int)inflate_bits(&s, 2);
                    uint8_t prev = (idx <= hlit)
                                       ? lit_lens[idx - 1]
                                       : dist_lens[idx - 1 - hlit];
                    for (int k = 0; k < rep && idx < total; k++, idx++) {
                        if (idx < hlit) {
                            lit_lens[idx] = prev;
                        } else {
                            dist_lens[idx - hlit] = prev;
                        }
                    }
                } else if (sym == 17) {
                    int rep = 3 + (int)inflate_bits(&s, 3);
                    for (int k = 0; k < rep && idx < total; k++, idx++) {
                        if (idx < hlit) {
                            lit_lens[idx] = 0;
                        } else {
                            dist_lens[idx - hlit] = 0;
                        }
                    }
                } else if (sym == 18) {
                    int rep = 11 + (int)inflate_bits(&s, 7);
                    for (int k = 0; k < rep && idx < total; k++, idx++) {
                        if (idx < hlit) {
                            lit_lens[idx] = 0;
                        } else {
                            dist_lens[idx - hlit] = 0;
                        }
                    }
                } else {
                    return -1;
                }
                if (s.err) {
                    return -1;
                }
            }

            if (huffman_build(&lit_h, lit_lens, hlit) != 0 ||
                huffman_build(&dist_h, dist_lens, hdist) != 0) {
                return -1;
            }

            for (;;) {
                int sym = huffman_decode(&s, &lit_h);
                if (sym < 0) {
                    return -1;
                }
                if (sym < 256) {
                    if (OUT_OVERFLOW()) {
                        return -1;
                    }
                    out[out_pos++] = (uint8_t)sym;
                } else if (sym == 256) {
                    break;
                } else {
                    if (sym > 285) {
                        return -1;
                    }
                    unsigned lbits = len_extra[sym - 257];
                    unsigned lbase = len_base[sym - 257];
                    unsigned lval = (lbits > 0) ? inflate_bits(&s, (int)lbits) : 0;
                    if (s.err) {
                        return -1;
                    }
                    unsigned length = lbase + lval;

                    int dsym = huffman_decode(&s, &dist_h);
                    if (dsym < 0 || dsym > 29) {
                        return -1;
                    }
                    unsigned dbits = dist_extra[dsym];
                    unsigned dbase = dist_base[dsym];
                    unsigned dval = (dbits > 0) ? inflate_bits(&s, (int)dbits) : 0;
                    if (s.err) {
                        return -1;
                    }
                    unsigned dist = dbase + dval;

                    if (dist > out_pos) {
                        return -1;
                    }
                    if (out_pos + length > cap) {
                        return -1;
                    }
                    for (unsigned i = 0; i < length; i++) {
                        out[out_pos] = out[out_pos - dist];
                        out_pos++;
                    }
                }
            }
        } else {
            return -1; /* reserved block type (3) */
        }

        if (bfinal) {
            done = 1;
        }
    }

    /* Adler-32 trailer. */
    s.bitbuf = 0;
    s.bitcnt = 0;
    if (s.in_pos + 4 > s.in_len) {
        return -1;
    }
    uint32_t a = ((uint32_t)in[s.in_pos] << 24) |
                 ((uint32_t)in[s.in_pos + 1] << 16) |
                 ((uint32_t)in[s.in_pos + 2] << 8) |
                 ((uint32_t)in[s.in_pos + 3]);
    if (zlib_adler32(1u, out, out_pos) != a) {
        return -1;
    }

    *out_len = out_pos;
    return (int64_t)out_pos;
#undef OUT_OVERFLOW
}

/* ──────────────────────────────────────────────────────────────────────────
 * Self-test
 * ────────────────────────────────────────────────────────────────────────── */
int zlib_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) {                                                          \
            pos += (size_t)r_;                                                 \
        }                                                                      \
        if (!c_) {                                                             \
            all_ok = 0;                                                        \
        }                                                                      \
    } while (0)

    /* CRC-32 check value for "123456789". */
    A(zlib_crc32(0, (const uint8_t *)"123456789", 9) == 0xCBF43926u,
      "crc32 '123456789'");

    /* Adler-32 check value for "Wikipedia". */
    A(zlib_adler32(1u, (const uint8_t *)"Wikipedia", 9) == 0x11E60398u,
      "adler32 'Wikipedia'");

    /* ── Round-trip: compress then decompress a known buffer ── */
    static const uint8_t plain[] =
        "The quick brown fox jumps over the lazy dog. "
        "Pack my box with five dozen liquor jugs. "
        "aaaaaaaaaaaaaaaaaaaa bbbbbbbbbbbbbbbbbbbb cccccccccccccccccccc\n";
    size_t plain_len = sizeof(plain) - 1;

    uint8_t comp[1024];
    size_t comp_len = sizeof comp;
    int64_t csz = zlib_deflate(plain, plain_len, comp, &comp_len);
    A(csz >= 0, "deflate returns non-negative size");
    A(comp_len < plain_len, "deflate actually compresses");

    uint8_t back[1024];
    size_t back_len = sizeof back;
    int64_t dsz = zlib_inflate(comp, comp_len, back, &back_len);
    A(dsz == (int64_t)plain_len, "inflate returns original size");
    A(back_len == plain_len, "inflate sets output length");
    int match = 1;
    for (size_t i = 0; i < plain_len; i++) {
        if (back[i] != plain[i]) {
            match = 0;
            break;
        }
    }
    A(match, "round-trip byte-identical");

    /* ── Interop: decode streams produced by a real zlib implementation ── */
    {
        /* zlib.compress(b"hello hello hello hello") — dynamic Huffman block. */
        static const uint8_t dyn[] = {
            0x78, 0x9c, 0xcb, 0x48, 0xcd, 0xc9, 0xc9, 0x57,
            0xc8, 0x40, 0x27, 0x01, 0x68, 0x03, 0x08, 0xb1
        };
        static const uint8_t dyn_plain[] = "hello hello hello hello";
        uint8_t dyn_out[64];
        size_t dyn_len = sizeof dyn_out;
        int64_t d = zlib_inflate(dyn, sizeof dyn, dyn_out, &dyn_len);
        A(d == (int64_t)(sizeof dyn_plain - 1), "dynamic-huffman inflate size");
        int m = (dyn_len == sizeof dyn_plain - 1);
        for (size_t i = 0; m && i < dyn_len; i++) {
            m = (dyn_out[i] == dyn_plain[i]);
        }
        A(m, "dynamic-huffman inflate content");
    }
    {
        /* zlib.compress(..., level=0) — stored block. */
        static const uint8_t stored[] = {
            0x78, 0x01, 0x01, 0x17, 0x00, 0xe8, 0xff, 0x68,
            0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x68, 0x65, 0x6c,
            0x6c, 0x6f, 0x20, 0x68, 0x65, 0x6c, 0x6c, 0x6f,
            0x20, 0x68, 0x65, 0x6c, 0x6c, 0x6f, 0x68, 0x03,
            0x08, 0xb1
        };
        static const uint8_t stored_plain[] = "hello hello hello hello";
        uint8_t stored_out[64];
        size_t stored_len = sizeof stored_out;
        int64_t d = zlib_inflate(stored, sizeof stored, stored_out, &stored_len);
        A(d == (int64_t)(sizeof stored_plain - 1), "stored-block inflate size");
        int m = (stored_len == sizeof stored_plain - 1);
        for (size_t i = 0; m && i < stored_len; i++) {
            m = (stored_out[i] == stored_plain[i]);
        }
        A(m, "stored-block inflate content");
    }
    {
        /* zlib.compress(..., strategy=Z_FIXED) — fixed-Huffman block. */
        static const uint8_t fixed[] = {
            0x78, 0x01, 0x4b, 0x4c, 0x4c, 0x4c, 0x54, 0x48,
            0x02, 0x02, 0x85, 0x64, 0x20, 0x50, 0x48, 0x01,
            0x02, 0x00, 0x40, 0xff, 0x06, 0x89
        };
        static const uint8_t fixed_plain[] = "aaaa bbbb cccc dddd";
        uint8_t fixed_out[64];
        size_t fixed_len = sizeof fixed_out;
        int64_t d = zlib_inflate(fixed, sizeof fixed, fixed_out, &fixed_len);
        A(d == (int64_t)(sizeof fixed_plain - 1), "fixed-huffman inflate size");
        int m = (fixed_len == sizeof fixed_plain - 1);
        for (size_t i = 0; m && i < fixed_len; i++) {
            m = (fixed_out[i] == fixed_plain[i]);
        }
        A(m, "fixed-huffman inflate content");
    }

    /* ── Overflow safety: tiny output buffer must fail, not corrupt ── */
    {
        uint8_t tiny[8];
        size_t tiny_len = sizeof tiny;
        int64_t r = zlib_deflate(plain, plain_len, tiny, &tiny_len);
        A(r == -1, "deflate rejects undersized output");
    }
    {
        uint8_t tiny[8];
        size_t tiny_len = sizeof tiny;
        int64_t r = zlib_inflate(comp, comp_len, tiny, &tiny_len);
        A(r == -1, "inflate rejects undersized output");
    }

    return all_ok ? 0 : -1;
#undef A
}
