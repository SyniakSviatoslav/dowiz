//! loops.rs — std shim over the no_std loop-card core.
//!
//! The orchestration state-machine (`LoopCard`, `LoopStatus`, `Certification`,
//! `Orchestrator`, `LoopError`, and the hand-rolled YAML subset parser) lives in
//! `dowiz_core::loops` and is re-exported here. This shim keeps only the
//! std-only pieces: the on-disk real-card round-trip test (`crate::vfs`).

pub use dowiz_core::loops::*;

#[cfg(test)]
mod tests {
    use super::*;

    // GREEN: a real on-disk CERTIFIED loop card parses and dispatches.
    #[test]
    fn real_certified_card_from_disk_dispatches() {
        // error-fix-convergence.yaml is the one CERTIFIED card in loops/.
        let src = crate::vfs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../loops/error-fix-convergence.yaml"
        ))
        .expect("loop yaml present");
        let card = LoopCard::from_yaml(&src).unwrap();
        assert_eq!(card.status(), LoopStatus::Certified);
        // No certification block → admit() skipped; CERTIFIED + real contracts → Ok.
        let t = Orchestrator::dispatch(&card).expect("certified card must dispatch");
        assert_eq!(t.id, "error-fix-convergence");
    }
}
