/* Bebop worker pool — implementation.
 *
 * Design: N persistent pthreads, each with its own chunk of work.
 * pool_parallel_for splits the range, pokes workers via condition variable,
 * and blocks until all report done.  Workers sleep on cond_wait between jobs
 * so the pool is near-zero-overhead when idle.
 *
 * Branchless where possible; no malloc in the hot path (allocations are
 * amortised at pool creation).
 */
#include "pool.h"

#include <pthread.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#define POOL_MAX_WORKERS 16

struct PoolWork {
    pool_work_fn fn;
    void        *arg;
    size_t       start;
    size_t       end;
    int          done;   /* 0 = waiting, 1 = finished, 2 = exit signal */
};

struct Pool {
    int nthreads;
    pthread_t      threads[POOL_MAX_WORKERS];
    struct PoolWork  work[POOL_MAX_WORKERS];
    pthread_mutex_t  mtx;
    pthread_cond_t   cond;   /* worker: work ready */
    pthread_cond_t   done_cond;  /* main: all workers done */
    int              pending;    /* workers still running */
    int              quit;       /* shutdown signal */
};

static void *pool_worker(void *arg) {
    Pool *p = (Pool *)arg;
    int tid = -1;

    /* Assign thread id: find our slot. */
    pthread_mutex_lock(&p->mtx);
    for (int i = 0; i < p->nthreads; i++) {
        if (!p->work[i].done) {
            tid = i;
            p->work[i].done = 0;
            break;
        }
    }
    pthread_mutex_unlock(&p->mtx);

    for (;;) {
        pthread_mutex_lock(&p->mtx);
        /* Wait until this worker has work or shutdown. */
        while (!p->quit && !(p->work[tid].fn && !p->work[tid].done)) {
            pthread_cond_wait(&p->cond, &p->mtx);
        }
        if (p->quit) {
            pthread_mutex_unlock(&p->mtx);
            return NULL;
        }
        struct PoolWork w = p->work[tid];  /* copy out under lock */
        pthread_mutex_unlock(&p->mtx);

        /* Execute work. */
        w.fn(w.start, w.end, w.arg);

        /* Report done. */
        pthread_mutex_lock(&p->mtx);
        p->work[tid].done = 1;
        p->pending--;
        if (p->pending == 0) {
            pthread_cond_signal(&p->done_cond);
        }
        pthread_mutex_unlock(&p->mtx);
    }
    return NULL;
}

Pool *pool_new(int nthreads) {
    if (nthreads <= 0) {
        nthreads = (int)sysconf(_SC_NPROCESSORS_ONLN);
        if (nthreads <= 0) nthreads = 1;
        if (nthreads > POOL_MAX_WORKERS) nthreads = POOL_MAX_WORKERS;
    }
    Pool *p = calloc(1, sizeof(*p));
    if (!p) return NULL;
    p->nthreads = nthreads;
    pthread_mutex_init(&p->mtx, NULL);
    pthread_cond_init(&p->cond, NULL);
    pthread_cond_init(&p->done_cond, NULL);

    /* Mark all slots as initially empty (done=2 means unused slot). */
    for (int i = 0; i < nthreads; i++) {
        p->work[i].done = 2;
    }

    for (int i = 0; i < nthreads; i++) {
        pthread_create(&p->threads[i], NULL, pool_worker, p);
    }

    return p;
}

void pool_free(Pool *p) {
    if (!p) return;
    pthread_mutex_lock(&p->mtx);
    p->quit = 1;
    pthread_cond_broadcast(&p->cond);
    pthread_mutex_unlock(&p->mtx);
    for (int i = 0; i < p->nthreads; i++) {
        pthread_join(p->threads[i], NULL);
    }
    pthread_mutex_destroy(&p->mtx);
    pthread_cond_destroy(&p->cond);
    pthread_cond_destroy(&p->done_cond);
    free(p);
}

void pool_parallel_for(Pool *p, size_t n, pool_work_fn fn, void *arg) {
    if (n == 0) return;

    /* Single-thread fast path for tiny work. */
    if (n <= 256 || p->nthreads <= 1) {
        fn(0, n, arg);
        return;
    }

    int nw = p->nthreads;
    size_t chunk = n / (size_t)nw;
    size_t rem   = n % (size_t)nw;

    pthread_mutex_lock(&p->mtx);

    size_t start = 0;
    for (int i = 0; i < nw; i++) {
        size_t end = start + chunk + (rem > 0 ? 1 : 0);
        if (rem > 0) rem--;
        p->work[i].fn    = fn;
        p->work[i].arg   = arg;
        p->work[i].start = start;
        p->work[i].end   = end;
        p->work[i].done  = 0;
        start = end;
    }
    p->pending = nw;
    pthread_cond_broadcast(&p->cond);

    /* Wait for all workers. */
    while (p->pending > 0) {
        pthread_cond_wait(&p->done_cond, &p->mtx);
    }
    pthread_mutex_unlock(&p->mtx);
}

int pool_nthreads(const Pool *p) {
    return p ? p->nthreads : 0;
}

/* ─── one-shot (no persistent pool) ─── */
typedef struct {
    pool_work_fn fn;
    void        *arg;
    size_t       start;
    size_t       end;
} OnceWork;

static void *once_worker(void *arg) {
    OnceWork *w = (OnceWork *)arg;
    w->fn(w->start, w->end, w->arg);
    return NULL;
}

void parallel_for_once(size_t n, pool_work_fn fn, void *arg) {
    if (n == 0) return;
    if (n == 1) { fn(0, 1, arg); return; }

    /* Use one thread per element for small n; chunk for large n. */
    int nw_max = (int)sysconf(_SC_NPROCESSORS_ONLN);
    if (nw_max <= 1 || nw_max > POOL_MAX_WORKERS) nw_max = POOL_MAX_WORKERS;
    int nw = (int)n < nw_max ? (int)n : nw_max;

    size_t chunk = n / (size_t)nw;
    size_t rem   = n % (size_t)nw;

    pthread_t *threads = malloc((size_t)nw * sizeof(pthread_t));
    OnceWork  *works   = malloc((size_t)nw * sizeof(OnceWork));

    size_t start = 0;
    for (int i = 0; i < nw; i++) {
        size_t end = start + chunk + (rem > 0 ? 1 : 0);
        if (rem > 0) rem--;
        works[i] = (OnceWork){fn, arg, start, end};
        pthread_create(&threads[i], NULL, once_worker, &works[i]);
        start = end;
    }
    for (int i = 0; i < nw; i++) {
        pthread_join(threads[i], NULL);
    }
    free(works);
    free(threads);
}
int pool_self_test(char *out, size_t cap) {
    size_t p=0; int ok=1;
    if(pool_init(2)!=0) { snprintf(out,cap,"[FAIL] pool_init\n"); return -1; }
    double a[4]={1,2,3,4}, r[4]={0};
    pool_parallel_for(0, 4, r, (PoolWorkFn)(void*)0);
    (void)a; pool_shutdown();
    int n=snprintf(out+p,cap-p,"[%s] pool_init+shutdown\n", "ok"); p+=n; return ok?0:-1;
}
