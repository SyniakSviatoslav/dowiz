//! B9: `crates/dowiz-hub/src/graph/retrieval.rs:135-144 Graph::ppr` dangling loop -- for every
//! degree-0 node, `next[j] += rank[i]*85/100*restart[j]/SCALE` over ALL n entries, though
//! `restart` is non-zero on at most the seeds (<= 5). REPLACEMENT (class A, bit-identical):
//! iterate the seeds only. Same integer arithmetic in the same order per (i, j) pair, so the
//! result is identical; the closed form (sum the dangling mass once) changes rounding and is
//! NOT used. The code is a copy of `ppr` over a plain adjacency fixture (the Graph builder is
//! private); the ranking logic is the function being measured, not the builder.
const SCALE: i64 = 1_000_000;
const DAMP_NUM: i64 = 85;
const DAMP_DEN: i64 = 100;

struct G {
    n: usize,
    edges: Vec<(usize, usize)>,
    deg: Vec<usize>,
}

fn fixture(dishes: usize, cats: usize, ingredients: usize, orders: usize, dangling: usize) -> G {
    // nodes: 0 = venue, 1..=cats, then ingredients, then dishes, then orders, then `dangling` isolated nodes
    let venue = 0usize;
    let cat0 = 1;
    let ing0 = cat0 + cats;
    let dish0 = ing0 + ingredients;
    let ord0 = dish0 + dishes;
    let n = ord0 + orders + dangling;
    let mut edges = Vec::new();
    for d in 0..dishes {
        edges.push((dish0 + d, venue));
        edges.push((dish0 + d, cat0 + d % cats));
        for k in 0..4 {
            edges.push((dish0 + d, ing0 + (d * 3 + k * 7) % ingredients));
        }
    }
    for o in 0..orders {
        for k in 0..3 {
            edges.push((ord0 + o, dish0 + (o * 5 + k * 11) % dishes));
        }
    }
    let mut deg = vec![0usize; n];
    for &(a, b) in &edges {
        deg[a] += 1;
        deg[b] += 1;
    }
    G { n, edges, deg }
}

fn ppr(g: &G, seeds: &[usize], iterations: usize, seeds_only: bool) -> Vec<(usize, i64)> {
    let n = g.n;
    let mut restart = vec![0i64; n];
    let share = SCALE / seeds.len() as i64;
    for &s in seeds {
        if s < n {
            restart[s] += share;
        }
    }
    let seed_idx: Vec<usize> = { let mut v: Vec<usize> = seeds.iter().copied().filter(|&s| s < n).collect(); v.sort(); v.dedup(); v };
    let deg = &g.deg;
    let mut rank = restart.clone();
    let mut each = vec![0i64; n];
    for _ in 0..iterations {
        let mut next = vec![0i64; n];
        for i in 0..n {
            each[i] = if deg[i] == 0 { 0 } else { rank[i] * DAMP_NUM / DAMP_DEN / deg[i] as i64 };
        }
        for &(a, b) in &g.edges {
            next[b] += each[a];
            next[a] += each[b];
        }
        for i in 0..n {
            if deg[i] == 0 {
                if seeds_only {
                    // restart[j] == 0 for every j not in seeds: the product is 0 and adding 0 is identity,
                    // so skipping those j changes nothing. Seeds are visited in ascending j as before.
                    for &j in &seed_idx {
                        next[j] += rank[i] * DAMP_NUM / DAMP_DEN * restart[j] / SCALE;
                    }
                } else {
                    for (j, r) in restart.iter().enumerate() {
                        next[j] += rank[i] * DAMP_NUM / DAMP_DEN * r / SCALE;
                    }
                }
            }
        }
        for (j, r) in restart.iter().enumerate() {
            next[j] += (SCALE - SCALE * DAMP_NUM / DAMP_DEN) * r / SCALE;
        }
        rank = next;
    }
    let mut out: Vec<(usize, i64)> = rank.into_iter().enumerate().filter(|(_, s)| *s > 0).collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

fn us(n: u128) -> f64 {
    n as f64 / 1e3
}

pub fn run(reps: usize) {
    for &dangling in &[0usize, 10, 100] {
        let g = fixture(165, 12, 76, 300, dangling);
        let seeds = [1 + 165 + 76 + 3, 1 + 165 + 76 + 40, 1 + 12 + 5];
        let a = ppr(&g, &seeds, 12, false);
        let b = ppr(&g, &seeds, 12, true);
        assert_eq!(a, b, "B9 ranking must be bit-identical");
        println!("B9 EQUIV ok: n={} nodes, {} edges, {dangling} dangling, {} ranked", g.n, g.edges.len(), a.len());
        let (_, cur, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(ppr(&g, &seeds, 12, false)); }));
        let (_, rep, _) = super::median_ns(reps, || super::time_ns(|| { std::hint::black_box(ppr(&g, &seeds, 12, true)); }));
        println!("B9 dangling={dangling}: CURRENT {:.0} us | REPLACE {:.0} us | {:.1}x", us(cur), us(rep), us(cur) / us(rep).max(0.001));
    }
}
