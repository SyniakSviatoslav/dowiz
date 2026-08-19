/* x25519.h — X25519 (RFC 7748) Curve25519 scalar multiplication.
   Constant-time Montgomery ladder, field arithmetic mod 2^255-19.
   Zero external dependencies (stdint only). */
#ifndef BP_X25519_H
#define BP_X25519_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Generate a keypair from a 32-byte seed.
   Fills pk[32] (public key — the u-coordinate) and sk[32] (private scalar
   with clamping applied). The seed itself is copied into sk and clamped. */
void x25519_keygen(const uint8_t seed[32], uint8_t pk[32], uint8_t sk[32]);

/* Compute a shared secret:  out = X25519(sk, peer_pk).
   sk is the caller's (clamped) private scalar, peer_pk the peer's public
   u-coordinate. Both are 32-byte little-endian, per RFC 7748. */
void x25519_shared_secret(const uint8_t sk[32], const uint8_t peer_pk[32],
                          uint8_t out[32]);

/* Self-test against RFC 7748 test vectors.  Returns 0 on PASS.
   Writes human-readable diagnostics into out (up to cap bytes). */
int x25519_self_test(char *out, size_t cap);

#ifdef __cplusplus
}
#endif

#endif /* BP_X25519_H */