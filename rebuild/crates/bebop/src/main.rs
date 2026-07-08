//! Bebop — OS-agnostic CLI host for the deterministic agent core.
//!
//! This binary is ONE host for [`bebop::core::Core`]; the core itself is OS-agnostic (no clock/RNG/
//! IO). The CLI demonstrates the kernel-backed lifecycle, runs the falsifiable self-test, and
//! refuses to start if any guardrail reads green-but-cannot-fail (Verified-by-Math).

use bebop::{
    Core, GuardKind, Line, SHIP_TEAL, SHIP, TAGLINE, brand, core, guard, say,
};
use domain::{
    Actor, Command, Context, OrderStatus, Ts,
    kernel::pricing::{FeeLocation, PriceInputs, PricingItem, PricingSnapshot, ProductInfo},
};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

fn teal(s: &str) -> String {
    format!("\x1b[38;2;70;176;164m{s}\x1b[0m") // #46B0A4 teal from the Cowboy Bebop spaceship
}

fn bone(s: &str) -> String {
    format!("\x1b[38;2;242;233;219m{s}\x1b[0m") // --bone #F2E9DB
}

fn print_line(line: &Line) {
    match line.tone {
        brand::Tone::Plain => eprintln!("{}", bone(&line.text)),
        brand::Tone::Brand => eprintln!("{} {}", teal(SHIP), bone(&line.text)),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("boot");

    match cmd {
        "boot" => boot(),
        "run" => run(),
        "replay" => replay(args.get(2).map(String::as_str)),
        "guard" => guard_check(args.get(2).map(String::as_str).unwrap_or("")),
        "self-test" => {
            match guard::self_test() {
                Ok(()) => {
                    println!("{} guardrail self-test: {} (RED+GREEN certified)", teal(SHIP), teal("PASS"));
                    std::process::exit(0);
                }
                Err(e) => {
                    eprintln!("{} self-test FAILED: {e}", teal(SHIP));
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("{} unknown command: {other}", teal(SHIP));
            eprintln!("usage: bebop <boot|run|replay <file>|guard <path>|self-test>");
            std::process::exit(2);
        }
    }
}

fn boot() {
    println!("{} {}", teal(SHIP), teal("BEBOP"));
    println!("{}", bone(TAGLINE));
    // The OS must prove it can fail before anything else runs.
    if let Err(e) = guard::self_test() {
        eprintln!("{} FATAL: guardrail self-test failed — {e}", teal(SHIP));
        std::process::exit(1);
    }
    print_line(&Line { text: brand::boot_link().to_string(), tone: brand::Tone::Brand });
}

fn run() {
    // A deterministic order lifecycle through the kernel door — no clock, no RNG.
    let mut core = Core::new();
    let t = Ts(1_700_000_000_000);

    // Observe the price authority (the shell's job). Pickup, one product @ 1000, qty 2, 20% tax.
    let mut product_map = HashMap::new();
    product_map.insert(
        "p1".to_string(),
        ProductInfo { name: "Pizza".into(), price: domain::Lek::new(1_000).unwrap() },
    );
    let snapshot = PricingSnapshot {
        product_map: &product_map,
        mod_map: &HashMap::new(),
        groups_by_product: &HashMap::new(),
    };
    let inputs = PriceInputs {
        snapshot,
        is_pickup: true,
        location: FeeLocation { delivery_fee_flat: None, free_delivery_threshold: None, min_order_value: None },
        distance_m: None,
        tiers: &[],
        rate_micro: 200_000,
        price_includes_tax: false,
    };
    let ctx = Context {
        binding: domain::kernel::policy::BindingState { has_active_binding: false, has_delivered_binding: false },
        refundable_paid: domain::Lek::ZERO,
        pricing: Some(inputs),
    };

    let steps: Vec<Command> = vec![
        Command::PlaceOrder { at: t, actor: Actor::Owner, cart: vec![PricingItem { product_id: "p1".into(), quantity: 2, modifier_ids: vec![] }] },
        Command::Confirm { at: t, actor: Actor::Owner },
        Command::StartPreparing { at: t, actor: Actor::Owner },
        Command::MarkReady { at: t, actor: Actor::Owner },
        Command::Dispatch { at: t, actor: Actor::Owner },
        Command::MarkDelivered { at: t, actor: Actor::System },
    ];

    println!("{} {}", teal(SHIP), bone("order lifecycle (kernel-backed):"));
    for c in steps {
        let step = core.apply(c, &ctx);
        if step.violations.is_empty() {
            println!("  {} {} → {} (cause {})", teal(SHIP), bone("ok"), bone(&format!("{:?}", step.state_after.status)), bone(&step.cause.0[..8]));
        } else {
            println!("  {} blocked: {:?}", teal(SHIP), step.violations);
        }
    }

    // Determinism proof: the exported log is byte-stable for the same inputs.
    let log = core.export_log();
    println!("  {} log bytes: {}", teal(SHIP), bone(&log.len().to_string()));

    // Replay proof: reconstruct from the log and confirm the state matches.
    let replayed = Core::from_log(&log).expect("log replays");
    assert_eq!(replayed.state().status, OrderStatus::Delivered, "replay must reconstruct");
    println!("  {} replay → {:?} (verified)", teal(SHIP), replayed.state().status);
    print_line(&Line { text: brand::ready().to_string(), tone: brand::Tone::Brand });
}

fn replay(file: Option<&str>) {
    let Some(path) = file else {
        eprintln!("{} usage: bebop replay <log.json>", teal(SHIP));
        std::process::exit(2);
    };
    let bytes = match std::fs::read(Path::new(path)) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{} cannot read {path}: {e}", teal(SHIP));
            std::process::exit(1);
        }
    };
    match Core::from_log(&bytes) {
        Ok(core) => println!("{} replayed → status {:?}, {} envelopes", teal(SHIP), core.state().status, core.log().len()),
        Err(e) => {
            eprintln!("{} replay failed: {e}", teal(SHIP));
            std::process::exit(1);
        }
    }
}

fn guard_check(target: &str) {
    let cwd = std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| "/".into());
    let verdict = guard::guard_path(target, &cwd);
    let msg = match verdict {
        GuardKind::RedLine => say("guard.redline").text.to_string(),
        GuardKind::Scope => say("guard.scope").text.to_string(),
        GuardKind::Ok => format!("{} in scope — proceed", SHIP),
    };
    let _ = std::io::stdout().flush();
    println!("{msg}");
    match verdict {
        GuardKind::Ok => std::process::exit(0),
        _ => std::process::exit(1),
    }
}

// keep `core` import referenced (used as bebop::core in doc comments / future hosts)
#[allow(unused_imports)]
use core as _core_marker;
