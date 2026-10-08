//! B1: the Worker's route table (workers/api/src/lib.rs:302-560, 256 routes) is rebuilt on EVERY
//! request (`route()` -> `router(Router::with_data(Req{now_ms}))`). workers-rs 0.8.5 keeps
//! `HashMap<Method, matchit::Router<Handler>>` and inserts one String pattern per route.
//! This bench rebuilds the same shape with matchit 0.7.3 (the version worker 0.8.5 pins) and
//! compares: CURRENT = build + match per request; REPLACEMENT = build once, match per request.
//! Equivalence: every probe path matches the same handler index with the same params on both.
use std::collections::HashMap;
use std::rc::Rc;

type Handler = Rc<dyn Fn(usize) -> usize>;
type Table = HashMap<String, matchit::Router<Handler>>;

fn routes() -> Vec<(String, String)> {
    let txt = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../routes.txt")).expect("routes.txt");
    txt.lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            Some((it.next()?.to_uppercase(), it.next()?.to_string()))
        })
        .collect()
}

/// What `lib.rs::router` does per request: 256 `insert`s, each allocating the pattern String
/// and an `Rc` handler, into a fresh `HashMap`.
fn build(routes: &[(String, String)]) -> Table {
    let mut t: Table = HashMap::new();
    for (i, (m, p)) in routes.iter().enumerate() {
        let h: Handler = Rc::new(move |x| x + i);
        // workers-rs: `self.handlers.entry(method).or_default().insert(pattern, func.clone())`
        t.entry(m.clone()).or_default().insert(p.clone(), h).expect("insert");
    }
    t
}

fn lookup(t: &Table, method: &str, path: &str) -> Option<(usize, Vec<(String, String)>)> {
    let r = t.get(method)?;
    let m = r.at(path).ok()?;
    let idx = (m.value)(0);
    let params = m.params.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    Some((idx, params))
}

const PROBES: &[(&str, &str)] = &[
    ("GET", "/healthz"),
    ("GET", "/api/public/locations/sushi-durres/menu"),
    ("POST", "/api/public/locations/sushi-durres/orders"),
    ("GET", "/api/public/locations/dubin/reservations/r_17/pass"),
    ("GET", "/api/staff/room"),
    ("POST", "/api/staff/orders/o_12/pay"),
    ("GET", "/api/owner/tables/main/4/qr.svg"),
    ("GET", "/api/order/o_9/taste"),
    ("POST", "/api/auth/login"),
    ("GET", "/api/learn/media/a/b/c.png"),
    ("GET", "/api/nothing/here"),
    ("DELETE", "/api/print/job/tok"),
    ("PUT", "/api/owner/products/p_1"),
    ("GET", "/api/version"),
];

pub fn run(reps: usize) {
    let routes = routes();
    println!("B1 router: {} routes, {} probes", routes.len(), PROBES.len());
    // ── equivalence: a table built once answers exactly what a table built per request answers ──
    let once = build(&routes);
    for (m, p) in PROBES {
        let fresh = build(&routes);
        assert_eq!(lookup(&fresh, m, p), lookup(&once, m, p), "probe {m} {p}");
    }
    let hits = PROBES.iter().filter(|(m, p)| lookup(&once, m, p).is_some()).count();
    println!("B1 EQUIV ok: {} probes agree ({hits} hit, {} miss)", PROBES.len(), PROBES.len() - hits);

    const N: u128 = 200; // requests per run
    let (lo, med, hi) = super::median_ns(reps, || {
        super::time_ns(|| {
            for k in 0..N as usize {
                let t = build(&routes);
                let (m, p) = PROBES[k % PROBES.len()];
                std::hint::black_box(lookup(&t, m, p));
            }
        })
    });
    println!("B1 CURRENT  build+match per request: median {:.1} us (min {:.1}, max {:.1})", med as f64 / N as f64 / 1e3, lo as f64 / N as f64 / 1e3, hi as f64 / N as f64 / 1e3);
    let (lo, med2, hi) = super::median_ns(reps, || {
        super::time_ns(|| {
            for k in 0..N as usize {
                let (m, p) = PROBES[k % PROBES.len()];
                std::hint::black_box(lookup(&once, m, p));
            }
        })
    });
    println!("B1 REPLACE  match only per request:  median {:.3} us (min {:.3}, max {:.3})", med2 as f64 / N as f64 / 1e3, lo as f64 / N as f64 / 1e3, hi as f64 / N as f64 / 1e3);
    println!("B1 RATIO {:.0}x; build alone = {:.1} us = {:.2}% of the 10 ms Free cap", med as f64 / med2.max(1) as f64, (med - med2) as f64 / N as f64 / 1e3, (med - med2) as f64 / N as f64 / 1e3 / 100.0);
}
