// ingest_common: owned lines() iterator row (the Rust serde-owned-lines analogue, std-only).
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
fn main() {
    let p = env::args().nth(1).unwrap();
    let f = fs::File::open(&p).unwrap();
    let mut lines: i64 = 0;
    let mut sum: i64 = 0;
    for ln in BufReader::new(f).lines() {
        let s = ln.unwrap();
        let id: i64 = s.split(',').next().unwrap().parse().unwrap();
        sum += id;
        lines += 1;
    }
    println!("{}", lines * 1000003 + sum);
}
