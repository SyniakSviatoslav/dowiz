//! micrograd.rs — minimal reverse-mode automatic differentiation
//! (scalar computational graph), the kernel-side autodiff engine.
//!
//! GROWTH-SUBSTRATE PRIMITIVE (Tier B2). The capture-field SIREN / 2D-Gaussian
//! splat fits (physics-ui-capture blueprint, C0..C3) need a deterministic,
//! dependency-free gradient engine so the kernel can fit a field from pixels
//! WITHOUT pulling in a vendor ML runtime (TF/XLA — explicitly rejected in the
//! Master-Integration plan). It is also the substrate for the deterministic
//! online-learner (B3) and the eqc equation-IR autodiff path.
//!
//! Design: each `Value` is `Rc<RefCell<ValueData>>` — a node holding its
//! `data`, accumulated `grad`, its `prev` children, and a `_backward` closure
//! that pushes the local gradient into each child. Ops wire `prev` + a closure;
//! `backward()` does ONE reverse-topological pass. Pure, deterministic, no
//! parallelism, no libm-nondeterminism hazard (same inputs ⇒ bit-identical
//! grads).
//!
//! Verified-by-Math: every op's local gradient is the hand-derived rule
//! (product / quotient / chain / power / sin / cos / exp / ln / tanh / relu);
//! see the `tests` module for hand-oracle checks.

use std::cell::RefCell;
use std::rc::Rc;

/// A differentiable scalar node in the computational graph.
///
/// Cheap to clone: a clone is another handle to the SAME node (`Rc`).
#[derive(Clone)]
pub struct Value(Rc<RefCell<ValueData>>);

struct ValueData {
    data: f64,
    grad: f64,
    /// Direct children in the graph (the operands that produced this node).
    prev: Vec<Value>,
    /// Reverse-mode local gradient push. `None` for leaves.
    backward: Option<Box<dyn Fn(&Value)>>,
}

impl Value {
    /// Constant leaf (gradient source / parameter).
    pub fn new(x: f64) -> Self {
        Value(Rc::new(RefCell::new(ValueData {
            data: x,
            grad: 0.0,
            prev: Vec::new(),
            backward: None,
        })))
    }

    /// Convenience alias: a differentiable variable (same as `new`).
    pub fn var(x: f64) -> Self {
        Self::new(x)
    }

    /// Internal: build a node whose `prev`/`backward` are wired by `wire`.
    /// `wire` receives `&self` (out node) once, after prev is set.
    fn binary(
        &self,
        other: &Value,
        data: f64,
        local: impl Fn(f64, f64, f64) -> (f64, f64) + 'static,
    ) -> Value {
        let sd = self.data();
        let od = other.data();
        let out = Value::new(data);
        out.0.borrow_mut().prev = vec![self.clone(), other.clone()];
        let s = self.clone();
        let o = other.clone();
        out.0.borrow_mut().backward = Some(Box::new(move |v: &Value| {
            let g = v.0.borrow().grad;
            let (gs, go) = local(g, sd, od);
            s.0.borrow_mut().grad += gs;
            o.0.borrow_mut().grad += go;
        }));
        out
    }

    fn unary(&self, data: f64, local: impl Fn(f64, f64) -> f64 + 'static) -> Value {
        let sd = self.data();
        let out = Value::new(data);
        out.0.borrow_mut().prev = vec![self.clone()];
        let s = self.clone();
        out.0.borrow_mut().backward = Some(Box::new(move |v: &Value| {
            let g = v.0.borrow().grad;
            s.0.borrow_mut().grad += local(g, sd);
        }));
        out
    }

    /// Read the node's current value.
    pub fn data(&self) -> f64 {
        self.0.borrow().data
    }

    /// Read the node's accumulated gradient (valid after `backward`).
    pub fn grad(&self) -> f64 {
        self.0.borrow().grad
    }

    /// Mutate the node's value in place (used by optimizers for the SGD step).
    /// Offline-only: the learner updates parameters from the node's local
    /// sample stream; no network, no remote call.
    pub fn set_data(&self, x: f64) {
        self.0.borrow_mut().data = x;
    }

    /// Reset the accumulated gradient to zero (call between independent graphs
    /// if reusing a `Value` as a parameter across steps).
    pub fn zero_grad(&self) {
        self.0.borrow_mut().grad = 0.0;
    }

    pub fn add(&self, o: &Value) -> Value {
        self.binary(o, self.data() + o.data(), |g, _, _| (g, g))
    }

    pub fn sub(&self, o: &Value) -> Value {
        self.binary(o, self.data() - o.data(), |g, _, _| (g, -g))
    }

    pub fn mul(&self, o: &Value) -> Value {
        self.binary(o, self.data() * o.data(), |g, a, b| (g * b, g * a))
    }

    pub fn div(&self, o: &Value) -> Value {
        self.binary(o, self.data() / o.data(), |g, a, b| {
            (g / b, -g * a / (b * b))
        })
    }

    pub fn neg(&self) -> Value {
        self.unary(-self.data(), |g, _| -g)
    }

    pub fn sin(&self) -> Value {
        self.unary(self.data().sin(), |g, a| g * a.cos())
    }

    pub fn cos(&self) -> Value {
        self.unary(self.data().cos(), |g, a| -g * a.sin())
    }

