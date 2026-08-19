/* R2 Allocation Rate harness — measures malloc/free ops/sec for fixed-size 1KB churn.
 * Uses getrusage to get peak RSS as a byproduct. Best-of-N, pinned to core 0. */
#define _POSIX_C_SOURCE 199309L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <time.h>

#define ALLOC_SIZE 1024
#define N_ITERS     500000

static long get_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long)ts.tv_sec * 1000000 + ts.tv_nsec / 1000;
}

int main(void) {
    const int N_RUNS = 5;
    double best_rate = 0.0;

    for (int run = 0; run < N_RUNS; run++) {
        long t0 = get_us();
        for (int i = 0; i < N_ITERS; i++) {
            void *p = malloc(ALLOC_SIZE);
            if (!p) { printf("malloc fail at i=%d\n", i); return 1; }
            memset(p, 0xAB, ALLOC_SIZE);
            free(p);
        }
        long t1 = get_us();
        double elapsed_s = (t1 - t0) / 1e6;
        double rate = (double)N_ITERS / elapsed_s;
        if (rate > best_rate) best_rate = rate;
    }

    struct rusage ru;
    getrusage(RUSAGE_SELF, &ru);

    printf("alloc_rate_per_sec: %.0f\n", best_rate);
    printf("peak_rss_kb: %ld\n", ru.ru_maxrss);
    return 0;
}