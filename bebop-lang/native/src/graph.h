/* Bebop graph — stateful graph execution engine (langgraph core, native).
 * DAG with nodes, conditional edges, state passed between nodes. */
#ifndef BEBOP_GRAPH_H
#define BEBOP_GRAPH_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#define GRAPH_MAX_NODES 32
#define GRAPH_MAX_EDGES 64

/* Forward ref */
typedef struct Graph Graph;

/* Node function: takes graph state, returns next action (or NULL for done). */
typedef const char *(*GraphNodeFn)(void *state, void *ctx);

typedef struct {
    const char   *name;
    GraphNodeFn   fn;
} GraphNode;

typedef struct {
    int from;            /* node index */
    int to;              /* node index (or -1 for conditional) */
    const char *cond;    /* condition string for conditional routing */
    int cond_target;     /* if cond matches, route here (-1 = default) */
} GraphEdge;

struct Graph {
    GraphNode nodes[GRAPH_MAX_NODES];
    int       n_nodes;
    GraphEdge edges[GRAPH_MAX_EDGES];
    int       n_edges;
    int       start_node;
};

/* Build graph. */
void graph_init(Graph *g, int start_node);
int  graph_add_node(Graph *g, const char *name, GraphNodeFn fn);
int  graph_add_edge(Graph *g, int from, int to);
int  graph_add_cond_edge(Graph *g, int from, const char *cond, int target);

/* Execute: run from start_node, passing state through nodes, following edges
 * until a node returns NULL. Returns the last action or NULL. */
const char *graph_run(const Graph *g, void *state, void *ctx);

/* ─── self-test ─────────────────────────────────────────────────────────── */
int graph_self_test(char *out, size_t cap);

#endif