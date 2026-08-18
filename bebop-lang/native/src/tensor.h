/* Bebop tensor — native tensor ops (pytorch core subset).
 * Pure C11, row-major, zero deps. */
#ifndef BEBOP_TENSOR_H
#define BEBOP_TENSOR_H

#include <stddef.h>

#define TENSOR_MAX_DIM 4

typedef struct {
    double  *data;   /* row-major flat array */
    size_t   shape[TENSOR_MAX_DIM];
    int      ndim;
    size_t   size;    /* total elements */
} Tensor;

/* Create/destroy. Caller owns data buffer. */
Tensor tensor_new(double *data, const size_t *shape, int ndim);
void   tensor_zero(Tensor *t);

/* Element access (flat index). */
double tensor_get(const Tensor *t, size_t idx);
void   tensor_set(Tensor *t, size_t idx, double v);

/* Unary ops (in-place). */
void tensor_relu(Tensor *t);
void tensor_sigmoid(Tensor *t);
void tensor_tanh(Tensor *t);
void tensor_scale(Tensor *t, double factor);

/* Binary ops (in-place on first arg). */
void tensor_add(Tensor *a, const Tensor *b);
void tensor_sub(Tensor *a, const Tensor *b);
void tensor_mul(Tensor *a, const Tensor *b);

/* Dot product of two 1D tensors. */
double tensor_dot(const Tensor *a, const Tensor *b);

/* Matrix multiply (2D tensors): C = A*B. Caller provides C data. */
void tensor_matmul(const Tensor *a, const Tensor *b, Tensor *c);

/* Softmax (in-place on 1D tensor). Returns sum of exp. */
double tensor_softmax(Tensor *t);

/* Mean-squared error between two tensors (same shape). */
double tensor_mse(const Tensor *a, const Tensor *b);

/* Sum all elements. */
double tensor_sum(const Tensor *t);

int tensor_self_test(char *out, size_t cap);

#endif