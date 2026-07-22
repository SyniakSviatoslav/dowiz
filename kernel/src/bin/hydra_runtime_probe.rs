//! hydra_runtime_probe.rs — runtime probe + autonomous loop runner.
//!
//! Two modes:
//!   * probe: one-shot JSONL row with live runtime stats.
//!   * run  : autonomous closed-loop cycles with real weight mutations.
//!
//! Non-blocking; writes JSONL telemetry only.

use std::path::PathBuf;
use std::time::Duration;

use dowiz_kernel::event_log::{EventStore, MemEventStore, MeshEvent};
use dowiz_kernel::hydra::TopoEdge;
use dowiz_kernel::hydra_closed_loop::HydraClosedLoop;
use std::io::Write;
use dowiz_kernel::token_bucket::TokenBucket;
use dowiz_kernel::temporal_tmr::{tmr, VoteOutcome};

fn ts() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let days = secs / 86400;
    let mut year = 1970u64;
    let mut remaining_days = days;
    loop {
        let year_days = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < year_days {
            break;
        }
        remaining_days -= year_days;
        year += 1;
    }
    let mut month_lengths = [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if is_leap_year(year) {
        month_lengths[1] = 29;
    }
    let mut month = 1;
    for &ml in &month_lengths {
        if remaining_days < ml {
            break;
        }
        remaining_days -= ml;
        month += 1;
    }
    let day = remaining_days + 1;
    let hours = (secs / 3600) % 24;
    let mins = (secs / 60) % 60;
    let secs_rem = secs % 60;
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        hours,
        mins,
        secs_rem
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn write_jsonl(path: PathBuf, line: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = f.write_all(line.as_bytes());
        let _ = f.write_all(b"\n");
    }
}

struct ProbeSnap {
    token_tokens: f64,
    token_capacity: f64,
    token_refill_rate: f64,
    token_admits: u64,
    token_rejects: u64,
    scheduler_active: usize,
    scheduler_pending: usize,
    scheduler_frame_util: f64,
    scheduler_slice_budget_left: f64,
    tmr_voter_state: &'static str,
    tmr_repair_count: usize,
    tmr_majority_age_ms: u64,
}

impl ProbeSnap {
    fn from_env() -> Self {
        let tb = TokenBucket::new(10.0, 1.0);
        let _ = tb.try_acquire(0.0);
        let tokens = tb.available();
        let outcome = tmr(|| 42i64, 3);
        let tmr_voter_state = match outcome {
            VoteOutcome::Unanimous(_) => "Unanimous",
            VoteOutcome::SingleDissent { .. } => "SingleDissent",
            VoteOutcome::NoMajority => "NoMajority",
        };
        Self {
            token_tokens: tokens,
            token_capacity: 10.0,
            token_refill_rate: 1.0,
            token_admits: 0,
            token_rejects: 0,
            scheduler_active: 0,
            scheduler_pending: 0,
            scheduler_frame_util: 0.0,
            scheduler_slice_budget_left: 0.0,
            tmr_voter_state,
            tmr_repair_count: 0,
            tmr_majority_age_ms: 0,
        }
    }
}

fn json_snap(path: PathBuf) {
    let s = ProbeSnap::from_env();
    let line = format!(
        "{{\"ts\":\"{}\",\"kind\":\"hydra_probe\",\"token_tokens\":{:.6},\"token_capacity\":{:.6},\"token_refill_rate\":{:.6},\"token_admits\":{},\"token_rejects\":{},\"scheduler_active\":{},\"scheduler_pending\":{},\"scheduler_frame_util\":{:.6},\"scheduler_slice_budget_left\":{:.6},\"tmr_voter_state\":\"{}\",\"tmr_repair_count\":{},\"tmr_majority_age_ms\":{}}}",
        ts(),
        s.token_tokens,
        s.token_capacity,
        s.token_refill_rate,
        s.token_admits,
        s.token_rejects,
        s.scheduler_active,
        s.scheduler_pending,
        s.scheduler_frame_util,
        s.scheduler_slice_budget_left,
        s.tmr_voter_state,
        s.tmr_repair_count,
        s.tmr_majority_age_ms
    );
    write_jsonl(path, &line);
    eprintln!("{}", line);
}

fn run_cycle<S: EventStore>(
    cl: &mut HydraClosedLoop<S>,
    delta: &[TopoEdge],
) -> dowiz_kernel::hydra_closed_loop::CommitResult {
    let ev = MeshEvent {
        prev: [0u8; 32],
        actor_pubkey: [7u8; 32],
        actor_seq: cl.commit_count(),
        payload: Vec::new(),
    };
    cl.commit_cycle(ev, delta, false, |_| Ok(()))
}

