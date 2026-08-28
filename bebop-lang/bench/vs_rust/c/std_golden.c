/* std_golden.c — M5 golden-vector driver: prints concrete fingerprints of
 * the C std implementations so the .bp twins can be compared against them
 * (before the C twins are removed at M7).
 *
 * Build: gcc -O2 -I ../../native/src -o std_golden std_golden.c \
 *        ../../native/src/sort.c ../../native/src/checksum.c \
 *        ../../native/src/rng.c ../../native/src/sha256.c -lm
 */
#include "checksum.h"
#include "rng.h"
#include "sha256.h"
#include "sort.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>

static uint64_t fold_u64(const uint64_t *v, size_t n) {
    uint64_t acc = 0;
    for (size_t i = 0; i < n; i++) acc = acc * 31 + v[i];
    return acc;
}

int main(void) {
    /* checksum: fold of "abc" */
    printf("checksum %llu\n",
           (unsigned long long)checksum_fold((const uint8_t *)"abc", 3));

    /* sort: sort_f64_asc over fixed integers (exact doubles), fold sorted */
    double items[] = {3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5};
    sort_f64_asc(items, sizeof items / sizeof items[0]);
    {
        uint64_t v[sizeof items / sizeof items[0]];
        for (size_t i = 0; i < sizeof items / sizeof items[0]; i++)
            v[i] = (uint64_t)items[i];
        printf("sort %llu\n", (unsigned long long)fold_u64(v, sizeof v / sizeof v[0]));
    }

    /* rng: rng_new(42,1) then 8 x rng_next_u64, fold */
    {
        Rng r = rng_new(42, 1);
        uint64_t v[8];
        for (int i = 0; i < 8; i++) v[i] = rng_next_u64(&r);
        printf("rng %llu\n", (unsigned long long)fold_u64(v, 8));
    }

    /* sha256("abc") — fold of the 8 big-endian u32 state words */
    {
        uint8_t out[32];
        sha256((const uint8_t *)"abc", 3, out);
        uint64_t v[8];
        for (int i = 0; i < 8; i++)
            v[i] = ((uint64_t)out[i * 4] << 24) |
                   ((uint64_t)out[i * 4 + 1] << 16) |
                   ((uint64_t)out[i * 4 + 2] << 8) |
                   ((uint64_t)out[i * 4 + 3]);
        printf("sha256 %llu\n", (unsigned long long)fold_u64(v, 8));
    }

    /* base64: no C twin — RFC 4648 test vectors, packed 4-char words.
     * "Man" (77,97,110) -> "TWFu" (84,87,70,117);
     * "Ma"  (77,97)      -> "TWE=" (84,87,69,61);
     * "M"   (77)         -> "TQ==" (84,81,61,61). */
    printf("base64 %llu %llu %llu\n",
           (unsigned long long)(84ULL * 16777216 + 87 * 65536 + 70 * 256 + 117),
           (unsigned long long)(84ULL * 16777216 + 87 * 65536 + 69 * 256 + 61),
           (unsigned long long)(84ULL * 16777216 + 81 * 65536 + 61 * 256 + 61));

    return 0;
}
