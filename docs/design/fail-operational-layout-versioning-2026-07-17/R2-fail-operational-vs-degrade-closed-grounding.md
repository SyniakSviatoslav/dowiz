# R2 — Fail-Operational / Gradient-State / Temporal-Interpolation vs the adopted degrade-closed doctrine (2026-07-17)

> Grounding pass per the operator's "враховуючи наявні плани та роадмапи". Read-only against the
> already-adopted CORE-ROADMAP-2026-07-17, the 185-item mesh-masterwork V2 ledger, and this
> session's own verification findings. This is an EXTENSION-vs-conflict adjudication, not a new
> initiative. Source concept: `00-SOURCE-DIALOGUE.md` lines 60-88.

---

## 0. TL;DR verdict

**(d) — combination, with a clean resolution, not an open operator gate.**

- **Critical / money-tier data: REAL CONTRADICTION of degrade-closed — but ALREADY adjudicated
  this session, so NOT a new operator question.** "Interpolate/extrapolate a missing-or-corrupt
  critical value rather than stop" is degrade-*OPEN* and directly collides with (i) the load-bearing
  degrade-closed idiom across the kernel, and (ii) the explicit, already-decided RED-LINE **"money
  = discrete integer channel, never interpolated"** (`13-BATCH4…:270-271` `[VERIFIED-CODE]`;
  `15-BATCH6…:353-354` "a fractional/interpolated money value is *unrepresentable*, not
  detected-and-corrected"). The doctrine question here is closed; the dialogue's proposal loses.
