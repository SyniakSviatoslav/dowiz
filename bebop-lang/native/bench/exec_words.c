#define _POSIX_C_SOURCE 200809L
/* exec_words.c -- run AArch64 word streams produced by `bebopc compilewords`.
 *
 * usage: exec_words <words.txt> [iters]
 *   words.txt: line 1 = count, then one decimal word per line.
 * Calls the code R times, timing each run with CLOCK_MONOTONIC, and prints
 * "result=<x> iters=<n> total_ns=<t>".
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>
#include <sys/mman.h>

typedef long (*fn1)(void);

static uint64_t now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: %s words.txt [iters]\n", argv[0]); return 2; }
    FILE *f = fopen(argv[1], "r");
    if (!f) { perror("open"); return 2; }
    int n = 0;
    if (fscanf(f, "%d", &n) != 1 || n <= 0 || n > (1 << 22)) { fprintf(stderr, "bad count\n"); return 2; }
    uint32_t *code = mmap(NULL, ((size_t)n * 4 + 4095) & ~4095ul,
                          PROT_READ | PROT_WRITE | PROT_EXEC,
                          MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (code == MAP_FAILED) { perror("mmap"); return 2; }
    for (int i = 0; i < n; i++) {
        unsigned long w;
        if (fscanf(f, "%lu", &w) != 1) { fprintf(stderr, "short read at %d\n", i); return 2; }
        memcpy((char *)code + (size_t)i * 4, &w, 4);
    }
    fclose(f);

    /* optional third arg: entry word offset (default 0) */
    int entry = 0;
    if (argc > 3) {
        char *line = NULL; size_t lc = 0;
        FILE *mf = fopen(argv[3], "r");
        if (!mf) { perror("manifest"); return 2; }
        while (getline(&line, &lc, mf) != -1)
            if (strncmp(line, "OFF", 3) == 0) {
                int cnt = 0; unsigned long off = 0;
                char *p = line + 3, *ne;
                while (*p == ' ') p++;
                cnt = (int)strtol(p, &ne, 10);
                for (int k = 0; k < cnt; k++) {
                    off = strtoul(ne, &ne, 10);
                }
                entry = (int)off;
            }
        fclose(mf);
        free(line);
    }

    int iters = argc > 2 ? atoi(argv[2]) : 100;
    fn1 fp = (fn1)((char *)code + (size_t)entry * 4);

    /* warmup + reference result (calls are pure) */
    volatile long sink = fp();
    long ref = fp();

    static uint64_t runs[4096];
    if (iters > 4096) iters = 4096;
    long acc = 0;
    for (int i = 0; i < iters; i++) {
        uint64_t t0 = now_ns();
        acc += fp();
        runs[i] = now_ns() - t0;
    }
    printf("result=%ld\n", ref);
    printf("ns");
    for (int i = 0; i < iters; i++) printf(" %llu", (unsigned long long)runs[i]);
    printf("\n");
    return 0;
}
