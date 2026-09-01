/* Bebop mesh — kernel-level mesh networking (native, zero-dep).
 * Tactical: node registry, flood routing, store-and-forward buffer. */
#ifndef BEBOP_MESH_H
#define BEBOP_MESH_H

#include <stddef.h>
#include <stdint.h>

#define MESH_MAX_NODES  32
#define MESH_BUF_SIZE   512
#define MESH_MAX_HOPS   16
#define MNODE_ID_BYTES  8

typedef struct { uint64_t hi, lo; } MeshNodeId;

typedef struct {
    MeshNodeId id;
    int        active;
    int        hops;
    int        last_seen;
} MeshNode;

typedef struct {
    MeshNodeId dst, src;
    uint8_t    ttl, priority;
    uint8_t    data[MESH_BUF_SIZE];
    uint16_t   len;
    int        custody;
} MeshBundle;

typedef struct {
    MeshNode   nodes[MESH_MAX_NODES];
    int        n_nodes;
    MeshBundle bundles[MESH_BUF_SIZE];
    int        n_bundles;
    int        tick;
    MeshNodeId self_id;
} Mesh;

void mesh_init(Mesh *m, uint64_t self_hi, uint64_t self_lo);
int  mesh_register(Mesh *m, uint64_t hi, uint64_t lo, int hops);
int  mesh_prune(Mesh *m, int timeout_ticks);
int  mesh_send(Mesh *m, uint64_t dst_hi, uint64_t dst_lo,
               const uint8_t *payload, uint16_t len, uint8_t priority);
int  mesh_recv(Mesh *m, MeshBundle *out);
void mesh_tick(Mesh *m);
int  mesh_flood(Mesh *m, const uint8_t *payload, uint16_t len);
int  mesh_self_test(char *out, size_t cap);

#endif