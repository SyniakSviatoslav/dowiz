/* Bebop session types — implementation. */
#include "session.h"

#include <stdio.h>
#include <string.h>

Ses *session_dual(const Ses *s) {
    if (!s || s->kind == SES_END) {
        return (Ses *)s; /* End is its own dual */
    }
    static Ses pool[64];
    static int pi = 0;
    Ses *d = &pool[pi++ % 64];
    d->kind = (s->kind == SES_SEND) ? SES_RECV : SES_SEND;
    d->payload = s->payload;
    d->next = session_dual(s->next);
    return d;
}

int session_is_self_dual(const Ses *s) {
    if (!s || s->kind == SES_END) {
        return 1;
    }
    /* a Send(T, S) is never its own dual (it becomes Recv(T, dual(S))) */
    return 0;
}

int session_print(const Ses *s, char *out, size_t cap) {
    size_t n = 0;
    const Ses *p = s;
    while (p) {
        switch (p->kind) {
            case SES_END:
                n += (size_t)snprintf(out + n, cap - n, "End");
                return (int)n;
            case SES_SEND:
                n += (size_t)snprintf(out + n, cap - n, "Send(");
                break;
            case SES_RECV:
                n += (size_t)snprintf(out + n, cap - n, "Recv(");
                break;
        }
        if (p->payload) {
            char ty[64];
            qtt_ty_print(p->payload, ty, sizeof ty);
            n += (size_t)snprintf(out + n, cap - n, "%s, ", ty);
        }
        p = p->next;
    }
    n += (size_t)snprintf(out + n, cap - n, "End");
    return (int)n;
}

int session_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    static Ses end = {SES_END, NULL, NULL};
    static Ses recv_bool = {SES_RECV, NULL, NULL};
    recv_bool.payload = &(Ty){.kind = TY_BOOL};
    recv_bool.next = &end;
    static Ses send_i64 = {SES_SEND, NULL, &recv_bool};
    send_i64.payload = &(Ty){.kind = TY_I64};

    /* dual: End is fixed */
    A(session_dual(&end) == &end, "dual(End) == End");

    /* dual(Send(i64, Recv(bool, End))) == Recv(i64, Send(bool, End)) */
    Ses *d = session_dual(&send_i64);
    A(d && d->kind == SES_RECV && d->payload->kind == TY_I64,
      "dual flips Send -> Recv, payload preserved");
    A(d->next && d->next->kind == SES_SEND && d->next->payload->kind == TY_BOOL,
      "dual flips the continuation Recv -> Send");
    A(d->next->next == &end, "dual terminates at End");

    /* involution: dual(dual(s)) == s */
    Ses *dd = session_dual(d);
    A(dd->kind == SES_SEND && dd->next->kind == SES_RECV,
      "dual(dual(s)) is isomorphic to s (involution)");

    /* self-dual: End is self-dual, a Send protocol is not */
    A(session_is_self_dual(&end), "End is self-dual");
    A(!session_is_self_dual(&send_i64), "Send(...) is not self-dual");

    /* rendering for diagnostics */
    char buf[256];
    session_print(&send_i64, buf, sizeof buf);
    A(strstr(buf, "Send(i64") != NULL, "print protocol Send(i64, ...");

    return all_ok ? 0 : -1;
}
