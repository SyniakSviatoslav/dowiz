/* Bebop token_bucket — GCRA + monotonic token bucket (port of dowiz
 * token_bucket.rs). Pure integer GCRA decision package; degrade-closed on
 * overflow (never a wrapping grant). */
#ifndef BEBOP_TOKEN_BUCKET_H
#define BEBOP_TOKEN_BUCKET_H

#include <stddef.h>
#include <stdint.h>

/* Pure GCRA transition. Returns 1 (grant, *out = new TAT) or 0 (deny). */
int gcra_decide(uint64_t now_ns, uint64_t tat_ns, uint64_t cost_ns,
                uint64_t burst_ns, uint64_t *out);

/* Monotonic-clock token bucket (single-threaded; caller injects now_ns). */
typedef struct {
    double capacity;
    double refill_rate;
    double tokens;
    uint64_t last_refill_ns;
} TokenBucket;

void tb_new(TokenBucket *b, double capacity, double refill_rate);
int tb_try_acquire(TokenBucket *b, double n, uint64_t now_ns);
double tb_available(TokenBucket *b, uint64_t now_ns);

int token_bucket_self_test(char *out, size_t cap);

#endif /* BEBOP_TOKEN_BUCKET_H */
