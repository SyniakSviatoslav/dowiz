use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::prompt_enrich::{seed_fabric_prompts, PromptEnrichEngine};

fn bench_load_db(c: &mut Criterion) {
    c.bench_function("enrich_lookup/load_db", |b| {
        b.iter(|| {
            let mut engine = PromptEnrichEngine::new();
            let seeds = seed_fabric_prompts();
            engine.ingest(seeds);
            black_box(engine.total());
        })
    });
}

fn bench_intent_detection(c: &mut Criterion) {
    let prompt_100ch = "I need you to write a comprehensive unit test suite for a Rust codebase \
        that implements a three-valued logic system. The system supports True, False, and Unknown \
        states with Kleene and Lukasiewicz implication operators. Please ensure full coverage.";
    assert!(prompt_100ch.len() >= 100, "prompt must be at least 100 chars");

    c.bench_function("enrich_lookup/intent_100ch", |b| {
        b.iter(|| {
            let intents = dowiz_kernel::prompt_enrich::detect_all_intents(black_box(prompt_100ch));
            black_box(intents);
        })
    });
}

fn bench_enrichment_report(c: &mut Criterion) {
    let prompt_100ch = "I need you to write a comprehensive unit test suite for a Rust codebase \
        that implements a three-valued logic system. The system supports True, False, and Unknown \
        states with Kleene and Lukasiewicz implication operators. Please ensure full coverage.";

    c.bench_function("enrich_lookup/report", |b| {
        let mut engine = PromptEnrichEngine::new();
        let seeds = seed_fabric_prompts();
        engine.ingest(seeds);
        b.iter(|| {
            let report = engine.enrich_report(black_box(prompt_100ch));
            black_box(report.primary_intent);
        })
    });
}

criterion_group!(
    benches,
    bench_load_db,
    bench_intent_detection,
    bench_enrichment_report
);
criterion_main!(benches);
