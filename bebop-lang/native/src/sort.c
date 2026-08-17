/* Bebop sort — implementation (port of dowiz sort.rs). NaN/Inf → end of order. */
#include "sort.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>

static int cmp_desc(const void *a, const void *b) {
    double x = *(const double *)a;
    double y = *(const double *)b;
    int nx = isnan(x);
    int ny = isnan(y);
    if (nx && ny) {
        return 0;
    }
    if (nx) {
        return 1;
    }
    if (ny) {
        return -1;
    }
    if (x < y) {
        return 1;
    }
    if (x > y) {
        return -1;
    }
    return 0;
}

static int cmp_asc(const void *a, const void *b) {
    double x = *(const double *)a;
    double y = *(const double *)b;
    int nx = isnan(x);
    int ny = isnan(y);
    if (nx && ny) {
        return 0;
    }
    if (nx) {
        return 1; /* NaN → end (asc too) */
    }
    if (ny) {
        return -1;
    }
    if (x < y) {
        return -1;
    }
    if (x > y) {
        return 1;
    }
    return 0;
}

void sort_f64_desc(double *items, size_t n) {
    qsort(items, n, sizeof(double), cmp_desc);
}

void sort_f64_asc(double *items, size_t n) {
    qsort(items, n, sizeof(double), cmp_asc);
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

    return all_ok ? 0 : -1;
}
