# ADRs — Architecture Decision Records

An ADR records a decision *before* the knowledge behind it is lost — it is a record of reasoning,
not a changelog entry. Date the reasoning and the code separately: if an ADR is written after its
code landed, say so explicitly in the ADR's own `Date:`/`Status:` line (e.g. "written post-hoc,
decision effective <commit>") rather than implying the decision preceded the implementation when it
did not.

(Hermetic-architecture audit finding Mentalism F3, 2026-07-16: ADRs 0007–0009 were all first
committed post-hoc in one commit — a systemic habit that silently weakens the guarantee an ADR is
supposed to give. This note exists so the next ADR doesn't repeat it silently.)