    pub fn exp(&self) -> Value {
        let d = self.data().exp();
        self.unary(d, move |g, _| g * d) // d/dx e^x = e^x
    }

    pub fn ln(&self) -> Value {
        self.unary(self.data().ln(), |g, a| g / a)
    }

    /// f(x) = x^p, p a constant (power rule: p·x^(p−1)).
    pub fn pow(&self, p: f64) -> Value {
        let a = self.data();
        self.unary(a.powf(p), move |g, _| g * p * a.powf(p - 1.0))
    }

    pub fn tanh(&self) -> Value {
        let d = self.data().tanh();
        self.unary(d, move |g, _| g * (1.0 - d * d))
    }

    pub fn relu(&self) -> Value {
        let a = self.data();
        self.unary(
            if a > 0.0 { a } else { 0.0 },
            move |g, _| if a > 0.0 { g } else { 0.0 },
        )
    }

    /// Reverse-topological order over the subgraph rooted at `self`.
    fn topo(&self) -> Vec<Value> {
        let mut seen = std::collections::HashSet::new();
        let mut order = Vec::new();
        fn visit(v: &Value, seen: &mut std::collections::HashSet<usize>, order: &mut Vec<Value>) {
            let key = Rc::as_ptr(&v.0) as usize;
            if seen.contains(&key) {
                return;
            }
            seen.insert(key);
            for p in &v.0.borrow().prev {
                visit(p, seen, order);
            }
            order.push(v.clone());
        }
        visit(self, &mut seen, &mut order);
        order
    }

    /// Reverse-mode backward: fills `.grad` of every leaf. Deterministic single
    /// pass over the reverse-topological order.
    pub fn backward(&self) {
        let order = self.topo();
        self.0.borrow_mut().grad = 1.0;
        for v in order.iter().rev() {
            // Take the closure out (we only need it once) and run it. Scope the
            // mutable borrow so it drops before `bw` re-borrows `v`.
            let bw = {
                let mut data = v.0.borrow_mut();
                data.backward.take()
            };
            if let Some(bw) = bw {
                bw(v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// f(x,y)=x·y at (2,3): ∂f/∂x=3, ∂f/∂y=2.
    #[test]
    fn product_rule() {
        let x = Value::var(2.0);
        let y = Value::var(3.0);
        let f = x.mul(&y);
        f.backward();
        assert!((x.grad() - 3.0).abs() < 1e-12);
        assert!((y.grad() - 2.0).abs() < 1e-12);
        assert!((f.data() - 6.0).abs() < 1e-12);
    }

    /// f(x)=sin(x²) at x=1: f=sin(1); ∂f/∂x=cos(1)·2x=2cos(1)≈1.0806046.
    #[test]
    fn chain_rule_sin_x2() {
        let x = Value::var(1.0);
        let x2 = x.mul(&x);
        let f = x2.sin();
        f.backward();
        let expected = 2.0 * (1.0_f64).cos();
        assert!((f.data() - (1.0_f64).sin()).abs() < 1e-12);
        assert!((x.grad() - expected).abs() < 1e-12);
    }

    /// Gradient-descent step on x² at x=5, lr=0.1: x' = 5 - 0.1·(2·5) = 4.
    #[test]
    fn sgd_step_on_x2() {
        let x = Value::var(5.0);
        let x2 = x.mul(&x); // f=x·x
        x2.backward();
        let lr = 0.1;
        let next = x.data() - lr * x.grad();
        assert!((next - 4.0).abs() < 1e-12);
    }

    /// f(x)=ln(exp(x)) at x=2: f=2, ∂f/∂x=1 (log/exp identity).
    #[test]
    fn exp_log_identity() {
        let x = Value::var(2.0);
        let e = x.exp();
        let f = e.ln();
        f.backward();
        assert!((f.data() - 2.0).abs() < 1e-12);
        assert!((x.grad() - 1.0).abs() < 1e-12);
    }

    /// f=x³ at x=2: f=8, ∂f/∂x=3·x²=12.
    #[test]
    fn power_rule_cube() {
        let x = Value::var(2.0);
        let f = x.pow(3.0);
        f.backward();
        assert!((f.data() - 8.0).abs() < 1e-12);
        assert!((x.grad() - 12.0).abs() < 1e-12);
    }

    /// f=x/y at (6,3): ∂f/∂x=1/3, ∂f/∂y=−6/9=−2/3.
    #[test]
    fn quotient_rule() {
        let x = Value::var(6.0);
        let y = Value::var(3.0);
        let f = x.div(&y);
        f.backward();
        assert!((x.grad() - 1.0 / 3.0).abs() < 1e-12);
        assert!((y.grad() - (-6.0 / 9.0)).abs() < 1e-12);
        assert!((f.data() - 2.0).abs() < 1e-12);
    }

    /// Deterministic: same inputs ⇒ identical gradients on every run.
    #[test]
    fn determinism() {
        let run = || {
            let a = Value::var(0.7);
            let b = Value::var(1.3);
            let c = a.mul(&b).add(&a).sin(); // sin(a·b + a)
            c.backward();
            (a.grad(), b.grad(), c.data())
        };
        let (g1a, g1b, d1) = run();
        let (g2a, g2b, d2) = run();
        assert_eq!(g1a, g2a);
        assert_eq!(g1b, g2b);
        assert_eq!(d1, d2);
    }
}
