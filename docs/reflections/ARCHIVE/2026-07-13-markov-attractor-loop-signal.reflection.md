---
CONTEXT:   Operator asked to integrate SLP3 App.A (Markov chains) + "strange attractors"
           into the largest closed loop (the self-improvement/doubt harness) for better
           SIGNAL FINDING, and to "make it full, not blind". Built a Markov-chain attractor
           detector and wired it advisory into loop-detector.sh.
DECISIONS: (1) Resolved "largest closed loop" = the self-improvement harness, whose sensor
           is loop-detector.sh. (2) Detector is ADVISORY (feeds, never overrides, the N=3
           counter) — signals inform, guardrails decide. (3) Zero-dep stdlib, fail-open.
           (4) Was honest that a finite Markov chain has no fractal strange attractor;
           implemented the rigorous analogs (recurrent class / limit cycle / entropy rate).
           (5) Built the general non-symmetric eigensolver (Faddeev-LeVerrier + Durand-Kerner)
           to close the hydraulic-loop μ≈−1 CODE GAP. (6) Did NOT self-write the serious-gate
           override even under "give you all permissions".
WHERE:     Latent defect, not a live failure: loop-detector.sh was a 0th-order detector —
           it only counted CONSECUTIVE failures on ONE identical signature (N=3).
WHY:       CAUSAL ROOT — a per-signature counter models the WRONG object. "Stuck" is a
           property of the DYNAMICS of the tool-event sequence (a recurrent orbit that never
           reaches a progress state), not of any single repeated error. So two real traps
           were invisible: (a) a limit cycle across ≥2 signatures (each counter keeps
           resetting), and (b) high-entropy churn that never reaches a green run. A second
           blindness: counting every successful Bash as "progress" let benign reads
           (ls/cat/grep) inflate the escape signal and mask churn — the fix is progress-vs-probe.
CONFIDENCE: high  (red→green + e2e proven: the live hook fires LIMIT_CYCLE/STRANGE_ATTRACTOR
           on synthetic traps and stays silent on healthy rhythms, incl. the adversarial
           low-entropy-but-progressing case escape=0.5)
NEXT-TIME: When a detector "counts", ask what OBJECT it models — a scalar counter often can't
           see a structural/temporal pattern. Reach for the process-level model (Markov chain,
           spectrum) when the failure mode is a sequence, not an event.
LINK:      tools/loop-signals/markov_attractor.py · tools/loop-signals/test_markov_attractor.py
           · .claude/hooks/loop-detector.sh (block "Markov attractor signal (advisory)")
           · REGRESSION-LEDGER row #18 PENDING (serious-gate human-gate; operator hand required)
---