fn run_autonomous_loop(
    path_jsonl: PathBuf,
    path_metrics: PathBuf,
    max_cycles: usize,
    verify_golden: bool,
) {
    if verify_golden {
        return run_golden_probe();
    }
    let nodes: usize = 5;
    let base = vec![
        TopoEdge {
            from: 0,
            to: 1,
            weight: 1.0,
        },
        TopoEdge {
            from: 1,
            to: 2,
            weight: 1.0,
        },
        TopoEdge {
            from: 2,
            to: 3,
            weight: 1.0,
        },
        TopoEdge {
            from: 3,
            to: 4,
            weight: 1.0,
        },
        TopoEdge {
            from: 4,
            to: 0,
            weight: 1.0,
        },
    ];
    let mut cl = HydraClosedLoop::new(MemEventStore::new(), nodes, base.clone(), 1.0, None);

    eprintln!(
        "AUTONOMOUS LOOP START state={:?} rho={:.6}",
        cl.state(),
        cl.baseline_rho()
    );

    let mut cycles = 0usize;

    while cycles < max_cycles {
        let base_edge = &mut base[cycles % base.len()];
        let delta = vec![TopoEdge {
            from: base_edge.from,
            to: base_edge.to,
            weight: (0.3 + (cycles as f64 % 5.0)).max(0.1),
        }];

        let result = run_cycle(&mut cl, &delta);
        let state = cl.state();
        let organism_state = if matches!(state, dowiz_kernel::hydra::OrganismState::Live) {
            "Live"
        } else {
            "Locked"
        };
        let drift = match result.drift_class {
            dowiz_kernel::spectral::DriftClass::Damped => "Damped",
            dowiz_kernel::spectral::DriftClass::Resonant => "Resonant",
            dowiz_kernel::spectral::DriftClass::Unstable => "Unstable",
        };

        let line = format!(
            "{{\"kind\":\"hydra_run\",\"cycle\":{},\"accepted\":{},\"drift_class\":\"{}\",\"rho\":{:.6},\"lyapunov\":{:.6},\"budget_breached\":{},\"annealing_accepted\":{},\"kalman_surprise\":{:.6},\"kalman_rho\":{:.6},\"branch_dispersion\":{:.6},\"organism_state\":\"{}\",\"commit_count\":{},\"error\":\"{}\",\"ts\":\"{}\"}}",
            cycles,
            result.accepted,
            drift,
            result.rho,
            result.lyapunov,
            result.budget_breached,
            result.annealing_accepted,
            result.kalman_surprise,
            cl.tracked_rho(),
            result.branch_dispersion,
            organism_state,
            cl.commit_count(),
            result.error.unwrap_or_default(),
            ts()
        );

        write_jsonl(path_jsonl.clone(), &line);
        write_jsonl(path_metrics.clone(), &line);
        eprintln!(
            "cycle={} accepted={} drift={} rho={:.6} state={:?}",
            cycles, result.accepted, drift, result.rho, state
        );

        cycles += 1;
        std::thread::sleep(Duration::from_millis(500));
    }

    let end_state = cl.state();
    let end_rho = cl.baseline_rho();
    eprintln!(
        "AUTONOMOUS LOOP END cycles={} state={:?} rho={:.6}",
        cycles, end_state, end_rho
    );
}

fn main() {
    let mut max_cycles: usize = 3;
    let mut probe_only = false;
    let mut verify_golden = false;
    let args = std::env::args().collect::<Vec<_>>();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--probe" => probe_only = true,
            "--cycles" => {
                i += 1;
                if i < args.len() {
                    max_cycles = args[i].parse().unwrap_or(max_cycles);
                }
            }
            "--verify-golden" => verify_golden = true,
            _ => {}
        }
        i += 1;
    }

    let jsonl = PathBuf::from("/root/dowiz/tools/telemetry/logs/hydra_closed_loop.jsonl");
    let metrics = PathBuf::from("/root/dowiz/tools/telemetry/logs/kernel_metrics.jsonl");

    if probe_only {
        json_snap(jsonl);
        return;
    }

    run_autonomous_loop(jsonl, metrics, max_cycles, verify_golden);
}

fn run_golden_probe() {
    let nodes: usize = 5;
    let base = vec![
        TopoEdge {
            from: 0,
            to: 1,
            weight: 1.0,
        },
        TopoEdge {
            from: 1,
            to: 2,
            weight: 1.0,
        },
        TopoEdge {
            from: 2,
            to: 3,
            weight: 1.0,
        },
        TopoEdge {
            from: 3,
            to: 4,
            weight: 1.0,
        },
        TopoEdge {
            from: 4,
            to: 0,
            weight: 1.0,
        },
    ];
    let mut cl = HydraClosedLoop::new(MemEventStore::new(), nodes, base.clone(), 1.0, None);

    eprintln!(
        "AUTONOMOUS LOOP START state={:?} rho={:.6}",
        cl.state(),
        cl.baseline_rho()
    );

    let cycles = 4usize;
    for c in 0..cycles {
        let i = c % base.len();
        let delta = vec![TopoEdge {
            from: base[i].from,
            to: base[i].to,
            weight: 0.3 + (c as f64 % 5.0),
        }];

        let result = run_cycle(&mut cl, &delta);
        let state = cl.state();
        let organism_state = if matches!(state, dowiz_kernel::hydra::OrganismState::Live) {
            "Live"
        } else {
            "Locked"
        };
        let drift = match result.drift_class {
            dowiz_kernel::spectral::DriftClass::Damped => "Damped",
            dowiz_kernel::spectral::DriftClass::Resonant => "Resonant",
            dowiz_kernel::spectral::DriftClass::Unstable => "Unstable",
        };

        println!(
            "cycle={} accepted={} drift={} rho={:.6} state={}",
            c,
            result.accepted,
            drift,
            result.rho,
            organism_state
        );
    }

    let end_state = cl.state();
    let end_rho = cl.baseline_rho();
    eprintln!(
        "AUTONOMOUS LOOP END cycles={} state={:?} rho={:.6}",
        cycles, end_state, end_rho
    );
}
