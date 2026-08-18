/* Bebop sort — implementation (port of dowiz sort.rs). NaN/Inf → end of order.
 *
 * Two fast paths, both O(n) radix sort on a monotonic u64 key (the same
 * "monotonic key + adaptive passes" pattern dowiz uses for canonical ordering):
 *
 * 1. Integer fast path: if every value is a non-negative exact integer < 2^53,
 *    the key is just the integer value (0..2^53-1), which is monotonic in
 *    double order. Only the low bytes that actually vary are sorted — 20-bit
 *    data runs 3 passes instead of 8.
 *
 * 2. General path: IEEE-754 total-order key (sign trick), 8-bit digits, passes
 *    adapt to the dynamic range of the key.
 *
 * NaN is partitioned to the end up front (both +NaN and -NaN), so the radix
 * only ever sees finite values. Single key array (doubles transformed in place
 * via a union), so scatter traffic is halved vs carrying a parallel value array.
 */
#include "sort.h"

#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SORT_STATIC_N (1u << 16)

static uint64_t sort_keys[SORT_STATIC_N];
static uint64_t sort_tmp[SORT_STATIC_N];

/* IEEE-754 total-order key for a finite f64 (ascending). */
static inline uint64_t f64_key(double x) {
    union { double d; uint64_t u; } v;
    v.d = x;
    return (v.u >> 63) ? ~v.u : (v.u | 0x8000000000000000ULL);
}

static inline double key_unpack(uint64_t k) {
    union { double d; uint64_t u; } v;
    v.u = (k >> 63) ? (k ^ 0x8000000000000000ULL) : ~k;
    return v.d;
}

/* LSD radix sort (8-bit digits) on u64 keys, `nbytes` passes (1..8). */
static void radix_sort_u64(uint64_t *keys, uint64_t *tmp, size_t n, int nbytes) {
    size_t cnt[256];
    uint64_t *src = keys, *dst = tmp;
    for (int byte = 0; byte < nbytes; byte++) {
        memset(cnt, 0, sizeof cnt);
        for (size_t i = 0; i < n; i++) {
            cnt[(src[i] >> (byte * 8)) & 0xff]++;
        }
        size_t total = 0;
        for (int b = 0; b < 256; b++) {
            size_t c = cnt[b];
            cnt[b] = total;
            total += c;
        }
        for (size_t i = 0; i < n; i++) {
            dst[cnt[(src[i] >> (byte * 8)) & 0xff]++] = src[i];
        }
        uint64_t *t = src; src = dst; dst = t;
    }
    if (src != keys) {
        memcpy(keys, src, n * sizeof *keys);
    }
}

static void insertion_sort_asc(double *items, size_t n) {
    for (size_t i = 1; i < n; i++) {
        double x = items[i];
        size_t j = i;
        while (j > 0 && items[j - 1] > x) {
            items[j] = items[j - 1];
            j--;
        }
        items[j] = x;
    }
}

/* Partition NaN to the end; return count of finite elements. */
static size_t partition_nan(double *items, size_t n) {
    size_t i = 0, j = n;
    while (i < j) {
        if (isnan(items[i])) {
            j--;
            double t = items[i];
            items[i] = items[j];
            items[j] = t;
        } else {
            i++;
        }
    }
    return i;
}

