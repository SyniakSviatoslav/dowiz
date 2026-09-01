/* Bebop Event — immutable event-sourced state machine (C3 invariant, port of
 * dowiz causal/order-machine). state = fold(events); a forbidden transition is
 * an ERROR, so an illegal state is unrepresentable. */
#ifndef BEBOP_EVENT_H
#define BEBOP_EVENT_H

#include <stddef.h>

typedef enum { ST_DRAFT, ST_PAID, ST_DELIVERED, ST_CANCELLED } State;
typedef enum { EV_PAY, EV_DELIVER, EV_CANCEL } Event;

const char *event_state_name(State s);
const char *event_name(Event e);

/* fold(state, event) -> new state. Returns 0 on success, -1 on a forbidden
 * transition (err filled). */
int event_fold(State s, Event e, State *out, char *err, size_t cap);

/* fold a sequence of events over an initial state. -1 on the first forbidden
 * transition. */
int event_fold_seq(State s, const Event *evs, size_t n, State *out, char *err,
                   size_t cap);

int event_self_test(char *out, size_t cap);

#endif /* BEBOP_EVENT_H */