- **Non-critical / telemetry-tier data: genuine, compatible EXTENSION (b) that does not exist
  yet.** Forward temporal interpolation/dead-reckoning of a telemetry stream (e.g. courier-position
  smoothing between GPS samples, ETA smoothing) is net-new capability. Item #145 (Survival Mode)
  today is "soft-refuse + serve the **last valid** tensor read-only + drop speculative work"
  (`ALREADY-EQUIVALENT`, `B3 §4`) — it holds the last VALID state, it does **not** extrapolate a
  GUESSED one. So the dialogue's "extrapolate the next state" is strictly beyond what's adopted, and
  for the lossy tier it is compatible with degrade-closed (that tier is already the eventual/adaptive-
  drop lane: items #14, #18).

**Net:** critical stays degrade-closed **unchanged** (already-decided, do not re-litigate);
non-critical gets a genuinely new interpolation capability. No doctrine reversal needed. The only
residual decision is a minor build-prioritization one (build telemetry interpolation now vs
defer-with-trigger under ponytail/YAGNI), not a doctrine conflict requiring adjudication.

---

## 1. What is already load-bearing (degrade-closed) — verified this pass

`degrade-closed` is not a slogan; it is the repeated collapse-direction of the whole kernel. Live
grep hits (this pass): `budget.rs:58,79,83,123,215`, `bounded_drainer.rs:10,16,68,74,120`,
`token_bucket.rs:63,82`, `event_log.rs:526` ("integrity is unprovable — degrade-closed rather than
falsely green"), `hydra.rs` lock-on-bad-integrity, `chaos.rs:574,595`. Roadmap: P-B/P-C/P-E/P-F/P-H
all carry the idiom. P-H A5 states the pole exactly: on sustained disk-full "**the log refuses
rather than lies**… the collapse is typed and safe-directed" (`BLUEPRINT-P-H-ops-telemetry.md:222`).

The invariant: **when a fact cannot be proven, the system REFUSES — it does not manufacture a
plausible substitute.** Temporal Interpolation, applied to a *critical* fact, is the exact inverse:
it manufactures a plausible substitute so the pipeline never stops. That is degrade-OPEN. The
masterwork already named this failure mode when it **REJECTED-ON-PHYSICS** item #4 (Speculative
Consensus): *"degrade-OPEN in a degrade-closed arch"* (ledger row 4).

---

## 2. Are #14 / #145 already the same pattern under other words?

| Dialogue concept | Nearest adopted item | Verdict in ledger | Same thing? |
|---|---|---|---|
| Tiered consistency (Fast-Finality Command / Eventual Data) | **#14** | `ALREADY-EQUIVALENT` — "the commutative/non-commutative split IS this (`event_log` vs `sync_pull`; ADR F-1)" | **Yes, exactly.** The command/data tier split the dialogue reaches for is already built. |
| Gradient State / Graceful Degradation / Survival Mode | **#145** | `ALREADY-EQUIVALENT` — "Survival-Mode soft-refuse + drift-gate non-persist + Locked-readable ('Static Safe Tensor')"; "Survival = endurance, not exclusion" (`event_log.rs:383`) | **Partly.** The *tiering* and *drop-speculative-work* halves are equivalent. The *last-valid-readable* half is equivalent. **But #145 serves the last VALID state; it does NOT forward-extrapolate a guessed one.** |

So: **the "gradient state by data-criticality tier" idea is already the adopted pattern** (Binary→
Gradient is just #14 + #145 restated). The one thing the new dialogue adds that is genuinely NOT in
the corpus is **forward temporal extrapolation of a missing/next value** (linear/quadratic
approximation of state that was never observed). That delta is real — and it is only safe on the
telemetry/eventual tier, never on the command/money tier.

This maps to the P-C three-way synthesis precisely (`BLUEPRINT-P-C-safety-self-healing.md §5`):
Self-Healing = redundant/error-correcting **math** (the last-valid golden re-seed, Reed-Solomon-style
recovery of a value that mathematically WAS there) — **not** interpolation-as-guessing. Snapshot
Re-entry = recovery from the **last VALID epoch**, explicitly "not an extrapolated/guessed one."
The dialogue's Temporal Interpolation is the guessing variant P-C already distinguished itself from.

---

## 3. The sharpest test — apply Fail-Operational LITERALLY to the live money vuln

**The finding (`G11-FAST-PATH-2Q-AUDIT.md:124-126`, `[VERIFIED-CODE]`):** `place_order_js` /
`apply_event_js` (`kernel/src/wasm.rs:276`) is a **stateless** function that **still trusts client
`unit_price`** (`:56/:64/:122/:131`) and has zero references to the event-log/idempotency guards.
The kernel HAS a correct money law (`compute_order_total`, `domain.rs:129-145`) but the product
charge path is dual-authority and does not yet call it as the authority (T2 flip pending,
`18-BATCH9…:163`). This is a live client-forged-total exposure.

**Apply the dialogue's own scheme literally.** The dialogue tags critical/navigation as
Priority-`00` = "рятувати будь-якою ціною" and says the **main rescue tool for `00` is Temporal
Interpolation**: "ядро бере останній валідний тензор і екстраполює стан … замість зупинки"
(`00-SOURCE-DIALOGUE.md:72-77`). Money/order-total is unambiguously the *most* critical tier → `00`
→ therefore, per the dialogue, on a bad total the kernel should **extrapolate a plausible total from
the last valid one and proceed, rather than reject.**

**Result: it makes the vulnerability strictly WORSE.** Today's bug is "accepts a value the client
supplied." The dialogue's fix would upgrade it to "when the supplied value looks wrong, *manufacture*
a plausible-looking value and accept that" — converting a trust-the-client hole into a
fabricate-and-accept hole, and destroying the one correct escape (hard reject → server recompute via
`compute_order_total`). The correct remediation is the **opposite** of Fail-Operational: server-side
recompute + hard-reject mismatch (collapse dual-authority, T2). The money RED-LINE
(`money = discrete integer channel, never interpolated`, Batch4:270/Batch6:353) already forbids the
dialogue's move by making a fractional/interpolated money value *type-unrepresentable*.

**The dialogue's own internal ambiguity is REAL and textually present** (as the task suspected).
Two consecutive exchanges give *opposite* rules for "critical":
- Exchange [lines 66-69]: on degradation **"навігаційні дані ігноруються"** — critical/navigation
  data is **IGNORED** (a fail-safe-ish, discard-the-bad-sample stance).
- Exchange [lines 72-83]: critical data is **"рятувати будь-якою ціною"** via interpolation —
  **SAVED at any cost** (fail-operational).

The dialogue never reconciled these. Neither is safe for money: "ignore the total" isn't even
applicable (the total is present-but-forged, not missing), and "interpolate the total" is worse than
the status quo. The honest conclusion is that **the dialogue's Priority-`00` semantics are
under-specified and self-contradictory**, and the corpus already resolved the money case in the only
safe direction (unrepresentable → refuse), independent of which of the two the dialogue "meant."

Category note that dissolves the tension cleanly: temporal interpolation is valid only for a
**sampled continuous physical quantity** (a sensor stream — the dialogue's avionics/ARINC-653/FPV
domain), where a dropped frame genuinely lies between two observed frames. An order total is a
**discrete authoritative fact computed from inputs**, not a sampled signal — there is no meaningful
"between two totals" to interpolate. So for money the technique is not merely risky, it is
**category-inapplicable**.

---

## 4. If (d): the scope boundary needs NO new machinery

The distinction "interpolatable telemetry vs never-interpolatable command/money" already has a
carrier in today's adopted design — three existing mechanisms compose to enforce it:

1. **Item #15 Priority enum (Critical/Telemetry/System), `ADOPT-AS-COMPOSITION`** — nested
   `TokenBucket` envelopes `BTreeMap<(PeerId, CapabilityClass), TokenBucket>`; the wire tier flag is
   "an envelope selector **checked against capability scope, NEVER self-assigned** (self-assign =
   RC-2 fast-lane)" (`B4 §2.6`). The dialogue's **2-bit header priority maps directly onto
   `CapabilityClass`.** The load-bearing guard is already there: a stream **cannot self-declare
   "I'm telemetry-tier, interpolate me."** A forged-total message therefore cannot smuggle itself
   into the interpolatable lane — the tier is capability-scoped, not header-trusted. This single
   property is what makes the extension safe and is exactly what the dialogue's raw "2 bits in the
   header decide everything" scheme LACKS.
2. **Item #14 non-commutative event_log path** — money/order transitions structurally live on the
   `event_log` (Fast-Finality/Command) side, "**never CRDT-merged**" (#9 money/order half
   `REJECT-ON-CORRECTNESS`, `event_log.rs:4`). Interpolation on that path is already unrepresentable.
3. **`money_guard.rs` / `money.rs` discrete-integer channel** (Batch4:269-271, Batch6:353) — the
   type-level RED-LINE that makes a fractional/interpolated money value un-typeable.

**Concrete boundary:** a new `interpolate_stale()` capability would be gated to
`CapabilityClass::Telemetry` (or `::Optional`) envelopes **only**; `Critical`/money/order envelopes
have no such method and route through the non-commutative event_log where the value is either a
verified recompute or a refusal. This is `#18`'s already-`ADOPT`ed "differentiated routing /
adaptive-drop" (note `#18` also **`REJECT`s** "selective cryptography by priority tier" — every recv
verifies both legs — so authenticity is never tiered even when *freshness* is). No new module, no new
wire field beyond the already-planned CapabilityClass selector; ponytail-clean.

