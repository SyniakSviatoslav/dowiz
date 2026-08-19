/* Bebop power_telemetry — accurate electricity-consumption accounting.
 * Reads V/I (ADC or I2C), computes instantaneous power P=V·I, integrates
 * energy E=∫P dt (joules + watt-hours), and formats a transmittable report.
 * no_std-safe: pure double arithmetic, no libm, no allocation. */
#ifndef BEBOP_POWER_TELEMETRY_H
#define BEBOP_POWER_TELEMETRY_H
#include <stddef.h>
#include <stdint.h>

typedef struct {
    double voltage_v;    /* sampled bus voltage (V) */
    double current_a;    /* sampled current (A) */
    double power_w;      /* instantaneous power P = V·I (W) */
    double energy_j;     /* accumulated energy (J) */
    double energy_wh;    /* accumulated energy (Wh) */
    double dt_s;         /* last sample interval (s) */
    unsigned long samples; /* total samples */
} PowerTelemetry;

/* Initialize an empty telemetry accumulator (zero energy). */
void pt_init(PowerTelemetry *t);

/* Accumulate one sample: P = v*i, E += P*dt. Returns instantaneous power. */
double pt_sample(PowerTelemetry *t, double voltage_v, double current_a, double dt_s);

/* Convenience: sample from the ADC/I2C interface (adc_read_*). */
double pt_sample_adc(PowerTelemetry *t, double dt_s);

/* Format a human+telemetry-readable report line into out (cap bytes).
 * Fields: V, A, W, J, Wh, samples. Returns bytes written (or would-be). */
int pt_report(const PowerTelemetry *t, char *out, size_t cap);

/* Serialize to a compact key=value line for transmission (UART/syscall). */
int pt_serialize(const PowerTelemetry *t, char *out, size_t cap);

int pt_self_test(char *out, size_t cap);
#endif
