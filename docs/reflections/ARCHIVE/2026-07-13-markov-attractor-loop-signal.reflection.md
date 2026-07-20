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

DISPOSITION (librarian, 2026-07-20): archived, no new lesson written — this reflection already
terminated in a ratchet artifact (REGRESSION-LEDGER row #18, promoted same day as this
reflection, commit a6a299b4) and the guardrail was not lost, only carried forward: the legacy
thin-layer removal (row #21) ported `tools/loop-signals/markov_attractor.py` byte-for-byte into
`kernel/src/markov.rs` + `kernel/src/bin/markov_attractor.rs`, which reproduce the Python's own
12-case corpus as VbM parity tests (see that file's header). Note for the record: the ledger row
itself shows no PENDING marker (it lists a real commit hash), so the "serious-gate human-gate"
this reflection flagged as outstanding appears to have been cleared after filing — not verified
further here, out of scope for this run. Separately: `.claude/hooks/loop-detector.sh`, the wiring
point this reflection's detector fed, is currently a no-op stub (repo-wide hook disable,
2026-07-15 operator directive) — the detector's logic is preserved and portable, but not
presently invoked by any hook.
