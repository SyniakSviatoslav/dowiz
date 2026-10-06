//! THE INFERENCE, IN INTEGERS ONLY (W-SNN row 1). No float type appears in this file and a test
//! (`snn/tests.rs::the_inference_source_names_no_float`) reads it to keep it that way; the golden
//! (`fixtures/snn/golden.json`) is written by a SECOND implementation in python ints
//! (`snn-train/ref.py`), so the two agree to the integer or the build is red.
//!
//! The model (training: `snn-train/train.py`, floats only there):
//!   x0_i  = v_i . Win / 1000                                   dish stalk (D), Q16, from 27 axes
//!   layer: y_G   = sum_{i in G} rho_t * x_i / |G|              group stalk, per group type t
//!          lap_i = sum_{G ni i} rho_t * (rho_t * x_i - y_G) / deg_i   sheaf Laplacian (rw-normalised)
//!          x_i  -= clamp(lap_i . W_l, -1, 1)                   neural sheaf diffusion step
//!   guest: h = g0 * (u . Win) + g1 * mean_w(x) + g2 * sum_t mean_c(y_t)
//!   score = (beta * cos_pm(lam*u, lam*v_i) + (1 - beta) * cos_pm(h, x_i)) >> 16
//! where `*` with a Q16 operand is `mq` (multiply, then `>> 16`, a floor in both languages) and
//! every other `/` truncates toward zero (Rust's `/`, `tdiv` in the reference).

use super::blob::Model;
use super::{Guest, Menu, AXES, GROUP_TYPES, Q};

/// (a * b) >> 16 over i128: a Q16 multiply, floored.
#[inline]
pub fn mq(a: i64, b: i64) -> i64 {
    ((i128::from(a) * i128::from(b)) >> 16) as i64
}

/// floor(sqrt(n)) by integer Newton from above; 0 for n <= 0.
pub fn isqrt(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let bits = 128 - n.leading_zeros() as i128;
    let mut x: i128 = 1 << ((bits + 1) / 2);
    loop {
        let y = (x + n / x) / 2;
        if y >= x {
            return x;
        }
        x = y;
    }
}

/// Cosine per mille, truncated toward zero; 0 when either side is all zero.
pub fn cos_pm(a: &[i64], b: &[i64]) -> i64 {
    let (mut dot, mut na, mut nb) = (0i128, 0i128, 0i128);
    for (x, y) in a.iter().zip(b) {
        let (x, y) = (i128::from(*x), i128::from(*y));
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0 || nb == 0 {
        return 0;
    }
    (dot * 1000 / isqrt(na * nb)) as i64
}

/// The stalks after the layers, and the group stalks of the last one.
pub struct Stalks {
    pub x: Vec<Vec<i64>>,
    pub y: Vec<Vec<Vec<i64>>>,
}

fn groups_of(m: &Model, menu: &Menu, members: &[Vec<Vec<usize>>], x: &[Vec<i64>]) -> Vec<Vec<Vec<i64>>> {
    (0..GROUP_TYPES)
        .map(|t| {
            (0..menu.n_groups[t])
                .map(|g| {
                    let mem = &members[t][g];
                    (0..m.d)
                        .map(|e| if mem.is_empty() { 0 } else { mem.iter().map(|&i| mq(m.rho[t][e], x[i][e])).sum::<i64>() / mem.len() as i64 })
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// The dish stalks of a menu (guest-independent: computed once per menu and query).
pub fn stalks(m: &Model, menu: &Menu) -> Stalks {
    let mut members: Vec<Vec<Vec<usize>>> = (0..GROUP_TYPES).map(|t| vec![Vec::new(); menu.n_groups[t]]).collect();
    for (i, d) in menu.dishes.iter().enumerate() {
        for &(t, g) in &d.groups {
            members[t as usize][g as usize].push(i);
        }
    }
    let mut x: Vec<Vec<i64>> = menu.dishes.iter().map(|d| (0..m.d).map(|e| (0..AXES).map(|a| d.v[a] * m.win[a][e]).sum::<i64>() / 1000).collect()).collect();
    for l in 0..m.layers {
        let y = groups_of(m, menu, &members, &x);
        x = menu
            .dishes
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let deg = d.groups.len() as i64;
                let lap: Vec<i64> = (0..m.d)
                    .map(|e| {
                        if deg == 0 {
                            return 0;
                        }
                        let s: i64 = d.groups.iter().map(|&(t, g)| {
                            let r = m.rho[t as usize][e];
                            mq(r, mq(r, x[i][e]) - y[t as usize][g as usize][e])
                        }).sum();
                        s / deg
                    })
                    .collect();
                (0..m.d).map(|e| x[i][e] - (0..m.d).map(|dd| mq(lap[dd], m.wl[l][dd][e])).sum::<i64>().clamp(-Q, Q)).collect()
            })
            .collect();
    }
    let y = groups_of(m, menu, &members, &x);
    Stalks { x, y }
}

/// The guest's stalk.
pub fn guest_stalk(m: &Model, s: &Stalks, g: &Guest) -> Vec<i64> {
    let hu: Vec<i64> = (0..m.d).map(|e| (0..AXES).map(|a| g.u[a] * m.win[a][e]).sum::<i64>() / 1000).collect();
    let sw: i64 = g.w.iter().map(|w| w.1).sum();
    let hw: Vec<i64> = (0..m.d)
        .map(|e| if sw > 0 { (g.w.iter().map(|&(i, w)| i128::from(w) * i128::from(s.x[i][e])).sum::<i128>() / i128::from(sw)) as i64 } else { 0 })
        .collect();
    let mut hc = vec![0i64; m.d];
    for t in 0..GROUP_TYPES {
        let sc: i64 = g.c[t].iter().map(|c| c.1).sum();
        if sc > 0 {
            for (e, h) in hc.iter_mut().enumerate() {
                *h += (g.c[t].iter().map(|&(k, c)| i128::from(c) * i128::from(s.y[t][k][e])).sum::<i128>() / i128::from(sc)) as i64;
            }
        }
    }
    (0..m.d).map(|e| mq(m.gam[0][e], hu[e]) + mq(m.gam[1][e], hw[e]) + mq(m.gam[2][e], hc[e])).collect()
}

/// Every dish's integer score for this guest, in menu order.
pub fn scores(m: &Model, menu: &Menu, s: &Stalks, g: &Guest) -> Vec<i64> {
    let h = guest_stalk(m, s, g);
    let lu: Vec<i64> = (0..AXES).map(|a| mq(m.lam[a], g.u[a] * Q / 1000)).collect();
    menu.dishes
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let lv: Vec<i64> = (0..AXES).map(|a| mq(m.lam[a], d.v[a] * Q / 1000)).collect();
            let (s1, s2) = (cos_pm(&lu, &lv), cos_pm(&h, &s.x[i]));
            (m.beta * s1 + (Q - m.beta) * s2) >> 16
        })
        .collect()
}
