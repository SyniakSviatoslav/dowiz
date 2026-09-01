/* Bebop Event — implementation (C3 event-sourced state machine). */
#include "event.h"

#include <stdio.h>

const char *event_state_name(State s) {
    switch (s) {
        case ST_DRAFT: return "draft";
        case ST_PAID: return "paid";
        case ST_DELIVERED: return "delivered";
        case ST_CANCELLED: return "cancelled";
    }
    return "?";
}

const char *event_name(Event e) {
    switch (e) {
        case EV_PAY: return "pay";
        case EV_DELIVER: return "deliver";
        case EV_CANCEL: return "cancel";
    }
    return "?";
}

int event_fold(State s, Event e, State *out, char *err, size_t cap) {
    switch (s) {
        case ST_DRAFT:
            switch (e) {
                case EV_PAY: *out = ST_PAID; return 0;
                case EV_CANCEL: *out = ST_CANCELLED; return 0;
                default: break;
            }
            break;
        case ST_PAID:
            switch (e) {
                case EV_DELIVER: *out = ST_DELIVERED; return 0;
                case EV_CANCEL: *out = ST_CANCELLED; return 0;
                default: break;
            }
            break;
        default:
            break; /* delivered + cancelled are terminal */
    }
    snprintf(err, cap, "forbidden transition: %s + %s", event_state_name(s),
             event_name(e));
    return -1;
}

int event_fold_seq(State s, const Event *evs, size_t n, State *out, char *err,
                   size_t cap) {
    State cur = s;
    for (size_t i = 0; i < n; i++) {
        if (event_fold(cur, evs[i], &cur, err, cap) != 0) {
            return -1;
        }
    }
    *out = cur;
    return 0;
}

int event_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
    char err[128];
    State st;
#define E(cond, name)                                                \
    do {                                                             \
        int r2 = snprintf(out + pos, cap - pos, "[%s] %s\n",         \
                          (cond) ? "ok" : "FAIL", name);             \
        if (r2 > 0) pos += (size_t)r2;                               \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    E(event_fold(ST_DRAFT, EV_PAY, &st, err, sizeof err) == 0 && st == ST_PAID,
      "draft + pay -> paid");
    E(event_fold(ST_PAID, EV_DELIVER, &st, err, sizeof err) == 0 &&
          st == ST_DELIVERED,
      "paid + deliver -> delivered");
    E(event_fold(ST_DRAFT, EV_CANCEL, &st, err, sizeof err) == 0 &&
          st == ST_CANCELLED,
      "draft + cancel -> cancelled");

    /* forbidden transitions are errors */
    E(event_fold(ST_DRAFT, EV_DELIVER, &st, err, sizeof err) == -1,
      "draft + deliver forbidden");
    E(event_fold(ST_DELIVERED, EV_PAY, &st, err, sizeof err) == -1,
      "delivered is terminal");
    E(event_fold(ST_CANCELLED, EV_PAY, &st, err, sizeof err) == -1,
      "cancelled is terminal");

    /* fold a sequence */
    Event seq[2] = {EV_PAY, EV_DELIVER};
    E(event_fold_seq(ST_DRAFT, seq, 2, &st, err, sizeof err) == 0 &&
          st == ST_DELIVERED,
      "fold_seq [pay, deliver] -> delivered");

    Event bad[2] = {EV_DELIVER, EV_PAY};
    E(event_fold_seq(ST_DRAFT, bad, 2, &st, err, sizeof err) == -1,
      "fold_seq [deliver, ...] -> forbidden");

    return all_ok ? 0 : -1;
}
