/* Bebop hex_util — canonical hex encode/decode (port of dowiz hex_util.rs). */
#ifndef BEBOP_HEX_UTIL_H
#define BEBOP_HEX_UTIL_H

#include <stddef.h>
#include <stdint.h>

/* encode bytes to lowercase hex (out must have len*2+1 bytes); returns out. */
char *hex_encode(const uint8_t *bytes, size_t len, char *out);

/* decode hex string to bytes; returns byte count, or -1 on error (err filled). */
int hex_decode(const char *hex, uint8_t *out, size_t cap, char *err, size_t cap_err);

int hex_is_hex_str(const char *s);

int hex_self_test(char *out, size_t cap);

#endif /* BEBOP_HEX_UTIL_H */
