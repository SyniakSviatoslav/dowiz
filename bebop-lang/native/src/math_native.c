/* Bebop math — native implementations (zero libc). */
#include "math_native.h"
#include <string.h>

/* ─── Scalar math (native, no libm) ──────────────────────────────────────── */

double math_sqrt(double x) {
    if (x <= 0) return 0;
    double r = x;
    for (int i = 0; i < 20; i++) r = 0.5 * (r + x / r);
    return r;
}

double math_exp(double x) {
    double r = 1.0, t = 1.0;
    for (int i = 1; i < 20; i++) { t *= x / i; r += t; }
    return r;
}

double math_ln(double x) {
    if (x <= 0) return -1e308;
    double r = x - 1.0;
    double t = r, s = r;
    for (int i = 2; i < 40; i++) { t *= -r; s += t / i; }
    return s;
}

double math_sin(double x) {
    double r = 0.0, t = x;
    for (int i = 1; i < 15; i++) { r += t; t *= -x*x / ((2*i)*(2*i+1)); }
    return r;
}

double math_cos(double x) {
    double r = 1.0, t = 1.0;
    for (int i = 1; i < 15; i++) { t *= -x*x / ((2*i-1)*(2*i)); r += t; }
    return r;
}

double math_atan2(double y, double x) {
    if (x == 0) return (y > 0) ? 1.57079632679 : (y < 0 ? -1.57079632679 : 0);
    double a = (y < 0 ? -y : y) / (x < 0 ? -x : x);
    double r = a;
    double t = a;
    for (int i = 1; i < 15; i++) { t *= -a*a; r += t / (2*i + 1); }
    if (x < 0) r = 3.14159265359 - r;
    return (y < 0) ? -r : r;
}

/* ─── Vector math ───────────────────────────────────────────────────────── */

double vec2_dot(Vec2 a, Vec2 b)  { return a.x*b.x + a.y*b.y; }
double vec2_cross(Vec2 a, Vec2 b) { return a.x*b.y - a.y*b.x; }
double vec2_len(Vec2 v)           { return math_sqrt(v.x*v.x + v.y*v.y); }
Vec2   vec2_norm(Vec2 v)          { double l = vec2_len(v); return (Vec2){v.x/l, v.y/l}; }
double vec2_dist(Vec2 a, Vec2 b)  { double dx=a.x-b.x, dy=a.y-b.y; return math_sqrt(dx*dx+dy*dy); }

double vec3_dot(Vec3 a, Vec3 b)   { return a.x*b.x + a.y*b.y + a.z*b.z; }
Vec3   vec3_cross(Vec3 a, Vec3 b) { return (Vec3){a.y*b.z-a.z*b.y, a.z*b.x-a.x*b.z, a.x*b.y-a.y*b.x}; }
double vec3_len(Vec3 v)           { return math_sqrt(v.x*v.x+v.y*v.y+v.z*v.z); }
Vec3   vec3_norm(Vec3 v)          { double l = vec3_len(v); return (Vec3){v.x/l, v.y/l, v.z/l}; }

/* ─── Matrix ─────────────────────────────────────────────────────────────── */

void mat_mul(const Mat *a, const Mat *b, Mat *out) {
    for (int i = 0; i < a->rows; i++)
        for (int j = 0; j < b->cols; j++) {
            double s = 0;
            for (int k = 0; k < a->cols; k++) s += a->data[i*a->cols+k] * b->data[k*b->cols+j];
            out->data[i*out->cols+j] = s;
        }
}

void mat_transpose(const Mat *m, Mat *out) {
    for (int i = 0; i < m->rows; i++)
        for (int j = 0; j < m->cols; j++)
            out->data[j*out->cols+i] = m->data[i*m->cols+j];
}

/* ─── Calculus ───────────────────────────────────────────────────────────── */

double calc_deriv(double (*f)(double), double x, double h) {
    return (f(x+h) - f(x-h)) / (2.0 * h);
}

double calc_integral(double (*f)(double), double a, double b, int n) {
    double dx = (b - a) / n, s = 0.5 * (f(a) + f(b));
    for (int i = 1; i < n; i++) s += f(a + i * dx);
    return s * dx;
}

double calc_gradient_descent(double (*f)(double), double (*df)(double),
                             double x0, double lr, int steps) {
    double x = x0;
    for (int i = 0; i < steps; i++) x -= lr * df(x);
    return x;
}

/* ─── Deep Learning ──────────────────────────────────────────────────────── */

double nn_relu(double x)          { return x > 0 ? x : 0; }
double nn_sigmoid(double x)       { return 1.0 / (1.0 + math_exp(-x)); }
double nn_tanh(double x)          { double e2 = math_exp(2*x); return (e2-1)/(e2+1); }

