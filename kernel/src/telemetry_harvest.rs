//! telemetry_harvest.rs — std host shim. The pure harvest ledger
//! (`HarvestRecord`, `HarvestReport`, `HarvestLedger`) lives in
//! `dowiz_core::telemetry_harvest`; `record` takes `now_ms` explicitly. The
//! kernel-side wiring probes (prompt_enrich + trinary telemetry) stay here.

pub use dowiz_core::telemetry_harvest::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Probe: enrich path MUST emit a record.
    #[test]
    fn probe_enrich_path_emits_record() {
        let mut ledger = HarvestLedger::new(100);
        let engine = crate::prompt_enrich::PromptEnrichEngine::new();
        let _ = engine.enrich_report_with_telemetry("fix the compilation bug", &mut ledger);
        assert!(!ledger.is_empty(), "enrich path must emit at least one record");
    }

    /// Probe: trinary path MUST emit a record.
    #[test]
    fn probe_trinary_path_emits_record() {
        use crate::trinary::{mul_with_telemetry, TriMatrix, Tri};
        let mut ledger = HarvestLedger::new(100);
        let mut a = TriMatrix::new(2, 2);
        a.set(0, 0, Tri::True);
        a.set(0, 1, Tri::False);
        a.set(1, 0, Tri::Unknown);
        a.set(1, 1, Tri::True);
        let mut b = TriMatrix::new(2, 2);
        b.set(0, 0, Tri::True);
        b.set(0, 1, Tri::Unknown);
        b.set(1, 0, Tri::False);
        b.set(1, 1, Tri::True);
        let _ = mul_with_telemetry(&a, &b, &mut ledger);
        assert!(!ledger.is_empty(), "trinary path must emit at least one record");
        assert_eq!(ledger.len(), 1, "exactly one record for one mul call");
    }

    /// Probe: intent detection path MUST emit a record.
    #[test]
    fn probe_intent_path_emits_record() {
        let mut ledger = HarvestLedger::new(100);
        let intents = crate::prompt_enrich::detect_all_intents_with_telemetry(
            "write a report on the architecture",
            &mut ledger,
        );
        assert!(!intents.is_empty(), "must detect at least one intent");
        assert!(!ledger.is_empty(), "intent path must emit at least one record");
    }
}
