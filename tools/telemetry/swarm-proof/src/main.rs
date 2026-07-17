//! swarm-proof CLI — print the (A) economic + (B) wall-clock proof.
//!
//!   swarm-proof            # full report
//!   swarm-proof --selftest # assert both claims hold

use swarm_proof::{
    crossover_n, sequential_cost, swarm_cost, time_tasks, BLUEPRINT_TOK, EXEC_TOK,
};
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        selftest();
        return;
    }
    report();
}

fn report() {
    println!("=== (A) ECONOMIC CROSSOVER (real 2026 prices) ===");
    for (arch, exec_t) in [("frontier", "cheap"), ("frontier", "mid"), ("mid", "cheap")] {
        let seq1 = sequential_cost(1, arch);
        let sw1 = swarm_cost(1, arch, exec_t);
        let nx = crossover_n(arch, exec_t).unwrap_or(0);
        let sw10 = swarm_cost(10, arch, exec_t);
        let seq10 = sequential_cost(10, arch);
        let pct = if seq10 > 0.0 {
            100.0 * (1.0 - sw10 / seq10)
        } else {
            0.0
        };
        println!(
            "  architect={:9} executor={:6}: 1-task seq=${:.4} swarm=${:.4} | crossover N={} | \
             N=10 swarm=${:.4} vs seq=${:.4} ({:.0}% cheaper)",
            arch, exec_t, seq1, sw1, nx, sw10, seq10, pct
        );
    }
    println!("  NOTE: crossover N is small (<=2); swarm wins for essentially any N>=2.");

    println!();
    println!("=== (B) ENGINE TIMING: parallel vs sequential fan-out (std::thread) ===");
    let n = 8;
    let task = 0.05;
    let t_serial = time_tasks(n, false, task);
    let t_par = time_tasks(n, true, task);
    println!(
        "  N={} x {:.2}s tasks: serial={:.3}s  parallel={:.3}s  speedup={:.1}x",
        n,
        task,
        t_serial,
        t_par,
        if t_par > 0.0 { t_serial / t_par } else { 0.0 }
    );
    println!(
        "  => parallel completes in ~max(task), serial in ~N*max(task) — the real wall-clock lever."
    );
    println!(
        "  (token draw per blueprint: architect in={:.0} out={:.0}; executor in={:.0} out={:.0})",
        BLUEPRINT_TOK.0, BLUEPRINT_TOK.1, EXEC_TOK.0, EXEC_TOK.1
    );
}

fn selftest() {
    // (A) crossover exists and is small; swarm wins at N=10.
    for (arch, exec_t) in [("frontier", "cheap"), ("frontier", "mid"), ("mid", "cheap")] {
        let nx = crossover_n(arch, exec_t).expect("crossover exists");
        assert!(nx <= 2, "crossover N <= 2");
        assert!(
            swarm_cost(10, arch, exec_t) < sequential_cost(10, arch),
            "swarm beats sequential at N=10"
        );
    }
    // (B) parallel faster than serial.
    let _ = Instant::now();
    let serial = time_tasks(8, false, 0.03);
    let parallel = time_tasks(8, true, 0.03);
    assert!(
        parallel < serial * 0.8,
        "parallel ({parallel:.3}s) < serial ({serial:.3}s)"
    );
    println!("SELFTEST PASS: economic crossover + parallel fan-out both hold");
}
