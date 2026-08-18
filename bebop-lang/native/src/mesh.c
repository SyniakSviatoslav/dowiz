/* Bebop mesh — implementation. */
#include "mesh.h"
#include <string.h>

void mesh_init(Mesh *m, uint64_t self_hi, uint64_t self_lo) {
    memset(m, 0, sizeof *m);
    m->self_id.hi = self_hi;
    m->self_id.lo = self_lo;
    /* Self is node 0 */
    m->nodes[0].id = m->self_id;
    m->nodes[0].active = 1;
    m->nodes[0].hops = 0;
    m->nodes[0].last_seen = 0;
    m->n_nodes = 1;
}

int mesh_register(Mesh *m, uint64_t hi, uint64_t lo, int hops) {
    /* Look for existing */
    for (int i = 0; i < m->n_nodes; i++) {
        if (m->nodes[i].id.hi == hi && m->nodes[i].id.lo == lo) {
            if (hops < m->nodes[i].hops) m->nodes[i].hops = hops;
            m->nodes[i].active = 1;
            m->nodes[i].last_seen = m->tick;
            return i;
        }
    }
    if (m->n_nodes >= MESH_MAX_NODES) return -1;
    int i = m->n_nodes++;
    m->nodes[i].id.hi = hi;
    m->nodes[i].id.lo = lo;
    m->nodes[i].hops = hops;
    m->nodes[i].active = 1;
    m->nodes[i].last_seen = m->tick;
    return i;
}

int mesh_prune(Mesh *m, int timeout_ticks) {
    int pruned = 0;
    for (int i = 1; i < m->n_nodes; i++) {
        if (m->nodes[i].active && (m->tick - m->nodes[i].last_seen) > timeout_ticks) {
            m->nodes[i].active = 0;
            pruned++;
        }
    }
    return pruned;
}

int mesh_send(Mesh *m, uint64_t dst_hi, uint64_t dst_lo,
              const uint8_t *payload, uint16_t len, uint8_t priority) {
    if (m->n_bundles >= MESH_BUF_SIZE || len > MESH_BUF_SIZE) return -1;
    int i = m->n_bundles++;
    m->bundles[i].dst.hi = dst_hi;  m->bundles[i].dst.lo = dst_lo;
    m->bundles[i].src = m->self_id;
    m->bundles[i].ttl = MESH_MAX_HOPS;
    m->bundles[i].priority = priority;
    m->bundles[i].len = len;
    m->bundles[i].custody = (priority >= 2) ? 1 : 0;
    memcpy(m->bundles[i].data, payload, len);
    return 0;
}

int mesh_recv(Mesh *m, MeshBundle *out) {
    for (int i = 0; i < m->n_bundles; i++) {
        MeshBundle *b = &m->bundles[i];
        if (b->ttl == 0) continue; /* expired */
        /* Delivered to us? */
        int to_us = (b->dst.hi == m->self_id.hi && b->dst.lo == m->self_id.lo);
        /* Accept custody for high-priority bundles addressed to others */
        if (to_us || b->custody) {
            *out = *b;
            /* Remove by swapping with last */
            m->bundles[i] = m->bundles[--m->n_bundles];
            return 1;
        }
    }
    return 0;
}

void mesh_tick(Mesh *m) {
    m->tick++;
    /* Retry bundles: decrement TTL, drop expired */
    for (int i = 0; i < m->n_bundles; i++) {
        if (m->bundles[i].ttl > 0) m->bundles[i].ttl--;
    }
}

int mesh_flood(Mesh *m, const uint8_t *payload, uint16_t len) {
    int sent = 0;
    for (int i = 1; i < m->n_nodes; i++) {
        if (m->nodes[i].active) {
            if (mesh_send(m, m->nodes[i].id.hi, m->nodes[i].id.lo,
                         payload, len, 2) == 0) sent++;
        }
    }
    return sent;
}

/* ─── self-test ─────────────────────────────────────────────────────────── */
#include <stdio.h>
int mesh_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    Mesh m;
    mesh_init(&m, 0xABCD, 0x1234);
    T(m.n_nodes == 1, "init: self is node 0");
    T(m.nodes[0].hops == 0, "self has 0 hops");

    mesh_register(&m, 0xBEEF, 0x5678, 1);
    T(m.n_nodes == 2, "register neighbour");
    T(m.nodes[1].hops == 1, "neighbour at hop 1");

    mesh_send(&m, 0xBEEF, 0x5678, (const uint8_t*)"hello", 5, 0);
    T(m.n_bundles == 1, "send enqueued");
    T(m.bundles[0].len == 5, "payload length");

    MeshBundle recv;
    T(mesh_recv(&m, &recv) == 0, "bundle not addressed to us");

    mesh_send(&m, m.self_id.hi, m.self_id.lo, (const uint8_t*)"self", 4, 0);
    T(mesh_recv(&m, &recv) == 1, "recv bundle for us");
    T(recv.len == 4, "recv correct length");
    T(recv.dst.hi == m.self_id.hi, "recv correct dst");

    mesh_tick(&m);
    T(m.tick == 1, "tick advances");

    int pruned = mesh_prune(&m, 0);
    T(pruned == 1, "neighbour pruned after timeout");

    MeshBundle fb;
    mesh_send(&m, 0xFFFF, 0x0001, (const uint8_t*)"flood", 5, 2);
    T(m.bundles[0].custody == 1, "urgency >= 2 gets custody");

#undef T
    return fail;
}