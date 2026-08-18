/* Bebop comprehensive benchmark — every primitive, honest methodology. */
#ifndef BEBOP_BENCH_ALL_H
#define BEBOP_BENCH_ALL_H

/* Run the full benchmark suite, print to stdout. Returns 0. */
int bench_all_run(void);

/* Run batched parallel benchmark (multi-core throughput mode). */
int bench_all_batched(void);

#endif /* BEBOP_BENCH_ALL_H */
