use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dowiz_kernel::trinary::{Rgb, Tri};

fn bench_trinary_not(c: &mut Criterion) {
    let inputs = [Tri::True, Tri::False, Tri::Unknown];
    c.bench_function("trinary_ops/not_3", |b| {
        b.iter(|| {
            for &t in &inputs {
                black_box(t.not());
            }
        })
    });
}

fn bench_trinary_and(c: &mut Criterion) {
    let inputs = [Tri::True, Tri::False, Tri::Unknown];
    let all_pairs: Vec<(Tri, Tri)> = inputs
        .iter()
        .flat_map(|&a| inputs.iter().map(move |&b| (a, b)))
        .collect();
    c.bench_function("trinary_ops/and_9", |b| {
        b.iter(|| {
            for &(a, b) in &all_pairs {
                black_box(a.and(b));
            }
        })
    });
}

fn bench_trinary_or(c: &mut Criterion) {
    let inputs = [Tri::True, Tri::False, Tri::Unknown];
    let all_pairs: Vec<(Tri, Tri)> = inputs
        .iter()
        .flat_map(|&a| inputs.iter().map(move |&b| (a, b)))
        .collect();
    c.bench_function("trinary_ops/or_9", |b| {
        b.iter(|| {
            for &(a, b) in &all_pairs {
                black_box(a.or(b));
            }
        })
    });
}

fn bench_rgb_roundtrip(c: &mut Criterion) {
    let inputs = [Tri::True, Tri::False, Tri::Unknown];
    c.bench_function("trinary_ops/rgb_roundtrip", |b| {
        b.iter(|| {
            for &t in &inputs {
                let rgb = Rgb::from_tri(t);
                let (r, g, b) = (rgb.0, rgb.1, rgb.2);
                black_box((r, g, b));
            }
        })
    });
}

criterion_group!(
    benches,
    bench_trinary_not,
    bench_trinary_and,
    bench_trinary_or,
    bench_rgb_roundtrip
);
criterion_main!(benches);
