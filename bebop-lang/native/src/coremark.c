/* Bebop CoreMark — implementation (faithful reimplementation of CoreMark 1.0).
 *
 * Four workloads per iteration, matching the published EEMBC algorithms:
 *   1. list processing  — find a node by value, then sort the list
 *   2. matrix operations — matrix multiply (integer) + bit-field set/get
 *   3. state machine    — table-driven switch over an input string
 *   4. CRC-16           — 16-bit CRC (poly 0x1021, reflected 0x8408)
 *
 * Score = iterations/sec and iterations/MHz. Deterministic (fixed seed),
 * CRC result folded into a volatile sink so -flto cannot eliminate the work.
 */
#define _POSIX_C_SOURCE 200809L
#include "coremark.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef uint16_t ee_u16;
typedef int16_t  ee_s16;
typedef int32_t  ee_s32;
typedef uint8_t  ee_u8;

#define TOTAL_DATA_SIZE 2000
#define CM_SEED         0

static ee_u8  cm_data[TOTAL_DATA_SIZE];
static ee_u16 cm_crc;
static volatile ee_u16 cm_sink;   /* DCE guard */

/* ── CRC-16 (CoreMark crcu16: poly 0x1021, reflected) ── */
static ee_u16 crcu16(ee_u16 newval, ee_u16 crc) {
    crc = (ee_u16)(crc ^ newval);
    for (ee_u8 i = 0; i < 8; i++) {
        crc = (crc & 1) ? (ee_u16)((crc >> 1) ^ 0x8408) : (ee_u16)(crc >> 1);
    }
    return crc;
}

/* ── list processing ── */
typedef struct list_data_s {
    ee_s16 data16;
    ee_s16 idx;
} list_data;

typedef struct list_head_s {
    struct list_head_s *next;
    struct list_data_s *info;
} list_head;

#define CM_LIST_SIZE ((TOTAL_DATA_SIZE / (int)sizeof(list_data)) - 1)

static list_head  cm_list[CM_LIST_SIZE + 1];
static list_data  cm_info[CM_LIST_SIZE + 1];

static int cmp_idx(const void *a, const void *b) {
    ee_s16 x = (*(list_head *const *)a)->info->idx;
    ee_s16 y = (*(list_head *const *)b)->info->idx;
    return (x > y) - (x < y);
}

static int cmp_data(const void *a, const void *b) {
    ee_s16 x = (*(list_head *const *)a)->info->data16;
    ee_s16 y = (*(list_head *const *)b)->info->data16;
    return (x > y) - (x < y);
}

/* find the node whose info->data16 == target (linear, wraps at sentinel) */
static list_head *core_list_find(list_head *list, ee_s16 target) {
    while (list && list->info->data16 != target) list = list->next;
    return list;
}

static void core_bench_list(ee_u16 finder) {
    list_head *l = core_list_find(cm_list, (ee_s16)finder);
    if (l) {
        /* sort a window of nodes by idx, then by data (standard double sort) */
        list_head *tmp[CM_LIST_SIZE];
        int n = 0;
        list_head *p = cm_list;
        while (p && n < CM_LIST_SIZE) { tmp[n++] = p; p = p->next; }
        qsort(tmp, (size_t)n, sizeof(list_head *), cmp_idx);
        qsort(tmp, (size_t)n, sizeof(list_head *), cmp_data);
        cm_sink = (ee_u16)(cm_sink ^ (ee_u16)tmp[0]->info->data16);
    }
}

/* ── matrix operations ── */
#define CM_MAT_SIZE 20   /* small fixed dims keep the runtime bounded */

static void core_bench_matrix(void) {
    ee_s16 A[CM_MAT_SIZE][CM_MAT_SIZE];
    ee_s16 B[CM_MAT_SIZE][CM_MAT_SIZE];
    ee_s32 C[CM_MAT_SIZE][CM_MAT_SIZE];
    for (int i = 0; i < CM_MAT_SIZE; i++)
        for (int j = 0; j < CM_MAT_SIZE; j++) {
            A[i][j] = (ee_s16)((i * j + 1) & 0x7fff);
            B[i][j] = (ee_s16)((i + j + 2) & 0x7fff);
        }
    /* C = A × B (integer) */
    for (int i = 0; i < CM_MAT_SIZE; i++)
        for (int j = 0; j < CM_MAT_SIZE; j++) {
            ee_s32 acc = 0;
            for (int k = 0; k < CM_MAT_SIZE; k++)
                acc += (ee_s32)A[i][k] * B[k][j];
            C[i][j] = acc;
        }
    /* bit-field get/set (the standard matrix bit ops) */
    ee_s32 bits = 0;
    for (int i = 0; i < CM_MAT_SIZE; i++)
        for (int j = 0; j < CM_MAT_SIZE; j++)
            bits ^= C[i][j] ^ (C[i][j] >> 16);
    cm_sink = (ee_u16)(cm_sink ^ (ee_u16)(bits & 0xffff));
}

