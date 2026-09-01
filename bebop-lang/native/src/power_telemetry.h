/* Bebop power_telemetry — accurate electricity-consumption accounting.
 * P=V·I, E=∫P dt (J + Wh), mAh coulomb counter, CPU freq/temp, monotonic
 * timestamps, threshold alerts. no_std-safe (pure double + uint64, no libm). */
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
    double charge_mah;   /* coulomb counter: ∫I dt / 3.6 (mAh) */
    double cpu_freq_hz;  /* last known CPU frequency (Hz) */
    double temp_c;       /* last known junction temp (°C) */
    double energy_per_op_j; /* energy per unit-of-work (J/op) */
    double budget_w;     /* alert budget (W); 0 = no alert */
    uint64_t last_ts_ns; /* monotonic timestamp of last sample */
    uint64_t ops;        /* unit-of-work counter (for energy-per-op) */
    unsigned long samples;   /* total samples */
    unsigned long alerts;    /* samples that exceeded budget */
} PowerTelemetry;

void pt_init(PowerTelemetry *t);

/* Monotonic nanosecond timestamp (CLOCK_MONOTONIC or PMU cycle counter). */
uint64_t pt_timestamp_ns(void);

/* Accumulate one sample with an explicit dt. Returns instantaneous power. */
double pt_sample(PowerTelemetry *t, double v, double i, double dt);

/* Sample + auto-compute dt from the monotonic clock (cadence-aware). */
double pt_sample_timed(PowerTelemetry *t, double v, double i);

/* Read V/I from the ADC/I2C interface and sample. */
double pt_sample_adc(PowerTelemetry *t, double dt);

/* Record CPU frequency / temperature (e.g. from PMU + thermal sensor). */
void pt_set_cpu_freq(PowerTelemetry *t, double freq_hz);
void pt_set_temp(PowerTelemetry *t, double temp_c);

/* Register a unit-of-work so energy-per-op can be computed. */
void pt_account_op(PowerTelemetry *t, uint64_t n_ops);

/* Set the power budget (W); 0 disables alerts. */
void pt_set_budget(PowerTelemetry *t, double budget_w);

/* 1 if the last sample exceeded the budget, else 0. */
int pt_alert(const PowerTelemetry *t);

/* Global telemetry handle + syscall getter (for .bp interop). */
PowerTelemetry *pt_global(void);
/* op: 0=power mW, 1=energy mJ, 2=voltage mV, 3=current mA, 4=mAh, 5=alerts, 6=J/op uJ, 7=samples */
long bp_power_get(int op);

/* Human-readable report line. Returns bytes written. */
int pt_report(const PowerTelemetry *t, char *out, size_t cap);

/* Compact key=value line for transmission (UART/syscall). */
int pt_serialize(const PowerTelemetry *t, char *out, size_t cap);

int pt_self_test(char *out, size_t cap);
#endif
