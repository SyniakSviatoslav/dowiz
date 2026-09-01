/* Bebop startup telemetry — binary size, exec→main latency, per-module
 * self-test timing. stdio-only (no sys/stat, no libm); the host links libc. */
#ifndef BEBOP_STARTUP_H
#define BEBOP_STARTUP_H

#include <stddef.h>
#include <stdint.h>

/* CLOCK_MONOTONIC timestamp in nanoseconds. */
uint64_t bp_mono_ns(void);

/* Size of the running binary (/proc/self/exe) in bytes, 0 on failure. */
uint64_t bp_binary_size(void);

/* Record the moment main() is entered. Call as the first statement of main
 * so bp_startup_ns() can report the pre-main init latency. */
void bp_startup_mark_main(void);

/* Pre-main init latency: earliest constructor -> main entry, ns (0 if unset). */
uint64_t bp_startup_ns(void);

/* True exec→now latency from the kernel process starttime (/proc/self/stat
 * field 22, jiffies). Coarse (~1/HZ) but covers kernel exec + loader, which
 * the constructor cannot see. 0 on failure. */
uint64_t bp_exec_ns(void);

/* One timed module self-test. */
typedef struct {
    const char *name;
    int (*test)(char *out, size_t cap);
} BpStartupModule;

/* Print binary size, startup latency, and a timed per-module self-test
 * table to stdout. Returns the number of failing modules (0 = all pass). */
int bp_startup_report(void);

#endif /* BEBOP_STARTUP_H */