---

## 5. Verdict for the operator

- **Doctrine:** **no conflict requiring adjudication.** Critical/money-tier stays degrade-closed;
  the "interpolate critical data" reading is a real contradiction that the corpus **already
  decided against** this session (money RED-LINE + item #4 rejection). Do not re-open it.
- **Extension:** telemetry-tier forward interpolation is a genuine, compatible net-new capability
  (beyond #145's last-valid-readable), safe because item #15's capability-scoped tier flag prevents
  self-promotion into the interpolatable lane.
- **Only real open item (minor, not a gate):** whether to BUILD telemetry interpolation now or
  defer-with-trigger under YAGNI. No consumer needs it yet (courier dead-reckoning is the plausible
  first one). Recommend **DEFER-WITH-TRIGGER**, consistent with item #138's over-engineering
  discipline — adopt the *tier boundary* now (it's free, #15 already carries it), build the
  *interpolator* only when a concrete telemetry consumer demands it.
- **Independent action item this pass surfaced (not this cluster's to fix):** the live G11
  forged-`unit_price` exposure remediation is the **opposite** of Fail-Operational — server recompute
  via `compute_order_total` + hard-reject + T2 dual-authority collapse. Flagged here because the
  dialogue's principle, applied to it, would have deepened the hole.

---

### Citations (all live-verified this pass)
- `00-SOURCE-DIALOGUE.md:60-88` (Fail-Operational / 2-bit priority / Temporal Interpolation; the
  internal 66-69 vs 72-83 contradiction).
- Kernel degrade-closed: `budget.rs`, `bounded_drainer.rs`, `token_bucket.rs`, `event_log.rs:526`,
  `chaos.rs`, `hydra.rs`. Roadmap: `BLUEPRINT-P-H-ops-telemetry.md:222` (A5 "refuses rather than lies").
- `BLUEPRINT-P-C-safety-self-healing.md §5` (three-way split: Self-Healing=math, Self-Termination=
  hard boundary, Snapshot Re-entry=last VALID epoch, not extrapolated).
- Masterwork V2 ledger: #14, #15, #18, #145 (rows 14, 15, 18, 145); #4 REJECT-ON-PHYSICS (row 4);
  #9 money-half REJECT-ON-CORRECTNESS (row 9).
- Money RED-LINE: `13-BATCH4…:269-272`, `15-BATCH6…:353-354` (`money_guard.rs`, never interpolated).
- Live money vuln: `G11-FAST-PATH-2Q-AUDIT.md:124-126`; T2 flip pending `18-BATCH9…:163`; kernel
  money law `domain.rs:129-145`.
