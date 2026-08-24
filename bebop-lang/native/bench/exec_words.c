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

/* Bump-arena cursor/end handed to native zeros() via fixed registers.
 * Global asm register variables: GCC reserves x27/x28 for the whole unit. */
register unsigned long g_x27 asm("x27");
register unsigned long g_x28 asm("x28");

/* Invoke JIT code with the bump-arena contract: x27 = cursor, x28 = end.
 * Local register vars inside ONE asm that also issues blr guarantee the
 * values are in the physical registers at call time no matter what -O
 * level or libc calls happen around us. Caller-saved regs are clobbered
 * per AAPCS; x27/x28 survive because the callee saves/restores them. */
static long call_jit(fn1 fp) {
    unsigned long c27 = g_x27;
    unsigned long c28 = g_x28;
    register long ret __asm__("x0");
    /* movs + blr in ONE asm block: nothing (libc, GCC itself) can touch
     * x27/x28 between loading them and entering JIT code. */
    __asm__ volatile(
        "mov x27, %1\n\t"
        "mov x28, %2\n\t"
        "blr %3\n\t"
        "mov %0, x0"
        : "=r"(ret)
        : "r"(c27), "r"(c28), "r"(fp)
        : "x1", "x2", "x3", "x4", "x5", "x6", "x7", "x8",
          "x9", "x10", "x11", "x12", "x13", "x14", "x15",
          "x16", "x17", "x18", "x30", "cc", "memory");
    return ret;
}

static uint64_t now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: %s words.txt [iters]\n", argv[0]); return 2; }
    /* Two input formats:
     *   .bin — raw flat binary of little-endian AArch64 words (the
     *          deployable artifact: count = filesize/4, entry = 0)
     *   otherwise — decimal text: count line, then words, optional OFF */
    int raw = (size_t)(strlen(argv[1]) - 4) < strlen(argv[1]) &&
              strcmp(argv[1] + strlen(argv[1]) - 4, ".bin") == 0;
    uint32_t *code;
    int n;
    if (raw) {
        FILE *fb = fopen(argv[1], "rb");
        if (!fb) { perror("open"); return 2; }
        fseek(fb, 0, SEEK_END);
        long sz = ftell(fb);
        fseek(fb, 0, SEEK_SET);
        n = (int)(sz / 4);
        if (n <= 0 || n > (1 << 22)) { fprintf(stderr, "bad size\n"); return 2; }
        code = mmap(NULL, ((size_t)n * 4 + 4095) & ~4095ul,
                    PROT_READ | PROT_WRITE | PROT_EXEC,
                    MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (code == MAP_FAILED) { perror("mmap"); return 2; }
        if (fread(code, 4, (size_t)n, fb) != (size_t)n) { fprintf(stderr, "short read\n"); return 2; }
        fclose(fb);
    } else {
    FILE *f = fopen(argv[1], "r");
    if (!f) { perror("open"); return 2; }
    n = 0;
    if (fscanf(f, "%d", &n) != 1 || n <= 0 || n > (1 << 22)) { fprintf(stderr, "bad count\n"); return 2; }
    code = mmap(NULL, ((size_t)n * 4 + 4095) & ~4095ul,
                PROT_READ | PROT_WRITE | PROT_EXEC,
                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (code == MAP_FAILED) { perror("mmap"); return 2; }
    for (int i = 0; i < n; i++) {
        unsigned long w;
        if (fscanf(f, "%lu", &w) != 1) { fprintf(stderr, "short read at %d\n", i); return 2; }
        memcpy((char *)code + (size_t)i * 4, &w, 4);
    }
    fclose(f);
    }
    __builtin___clear_cache((char *)code, (char *)code + (size_t)n * 4);

    /* optional third arg:
     *   raw .bin : entry word offset as a plain number
     *   text/.full: path to a manifest file carrying OFF lines */
    int entry = 0;
    if (raw && argc > 3) { entry = atoi(argv[3]); }
    if (argc <= 3) {
        size_t fl = strlen(argv[1]);
        if (fl > 5 && strcmp(argv[1] + fl - 5, ".full") == 0) {
            FILE *mf = fopen(argv[1], "r");
            if (mf) {
                char *line = NULL; size_t lc = 0;
                while (getline(&line, &lc, mf) != -1)
                    if (strncmp(line, "OFF", 3) == 0) {
                        int cnt = 0; unsigned long off = 0;
                        char *q = line + 3, *ne;
                        cnt = (int)strtol(q, &ne, 10);
                        for (int k = 0; k < cnt; k++) off = strtoul(ne, &ne, 10);
                        entry = (int)off;
                    }
                free(line); fclose(mf);
            }
        }
    }
    if (raw) {
        /* entry already taken from argv[3] above */
    } else if (argc > 3) {
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

    /* Bump arena for native zeros(): cursor in x27, end in x28. Monotonic
     * for the whole process run; never reset across nested calls. */
    enum { ARENA_BYTES = 64 << 20 };
    static char *arena_base;
    arena_base = mmap(NULL, ARENA_BYTES, PROT_READ | PROT_WRITE,
                      MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (arena_base == MAP_FAILED) { perror("arena"); return 2; }
    int iters = argc > 2 ? atoi(argv[2]) : 100;
    fn1 fp = (fn1)((char *)code + (size_t)entry * 4);
    fprintf(stderr, "JITBASE=%p entry=%d\n", (void*)code, entry);
    g_x27 = (unsigned long)arena_base;
    g_x28 = (unsigned long)arena_base + ARENA_BYTES;

    /* warmup + reference result (calls are pure) */
    volatile long sink = call_jit(fp); (void)sink;
    long ref = call_jit(fp);

    static uint64_t runs[4096];
    if (iters > 4096) iters = 4096;
    long acc = 0;
    for (int i = 0; i < iters; i++) {
        uint64_t t0 = now_ns();
        acc += call_jit(fp);
        runs[i] = now_ns() - t0;
    }
    printf("result=%ld\n", ref);
    printf("ns");
    for (int i = 0; i < iters; i++) printf(" %llu", (unsigned long long)runs[i]);
    printf("\n");
    return 0;
}
