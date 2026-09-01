/* Bebop hex_util — implementation (port of dowiz hex_util.rs). */
#include "hex_util.h"

#include <stdio.h>
#include <string.h>

static int hex_nibble(char ch) {
    if (ch >= '0' && ch <= '9') {
        return ch - '0';
    }
    if (ch >= 'a' && ch <= 'f') {
        return ch - 'a' + 10;
    }
    if (ch >= 'A' && ch <= 'F') {
        return ch - 'A' + 10;
    }
    return -1;
}

char *hex_encode(const uint8_t *bytes, size_t len, char *out) {
    static const char HEX[] = "0123456789abcdef";
    for (size_t i = 0; i < len; i++) {
        out[i * 2] = HEX[bytes[i] >> 4];
        out[i * 2 + 1] = HEX[bytes[i] & 0x0f];
    }
    out[len * 2] = '\0';
    return out;
}

int hex_decode(const char *hex, uint8_t *out, size_t cap, char *err, size_t cap_err) {
    size_t len = strlen(hex);
    if (len % 2 != 0) {
        if (err) {
            snprintf(err, cap_err, "odd length %zu", len);
        }
        return -1;
    }
    size_t n = len / 2;
    if (n > cap) {
        if (err) {
            snprintf(err, cap_err, "too long");
        }
        return -1;
    }
    for (size_t i = 0; i < n; i++) {
        int hi = hex_nibble(hex[i * 2]);
        int lo = hex_nibble(hex[i * 2 + 1]);
        if (hi < 0 || lo < 0) {
            if (err) {
                snprintf(err, cap_err, "invalid char at %zu", i * 2);
            }
            return -1;
        }
        out[i] = (uint8_t)((hi << 4) | lo);
    }
    return (int)n;
}

int hex_is_hex_str(const char *s) {
    size_t len = strlen(s);
    if (len == 0 || len % 2 != 0) {
        return 0;
    }
    for (size_t i = 0; i < len; i++) {
        if (hex_nibble(s[i]) < 0) {
            return 0;
        }
    }
    return 1;
}

int hex_self_test(char *out, size_t cap) {
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

    char buf[128];
    uint8_t d[4] = {0xde, 0xad, 0xbe, 0xef};
    A(strcmp(hex_encode(d, 4, buf), "deadbeef") == 0, "encode deadbeef");

    uint8_t ob[16];
    A(hex_decode("dead", ob, 16, NULL, 0) == 2 && ob[0] == 0xde && ob[1] == 0xad,
      "decode dead");
    A(hex_decode("abc", ob, 16, NULL, 0) == -1, "odd length rejected");
    A(hex_decode("xyz", ob, 16, NULL, 0) == -1, "invalid char rejected");

    uint8_t orig[7] = {0, 1, 127, 128, 255, 0xab, 0xcd};
    hex_encode(orig, 7, buf);
    int rn = hex_decode(buf, ob, 16, NULL, 0);
    A(rn == 7 && memcmp(orig, ob, 7) == 0, "roundtrip");

    A(hex_is_hex_str("deadbeef") && hex_is_hex_str("DEADBEEF"), "is_hex_str valid");
    A(!hex_is_hex_str("xyz") && !hex_is_hex_str("abc") && !hex_is_hex_str(""),
      "is_hex_str invalid");

    A(hex_decode("DEAD", ob, 16, NULL, 0) == 2 && ob[0] == 0xde, "uppercase decode");

    return all_ok ? 0 : -1;
}
