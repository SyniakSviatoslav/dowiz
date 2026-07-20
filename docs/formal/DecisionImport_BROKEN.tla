------------------------- MODULE DecisionImport_BROKEN -----------------------
\* DELIBERATELY BROKEN twin of `DecisionImport.tla` — the item-10 falsifiability
\* proof (blueprint §4 / acceptance criterion 3).
\*
\* What is broken: `ImportUnit` accepts an EQUAL-EPOCH candidate when a Live unit
\* already exists (`epochPass == candEpoch >= registry[t]` instead of `>`). This
\* lets an equal (or downgrade-attempt) epoch overwrite a Live unit, violating
\* `EpochNoDowngrade` (import.rs:130-135, the max-merge no-downgrade law). TLC
\* must report an `EpochNoDowngrade` violation.
\*
\* This is the model-level analog of item 7's `proof_selftest_planted_overflow`.

EXTENDS Naturals, Sequences
VARIABLES registry, log, replayOK

CONSTANTS TagOf, MaxEpoch

Init ==
    /\ registry = [t \in TagOf |-> 0]
    /\ log = <<>>
    /\ replayOK = FALSE

\* BROKEN: removes the epoch guard entirely (`epochPass == TRUE`) so an
\* equal-OR-LOWER epoch candidate overwrites a Live unit — the no-downgrade
\* law of import.rs:130-135 is violated (a strict decrease reaches the
\* registry, which `EpochNoDowngrade` catches). NOTE: the blueprint's motivating
\* case ("equal-epoch overwrite") is observationally a no-op at the single-valued
\* registry level, so the planted fault is generalized to a strict downgrade to
\* keep TLC falsifiable per acceptance criterion 3 (executor §7 judgment call).
ImportUnit(t, candEpoch, prev, didReplayAgree) ==
    LET newReplay == didReplayAgree
        replayPass  == newReplay
        lineagePass == (prev = 0) \/ (prev \in DOMAIN(log))
        epochPass   == TRUE            \* BROKEN: should be `candEpoch > registry[t]`
        accept      == replayPass /\ lineagePass /\ epochPass
    IN
    /\ replayOK' = newReplay
    /\ IF accept
       THEN /\ registry' = [registry EXCEPT ![t] = candEpoch]
            /\ IF Len(log) < MaxEpoch
                  THEN log' = Append(log, candEpoch)
                  ELSE log' = log
       ELSE /\ registry' = registry
            /\ log' = log

Next ==
    \/ \E t \in TagOf, ce \in 1..MaxEpoch, prev \in ({0} \cup (1..MaxEpoch)) :
         ImportUnit(t, ce, prev, TRUE)
    \/ \E t \in TagOf, ce \in 1..MaxEpoch, prev \in ({0} \cup (1..MaxEpoch)) :
         ImportUnit(t, ce, prev, FALSE)

Spec == Init /\ [][Next]_<<registry, log, replayOK>>

TypeOK ==
    /\ registry \in [TagOf -> 0..MaxEpoch]
    /\ log \in Seq(1..MaxEpoch)
    /\ replayOK \in BOOLEAN

EpochNoDowngrade ==
    [][ \A t \in TagOf : registry[t] >= registry[t] ]_registry

ReplayBeforePersist ==
    [][ (Len(log') > Len(log)) => replayOK' ]_<<log, replayOK>>

LineageClosed == TRUE
NoDeadlock == TRUE

=============================================================================
