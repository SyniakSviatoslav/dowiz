/* Bebop pid — implementation (port of dowiz pid.rs). */
#include "pid.h"

#include <stdio.h>
#include <string.h>

/* Single authority for the anti-windup epsilon (matches dowiz KI_EPSILON). */
#define PID_KI_EPSILON 0.001

/* IEEE-754 double: exponent field (bits 62..52) all-ones marks NaN or ±Inf.
 * No <math.h>/<float.h> needed; deterministic under -O2. */
static int pid_is_finite(double v) {
    uint64_t bits;
    memcpy(&bits, &v, sizeof bits);
    return ((bits >> 52) & 0x7FFULL) != 0x7FFULL;
}

/* Non-finite (NaN/±Inf) fails closed to 0.0, matching dowiz sanitize_f64. */
static double pid_sanitize(double v) {
    return pid_is_finite(v) ? v : 0.0;
}

/* nearest integer, ties away from zero (matches f64::round). Safe for any
 * finite input: beyond 2^53 a double has no fractional part. */
static double pid_round(double x) {
    if (x >= 9007199254740992.0 || x <= -9007199254740992.0) {
        return x; /* already integral */
    }
    double t = (double)(long long)x; /* trunc toward zero */
    double frac = x - t;
    if (frac < 0.0) frac = -frac;
    if (frac >= 0.5) {
        return x > 0.0 ? t + 1.0 : t - 1.0;
    }
    return t;
}

/* Core control law — bit-for-bit faithful to dowiz pid_step_f64:
 *   error = sp - mv (sp/mv sanitized)
 *   p_term = kp * error
 *   integral += error, clamped to ±max/ki_eff  (anti-windup)
 *   i_term = ki * integral
 *   d_term = kd * (error - prev_error)
 *   out = output + p_term + i_term + d_term, clamped to [min, max]
 * Non-finite result fails closed to max. */
static double pid_step(double setpoint, double measurement, double kp,
                       double ki, double kd, double min, double max,
                       double *integral, double *prev_error, double output) {
    double sp = pid_sanitize(setpoint);
    double mv = pid_sanitize(measurement);
    double error = sp - mv;
    double p_term = kp * error;

    *integral += error;
    double ki_eff = ki > PID_KI_EPSILON ? ki : PID_KI_EPSILON;
    double max_i = max / ki_eff;
    /* Rust clamps integral to [-max_i, max_i]; that assumes max_i >= 0. When
     * max < 0 (degenerate config) use the symmetric magnitude instead. */
    double bound = max_i < 0.0 ? -max_i : max_i;
    if (*integral > bound) *integral = bound;
    if (*integral < -bound) *integral = -bound;
    double i_term = ki * *integral;

    double derivative = error - *prev_error;
    double d_term = kd * derivative;
    *prev_error = error;

    double out = output + p_term + i_term + d_term;
    if (out < min) out = min;
    if (out > max) out = max;
    if (!pid_is_finite(out)) out = max;
    return out;
}

BebopPidConfig pid_config_new(double kp, double ki, double kd, double min,
                              double max) {
    BebopPidConfig cfg;
    cfg.kp = pid_sanitize(kp);
    cfg.ki = pid_sanitize(ki);
    cfg.kd = pid_sanitize(kd);
    cfg.min = pid_sanitize(min);
    cfg.max = pid_sanitize(max);
    return cfg;
}

BebopPidConfig pid_config_sanitize(BebopPidConfig cfg) {
    if (cfg.ki < 0.0) cfg.ki = 0.0;
    if (cfg.kp < 0.0) cfg.kp = 0.0;
    if (cfg.kd < 0.0) cfg.kd = 0.0;
    if (cfg.min > cfg.max) {
        double avg = (cfg.min + cfg.max) / 2.0;
        cfg.min = avg;
        cfg.max = avg;
    }
    return cfg;
}

BebopPid pid_new(double kp, double ki, double kd, double min, double max) {
    BebopPid pid;
    pid.config = pid_config_sanitize(pid_config_new(kp, ki, kd, min, max));
    pid.integral = 0.0;
    pid.prev_error = 0.0;
    pid.output = pid.config.max;
    return pid;
}

