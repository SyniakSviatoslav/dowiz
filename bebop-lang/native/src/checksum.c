/* Bebop checksum — implementation (port of dowiz checksum.rs). */
#include "checksum.h"

#include <stdio.h>

uint64_t checksum_fold(const uint8_t *data, size_t len) {
    uint64_t acc = 0;
    for (size_t i = 0; i < len; i++) {
        acc = acc * 31 + data[i]; /* u64 wrapping is well-defined */
    }
    return acc;
}

int checksum_self_test(char *out, size_t cap) {
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

    A(checksum_fold((const uint8_t *)"", 0) == 0, "empty -> 0");

    const uint8_t d1[3] = {1, 2, 3};
    A(checksum_fold(d1, 3) == checksum_fold(d1, 3), "deterministic");

    const uint8_t d2[3] = {1, 2, 4};
    A(checksum_fold(d1, 3) != checksum_fold(d2, 3), "sensitive to change");

    uint64_t expect = ((97ULL * 31 + 98) * 31 + 99);
    A(checksum_fold((const uint8_t *)"abc", 3) == expect, "known value 'abc'");

    return all_ok ? 0 : -1;
}
