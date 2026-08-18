/* Bebop graph — implementation. */
#include "graph.h"
#include <string.h>
#include <stdio.h>

void graph_init(Graph *g, int start_node) {
    memset(g, 0, sizeof *g);
    g->start_node = start_node;
}

int graph_add_node(Graph *g, const char *name, GraphNodeFn fn) {
    if (g->n_nodes >= GRAPH_MAX_NODES) return -1;
    g->nodes[g->n_nodes].name = name;
    g->nodes[g->n_nodes].fn = fn;
    return g->n_nodes++;
}

int graph_add_edge(Graph *g, int from, int to) {
    if (g->n_edges >= GRAPH_MAX_EDGES) return -1;
    g->edges[g->n_edges].from = from;
    g->edges[g->n_edges].to = to;
    g->edges[g->n_edges].cond = NULL;
    return g->n_edges++;
}

int graph_add_cond_edge(Graph *g, int from, const char *cond, int target) {
    if (g->n_edges >= GRAPH_MAX_EDGES) return -1;
    g->edges[g->n_edges].from = from;
    g->edges[g->n_edges].to = -1;
    g->edges[g->n_edges].cond = cond;
    g->edges[g->n_edges].cond_target = target;
    return g->n_edges++;
}

const char *graph_run(const Graph *g, void *state, void *ctx) {
    int current = g->start_node;
    const char *last_action = NULL;
    for (int iter = 0; iter < 1000; iter++) { /* safety limit */
        const GraphNode *node = &g->nodes[current];
        if (!node->fn) return last_action;
        const char *action = node->fn(state, ctx);
        if (!action) return last_action; /* terminal */
        last_action = action;
        /* Find next node via edges */
        int next = -1;
        for (int e = 0; e < g->n_edges; e++) {
            if (g->edges[e].from != current) continue;
            if (g->edges[e].cond) {
                if (g->edges[e].cond && strcmp(action, g->edges[e].cond) == 0) {
                    next = g->edges[e].cond_target; break;
                }
                if (g->edges[e].to >= 0) next = g->edges[e].to; /* fallback */
            } else {
                next = g->edges[e].to; break;
            }
        }
        if (next < 0) return action;
        current = next;
    }
    return last_action;
}

/* ─── self-test ─────────────────────────────────────────────────────────── */
typedef struct { int x; int log[8]; int nlog; } TestState;

static const char *node_start(void *s, void *ctx) {
    (void)ctx;
    TestState *st = (TestState *)s;
    st->x = 5;
    st->log[st->nlog++] = 1;
    return "go";
}
static const char *node_mid(void *s, void *ctx) {
    (void)ctx;
    TestState *st = (TestState *)s;
    st->x *= 2;
    st->log[st->nlog++] = 2;
    return st->x > 5 ? "done" : "retry";
}
static const char *node_end(void *s, void *ctx) {
    (void)ctx;
    TestState *st = (TestState *)s;
    st->log[st->nlog++] = 3;
    return NULL;
}

int graph_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    Graph g;
    graph_init(&g, 0);
    graph_add_node(&g, "start", node_start);
    graph_add_node(&g, "mid", node_mid);
    graph_add_node(&g, "end", node_end);
    graph_add_edge(&g, 0, 1);
    graph_add_edge(&g, 1, 2);

    TestState st = {0};
    graph_run(&g, &st, NULL);
    T(st.x == 10, "graph: start(5) -> mid(*2) -> end");
    T(st.nlog == 3, "all 3 nodes visited");
    T(st.log[0] == 1 && st.log[1] == 2 && st.log[2] == 3, "nodes in order");

#undef T
    return fail;
}