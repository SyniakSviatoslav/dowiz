/* Bebop arena — implementation. */
#include "arena.h"
#include <stdio.h>
#include <string.h>
#define ARENA_ALIGN 64
void arena_init(Arena *a, void *mem, size_t cap) {
    memset(a, 0, sizeof *a);
    /* Align the base pointer to 64 so arena_alloc returns cache-line-aligned
     * addresses regardless of the caller's buffer alignment (e.g. stack). */
    uintptr_t base = (uintptr_t)mem;
    uintptr_t aligned = (base + ARENA_ALIGN - 1) & ~(uintptr_t)(ARENA_ALIGN - 1);
    a->mem = (unsigned char *)aligned;
    a->cap = cap - (size_t)(aligned - base);
}
void *arena_alloc(Arena *a, size_t n) {
    if (a->used + n > a->cap) return NULL;
    size_t off = (a->used + ARENA_ALIGN - 1) & ~(size_t)(ARENA_ALIGN - 1);
    if (off + n > a->cap) return NULL;
    void *p = a->mem + off; a->used = off + n; return p;
}
void arena_reset(Arena *a) { a->used = 0; }
void vec_init(Vec *v, Arena *a, size_t es, size_t ic) {
    memset(v,0,sizeof *v); v->arena=a; v->data=arena_alloc(a,es*ic); v->cap=v->data?ic:0;
}
void *vec_push(Vec *v, size_t es, const void *e) {
    if (v->len >= v->cap) {
        size_t nc = v->cap ? v->cap*2 : 4;
        void *nd = arena_alloc(v->arena, es*nc);
        if (!nd) return NULL;
        memcpy(nd, v->data, v->len*es);
        v->data = nd; v->cap = nc;
    }
    unsigned char *d = (unsigned char*)v->data + v->len*es;
    if (e) memcpy(d, e, es); else memset(d, 0, es);
    v->len++; return d;
}

void ring_init(Ring *r, Arena *a, size_t n, size_t es) {
    memset(r,0,sizeof *r); r->buf=arena_alloc(a,es*n); r->cap=r->buf?n:0;
}
int ring_enq(Ring *r, size_t es, const void *e) {
    size_t nxt = (r->head+1) % r->cap;
    if (nxt == r->tail) return -1;
    memcpy(r->buf+r->head*es, e, es);
    __atomic_store_n(&r->head, nxt, __ATOMIC_RELEASE);
    return 0;
}
int ring_deq(Ring *r, size_t es, void *e) {
    if (r->head == r->tail) return -1;
    memcpy(e, r->buf+r->tail*es, es);
    size_t nxt = (r->tail+1) % r->cap;
    __atomic_store_n(&r->tail, nxt, __ATOMIC_ACQUIRE);
    return 0;
}
int ring_empty(const Ring *r) { return r->head == r->tail; }
int arena_self_test(char *out, size_t cap) {
    size_t p=0; int ok=1;
    unsigned char mem[4096]; Arena aa; arena_init(&aa,mem,4096);
    void *x=arena_alloc(&aa,128);
    if (((uintptr_t)x & 63) == 0) { ok=1; } else { ok=0; }
    snprintf(out+p,cap-p,"[%s] arena 64B align\n",ok?"ok":"FAIL"); p=strlen(out);
    Vec v; vec_init(&v,&aa,4,2); int a=42,b=99;
    vec_push(&v,4,&a); vec_push(&v,4,&b);
    int g0=((int*)v.data)[0], g1=((int*)v.data)[1];
    int vok=(v.len==2 && g0==42 && g1==99);
    snprintf(out+p,cap-p,"[%s] vec push/get\n",vok?"ok":"FAIL"); p=strlen(out);
    if (!vok) ok=0;
    int c=7,d=3; vec_push(&v,4,&c); vec_push(&v,4,&d);
    int vgok=(v.len==4 && v.cap==4);
    snprintf(out+p,cap-p,"[%s] vec grow 2->4\n",vgok?"ok":"FAIL"); p=strlen(out);
    if (!vgok) ok=0;
    Ring rr; ring_init(&rr,&aa,8,4); int e=5,f=0;
    int re1=(ring_enq(&rr,4,&e)==0);
    int re2=(ring_deq(&rr,4,&f)==0 && f==5);
    int re3=ring_empty(&rr);
    snprintf(out+p,cap-p,"[%s] ring enq/deq/empty\n",(re1&&re2&&re3)?"ok":"FAIL");
    return ok?0:-1;
}
ArenaSnapshot arena_snapshot_take(Arena *a) { ArenaSnapshot s={a,a->used}; return s;
}
void arena_snapshot_restore(ArenaSnapshot snap) { snap.arena->used=snap.offset;
}
size_t arena_used(const Arena *a) { return a->used; }
