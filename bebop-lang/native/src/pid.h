/* Bebop pid — proportional/integral/derivative controller with anti-windup and
 * output clamping (port of dowiz pid.rs). */
#ifndef BEBOP_PID_H
#define BEBOP_PID_H

#include <stddef.h>
#include <stdint.h>

/* PID gains and output limits (f64, full precision). */
typedef struct {
    double kp, ki, kd;
    double min, max;
} BebopPidConfig;

/* Scalar PID controller state. Tracks error = setpoint - measurement.
 * Proportional reacts to current error, integral accumulates (clamped
 * anti-windup), derivative reacts to error rate. Output is always clamped to
 * [config.min, config.max]. */
typedef struct {
    BebopPidConfig config;
    double integral;
    double prev_error;
    double output;
} BebopPid;

/* Construct a config: non-finite inputs fail closed to 0.0 (no gain clamping,
 * no min/max swap). Mirrors dowiz PidConfig::new. */
BebopPidConfig pid_config_new(double kp, double ki, double kd, double min,
                              double max);

/* Sanitize a config: clamp kp/ki/kd to non-negative, force min <= max.
 * Mirrors dowiz PidConfig::sanitize. */
BebopPidConfig pid_config_sanitize(BebopPidConfig cfg);

/* Construct a controller from gains + limits (fully sanitized); output starts
 * at config.max. Mirrors dowiz PidController::new. */
BebopPid pid_new(double kp, double ki, double kd, double min, double max);

/* One control step; returns (and stores) the clamped output. */
double pid_update(BebopPid *pid, double setpoint, double measurement);

/* Clear integral + derivative state (output is left untouched). */
void pid_reset(BebopPid *pid);

double pid_output(const BebopPid *pid);

/* Recommended concurrency = round(output) clamped to >= 1 (saturating cast).
 * Mirrors dowiz PidController::recommended. */
uint64_t pid_recommended(const BebopPid *pid);

/* Appends "[ok]/[FAIL] NAME" lines; returns 0 if all checks pass, else -1. */
int pid_self_test(char *out, size_t cap);

#endif /* BEBOP_PID_H */