double pid_update(BebopPid *pid, double setpoint, double measurement) {
    pid->output = pid_step(setpoint, measurement, pid->config.kp, pid->config.ki,
                           pid->config.kd, pid->config.min, pid->config.max,
                           &pid->integral, &pid->prev_error, pid->output);
    return pid->output;
}

void pid_reset(BebopPid *pid) {
    pid->integral = 0.0;
    pid->prev_error = 0.0;
}

double pid_output(const BebopPid *pid) {
    return pid->output;
}

uint64_t pid_recommended(const BebopPid *pid) {
    double r = pid_round(pid->output);
    if (r < 1.0) r = 1.0;
    if (r >= 18446744073709551616.0) {
        return UINT64_MAX; /* saturate, like Rust `as usize` */
    }
    return (uint64_t)r;
}

int pid_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name) do { \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n", (cond) ? "ok" : "FAIL", name); \
        if (r_ > 0) pos += (size_t)r_; \
        if (!(cond)) all_ok = 0; \
    } while (0)

    /* 1. config sanitize clamps negative gains and swaps inverted limits */
    {
        BebopPidConfig c =
            pid_config_sanitize(pid_config_new(-1.0, -0.5, -0.3, 100.0, 0.0));
        A(c.kp >= 0.0 && c.ki >= 0.0 && c.kd >= 0.0 && c.min <= c.max,
          "config sanitize clamps gains / swaps limits");
    }

    /* 2. PID converges toward setpoint */
    {
        BebopPid p = pid_new(0.8, 0.1, 0.3, 1.0, 100.0);
        int i;
        for (i = 0; i < 50; i++) {
            pid_update(&p, 10.0, pid_output(&p));
        }
        A(pid_output(&p) - 10.0 < 1.0 && 10.0 - pid_output(&p) < 1.0,
          "PID converges to setpoint");
    }

    /* 3. integral anti-windup keeps output clamped under extreme error */
    {
        BebopPid p = pid_new(2.0, 1.0, 0.0, 0.0, 100.0);
        int i;
        for (i = 0; i < 100; i++) {
            pid_update(&p, 0.0, 100.0);
        }
        A(pid_output(&p) >= 0.0 && pid_output(&p) <= 100.0 &&
              pid_is_finite(p.integral),
          "integral windup clamped");
    }

    /* 4. output saturates at max on huge positive error */
    {
        BebopPid p = pid_new(5.0, 0.5, 1.0, 0.0, 20.0);
        int i;
        for (i = 0; i < 30; i++) {
            pid_update(&p, 1000.0, 0.0);
        }
        A(pid_output(&p) <= 20.0 && pid_is_finite(pid_output(&p)),
          "output saturates at max");
    }

    /* 5. NaN inputs fail closed (finite output / finite integral) */
    {
        BebopPid p = pid_new(1.0, 0.2, 0.3, 0.0, 100.0);
        uint64_t nan_bits = 0x7FF8000000000000ULL;
        double nan;
        double o1, o2;
        memcpy(&nan, &nan_bits, sizeof nan);
        o1 = pid_update(&p, nan, 5.0);
        o2 = pid_update(&p, 10.0, nan);
        A(pid_is_finite(o1) && pid_is_finite(o2) && pid_is_finite(p.integral),
          "NaN input fails closed");
    }

    /* 6. deterministic: two identical controllers produce identical outputs */
    {
        BebopPid a = pid_new(0.5, 0.1, 0.2, 0.0, 50.0);
        BebopPid b = pid_new(0.5, 0.1, 0.2, 0.0, 50.0);
        int i;
        int same = 1;
        for (i = 0; i < 20; i++) {
            double sp = (i < 10) ? 10.0 : 25.0;
            double mv = 8.0 + (double)i * 0.3;
            if (pid_update(&a, sp, mv) != pid_update(&b, sp, mv)) {
                same = 0;
            }
        }
        A(same, "deterministic across identical runs");
    }

    return all_ok ? 0 : -1;
}
