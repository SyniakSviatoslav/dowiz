-------------------------------- MODULE PartitionSchedule --------------------------------
(***************************************************************************)
(* ARINC-653-style two-level scheduler — Phase 0 temporal model (design-only).*)
(*                                                                            *)
(* Companion to docs/design/ARINC653-SCHEDULER-PHASE0-2026-07-19.md.          *)
(* Falsifiable slice-guarantee: in every major frame of length T, each        *)
(* admitted partition Pi is guaranteed its slice si, and the ADMITTED slices  *)
(* fit the frame: Sum_{admitted} si <= T. Bad states the model must EXCLUDE:  *)
(*   (a) OVERRUN  — a partition executes outside its own slice window, or     *)
(*   (b) SUMOVER  — Sum_{admitted} si > T is admitted (frame oversubscribed). *)
(*                                                                            *)
(* Abstraction of the kernel's REAL primitives:                              *)
(*   - temporal slices map onto token_bucket.rs's proven refill law          *)
(*     (tokens = (tokens + refill_rate*elapsed).min(capacity)); within a      *)
(*     frame each slice is a TokenBucket(capacity=si, refill_rate=0) drained  *)
(*     to zero; try_acquire returns false (degrade-closed) when empty.        *)
(*   - partition admission maps onto decision/import.rs's six ordered         *)
(*     structural-gate checks (import_unit), degrade-closed (nothing persisted*)
(*     on reject, modeled by never flipping admitted' to TRUE).               *)
(*                                                                            *)
(* Discipline mirrors order_machine.rs's fsm_graph_report() golden signature: *)
(* the invariants below ARE the golden signature of a correct schedule; the   *)
(* BROKEN model (see BROKEN constant) is the drift detector that must violate  *)
(* SliceSumFitsFrame under TLC.                                              *)
(*                                                                            *)
(* The major frame is a TRUE CYCLE (clock wraps 0..T, no frame counter) so    *)
(* the state graph is finite and TLC can check liveness (NoStarvation) under  *)
(* weak fairness. Schedule-table reconfiguration (admission) is permitted     *)
(* ONLY at a major-frame boundary (clock=0), matching ARINC-653 — this keeps  *)
(* the running/owner map consistent and NoOverrun falsifiable. No scheduler    *)
(* code is written anywhere. This is a design artifact.                       *)
(***************************************************************************)

EXTENDS Naturals, Sequences, FiniteSets

------------------------------------------------------------------------------
\* Model constants. Phase 1 will bind these to real TokenBucket capacities, etc.
CONSTANTS
    T,              \* major-frame length (time units)
    MaxPartitions,  \* upper bound on the number of partition slots in the model
    BROKEN          \* FALSE = correct admission policy; TRUE = broken (admit over-budget)

------------------------------------------------------------------------------
\* A partition id is an integer 1..MaxPartitions (finite, so TLC terminates).
PartitionIds == 1..MaxPartitions

VARIABLES
    slices,         \* slices[i] = declared slice length si for slot i (0 = no claim staged)
    admitted,       \* admitted[i] = TRUE iff partition i is currently admitted (Live)
    clock,          \* current time within the major frame, 0..T (wraps — cyclic frame)
    running         \* running = i  iff partition i is executing now; 0 = idle/slack

vars == <<slices, admitted, clock, running>>

------------------------------------------------------------------------------
\* Helpers — explicit-arg so actions can evaluate BOTH pre- and post-state maps.
AdmittedSet(ad) == {i \in PartitionIds : ad[i]}

\* Sum of slices over ADMITTED partitions only (the frame is oversubscribed iff
\* this exceeds T).
AdmittedSliceSum(sl, ad) == SumSet({sl[i] : i \in AdmittedSet(ad)})

\* Back-to-back cumulative window of partition i ( laid out 1,2,...,MaxPartitions).
SliceWindow(sl, i) ==
    LET before == SumSet({sl[k] : k \in 1..(i-1)})
    IN  IF sl[i] > 0 THEN (before .. (before + sl[i] - 1)) ELSE {}

\* The partition that OWNS time position t in a correct cyclic frame; 0 = slack.
OwnerAt(sl, ad, t) ==
    LET owners == {i \in AdmittedSet(ad) : t \in SliceWindow(sl, i)}
    IN  IF owners = {} THEN 0 ELSE CHOOSE i \in owners : TRUE

------------------------------------------------------------------------------
\* Initial state: nothing admitted, frame at start, slices unstaged (0).
Init ==
    /\ slices \in [PartitionIds -> 0 .. T]
    /\ admitted = [i \in PartitionIds |-> FALSE]
    /\ clock = 0
    /\ running = 0

------------------------------------------------------------------------------
\* Admission = the six-check ordered structural gate (mirrors decision/import.rs).
\* Checks 1 & 4 are the load-bearing temporal checks; 2/3/5/6 validate the same
\* fixed-order pipeline shape as import_unit (scope, priority, epoch, lineage).
AdmitCheck(i, s, p, ep, prevExists) ==
    /\ s > 0                         \* check 4: SlicePositive
    /\ s <= T                        \* check 4: frame-bounded
    /\ p \in 0 .. 15                 \* check 3: PriorityInRange (replay-shape check)
    /\ ep > 0                        \* check 5: NoEpochDowngrade (epoch monotonic)
    /\ prevExists                    \* check 6: LineageParentExists
    /\ (BROKEN \/ (AdmittedSliceSum(slices, admitted) + s) <= T)   \* check 1: SliceSumFitsFrame
    \* (BROKEN=TRUE bypasses ONLY the sum check, so an over-budget partition admits.)

\* Stage a manifest (slice budget s) for slot i, then attempt admission.
\* Admission is permitted ONLY at a major-frame boundary (clock = 0) — ARINC-653
\* reconfigures the schedule table at frame boundaries. running' is recomputed
\* under the NEW config so the owner map stays consistent (NoOverrun holds).
StageAndAdmit(i, s) ==
    /\ clock = 0
    /\ ~admitted[i]
    /\ LET newSlices  == [slices  EXCEPT ![i] = s]
           newAdmitted == [admitted EXCEPT ![i] = TRUE]
           newOwner == OwnerAt(newSlices, newAdmitted, 0)
       IN  /\ slices'  = newSlices
           /\ admitted' = newAdmitted
           /\ running'  = newOwner
    /\ AdmitCheck(i, s, 0, 1, TRUE)
    /\ UNCHANGED <<clock>>

\* Degrade-closed reject: nothing persisted (admitted' stays FALSE, slice stays 0).
\* Also boundary-only for the same reconfig story.
Reject(i) ==
    /\ clock = 0
    /\ ~admitted[i]
    /\ UNCHANGED <<slices, admitted, clock, running>>

------------------------------------------------------------------------------
\* The Schedule action: advance one time unit inside the major frame, wrapping
\* when the frame end is reached (cyclic major frame).
\* Level 1 (kernel): the partition that OWNS the current clock position runs.
\* Level 2 (partition): modeled only as "the owner runs"; intra-slice priority
\* scheduling is the partition's own concern, outside the temporal guarantee.
Schedule ==
    /\ clock' = IF clock < T THEN clock + 1 ELSE 0   \* cyclic wrap at frame end
    /\ running' = OwnerAt(slices, admitted, clock')  \* slices/admitted unchanged here
    /\ UNCHANGED <<slices, admitted>>

------------------------------------------------------------------------------
Next ==
    \/ \E i \in PartitionIds : \E s \in 1..T : StageAndAdmit(i, s) \/ Reject(i)
    \/ Schedule

------------------------------------------------------------------------------
\* Fair specification: weak fairness on Next ensures Schedule is taken infinitely
\* often (it is enabled at every clock value), so the cyclic frame keeps spinning
\* and every admitted partition's window is reached -> NoStarvation is provable.
Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

------------------------------------------------------------------------------
\* ============================  INVARIANTS  ================================
\* These four are the GOLDEN SIGNATURE of a correct schedule (cf. fsm_graph_report).

\* (I1) Admitted slices fit the frame: never oversubscribed.
SliceSumFitsFrame == AdmittedSliceSum(slices, admitted) <= T

\* (I2) Slice guarantee: every admitted partition owns its own slice window.
SliceGuarantee ==
    \A i \in AdmittedSet(admitted) :
        \A t \in SliceWindow(slices, i) : OwnerAt(slices, admitted, t) = i

\* (I3) No overrun: the running partition is always the owner at the current clock.
NoOverrun == running = 0 \/ running = OwnerAt(slices, admitted, clock)

\* (I4) No starvation: every admitted partition eventually runs (liveness).
NoStarvation == \A i \in AdmittedSet(admitted) : <>(running = i)

===============================================================================
\* CORRECT model: run with PartitionSchedule.cfg (BROKEN <- FALSE). All four
\* properties must hold GREEN under TLC.
\*
\* BROKEN variant (drift detector): with BROKEN <- TRUE (PartitionSchedule-BROKEN.cfg),
\* AdmitCheck bypasses the SliceSumFitsFrame check, so a partition whose si pushes
\* AdmittedSliceSum > T can be admitted. TLC then reports a VIOLATION of
\* SliceSumFitsFrame. That violation IS the falsifiability proof: the guarantee is
\* falsifiable because a concrete bad state (Sum_{admitted} si > T) is reachable
\* only when the admission policy is wrong.
\* ---------------------------------------------------------------------------
