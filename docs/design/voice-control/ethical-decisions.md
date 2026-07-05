# Ethical Decisions — voice-control

Council: Triadic (Architect · Breaker · Counsel). Date: 2026-06-30. Owner of record: operator (SyniakSviatoslav).

This file records the disposition of every Counsel ETHICAL-STOP and the human-gated decisions the design defers. Counsel does not override a conscious human; the council does not exit until each ETHICAL-STOP has a recorded decision. Phase 0/1 (engine build + cheap local laptop probe, flag-dark) was judged **ETHICS: CLEAR** by Counsel round 3.

## Resolved at design-time (no human decision required)

| ID | ETHICAL-STOP | Disposition | Rationale |
|----|--------------|-------------|-----------|
| STOP-2 | Disclosure-sheet dark-pattern (OK-by-default) | **DISSOLVED** | Voice is off-by-default ("Not now" = no-op default); persistent on/off setting backs it; sheet appears *after* the mic-tap gesture (informs a gesture already made); guardrail asserts decline leaves mic unactivated + touch working + the two affordances visually equal (`proposal.md:434-438`, test `:608`). |

## Deferred human-gate decisions (recorded; each blocks a specific downstream action, NOT Phase 0/1)

| ID | Gate | Blocks | Trigger / required decision | Owner |
|----|------|--------|------------------------------|-------|
| STOP-1 | Worker/courier voice = labour-surveillance gradient | **DEFERRED — out of active build** | **DEFERRED (operator 2026-06-30): admin/courier voice is removed from the active build and deferred to a separate future council, so this is no longer a near-term gate.** It **re-opens only if a future council takes up admin/courier voice** — at which point the recorded human decision (affirming no-transcript / no-audio / no-per-worker-telemetry as a guardrail before any worker-facing voice ships) is required at that future Phase-3/4 entry. The C-1 actor-anonymous telemetry-schema guardrail (no `courier_id`/`user_id`/latency) stays locked from Phase 0/1, so the gradient remains *not pre-installed*. | operator (future council) |
| R-J | Demand evidence — does anyone actually want voice? | **Phase-1 code** (storefront MicFab) | Before committing Phase-1 build: state the evidence of user demand; if none, an explicit decision that we are building it knowingly without demonstrated demand. Does NOT block the cheap Phase-0 probe. | operator |
| R-I / C-4 | Eval-corpus data-controller + recruitment ethics | **Real-device human recording** (Phase-2 prep) | Before recording real `sq`/`en`/`uk` speakers: name the data controller and affirm the six consent conditions (non-coercive recruitment, explicitly **not** the platform's own couriers/workforce, fair pay, withdrawal right, protocol-scoped consent, vulnerable-population safeguard — `proposal.md:474-487`). Does NOT block the engine build or the architect's own-voice laptop probe. | operator |

## Accessibility framing

Accessibility/inclusion is **NOT** claimed as a justification for Phase 0/1 — relabelled "convenience for capable (WebGPU) devices" (honest drop, Counsel round 2). Any future accessibility-as-reason re-opens as NEEDS-HUMAN and must first close the gap (TTS read-back + reckoning with whom the capability floor excludes) and publish the measured WebGPU-availability rate in the Phase-0 gate report (R-K, required field).

## Status

PHASE 0/1: ethically cleared by the council. **Scope narrowing (operator 2026-06-30): active build = client-facing flow only (storefront menu + checkout READ_ONLY); admin/courier voice removed and deferred to a separate future council.** Consequently **STOP-1 is DEFERRED — out of the active build** (no longer a near-term gate; re-opens only if a future council takes up admin/courier voice). The two storefront gates remain **live** and must each carry a recorded decision in this file before their trigger action: **R-J** (demand evidence → gates Phase-1 storefront code) and **R-I/C-4** (corpus consent → gates real-device recording).
