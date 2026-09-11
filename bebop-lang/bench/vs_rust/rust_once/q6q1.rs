// q6q1.rs -- B7 step 2 Rust twin: computes Q6 and Q1 folds for a given N
// (matching bench/oracles/tpch.py exactly). Generator is bit-for-bit identical.
// Usage: q6q1 <N> <q6|q1>   (prints <fold>)
fn lcg(x: u64) -> u64 {
    x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}
fn gen_rows(n: usize, seed: u64) -> Vec<(i64, i64, i64, i64, i64, i64, i64)> {
    let mut x = seed;
    let mut rows = Vec::with_capacity(n);
    for _ in 0..n {
        x = lcg(x);
        let shipdate = (727198i64 + (x % 2557) as i64) as i64;
        x = lcg(x);
        let discount = (x % 11) as i64;
        x = lcg(x);
        let quantity = 1 + (x % 50) as i64;
        x = lcg(x);
        let unit_price_cents = 90000 + (x % 120100) as i64;
        let extendedprice = quantity * unit_price_cents;
        x = lcg(x);
        let returnflag = (x % 3) as i64;
        x = lcg(x);
        let linestatus = (x % 2) as i64;
        x = lcg(x);
        let tax = (x % 9) as i64;
        rows.push((shipdate, discount, quantity, extendedprice, returnflag, linestatus, tax));
    }
    rows
}
fn q6_fold(rows: &[(i64, i64, i64, i64, i64, i64, i64)]) -> i64 {
    let mut total: i64 = 0;
    for &(shipdate, discount, quantity, extendedprice, _, _, _) in rows {
        if shipdate >= 727929 && shipdate < 728294 && discount >= 5 && discount <= 7 && quantity < 24 {
            total = total.wrapping_add(extendedprice.wrapping_mul(discount));
        }
    }
    total
}
fn q1_fold(rows: &[(i64, i64, i64, i64, i64, i64, i64)]) -> i64 {
    let mut groups = std::collections::HashMap::new();
    for &(shipdate, discount, quantity, extendedprice, returnflag, linestatus, _) in rows {
        if shipdate > 729634 {
            continue;
        }
        let g = groups.entry((returnflag, linestatus)).or_insert([0i64; 4]);
        g[0] += 1;
        g[1] += quantity;
        g[2] += extendedprice;
        g[3] += extendedprice * (100 - discount);
    }
    let mut keys: Vec<_> = groups.keys().collect();
    keys.sort();
    let mut vals = Vec::new();
    for &k in &keys {
        let g = groups[k];
        vals.extend_from_slice(&g);
    }
    let mut acc: i64 = 0;
    for &v in &vals {
        acc = acc.wrapping_mul(1000003).wrapping_add(v);
    }
    acc
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = match args.get(1) {
        Some(s) => s.parse::<usize>().unwrap_or(600000),
        None => 600000,
    };
    let query = args.get(2).map(|s| s.as_str()).unwrap_or("q6");
    let rows = gen_rows(n, 20260906);
    match query {
        "q6" => println!("{}", q6_fold(&rows)),
        "q1" => println!("{}", q1_fold(&rows)),
        _ => {
            eprintln!("usage: q6q1 <N> <q6|q1>");
            std::process::exit(1);
        }
    }
}
