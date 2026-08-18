/* Bebop autonomic — implementation (port of dowiz autonomic.rs). */

#include "autonomic.h"
#include <math.h>

/* ─── BoundedRate ────────────────────────────────────────────────────────── */

BoundedRate bounded_rate_new(double v) {
    if (isnan(v)) return (BoundedRate){0.0};
    if (v < 0.0)  return (BoundedRate){0.0};
    if (v > 100.0) return (BoundedRate){100.0};
    return (BoundedRate){v};
}

bool bounded_rate_try(double v, BoundedRate *out) {
    if (isnan(v) || v < 0.0 || v > 100.0) return false;
    out->value = v;
    return true;
}

/* ─── gain-scheduling table ───────────────────────────────────────────────
 * LAW_TABLE[verdict][drift_class] → {direction, rate}
 * Healthy:   hold unless drift says otherwise
 * Degrading: degrade, faster if unstable
 * Unstable:  emergency degrade at max rate
 */
static const Adjustment LAW_TABLE[3][4] = {
    /*               DAMPED           RESONANT        UNSTABLE        UNKNOWN  */
    /* HEALTHY   */ {{ 0,{0.0}}, { 0,{5.0}}, {-1,{20.0}}, { 0,{0.0}}},
    /* DEGRADING */ {{-1,{10.0}}, {-1,{30.0}}, {-1,{60.0}}, {-1,{10.0}}},
    /* UNSTABLE  */ {{-1,{80.0}}, {-1,{90.0}}, {-1,{100.0}}, {-1,{60.0}}},
};

Adjustment autonomic_schedule(MarkovVerdict v, DriftClass d) {
    return LAW_TABLE[(int)v][(int)d];
}

/* ─── self-test ─────────────────────────────────────────────────────────── */

static int nwrite(char *b, size_t c, const char *s) {
    size_t n = 0; while (s[n]) n++; if (n > c) n = c;
    for (size_t i = 0; i < n; i++) b[i] = s[i];
    return (int)n;
}
#define T(cond, msg) do { ok++; \
    int _n; \
    if (!(cond)) { fail++; \
        _n=nwrite(out,cap,"[FAIL] ");out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,msg);out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,"\n");out+=_n;cap-=(size_t)_n; \
    } else { \
        _n=nwrite(out,cap,"[ok] ");out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,msg);out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,"\n");out+=_n;cap-=(size_t)_n; \
    } \
} while(0)

int autonomic_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;

    /* BoundedRate clamping */
    BoundedRate br = bounded_rate_new(50.0);
    T(br.value == 50.0, "BoundedRate(50) == 50");
    br = bounded_rate_new(-10.0);
    T(br.value == 0.0, "BoundedRate(-10) clamped to 0");
    br = bounded_rate_new(200.0);
    T(br.value == 100.0, "BoundedRate(200) clamped to 100");
    br = bounded_rate_new(NAN);
    T(br.value == 0.0, "BoundedRate(NaN) clamped to 0");

    /* Rejecting constructor */
    BoundedRate okbr;
    T(bounded_rate_try(75.0, &okbr) && okbr.value == 75.0, "try_from(75) OK");
    T(!bounded_rate_try(-1.0, &okbr), "try_from(-1) rejected");
    T(!bounded_rate_try(101.0, &okbr), "try_from(101) rejected");

    /* Gain-scheduling: healthy + damped → hold */
    Adjustment a = autonomic_schedule(MKV_HEALTHY, DC_DAMPED);
    T(a.direction == 0 && a.rate.value == 0.0,
      "healthy+damped → hold");

    /* Degrading + unstable → emergency degrade */
    a = autonomic_schedule(MKV_DEGRADING, DC_UNSTABLE);
    T(a.direction == -1 && a.rate.value == 60.0,
      "degrading+unstable → degrade @ 60%");

    /* Unstable + unknown → safe degrade */
    a = autonomic_schedule(MKV_UNSTABLE, DC_UNKNOWN);
    T(a.direction == -1 && a.rate.value == 60.0,
      "unstable+unknown → degrade @ 60%");

    #undef T
    return fail;
}
