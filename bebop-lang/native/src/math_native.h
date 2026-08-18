/* Bebop math — native math primitives (calculus, geometry, linear algebra).
 * Pure C11, zero deps. */
#ifndef BEBOP_MATH_H
#define BEBOP_MATH_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

/* ─── Scalar math ─────────────────────────────────── */
double  math_sqrt(double x);
double  math_exp(double x);
double  math_ln(double x);
double  math_sin(double x);
double  math_cos(double x);
double  math_atan2(double y, double x);

/* ─── Vector (geometry) ───────────────────────────── */
typedef struct { double x, y; } Vec2;
typedef struct { double x, y, z; } Vec3;

double  vec2_dot(Vec2 a, Vec2 b);
double  vec2_cross(Vec2 a, Vec2 b);
double  vec2_len(Vec2 v);
Vec2    vec2_norm(Vec2 v);
double  vec2_dist(Vec2 a, Vec2 b);

double  vec3_dot(Vec3 a, Vec3 b);
Vec3    vec3_cross(Vec3 a, Vec3 b);
double  vec3_len(Vec3 v);
Vec3    vec3_norm(Vec3 v);

/* ─── Matrix ──────────────────────────────────────── */
/* Dynamic matrix (caller allocates data). */
typedef struct {
    double *data;  /* row-major, size = rows*cols */
    int     rows, cols;
} Mat;

void    mat_mul(const Mat *a, const Mat *b, Mat *out);
void    mat_transpose(const Mat *m, Mat *out);

/* ─── Calculus ────────────────────────────────────── */
/* Numerical derivative: (f(x+h)-f(x-h))/(2h) */
double  calc_deriv(double (*f)(double), double x, double h);

/* Riemann sum integral: ∫[a,b] f(x) dx, n subintervals */
double  calc_integral(double (*f)(double), double a, double b, int n);

/* Gradient descent: minimize f starting from x0 */
double  calc_gradient_descent(double (*f)(double), double (*df)(double),
                              double x0, double lr, int steps);

/* ─── Deep Learning ───────────────────────────────── */
/* ReLU, sigmoid, tanh — basic activations */
double  nn_relu(double x);
double  nn_sigmoid(double x);
double  nn_tanh(double x);

/* Softmax over n values (in-place, returns sum for verification) */
double  nn_softmax(double *x, int n);

/* Cross-entropy loss: -sum(y[i]*log(p[i])) */
double  nn_cross_entropy(const double *y, const double *p, int n);

/* Dense layer: out = W*x + b. W is row-major [out_dim][in_dim] */
void    nn_dense(const double *W, const double *x, const double *b,
                 int in_dim, int out_dim, double *out);

/* Mean-squared error */
double  nn_mse(const double *y, const double *p, int n);

/* ─── self-test ──────────────────────────────────── */
int     math_self_test(char *out, size_t cap);

#endif