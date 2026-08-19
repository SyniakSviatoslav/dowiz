/* Bebop zlib — native DEFLATE/zlib compression (RFC 1950/1951).
 * No libc, no external deps. Implements CRC-32, Adler-32, an LZ77 + fixed-Huffman
 * DEFLATE encoder, and a full inflate (stored / fixed / dynamic Huffman blocks). */
#ifndef BEBOP_ZLIB_H
#define BEBOP_ZLIB_H

#include <stddef.h>
#include <stdint.h>

/* CRC-32 (IEEE 802.3, reflected polynomial 0xEDB88320). */
uint32_t zlib_crc32(uint32_t crc, const uint8_t *data, size_t len);

/* Adler-32 (mod 65521 rolling checksum). */
uint32_t zlib_adler32(uint32_t adler, const uint8_t *data, size_t len);

/* zlib_deflate — compress `in` into a zlib stream (RFC 1950 wrapper + DEFLATE).
 * `*out_len` holds the capacity on entry and is set to the compressed size on
 * success. Returns the compressed size, or -1 on error / output overflow. */
int64_t zlib_deflate(const uint8_t *in, size_t in_len,
                     uint8_t *out, size_t *out_len);

/* zlib_inflate — decompress a zlib stream (RFC 1950 wrapper + DEFLATE).
 * `*out_len` holds the capacity on entry and is set to the decompressed size on
 * success. Returns the decompressed size, or -1 on error / output overflow. */
int64_t zlib_inflate(const uint8_t *in, size_t in_len,
                     uint8_t *out, size_t *out_len);

int zlib_self_test(char *out, size_t cap);

#endif /* BEBOP_ZLIB_H */
