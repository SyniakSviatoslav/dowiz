//! `lm` — native-kernel living-memory retrieval CLI.
//!
//! Replaces the out-of-tree `tools/telemetry/living_memory.py` (177ms Python
//! per query). This binary links `dowiz_kernel` directly and ranks the live
//! memory corpus with the kernel's own deterministic BM25 + trigram fusion
//! (`dowiz_kernel::retrieval::PrimaryRecall`) — in-process, zero subprocess,
//! zero interpreter. Max speed.
//!
//! Usage:
//!   lm --dir /root/.claude/projects/-root-dowiz/memory --query kalman --k 5
//!   lm --selftest        # ingest a temp corpus + assert deterministic ranking

use dowiz_kernel::retrieval::recall::PrimaryRecall;
use std::path::PathBuf;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut dir: Option<PathBuf> = None;
    let mut query: Option<String> = None;
    let mut k: usize = 5;
    let mut selftest = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--dir" => {
                i += 1;
                dir = Some(PathBuf::from(&args[i]));
            }
            "--query" => {
                i += 1;
                query = Some(args[i].clone());
            }
            "--k" => {
                i += 1;
                k = args[i].parse().unwrap_or(5);
            }
            "--selftest" => selftest = true,
            "-h" | "--help" => {
                println!(
                    "lm — native-kernel living-memory retrieval\n\
                     usage: lm --dir DIR --query Q --k K\n\
                     \x20      lm --selftest"
                );
                exit(0);
            }
            other => {
                eprintln!("lm: unknown arg `{other}`");
                exit(2);
            }
        }
        i += 1;
    }

    if selftest {
        selftest_run();
        return;
    }

    let dir = match dir {
        Some(d) => d,
        None => {
            eprintln!("lm: --dir is required (or use --selftest)");
            exit(2);
        }
    };
    let query = match query {
        Some(q) => q,
        None => {
            eprintln!("lm: --query is required");
            exit(2);
        }
    };

    let pr = match PrimaryRecall::from_dir(&dir) {
        Ok(pr) => pr,
        Err(e) => {
            eprintln!("lm: {e}");
            exit(1);
        }
    };
    for (id, score) in pr.recall_at_k(&query, k) {
        println!("{}\t{:.4}", id, score);
    }
}

/// RED→GREEN self-test: a tiny temp corpus proves (a) from_dir ingests real
/// files, (b) a lexical query surfaces the right doc at rank 1, (c) ranking is
/// deterministic across two calls.
fn selftest_run() {
    let tmp = std::env::temp_dir().join(format!("lm_selftest_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("mk temp");
    std::fs::write(tmp.join("kalman.md"), "kalman filter estimates state from noisy measurements with prediction and update steps").unwrap();
    std::fs::write(tmp.join("delivery.md"), "delivery flow tracks the courier from pickup to dropoff").unwrap();
    std::fs::write(tmp.join("refund.md"), "refund policy returns money to the customer").unwrap();

    let pr = PrimaryRecall::from_dir(&tmp).expect("ingest temp corpus");
    let a = pr.recall_at_k("kalman", 3);
    let b = pr.recall_at_k("kalman", 3);
    assert_eq!(a, b, "ranking must be deterministic");
    assert!(!a.is_empty(), "expected hits");
    assert_eq!(a[0].0, "kalman", "lexical query must surface kalman.md first");
    println!(
        "SELFTEST PASS: ingested {} docs, 'kalman' -> rank1={} score={:.4}",
        a.len(),
        a[0].0,
        a[0].1
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