double nn_softmax(double *x, int n) {
    double mx = x[0], s = 0;
    for (int i = 1; i < n; i++) if (x[i] > mx) mx = x[i];
    for (int i = 0; i < n; i++) { x[i] = math_exp(x[i] - mx); s += x[i]; }
    for (int i = 0; i < n; i++) x[i] /= s;
    return s;
}

double nn_cross_entropy(const double *y, const double *p, int n) {
    double ce = 0;
    for (int i = 0; i < n; i++) if (y[i] > 0) ce -= y[i] * math_ln(p[i] + 1e-15);
    return ce;
}

void nn_dense(const double *W, const double *x, const double *b,
              int in_dim, int out_dim, double *out) {
    for (int i = 0; i < out_dim; i++) {
        double s = b ? b[i] : 0;
        for (int j = 0; j < in_dim; j++) s += W[i*in_dim+j] * x[j];
        out[i] = s;
    }
}

double nn_mse(const double *y, const double *p, int n) {
    double s = 0;
    for (int i = 0; i < n; i++) { double d = y[i]-p[i]; s += d*d; }
    return s / n;
}

static double math_sq(double x) { return x*x; }
static double math_dsq(double x) { return 2*x; }

/* ─── self-test ─────────────────────────────────────────────────────────── */
#include <stdio.h>
int math_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    /* sqrt */
    double s = math_sqrt(4.0);
    T(s > 1.99 && s < 2.01, "sqrt(4) ≈ 2");
    T(math_sqrt(0) == 0, "sqrt(0) = 0");

    /* sin/cos */
    T(math_sin(0) < 1e-10 && math_sin(0) > -1e-10, "sin(0) = 0");
    T(math_cos(0) > 0.99 && math_cos(0) < 1.01, "cos(0) = 1");

    /* exp/ln */
    T(math_exp(0) > 0.99 && math_exp(0) < 1.01, "exp(0) = 1");
    double l = math_ln(math_exp(1.5));
    T(l > 1.49 && l < 1.51, "ln(exp(1.5)) ≈ 1.5");

    /* vectors */
    Vec2 a = {3, 4};
    T(vec2_len(a) > 4.99 && vec2_len(a) < 5.01, "|(3,4)| = 5");
    Vec3 v1 = {1,0,0}, v2 = {0,1,0};
    Vec3 c = vec3_cross(v1, v2);
    T(c.z > 0.99, "cross((1,0,0),(0,1,0)) = (0,0,1)");

    /* matrix */
    double dA[] = {1,2,3,4}, dB[] = {5,6,7,8}, dC[4];
    Mat A = {dA,2,2}, B = {dB,2,2}, C = {dC,2,2};
    mat_mul(&A, &B, &C);
    T(C.data[0] == 19 && C.data[3] == 50, "[1,2;3,4]*[5,6;7,8] = [19,22;43,50]");

    /* calculus */
    T(calc_deriv(math_sq, 3.0, 0.001) > 5.99 && calc_deriv(math_sq, 3.0, 0.001) < 6.01, "d/dx x^2 at 3 = 6");
    double ig = calc_integral(math_sq, 0, 1, 1000);
    T(ig > 0.33 && ig < 0.34, "∫x^2 [0,1] ≈ 1/3");
    double gd = calc_gradient_descent(math_sq, math_dsq, 5.0, 0.1, 100);
    T(gd < 0.01 && gd > -0.01, "gradient descent on x^2 → 0");

    /* NN */
    T(nn_relu(-3) == 0 && nn_relu(3) == 3, "ReLU(-3)=0, ReLU(3)=3");
    double si = nn_sigmoid(0);
    T(si > 0.49 && si < 0.51, "sigmoid(0)=0.5");

    double sx[3] = {1,2,3};
    nn_softmax(sx, 3);
    double sum = sx[0]+sx[1]+sx[2];
    T(sum > 0.99 && sum < 1.01, "softmax sums to 1");

    double w[] = {1,2,3,4}, x[] = {2,1}, b[] = {0,0}, o[2];
    nn_dense(w, x, b, 2, 2, o);
    T(o[0] == 4 && o[1] == 10, "dense([1,2;3,4]*[2;1]) = [4;10]");

    double y[] = {1,0,0}, p[] = {0.7, 0.2, 0.1};
    double ce = nn_cross_entropy(y, p, 3);
    T(ce > 0, "cross-entropy > 0");

    double mse = nn_mse(y, p, 3);
    T(mse > 0, "MSE > 0");

#undef T
    return fail;
}