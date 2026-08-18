#include <math.h>
/* Bebop tensor — implementation. */
#include "tensor.h"
#include <string.h>
#include <stdio.h>

Tensor tensor_new(double *data, const size_t *shape, int ndim) {
    Tensor t; memset(&t, 0, sizeof t);
    if (ndim > TENSOR_MAX_DIM) ndim = TENSOR_MAX_DIM;
    t.ndim = ndim; t.data = data; t.size = 1;
    for (int i = 0; i < ndim; i++) { t.shape[i] = shape[i]; t.size *= shape[i]; }
    return t;
}
void tensor_zero(Tensor *t) { for (size_t i = 0; i < t->size; i++) t->data[i] = 0; }
double tensor_get(const Tensor *t, size_t i) { return i < t->size ? t->data[i] : 0; }
void tensor_set(Tensor *t, size_t i, double v) { if (i < t->size) t->data[i] = v; }
void tensor_relu(Tensor *t) { for (size_t i = 0; i < t->size; i++) if (t->data[i] < 0) t->data[i] = 0; }
void tensor_sigmoid(Tensor *t) { for (size_t i = 0; i < t->size; i++) t->data[i] = 1.0/(1.0+exp(-t->data[i])); }
void tensor_tanh(Tensor *t) { for (size_t i = 0; i < t->size; i++) { double e=exp(2*t->data[i]); t->data[i]=(e-1)/(e+1); } }
void tensor_scale(Tensor *t, double f) { for (size_t i = 0; i < t->size; i++) t->data[i] *= f; }
void tensor_add(Tensor *a, const Tensor *b) { for (size_t i = 0; i < a->size; i++) a->data[i] += b->data[i]; }
void tensor_sub(Tensor *a, const Tensor *b) { for (size_t i = 0; i < a->size; i++) a->data[i] -= b->data[i]; }
void tensor_mul(Tensor *a, const Tensor *b) { for (size_t i = 0; i < a->size; i++) a->data[i] *= b->data[i]; }
double tensor_dot(const Tensor *a, const Tensor *b) { double s=0; for (size_t i=0;i<a->size;i++) s+=a->data[i]*b->data[i]; return s; }
void tensor_matmul(const Tensor *a, const Tensor *b, Tensor *c) {
    for (int i = 0; i < (int)a->shape[0]; i++)
        for (int j = 0; j < (int)b->shape[1]; j++) {
            double s = 0;
            for (int k = 0; k < (int)a->shape[1]; k++) s += a->data[i*(int)a->shape[1]+k] * b->data[k*(int)b->shape[1]+j];
            c->data[i*(int)c->shape[1]+j] = s;
        }
}
double tensor_softmax(Tensor *t) { double mx=t->data[0],s=0; for (size_t i=1;i<t->size;i++)if(t->data[i]>mx)mx=t->data[i]; for (size_t i=0;i<t->size;i++){t->data[i]=exp(t->data[i]-mx);s+=t->data[i];} for (size_t i=0;i<t->size;i++)t->data[i]/=s; return s; }
double tensor_mse(const Tensor *a, const Tensor *b) { double s=0; for (size_t i=0;i<a->size;i++){double d=a->data[i]-b->data[i];s+=d*d;} return s/a->size; }
double tensor_sum(const Tensor *t) { double s=0; for (size_t i=0;i<t->size;i++)s+=t->data[i]; return s; }

int tensor_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)
    double d[] = {1,2,3,4,5,6};
    size_t sh[] = {2,3};
    Tensor t = tensor_new(d, sh, 2);
    T(t.size == 6, "2x3 tensor size 6");
    T(tensor_get(&t, 0) == 1, "t[0,0]=1");
    tensor_relu(&t);
    T(tensor_get(&t, 0) == 1, "ReLU(1)=1");

    double a[] = {1,2}, b[] = {3,4};
    size_t s1[] = {2};
    Tensor ta = tensor_new(a, s1, 1), tb = tensor_new(b, s1, 1);
    T(tensor_dot(&ta, &tb) == 11, "dot([1,2],[3,4])=11");

    double dd[] = {1,0,0,0,1,0,0,0,1};
    size_t sh2[] = {3,3};
    Tensor eye = tensor_new(dd, sh2, 2);
    double so[] = {0,0,0};
    tensor_softmax(&eye);
    T(tensor_sum(&eye) > 0.99 && tensor_sum(&eye) < 1.01, "softmax of [1,0,0;0,1,0;0,0,1] sums to 1 per row");

    T(tensor_mse(&ta, &tb) == 4.0, "MSE([1,2],[3,4])=4");
#undef T
    return fail;
}