// Generates real P83 span-metrics data for ML-DSA-65 signature verification — a
// genuinely microsecond-scale operation (unlike place_order's sub-microsecond
// decide/fold, which rounds to 0us at this instrument's granularity). Run:
// `DOWIZ_SPAN_METRICS_DIR=<dir> cargo run --release --features "telemetry pq"
// --example gen_real_spans_pq -- <n>` (n defaults to 500 — real ML-DSA verify is
// slow enough that 500 iterations is already representative).
use dowiz_kernel::pq::dsa;
use dowiz_kernel::span_metrics::instrument;

fn main() {
    let dir = std::env::var_os("DOWIZ_SPAN_METRICS_DIR").map(std::path::PathBuf::from);
    match dowiz_kernel::span_metrics::init(dir) {
        Ok(()) => eprintln!("gen_real_spans_pq: span observer installed"),
        Err(()) => eprintln!("gen_real_spans_pq: an observer was already installed (ok)"),
    }

    let n: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(500);

    let seed = [7u8; dsa::SEEDBYTES];
    let (pk, sk) = dsa::keygen(&seed);
    let msg = b"dowiz real-metrics span probe";
    let rnd = [3u8; dsa::RNDBYTES];
    let sig = dsa::sign(&sk, msg, &rnd);

    let mut ok_count = 0usize;
    for _ in 0..n {
        if instrument::mldsa_verify(&pk, msg, &sig) {
            ok_count += 1;
        }
    }
    eprintln!("gen_real_spans_pq: exercised mldsa_verify x{n} (ok={ok_count}) — real spans written");
}
