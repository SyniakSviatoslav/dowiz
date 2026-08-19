/* Bebop power_telemetry — implementation. */
#define _POSIX_C_SOURCE 200809L
#include "power_telemetry.h"
#include "adc.h"
#include <stdio.h>
#include <string.h>
#include <time.h>

/* Single global telemetry accumulator (per-core on bare-metal; one here). */
static PowerTelemetry g_pt;

void pt_init(PowerTelemetry *t) {
    memset(t, 0, sizeof *t);
}

PowerTelemetry *pt_global(void) { return &g_pt; }

uint64_t pt_timestamp_ns(void) {
    struct timespec ts;
    clock_gettime(1 /* CLOCK_MONOTONIC */, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

double pt_sample(PowerTelemetry *t, double v, double i, double dt) {
    double p = v * i;
    if (dt < 0.0) dt = 0.0;
    t->voltage_v = v;
    t->current_a = i;
    t->power_w = p;
    t->energy_j += p * dt;
    t->energy_wh = t->energy_j / 3600.0;
    t->charge_mah += i * dt / 3.6;
    if (t->ops > 0) t->energy_per_op_j = t->energy_j / (double)t->ops;
    if (t->budget_w > 0.0 && p > t->budget_w) t->alerts++;
    t->samples++;
    return p;
}

double pt_sample_timed(PowerTelemetry *t, double v, double i) {
    uint64_t now = pt_timestamp_ns();
    double dt = 0.0;
    if (t->last_ts_ns != 0 && now >= t->last_ts_ns) {
        dt = (double)(now - t->last_ts_ns) / 1e9;
    }
    t->last_ts_ns = now;
    return pt_sample(t, v, i, dt);
}

double pt_sample_adc(PowerTelemetry *t, double dt) {
    return pt_sample(t, adc_read_voltage(), adc_read_current(), dt);
}

void pt_set_cpu_freq(PowerTelemetry *t, double f) { t->cpu_freq_hz = f; }
void pt_set_temp(PowerTelemetry *t, double c) { t->temp_c = c; }
void pt_account_op(PowerTelemetry *t, uint64_t n) {
    t->ops += n;
    if (t->ops > 0) t->energy_per_op_j = t->energy_j / (double)t->ops;
}
void pt_set_budget(PowerTelemetry *t, double b) { t->budget_w = b; }
int pt_alert(const PowerTelemetry *t) {
    return (t->budget_w > 0.0 && t->power_w > t->budget_w) ? 1 : 0;
}

/* syscall interface for .bp: returns a value scaled to integer.
 * op: 0=power mW, 1=energy mJ, 2=voltage mV, 3=current mA, 4=mAh, 5=alerts, 6=J/op uJ */
long bp_power_get(int op) {
    switch (op) {
        case 0: return (long)(g_pt.power_w * 1000.0);
        case 1: return (long)(g_pt.energy_j * 1000.0);
        case 2: return (long)(g_pt.voltage_v * 1000.0);
        case 3: return (long)(g_pt.current_a * 1000.0);
        case 4: return (long)(g_pt.charge_mah * 1000.0);
        case 5: return (long)g_pt.alerts;
        case 6: return (long)(g_pt.energy_per_op_j * 1000000.0);
        case 7: return (long)g_pt.samples;
        default: return 0;
    }
}

int pt_report(const PowerTelemetry *t, char *out, size_t cap) {
    return snprintf(out, cap,
        "V=%.3fV I=%.3fA P=%.3fW E=%.3fJ %.6fWh %.3fmAh %.0fHz %.1fC %.3fJ/op alerts=%lu",
        t->voltage_v, t->current_a, t->power_w, t->energy_j, t->energy_wh,
        t->charge_mah, t->cpu_freq_hz, t->temp_c, t->energy_per_op_j, t->alerts);
}

int pt_serialize(const PowerTelemetry *t, char *out, size_t cap) {
    return snprintf(out, cap,
        "pt V=%.3f I=%.3f P=%.3f J=%.3f Wh=%.6f mAh=%.3f f=%.0f C=%.1f jop=%.6f n=%lu a=%lu",
        t->voltage_v, t->current_a, t->power_w, t->energy_j, t->energy_wh,
        t->charge_mah, t->cpu_freq_hz, t->temp_c, t->energy_per_op_j,
        t->samples, t->alerts);
}

int pt_self_test(char *out, size_t cap) {
    size_t p = 0; int ok = 1;
    PowerTelemetry t; pt_init(&t);
    double pw = pt_sample(&t, 5.0, 2.0, 1.0);
    p += snprintf(out+p, cap-p, "[%s] P=10W\n", (pw > 9.999 && pw < 10.001) ? "ok" : "FAIL");
    if (!(pw > 9.999 && pw < 10.001)) ok = 0;
    p += snprintf(out+p, cap-p, "[%s] mAh=0.5556\n", (t.charge_mah > 0.5555 && t.charge_mah < 0.5557) ? "ok" : "FAIL");
    if (!(t.charge_mah > 0.5555 && t.charge_mah < 0.5557)) ok = 0;
    pt_sample(&t, 3.3, 0.5, 2.0);
    p += snprintf(out+p, cap-p, "[%s] E=13.3J\n", (t.energy_j > 13.29 && t.energy_j < 13.31) ? "ok" : "FAIL");
    if (!(t.energy_j > 13.29 && t.energy_j < 13.31)) ok = 0;
    p += snprintf(out+p, cap-p, "[%s] mAh=0.8333\n", (t.charge_mah > 0.8332 && t.charge_mah < 0.8334) ? "ok" : "FAIL");
    if (!(t.charge_mah > 0.8332 && t.charge_mah < 0.8334)) ok = 0;
    pt_account_op(&t, 100);
    p += snprintf(out+p, cap-p, "[%s] J/op=0.133\n", (t.energy_per_op_j > 0.1329 && t.energy_per_op_j < 0.1331) ? "ok" : "FAIL");
    if (!(t.energy_per_op_j > 0.1329 && t.energy_per_op_j < 0.1331)) ok = 0;
    pt_set_budget(&t, 5.0);
    pt_sample(&t, 5.0, 2.0, 1.0);
    p += snprintf(out+p, cap-p, "[%s] alert fired\n", (pt_alert(&t) == 1 && t.alerts == 1) ? "ok" : "FAIL");
    if (!(pt_alert(&t) == 1 && t.alerts == 1)) ok = 0;
    pt_set_cpu_freq(&t, 1600000000.0);
    pt_set_temp(&t, 45.0);
    p += snprintf(out+p, cap-p, "[%s] freq/temp set\n", (t.cpu_freq_hz == 1600000000.0 && t.temp_c == 45.0) ? "ok" : "FAIL");
    if (!(t.cpu_freq_hz == 1600000000.0 && t.temp_c == 45.0)) ok = 0;
    uint64_t a = pt_timestamp_ns();
    uint64_t b = pt_timestamp_ns();
    p += snprintf(out+p, cap-p, "[%s] monotonic ts\n", (b >= a) ? "ok" : "FAIL");
    if (b < a) ok = 0;
    /* syscall getter: sample into the GLOBAL accumulator, then read back */
    pt_init(&g_pt);
    pt_sample(&g_pt, 5.0, 2.0, 1.0); /* 10W, 2A for 1s -> 0.5556 mAh */
    p += snprintf(out+p, cap-p, "[%s] power_get P=10000mW\n", (bp_power_get(0) == 10000) ? "ok" : "FAIL");
    if (bp_power_get(0) != 10000) ok = 0;
    p += snprintf(out+p, cap-p, "[%s] power_get mAh\n", (bp_power_get(4) >= 555 && bp_power_get(4) <= 556) ? "ok" : "FAIL");
    if (!(bp_power_get(4) >= 555 && bp_power_get(4) <= 556)) ok = 0;
    char rep[256]; pt_report(&t, rep, sizeof rep);
    p += snprintf(out+p, cap-p, "[ok] report: %s\n", rep);
    return ok ? 0 : 1;
}