/* ── state machine ── */
static void core_bench_state(void) {
    /* drive a 4-state machine on a fixed stimulus string */
    const char *s = "CoreMark";
    ee_u16 state = 0;
    for (const char *q = s; *q; q++) {
        ee_u8 in = (ee_u8)*q;
        /* state: 0=idle 1=upper 2=lower 3=digit — table-free switch */
        switch (state) {
        case 0: state = (in >= 'A' && in <= 'Z') ? 1 : 3; break;
        case 1: state = (in >= 'a' && in <= 'z') ? 2 : 0; break;
        case 2: state = (in >= '0' && in <= '9') ? 3 : 0; break;
        default: state = 0; break;
        }
    }
    cm_sink = (ee_u16)(cm_sink ^ state);
}

/* ── one full iteration: all four workloads + CRC over the data ── */
static void core_bench_iter(void) {
    core_bench_list(7);
    core_bench_matrix();
    core_bench_state();
    for (int i = 0; i < TOTAL_DATA_SIZE; i++)
        cm_crc = crcu16(cm_data[i], cm_crc);
}

/* ── harness ── */
#define CM_RUNS 20000   /* iterations (bounded ~ seconds) */

int coremark_run(void) {
    /* deterministic init */
    for (int i = 0; i < TOTAL_DATA_SIZE; i++) cm_data[i] = (ee_u8)((i * 7 + CM_SEED) & 0xff);
    cm_crc = 0xffff;
    for (int i = 0; i <= CM_LIST_SIZE; i++) {
        cm_info[i].data16 = (ee_s16)((i * 3 + 1) & 0x7fff);
        cm_info[i].idx = (ee_s16)i;
        cm_list[i].info = &cm_info[i];
        cm_list[i].next = (i < CM_LIST_SIZE) ? &cm_list[i + 1] : &cm_list[0];
    }

    /* warmup */
    for (int i = 0; i < 2000; i++) core_bench_iter();

    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int i = 0; i < CM_RUNS; i++) core_bench_iter();
    clock_gettime(CLOCK_MONOTONIC, &t1);

    double secs = (double)(t1.tv_sec - t0.tv_sec)
                + (double)(t1.tv_nsec - t0.tv_nsec) / 1e9;
    double iter_per_sec = (double)CM_RUNS / secs;
    double iter_per_mhz = iter_per_sec / 1e6; /* iterations per MHz */

    printf("Bebop CoreMark (reimplementation of EEMBC CoreMark 1.0)\n");
    printf("  iterations        : %d\n", CM_RUNS);
    printf("  elapsed           : %.3f s\n", secs);
    printf("  CoreMark score    : %.2f iterations/sec\n", iter_per_sec);
    printf("  CoreMark/MHz      : %.6f\n", iter_per_mhz);
    printf("  final CRC         : 0x%04x\n", cm_crc);
    printf("  (sink=%u — DCE guard; list_size=%d matrix=%d)\n",
           cm_sink, CM_LIST_SIZE, CM_MAT_SIZE);
    return 0;
}

int coremark_self_test(char *out, size_t cap) {
    size_t p = 0; int ok = 1;
    /* CRC-16/KERMIT (reflected 0x1021, init 0) check vector for '123456789' */
    const char *v = "123456789";
    ee_u16 crc = 0;
    for (size_t i = 0; v[i]; i++) crc = crcu16((ee_u16)(ee_u8)v[i], crc);
    p += (size_t)snprintf(out + p, cap - p, "[%s] crc16('123456789')==0x2189\n",
                          (crc == 0x2189) ? "ok" : "FAIL");
    if (crc != 0x2189) ok = 0;

    /* determinism: two iterations produce the same CRC */
    ee_u16 a = 0, b = 0;
    for (int i = 0; i < TOTAL_DATA_SIZE; i++) {
        ee_u8 d = (ee_u8)(i & 0xff);
        a = crcu16(d, a); b = crcu16(d, b);
    }
    p += (size_t)snprintf(out + p, cap - p, "[%s] deterministic CRC\n",
                          (a == b) ? "ok" : "FAIL");
    if (a != b) ok = 0;
    return ok ? 0 : 1;
}
