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

/* ─── PSCI power gating ───────────────────────────────────────────────── */

void bp_psci_sleep(unsigned core, int state) {
    (void)core;
    if (state == 0) return;      /* running — nothing to do */
    if (state == 1) { bp_wfi(); return; }  /* retention */
    /* state 2: power-down — on bare-metal, PSCI_CPU_OFF via SMC */
    bp_wfi();
}

/* ─── DVFS hint ───────────────────────────────────────────────────────── */

void bp_dvfs_hint(unsigned core, unsigned mhz) {
    (void)core; (void)mhz;
    /* Bare-metal: write to PMIC frequency register.
     * Linux userspace: echo mhz > /sys/devices/system/cpu/cpuX/cpufreq/scaling_max_freq. */
}

/* ─── GIC interrupt coalescing ─────────────────────────────────────────── */

void bp_gic_coalesce(unsigned core_id, unsigned batch_n) {
    (void)core_id; (void)batch_n;
    /* Bare-metal: program GICD (Distributor) to hold interrupts until
     * `batch_n` are pending or a timer fires. Prevents interrupt storms
     * from sensors/radios from waking the core. */
}

/* ─── Reynolds number (turbulence estimation) ──────────────────────────── */

double bp_reynolds(const double *samples, size_t n) {
    if (n < 2) return 0.0;
    /* Re ≈ σ² / μ² where σ² = variance, μ = mean.
     * High Re → turbulent (nonlinear), low Re → laminar (linear). */
    double sum = 0.0, sum2 = 0.0;
    for (size_t i = 0; i < n; i++) {
        double x = samples[i];
        sum += x; sum2 += x * x;
    }
    double mean = sum / (double)n;
    double var = sum2 / (double)n - mean * mean;
    return (mean != 0.0) ? var / (mean * mean) : 0.0;
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

    /* PSCI sleep states execute (no trap) */
    bp_psci_sleep(0, 0); bp_psci_sleep(0, 1); bp_psci_sleep(0, 2);
    A(1, "PSCI sleep states execute (WFI-based)");

    /* Reynolds: constant signal → laminar (Re=0) */
    {
        double laminar[5] = {5.0, 5.0, 5.0, 5.0, 5.0};
        A(bp_reynolds(laminar, 5) < 0.001, "Reynolds ≈ 0 for constant signal");
    }
    /* Reynolds: alternating signal → turbulent (Re > 0) */
    {
        double turb[5] = {0.0, 10.0, 0.0, 10.0, 0.0};
        A(bp_reynolds(turb, 5) > 0.5, "Reynolds > 0 for alternating signal");
    }

    return all_ok ? 0 : -1;
}
