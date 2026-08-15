//! trinary.rs — std host shim (pure `Tri`/`TriMatrix`/matrix algebra live in
//! `dowiz_core::trinary`; the two telemetry-recording methods that need
//! `crate::telemetry_harvest::HarvestLedger` stay here as free functions).

pub use dowiz_core::trinary::*;

use crate::telemetry_harvest::HarvestLedger;

/// Matrix multiplication with harvest telemetry recording (kernel-side seam:
/// the no_std core has no `HarvestLedger`).
pub fn mul_with_telemetry(a: &TriMatrix, other: &TriMatrix, ledger: &mut HarvestLedger) -> TriMatrix {
    let result = a.mul(other);
    let success = result.stability_index() > 0.0;
    let value = result.stability_index();
    let cost = (a.rows * a.cols * other.cols) as f64;
    ledger.record("trinary", "mat_mul", success, value, cost);
    result
}

/// Kalman predict with harvest telemetry recording (kernel-side seam).
pub fn kalman_predict_with_telemetry(
    a: &TriMatrix,
    prev: &TriMatrix,
    gain: f64,
    ledger: &mut HarvestLedger,
) -> TriMatrix {
    let result = a.kalman_predict(prev, gain);
    let success = result.stability_index() > 0.0;
    let value = result.stability_index();
    let cost = a.data.len() as f64;
    ledger.record("trinary", "kalman_predict", success, value, cost);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trinary_mul_with_telemetry_emits_record() {
        let mut ledger = HarvestLedger::new(100);
        let mut a = TriMatrix::new(2, 2);
        a.set(0, 0, Tri::True); a.set(0, 1, Tri::False);
        a.set(1, 0, Tri::Unknown); a.set(1, 1, Tri::True);
        let mut b = TriMatrix::new(2, 2);
        b.set(0, 0, Tri::True); b.set(0, 1, Tri::Unknown);
        b.set(1, 0, Tri::False); b.set(1, 1, Tri::True);
        let _ = mul_with_telemetry(&a, &b, &mut ledger);
        assert_eq!(ledger.len(), 1, "mul_with_telemetry must emit exactly 1 record");
        let recs = ledger.records();
        assert_eq!(recs[0].model, "trinary");
        assert_eq!(recs[0].task, "mat_mul");
        assert!(recs[0].success);
    }

    #[test]
    fn trinary_kalman_with_telemetry_emits_record() {
        let mut ledger = HarvestLedger::new(100);
        let mut present = TriMatrix::new(1, 2);
        present.set(0, 0, Tri::True); present.set(0, 1, Tri::False);
        let mut past = TriMatrix::new(1, 2);
        past.set(0, 0, Tri::True); past.set(0, 1, Tri::True);
        let _ = kalman_predict_with_telemetry(&present, &past, 0.2, &mut ledger);
        assert_eq!(ledger.len(), 1, "kalman_predict_with_telemetry must emit exactly 1 record");
        let recs = ledger.records();
        assert_eq!(recs[0].task, "kalman_predict");
    }
}
