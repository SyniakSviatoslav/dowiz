// ingest_best: read whole file, hand-parse `id,u,v,cell,label` lines (the Rust best row).
use std::env;
use std::fs;
fn main() {
    let p = env::args().nth(1).unwrap();
    let d = fs::read(&p).unwrap();
    let mut lines: i64 = 0;
    let mut sum: i64 = 0;
    let mut i = 0;
    while i < d.len() {
        let mut id: i64 = 0;
        while d[i] != b',' {
            id = id * 10 + (d[i] - b'0') as i64;
            i += 1;
        }
        sum += id;
        while d[i] != b'\n' {
            i += 1;
        }
        i += 1;
        lines += 1;
    }
    println!("{}", lines * 1000003 + sum);
}
