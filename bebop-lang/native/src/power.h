/* Bebop power management — WFI/WFE, PMU, affinity, energy metering. */
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

/* Run the power self-test. Returns 0 on success. */
int power_self_test(char *out, size_t cap);

#endif /* BEBOP_POWER_H */
