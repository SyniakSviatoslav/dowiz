/* Bebop chain — sequential tool-call pipeline (langchain core, native). */
#ifndef BEBOP_CHAIN_H
#define BEBOP_CHAIN_H

#include <stddef.h>
#include <stdint.h>

#define CHAIN_MAX_STEPS 16
#define CHAIN_BUF_SIZE  2048

/* A chain step: takes input string, writes output to buf, returns chars written (or -1). */
typedef int (*ChainStepFn)(const char *input, char *output, size_t cap, void *ctx);

typedef struct {
    ChainStepFn fn;
    const char *name;
} ChainStep;

typedef struct {
    ChainStep steps[CHAIN_MAX_STEPS];
    int       n_steps;
    char      buf[CHAIN_BUF_SIZE];
} Chain;

/* Build and run. */
void chain_init(Chain *c);
int  chain_add(Chain *c, const char *name, ChainStepFn fn);
int  chain_run(Chain *c, const char *input, char *output, size_t cap, void *ctx);

/* Common built-in steps. */
int  chain_step_uppercase(const char *in, char *out, size_t cap, void *ctx);
int  chain_step_reverse(const char *in, char *out, size_t cap, void *ctx);
int  chain_step_echo(const char *in, char *out, size_t cap, void *ctx);

int  chain_self_test(char *out, size_t cap);

#endif