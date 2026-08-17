/* Bebop session types (catalog #12) — linear protocols for deadlock-free
 * agent interaction.
 *
 * A session type is a protocol: Send(T, S) | Recv(T, S) | End. Duality swaps
 * send↔recv recursively, so two endpoints that run a protocol and its dual can
 * only compose into a well-typed, deadlock-free conversation (classic π-calculus
 * session-type discipline). Each protocol step is QTT-linear (Q_ONE): consumed
 * exactly once, so a message can never be duplicated or dropped.
 */
#ifndef BEBOP_SESSION_H
#define BEBOP_SESSION_H

#include <stddef.h>

#include "qtt.h"

typedef enum { SES_END = 0, SES_SEND = 1, SES_RECV = 2 } SesKind;

typedef struct Ses {
    SesKind kind;
    Ty *payload;      /* message type (SEND/RECV), NULL for END */
    struct Ses *next; /* continuation protocol */
} Ses;

/* Dual of a protocol: End↦End, Send↦Recv, Recv↦Send, recursively. The dual is
 * the other endpoint's view of the same conversation. */
Ses *session_dual(const Ses *s);

/* Is the protocol self-dual (its dual is itself)? Only End and symmetric
 * protocols are. Used to detect a fixed-point under duality. */
int session_is_self_dual(const Ses *s);

/* Render a protocol to text (for diagnostics). Returns chars written. */
int session_print(const Ses *s, char *out, size_t cap);

int session_self_test(char *out, size_t cap);

#endif /* BEBOP_SESSION_H */
