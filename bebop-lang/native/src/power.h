/* Bebop power management — WFI/WFE, PMU, affinity, energy metering, telemetry. */
#ifndef BEBOP_POWER_H
#define BEBOP_POWER_H

#include <stddef.h>

/* AArch64 sleep/event primitives (bare-metal). */
void bp_wfi(void);
void bp_wfe(void);
void bp_sev(void);

/* PMU: initialize cycle counter, read cycles since init. */
void bp_pmu_init(void);
unsigned long bp_pmu_cycles(void);
/* Returns 1 if user-mode PMU access is available (guarded, no SIGILL). */
int bp_pmu_available(void);

/* Pin the current thread to CPU `cpu` (energy-efficient core selection). */
int bp_cpu_pin(int cpu);

/* Energy estimate in joules for `cycles` (placeholder 1 pJ/cycle model). */
double bp_energy_joules(unsigned long cycles);

/* PSCI power state IDs: 0=running, 1=retention, 2=power-down. */
void bp_psci_sleep(unsigned core, int state);
/* DVFS hint: target frequency in MHz (0 = hardware default). */
void bp_dvfs_hint(unsigned core, unsigned mhz);
/* GIC interrupt coalescing: batch upto N interrupts before waking core. */
void bp_gic_coalesce(unsigned core_id, unsigned batch_n);
/* Reynolds number: estimate input signal turbulence (scaled Re). */
double bp_reynolds(const double *samples, size_t n);

/* ─── Precise power telemetry ─────────────────────────────────────────── */

/* A single power/energy sample, hardware-precise where available. */
typedef struct {
    unsigned long long cycles;   /* PMU cycle count since bp_pmu_init */
    unsigned freq_mhz;           /* current CPU frequency (from /sys, else model) */
    double voltage_mv;           /* rail voltage in millivolts */
    double current_ma;           /* drawn current in milliamps */
    double power_mw;             /* instantaneous power = V*I */
    double energy_uj;            /* cumulative energy in microjoules */
    unsigned long long ts_ns;    /* CLOCK_MONOTONIC timestamp in ns */
} BpPowerSample;

/* Read the current CPU frequency in MHz (0 = unavailable). */
unsigned bp_power_freq_mhz(void);

/* Sample instantaneous power/energy. Returns 0 on success. */
int bp_power_sample(BpPowerSample *out);

/* Serialize a sample as a JSON telemetry packet into buf. Returns bytes. */
int bp_power_telemetry_json(const BpPowerSample *s, char *buf, size_t cap);

/* ─── Full resource telemetry (power + memory + cpu + gpu + speed) ────── */

typedef struct {
    /* Power */
    unsigned long long cycles;   /* PMU cycles since init */
    unsigned freq_mhz;           /* CPU frequency */
    double voltage_mv, current_ma, power_mw, energy_uj;
    /* Memory (bytes) */
    unsigned long rss_bytes;     /* resident set size */
    unsigned long vm_bytes;      /* virtual memory size */
    /* CPU + GPU utilization (percent 0..100) */
    double cpu_usage_pct;
    double gpu_usage_pct;        /* 0 when no GPU present */
    /* Execution speed */
    unsigned long long ts_ns;    /* CLOCK_MONOTONIC ns */
    double elapsed_ms;           /* wall-clock since last sample */
    double mips;                 /* million instructions/sec estimate */
    double ipc;                  /* instructions per cycle estimate */
} BpTelemetry;

/* Read /proc/self memory usage (RSS + VM) into *rss and *vm. Returns 0. */
int bp_mem_usage(unsigned long *rss, unsigned long *vm);

/* Read CPU utilization since last call (0..100) or -1 on failure. */
double bp_cpu_usage_pct(void);

/* Sample full resource telemetry. Returns 0 on success. */
int bp_telemetry_sample(BpTelemetry *out);

/* Serialize full telemetry as JSON. Returns bytes. */
int bp_telemetry_json(const BpTelemetry *t, char *buf, size_t cap);

/* Run the power self-test. Returns 0 on success. */
int power_self_test(char *out, size_t cap);

#endif /* BEBOP_POWER_H */
