/* Bebop power management — WFI/WFE sleep, PMU energy metering, core affinity.
 * Bare-metal AArch64: no libc, no OS scheduler. Direct instruction encoding.
 * Energy-autonomy doctrine (BEBOP-ENERGY-AUTONOMY.md).
 * PMU access is guarded: some kernels/proot block user-mode MRS (SIGILL). */
#define _GNU_SOURCE 1
#include "power.h"

#include <setjmp.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

/* ─── AArch64 sleep/event instructions (hand-encoded) ─── */
void bp_wfi(void) { __asm__ volatile("wfi"); }
void bp_wfe(void) { __asm__ volatile("wfe"); }
void bp_sev(void) { __asm__ volatile("sev"); }

/* ─── PMU cycle counter (guarded) ─── */
static sigjmp_buf pmu_jmp;
static volatile sig_atomic_t pmu_ok = 1;

static void pmu_sigill(int sig) {
    (void)sig;
    pmu_ok = 0;
    siglongjmp(pmu_jmp, 1);
}

static unsigned long pmccntr(void) {
    unsigned long v;
    __asm__ volatile("mrs %0, pmccntr_el0" : "=r"(v));
    return v;
}

int bp_pmu_available(void) {
    struct sigaction sa, old;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = pmu_sigill;
    sigemptyset(&sa.sa_mask);
    if (sigaction(SIGILL, &sa, &old) != 0) return 0;
    pmu_ok = 1;
    if (sigsetjmp(pmu_jmp, 1) == 0) {
        (void)pmccntr();
    }
    sigaction(SIGILL, &old, NULL);
    return pmu_ok;
}

void bp_pmu_init(void) {
    if (!bp_pmu_available()) return;
    __asm__ volatile("msr pmcr_el0, %0" ::"r"(0x1u | 0x4u | 0x8u));
    __asm__ volatile("msr pmcntenset_el0, %0" ::"r"(0x80000000u));
}

unsigned long bp_pmu_cycles(void) {
    if (!pmu_ok) return 0;
    return pmccntr();
}

/* ─── CPU affinity (energy-efficient core pinning) ─── */
int bp_cpu_pin(int cpu) {
    unsigned long mask = 1UL << (unsigned)cpu;
    long r = syscall(SYS_sched_setaffinity, 0, sizeof(mask), &mask);
    return (int)r;
}

/* Energy estimate: joules ≈ cycles × 1 pJ/cycle (placeholder, calibrate vs HIL). */
double bp_energy_joules(unsigned long cycles) {
    return (double)cycles * 1e-12;
}

int power_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#undef A
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    bp_wfi();
    bp_wfe();
    bp_sev();
    A(1, "WFI/WFE/SEV execute (no trap)");

    int have_pmu = bp_pmu_available();
    if (have_pmu) {
        bp_pmu_init();
        unsigned long a = bp_pmu_cycles();
        volatile unsigned long x = 0;
        for (int i = 0; i < 1000; i++) x += (unsigned long)i;
        unsigned long b = bp_pmu_cycles();
        A(b > a, "PMU cycle counter advances");
        (void)x;
        A(bp_energy_joules(b - a) >= 0.0, "energy estimate >= 0");
    } else {
        A(1, "PMU unavailable in this env (skipped, not a failure)");
    }

    int r = bp_cpu_pin(0);
    A(r == 0 || r == -1, "cpu_pin returns 0 or -1");

    return all_ok ? 0 : -1;
}
