/* Bebop token_bucket — implementation (port of dowiz token_bucket.rs). */
#include "token_bucket.h"

#include <stdio.h>

int gcra_decide(uint64_t now_ns, uint64_t tat_ns, uint64_t cost_ns,
                uint64_t burst_ns, uint64_t *out) {
    uint64_t allow_at = tat_ns > now_ns ? tat_ns : now_ns;
    if (allow_at > UINT64_MAX - cost_ns) {
        return 0; /* overflow → deny, never a wrapping grant */
    }
    uint64_t new_tat = allow_at + cost_ns;
    if (now_ns > UINT64_MAX - burst_ns) {
        return 0; /* overflow → deny */
    }
    uint64_t limit = now_ns + burst_ns;
    if (new_tat > limit) {
        return 0;
    }
    *out = new_tat;
    return 1;
}

void tb_new(TokenBucket *b, double capacity, double refill_rate) {
    b->capacity = capacity;
    b->refill_rate = refill_rate;
    b->tokens = capacity;
    b->last_refill_ns = 0;
}

static void tb_refill(TokenBucket *b, uint64_t now_ns) {
    uint64_t elapsed_ns =
        now_ns > b->last_refill_ns ? now_ns - b->last_refill_ns : 0;
    double elapsed_secs = (double)elapsed_ns / 1e9;
    if (elapsed_secs > 0.0) {
        b->tokens += b->refill_rate * elapsed_secs;
        if (b->tokens > b->capacity) {
            b->tokens = b->capacity;
        }
        if (b->tokens < 0.0) {
            b->tokens = 0.0;
        }
        b->last_refill_ns = now_ns;
    }
}

int tb_try_acquire(TokenBucket *b, double n, uint64_t now_ns) {
    tb_refill(b, now_ns);
    if (b->tokens >= n) {
        b->tokens -= n;
        return 1;
    }
    return 0;
}

double tb_available(TokenBucket *b, uint64_t now_ns) {
    tb_refill(b, now_ns);
    return b->tokens;
}

int token_bucket_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) {                                                          \
            pos += (size_t)r_;                                                 \
        }                                                                      \
        if (!c_) {                                                             \
            all_ok = 0;                                                        \
        }                                                                      \
    } while (0)

    uint64_t tat = 0;
    A(gcra_decide(0, 0, 30, 100, &tat) && tat == 30, "gcra grant 1");
    A(gcra_decide(0, 30, 30, 100, &tat) && tat == 60, "gcra grant 2");
    A(gcra_decide(0, 60, 30, 100, &tat) && tat == 90, "gcra grant 3");
    A(!gcra_decide(0, 90, 30, 100, &tat), "gcra deny 4th (burst)");
    A(!gcra_decide(0, 1, UINT64_MAX, UINT64_MAX, &tat), "gcra overflow deny");
    A(!gcra_decide(UINT64_MAX - 1, 0, 1, UINT64_MAX, &tat), "gcra burst overflow deny");

    TokenBucket b;
    tb_new(&b, 10.0, 1.0);
    A(tb_try_acquire(&b, 3.0, 0) && tb_try_acquire(&b, 3.0, 0) &&
          tb_try_acquire(&b, 3.0, 0),
      "3 grants within capacity");
    A(!tb_try_acquire(&b, 3.0, 0), "4th denied");

    TokenBucket c;
    tb_new(&c, 1.0, 100.0);
    A(tb_try_acquire(&c, 1.0, 0), "first acquire drains full bucket");
    A(!tb_try_acquire(&c, 1.0, 0), "empty refuse");
    A(tb_try_acquire(&c, 1.0, 20000000), "refilled after 20ms");

    return all_ok ? 0 : -1;
}
