/* Bebop power_telemetry — implementation. */
#include "power_telemetry.h"
#include "adc.h"
#include <stdio.h>
#include <string.h>

void pt_init(PowerTelemetry *t) {
    memset(t, 0, sizeof *t);
}

double pt_sample(PowerTelemetry *t, double v, double i, double dt) {
    double p = v * i;
    if (dt < 0.0) dt = 0.0;
    t->voltage_v = v;
    t->current_a = i;
    t->power_w = p;
    t->dt_s = dt;
    t->energy_j += p * dt;
    t->energy_wh = t->energy_j / 3600.0;
    t->samples++;
    return p;
}

double pt_sample_adc(PowerTelemetry *t, double dt) {
    return pt_sample(t, adc_read_voltage(), adc_read_current(), dt);
}

int pt_report(const PowerTelemetry *t, char *out, size_t cap) {
    return snprintf(out, cap,
        "V=%.3fV I=%.3fA P=%.3fW E=%.3fJ %.6fWh samples=%lu\n",
        t->voltage_v, t->current_a, t->power_w,
        t->energy_j, t->energy_wh, t->samples);
}

int pt_serialize(const PowerTelemetry *t, char *out, size_t cap) {
    return snprintf(out, cap,
        "pt V=%.3f I=%.3f P=%.3f J=%.3f Wh=%.6f n=%lu\n",
        t->voltage_v, t->current_a, t->power_w,
        t->energy_j, t->energy_wh, t->samples);
}

int pt_self_test(char *out, size_t cap) {
    size_t p = 0; int ok = 1;
    PowerTelemetry t; pt_init(&t);
    double pw = pt_sample(&t, 5.0, 2.0, 1.0);
    p += snprintf(out+p, cap-p, "[%s] P=10W\n", (pw > 9.999 && pw < 10.001) ? "ok" : "FAIL");
    if (!(pw > 9.999 && pw < 10.001)) ok = 0;
    pt_sample(&t, 3.3, 0.5, 2.0);
    p += snprintf(out+p, cap-p, "[%s] E=13.3J\n", (t.energy_j > 13.29 && t.energy_j < 13.31) ? "ok" : "FAIL");
    if (!(t.energy_j > 13.29 && t.energy_j < 13.31)) ok = 0;
    p += snprintf(out+p, cap-p, "[%s] Wh conversion\n", (t.energy_wh > 0.00369 && t.energy_wh < 0.00370) ? "ok" : "FAIL");
    if (!(t.energy_wh > 0.00369 && t.energy_wh < 0.00370)) ok = 0;
    pt_sample(&t, 5.0, 1.0, -1.0);
    p += snprintf(out+p, cap-p, "[%s] negative dt clamps\n", (t.energy_j > 13.29 && t.energy_j < 13.31) ? "ok" : "FAIL");
    if (!(t.energy_j > 13.29 && t.energy_j < 13.31)) ok = 0;
    p += snprintf(out+p, cap-p, "[%s] samples=3\n", (t.samples == 3) ? "ok" : "FAIL");
    if (t.samples != 3) ok = 0;
    char rep[128]; pt_report(&t, rep, sizeof rep);
    p += snprintf(out+p, cap-p, "[ok] report: %s\n", rep);
    return ok ? 0 : 1;
}
