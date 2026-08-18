/* Bebop worker pool — multi-core parallel dispatch (pthreads).
 *
 * Each worker thread sleeps on a condition variable until work arrives.
 * pool_parallel_for splits [0, n) across workers and blocks until all finish.
 * The pool is created once at startup and reused across benchmark runs.
 *
 * Thread-safe: main thread dispatches, workers execute, no locking in the
 * hot path — each worker owns its chunk exclusively.
 */
#ifndef BEBOP_POOL_H
#define BEBOP_POOL_H

#include <stddef.h>

/* Opaque pool; call pool_new / pool_free once. */
typedef struct Pool Pool;

/* Work function: process elements [start, end).  arg is the same user pointer
 * passed to pool_parallel_for. */
typedef void (*pool_work_fn)(size_t start, size_t end, void *arg);

/* Create a fixed-size pool.  nthreads=0 → nproc (capped at POOL_MAX_WORKERS). */
Pool *pool_new(int nthreads);
void  pool_free(Pool *p);

/* Split [0, n) into nthreads chunks and run fn(start, end, arg) on each worker
 * in parallel.  Blocks until all workers return. */
void pool_parallel_for(Pool *p, size_t n, pool_work_fn fn, void *arg);

/* Number of worker threads in the pool. */
int  pool_nthreads(const Pool *p);

/* Simple one-shot: fork n workers to run fn in parallel on [0,n),
 * then join.  No persistent pool — use for cold paths.  Internal creation
 * overhead per call. */
void parallel_for_once(size_t n, pool_work_fn fn, void *arg);

#endif /* BEBOP_POOL_H */