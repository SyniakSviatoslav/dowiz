//! es-bench -- R-EMBED-STORE P7 (§4.6): bebop-store vs redb, LMDB (heed) and SQLite on the same
//! workloads in the same process, and the bebop regression gate.
//!
//!   es-bench full <engines|all> <N> <VS> <reps> [rows]   the table (needs --features competitors
//!                                                         for anything but `bebop`)
//!   es-bench gate <baseline> [reps]                       0 green / 1 RED / 2 no usable baseline
//!   es-bench gate-record [reps] [runs] [tol_pct]          print a baseline file
//!
//! Scratch databases live under $ES_BENCH_DIR (default ./es-bench-data) and are deleted after
//! each engine. Engines: bebop, bebop4 (v4 tree through Db), redb, lmdb, sqlite; rows: load open verify get_hot prefix
//! get_cold put put_commit compact put_durable put_normal delete.
use es_bench::{bebop, bebop4, gate, work};
#[cfg(feature = "competitors")]
use es_bench::{lmdb_e, redb_e, sqlite_e};

use std::path::PathBuf;

fn arg<T: std::str::FromStr>(a: &[String], i: usize, d: T) -> T {
    a.get(i).and_then(|s| s.parse().ok()).unwrap_or(d)
}

fn scratch() -> PathBuf {
    PathBuf::from(std::env::var("ES_BENCH_DIR").unwrap_or_else(|_| "es-bench-data".into()))
}

fn run_one(engine: &str, c: &work::Cfg) -> Vec<(String, u64)> {
    match engine {
        "bebop" => work::run::<bebop::Bebop>(c),
        "bebop4" => work::run::<bebop4::Bebop4>(c),
        #[cfg(feature = "competitors")]
        "redb" => work::run::<redb_e::Redb>(c),
        #[cfg(feature = "competitors")]
        "lmdb" => work::run::<lmdb_e::Lmdb>(c),
        #[cfg(feature = "competitors")]
        "sqlite" => work::run::<sqlite_e::Sqlite>(c),
        _ => {
            eprintln!("es-bench: engine {engine} unknown or not built (competitors need --features competitors)");
            std::process::exit(2)
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str).unwrap_or("") {
        "full" => {
            let engines: Vec<String> = match a.get(2).map(String::as_str).unwrap_or("all") {
                "all" => ["bebop", "redb", "lmdb", "sqlite"].map(String::from).to_vec(),
                list => list.split(',').map(String::from).collect(),
            };
            let c = work::Cfg {
                n: arg(&a, 3, 10_000),
                vs: arg(&a, 4, 256),
                reps: arg(&a, 5, 5).max(5),
                dir: scratch(),
                only: a.get(6).map(|s| s.split(',').map(String::from).collect()).unwrap_or_default(),
            };
            println!("# es-bench full engines={engines:?} N={} VS={} reps={} arch={}", c.n, c.vs, c.reps, std::env::consts::ARCH);
            let mut sums: Vec<(String, String, u64)> = vec![];
            for en in &engines {
                for (row, s) in run_one(en, &c) {
                    sums.push((row, en.clone(), s));
                }
            }
            let mut bad = 0;
            for (row, en, s) in &sums {
                if let Some((_, e0, s0)) = sums.iter().find(|(r, _, _)| r == row) {
                    if s0 != s {
                        println!("CHECK|{row}|{en} read {s}, {e0} read {s0}|MISMATCH");
                        bad += 1;
                    }
                }
            }
            println!("CHECK|checksums|{} rows compared across engines|{}", sums.len(), if bad == 0 { "EQUAL" } else { "MISMATCH" });
            let _ = std::fs::remove_dir_all(scratch());
            std::process::exit(if bad == 0 { 0 } else { 1 });
        }
        "gate" => {
            let base = PathBuf::from(a.get(2).expect("es-bench gate <baseline> [reps]"));
            let rc = gate::check(&base, &scratch().join("gate"), arg(&a, 3, 15));
            let _ = std::fs::remove_dir_all(scratch());
            std::process::exit(rc)
        }
        "gate-record" => {
            gate::record(&scratch().join("gate"), arg(&a, 2, 15), arg(&a, 3, 5), arg(&a, 4, 15.0));
            let _ = std::fs::remove_dir_all(scratch());
        }
        _ => {
            eprintln!("usage: es-bench full <engines|all> <N> <VS> <reps> [rows] | gate <baseline> [reps] | gate-record [reps] [runs] [tol]");
            std::process::exit(2)
        }
    }
}
