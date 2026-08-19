/* Bebop startup telemetry — binary size, exec→main latency, per-module
 * self-test timing. stdio-only (no sys/stat, no libm); the host links libc. */
#define _POSIX_C_SOURCE 200809L

#include "startup.h"

#include "adc.h"
#include "aes_gcm.h"
#include "arena.h"
#include "atomic.h"
#include "autonomic.h"
#include "calyx.h"
#include "chain.h"
#include "checksum.h"
#include "codegen.h"
#include "complex.h"
#include "compute.h"
#include "comptime.h"
#include "contract.h"
#include "effect.h"
#include "event.h"
#include "fft.h"
#include "fmt.h"
#include "graph.h"
#include "gt.h"
#include "hex_util.h"
#include "hydra.h"
#include "hyper.h"
#include "jittable.h"
#include "lmem.h"
#include "markov.h"
#include "math_native.h"
#include "mem.h"
#include "memristor.h"
#include "mesh.h"
#include "modular.h"
#include "money.h"
#include "native.h"
#include "noether.h"
#include "ntt.h"
#include "ntt32.h"
#include "oracle.h"
#include "pac.h"
#include "pid.h"
#include "pool.h"
#include "power.h"
#include "power_telemetry.h"
#include "pq.h"
#include "qtt.h"
#include "rng.h"
#include "session.h"
#include "sha256.h"
#include "smt.h"
#include "sort.h"
#include "spectral.h"
#include "stats.h"
#include "supervise.h"
#include "syscall.h"
#include "tensor.h"
#include "termination.h"
#include "tls.h"
#include "token_bucket.h"
#include "trig.h"
#include "typereflect.h"
#include "typereg.h"
#include "verify.h"
#include "verifier.h"
#include "vir.h"
#include "vsa.h"
#include "x25519.h"
#include "x86_64.h"
#include "zlib.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

/* Earliest user code: runs before main, after libc/loader init. */
static uint64_t g_ctor_ns;
static uint64_t g_main_ns;

__attribute__((constructor(101))) static void startup_ctor(void) {
    g_ctor_ns = bp_mono_ns();
}

uint64_t bp_mono_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

void bp_startup_mark_main(void) {
    g_main_ns = bp_mono_ns();
}

uint64_t bp_startup_ns(void) {
    if (g_main_ns <= g_ctor_ns) return 0;
    return g_main_ns - g_ctor_ns;
}

uint64_t bp_binary_size(void) {
    FILE *f = fopen("/proc/self/exe", "rb");
    if (!f) return 0;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return 0; }
    long sz = ftell(f);
    fclose(f);
    return sz > 0 ? (uint64_t)sz : 0;
}

uint64_t bp_exec_ns(void) {
    FILE *f = fopen("/proc/self/stat", "r");
    if (!f) return 0;
    char buf[1024];
    size_t n = fread(buf, 1, sizeof buf - 1, f);
    fclose(f);
    if (n == 0) return 0;
    buf[n] = '\0';
    /* field 2 (comm) may contain spaces/parens; skip to the last ')' */
    char *p = strrchr(buf, ')');
    if (!p) return 0;
    /* starttime = field 22; fields 3..22 follow ')', i.e. the 20th token */
    char *tok = strtok(p + 1, " ");
    for (int fld = 3; tok && fld < 22; fld++) tok = strtok(NULL, " ");
    if (!tok) return 0;
    unsigned long long ticks = strtoull(tok, NULL, 10);
    if (ticks == 0) return 0;
    long hz = sysconf(_SC_CLK_TCK);
    if (hz <= 0) hz = 100;
    unsigned long long start_ns =
        ticks * (1000000000ULL / (unsigned long long)hz);
    uint64_t now = bp_mono_ns();
    if (now <= start_ns) return 0;
    return now - start_ns;
}

/* ─── per-module self-test timing table ─────────────────────────────── */