static void sort_finite(double *items, size_t n, int descending) {
    if (n < 2) return;
    if (n < 64) {
        insertion_sort_asc(items, n);
        if (descending) {
            for (size_t i = 0; i < n / 2; i++) {
                double t = items[i];
                items[i] = items[n - 1 - i];
                items[n - 1 - i] = t;
            }
        }
        return;
    }

    uint64_t *keys, *tmp;
    int heap = n > SORT_STATIC_N;
    if (heap) {
        keys = malloc(n * sizeof *keys);
        tmp = malloc(n * sizeof *tmp);
        if (!keys || !tmp) {
            free(keys); free(tmp);
            insertion_sort_asc(items, n);
            if (descending) {
                for (size_t i = 0; i < n / 2; i++) {
                    double t = items[i];
                    items[i] = items[n - 1 - i];
                    items[n - 1 - i] = t;
                }
            }
            return;
        }
    } else {
        keys = sort_keys;
        tmp = sort_tmp;
    }

    /* Detect the integer fast path: all values non-negative exact integers
     * < 2^53. The integer value is then a monotonic sort key, and we only need
     * the low bytes that vary. */
    int all_int = 1;
    uint64_t max_int = 0;
    for (size_t i = 0; i < n; i++) {
        double x = items[i];
        if (!(x >= 0.0 && x < 9007199254740992.0 && x == floor(x))) {
            all_int = 0;
            break;
        }
        uint64_t iv = (uint64_t)x;
        keys[i] = iv;
        if (iv > max_int) max_int = iv;
    }

    int nbytes;
    if (all_int) {
        /* integer keys: sort ascending by value; invert for descending. */
        if (descending) {
            for (size_t i = 0; i < n; i++) keys[i] = ~keys[i];
        }
        nbytes = 0;
        uint64_t v = max_int;
        while (v) { nbytes++; v >>= 8; }
        if (nbytes == 0) nbytes = 1;
        radix_sort_u64(keys, tmp, n, nbytes);
        for (size_t i = 0; i < n; i++) {
            items[i] = (double)(descending ? ~keys[i] : keys[i]);
        }
    } else {
        /* general path: IEEE total-order key */
        for (size_t i = 0; i < n; i++) {
            keys[i] = f64_key(items[i]);
        }
        if (descending) {
            for (size_t i = 0; i < n; i++) keys[i] = ~keys[i];
        }
        radix_sort_u64(keys, tmp, n, 8);
        if (descending) {
            for (size_t i = 0; i < n; i++) items[i] = key_unpack(~keys[i]);
        } else {
            for (size_t i = 0; i < n; i++) items[i] = key_unpack(keys[i]);
        }
    }

    if (heap) {
        free(keys); free(tmp);
    }
}

void sort_f64_desc(double *items, size_t n) {
    size_t m = partition_nan(items, n);
    sort_finite(items, m, 1);
}

void sort_f64_asc(double *items, size_t n) {
    size_t m = partition_nan(items, n);
    sort_finite(items, m, 0);
}

int sort_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          (cond) ? "ok" : "FAIL", name);                       \
        if (r_ > 0) {                                                          \
            pos += (size_t)r_;                                                 \
        }                                                                      \
        if (!(cond)) {                                                         \
            all_ok = 0;                                                        \
        }                                                                      \
    } while (0)

    double a[5] = {3.0, 1.0, 2.0, 5.0, 4.0};
    sort_f64_asc(a, 5);
    A(a[0] == 1.0 && a[4] == 5.0, "asc sort");
    sort_f64_desc(a, 5);
    A(a[0] == 5.0 && a[4] == 1.0, "desc sort");

    double b[4] = {2.0, NAN, 1.0, 3.0};
    sort_f64_asc(b, 4);
    A(isnan(b[3]), "NaN pushed to end");
    A(b[0] == 1.0 && b[1] == 2.0 && b[2] == 3.0, "finite order preserved");

    double c[6] = {-0.0, 2.0, -3.0, 0.0, INFINITY, -INFINITY};
    sort_f64_asc(c, 6);
    A(c[0] == -INFINITY && c[1] == -3.0 && c[2] == -0.0 && c[3] == 0.0 &&
          c[4] == 2.0 && c[5] == INFINITY, "full-order asc");
    sort_f64_desc(c, 6);
    A(c[0] == INFINITY && c[1] == 2.0 && c[2] == 0.0 && c[3] == -0.0 &&
          c[4] == -3.0 && c[5] == -INFINITY, "full-order desc");

    double big[1000];
    for (int i = 0; i < 1000; i++) {
        big[i] = (double)((i * 2654435761u) & 0xfffff) * ((i & 1) ? 1.0 : -1.0);
    }
    sort_f64_asc(big, 1000);
    int sorted = 1;
    for (int i = 1; i < 1000; i++) {
        if (big[i - 1] > big[i]) sorted = 0;
    }
    A(sorted, "1000-element asc sorted");
    sort_f64_desc(big, 1000);
    sorted = 1;
    for (int i = 1; i < 1000; i++) {
        if (big[i - 1] < big[i]) sorted = 0;
    }
    A(sorted, "1000-element desc sorted");

    double d[10000];
    for (int i = 0; i < 10000; i++) d[i] = (double)i;
    sort_f64_asc(d, 10000);
    sorted = 1;
    for (int i = 1; i < 10000; i++) if (d[i - 1] > d[i]) sorted = 0;
    A(sorted, "10000-element sorted input stays sorted");
    for (int i = 0; i < 10000; i++) d[i] = (double)(10000 - i);
    sort_f64_asc(d, 10000);
    sorted = 1;
    for (int i = 1; i < 10000; i++) if (d[i - 1] > d[i]) sorted = 0;
    A(sorted, "10000-element reverse input sorted");

    return all_ok ? 0 : -1;
}
