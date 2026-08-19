/* Bebop tls — ChaCha20 + Poly1305 + AEAD (RFC 8439). No libc. */
#ifndef BEBOP_TLS_H
#define BEBOP_TLS_H
#include <stdint.h>
#include <stddef.h>
void chacha20_block(const uint8_t key[32], const uint8_t nonce[12], uint32_t ctr, uint8_t out[64]);
void chacha20_xor(const uint8_t key[32], const uint8_t nonce[12], uint32_t ctr, const uint8_t *in, uint8_t *out, size_t n);
void poly1305(const uint8_t key[32], const uint8_t *msg, size_t n, uint8_t tag[16]);
int chacha20_poly1305_encrypt(const uint8_t key[32], const uint8_t nonce[12], const uint8_t *aad, size_t aadlen, const uint8_t *pt, uint8_t *ct, size_t n, uint8_t tag[16]);
int chacha20_poly1305_decrypt(const uint8_t key[32], const uint8_t nonce[12], const uint8_t *aad, size_t aadlen, const uint8_t *ct, uint8_t *pt, size_t n, const uint8_t tag[16]);
int ct_compare(const uint8_t *a, const uint8_t *b, size_t n);
int tls_self_test(char *out, size_t cap);
#endif