static const BpStartupModule MODS[] = {
    {"qtt", qtt_self_test},
    {"qtt_check", qtt_check_test},
    {"qtt_eval", qtt_eval_test},
    {"qtt_struct", qtt_struct_test},
    {"qtt_enum", qtt_enum_test},
    {"qtt_dep", qtt_dep_test},
    {"qtt_effect", qtt_effect_test},
    {"qtt_conv", qtt_conv_test},
    {"qtt_proof", qtt_proof_test},
    {"qtt_nat", qtt_nat_test},
    {"qtt_str", qtt_str_test},
    {"qtt_array", qtt_array_test},
    {"qtt_universe", qtt_universe_test},
    {"termination", qtt_termination_test},
    {"verify", verify_self_test},
    {"verifier", verifier_self_test},
    {"vsa", vsa_self_test},
    {"typereg", typereg_self_test},
    {"codegen", codegen_self_test},
    {"native", native_self_test},
    {"vir", vir_self_test},
    {"vir_atomic", vir_atomic_self_test},
    {"x86_64", x86_64_self_test},
    {"gt", gt_self_test},
    {"ntt", ntt_self_test},
    {"ntt32", ntt32_self_test},
    {"hyper", hyper_self_test},
    {"mem", mem_self_test},
    {"lmem", lmem_self_test},
    {"hydra", hydra_self_test},
    {"fft", fft_self_test},
    {"money", money_self_test},
    {"arena", arena_self_test},
    {"event", event_self_test},
    {"modular", modular_self_test},
    {"complex", complex_self_test},
    {"sort", sort_self_test},
    {"token_bucket", token_bucket_self_test},
    {"checksum", checksum_self_test},
    {"hex", hex_self_test},
    {"trig", trig_self_test},
    {"rng", rng_self_test},
    {"stats", stats_self_test},
    {"pid", pid_self_test},
    {"markov", markov_self_test},
    {"spectral", spectral_self_test},
    {"autonomic", autonomic_self_test},
    {"noether", noether_self_test},
    {"math", math_self_test},
    {"graph", graph_self_test},
    {"chain", chain_self_test},
    {"tensor", tensor_self_test},
    {"oracle", oracle_self_test},
    {"mesh", mesh_self_test},
    {"pool", pool_self_test},
    {"atomic", atomic_self_test},
    {"smt", smt_self_test},
    {"contract", contract_self_test},
    {"comptime", comptime_self_test},
    {"fmt", fmt_self_test},
    {"power", power_self_test},
    {"syscall", syscall_self_test},
    {"typereflect", typereflect_self_test},
    {"session", session_self_test},
    {"supervise", supervise_self_test},
    {"effect", effect_self_test},
    {"jittable", jittable_self_test},
    {"pac", pac_self_test},
    {"pq", pq_self_test},
    {"zlib", zlib_self_test},
    {"sha256", sha256_self_test},
    {"x25519", x25519_self_test},
    {"aes_gcm", aes_gcm_self_test},
    {"tls", tls_self_test},
    {"calyx", calyx_self_test},
    {"memristor", memristor_self_test},
    {"adc", adc_self_test},
    {"compute", compute_self_test},
    {"pt", pt_self_test},
};

#define N_MODS (sizeof MODS / sizeof MODS[0])

int bp_startup_report(void) {
    uint64_t bin = bp_binary_size();
    uint64_t exec_ns = bp_exec_ns();
    uint64_t startup_ns = bp_startup_ns();

    printf("bebopc binary size   : %llu bytes (%.2f KiB)\n",
           (unsigned long long)bin, (double)bin / 1024.0);
    printf("startup exec->main   : %llu ns (%.2f us)  [kernel starttime]\n",
           (unsigned long long)exec_ns, (double)exec_ns / 1000.0);
    printf("startup ctor->main   : %llu ns (%.2f us)  [pre-main init]\n",
           (unsigned long long)startup_ns, (double)startup_ns / 1000.0);

    char out[16384];
    int fails = 0;
    uint64_t total_ns = 0;

    printf("\n%-18s %6s %10s\n", "module", "status", "time");
    printf("-------------------------------------------\n");
    for (size_t i = 0; i < N_MODS; i++) {
        uint64_t t0 = bp_mono_ns();
        int ok = MODS[i].test(out, sizeof out);
        uint64_t t1 = bp_mono_ns();
        uint64_t dt = t1 - t0;
        total_ns += dt;
        if (ok != 0) fails++;
        printf("%-18s %6s %8.2f us\n", MODS[i].name,
               ok == 0 ? "ok" : "FAIL", (double)dt / 1000.0);
    }
    printf("-------------------------------------------\n");
    printf("%-18s %6s %8.2f us  (%zu modules, %d failing)\n",
           "total", "", (double)total_ns / 1000.0, N_MODS, fails);
    return fails;
}
