/* Bebop energy-per-operation benchmark (checklist §11).
 *
 * HONEST SOFTWARE MODEL: on a Linux userspace host there is no PMIC/ADC, so
 * this module estimates energy from measured wall-clock time × measured CPU
 * frequency × a nominal energy-per-cycle constant (a documented model
 * parameter, ~1 nJ/cycle for a mid-range AArch64 core at nominal voltage).
 *
 * This yields RELATIVE energy-efficiency (J/op, J/Mop) that is directly
 * comparable between languages measured the same way. It is NOT an absolute
 * hardware power measurement — on bare-metal, replace the cycle-based model
 * with real INA219/INA226 reads via power_telemetry (pt_sample_adc) and the
 * numbers become physical joules.
 */
#ifndef BEBOP_ENERGY_H
#define BEBOP_ENERGY_H
#include <stddef.h>

int energy_run(void);
int energy_self_test(char *out, size_t cap);
#endif
