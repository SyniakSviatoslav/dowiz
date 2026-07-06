# Resolution — sovereign-core-money-boundary-0b1 (money-boundary extraction council)

Role: System Architect (RESOLVE round). Inputs: `proposal.md` (v1), `breaker-findings.md`,
`counsel-opinion.md`. Each Breaker finding below → **fix** / **accept-risk** / **defer-flag**, with
justification + owner. Each Counsel ETHICAL-STOP → answer. `proposal.md` has been updated in place
with the exact new signatures / diffs an implementer copies 1:1; this file is the disposition log.

Severity roll-up going in: CRITICAL 0 · HIGH 1 · MED 2 · LOW 3 · ETHICAL-STOP 0 (1 open question).

---

## HIGH

### H1 — `apply_tax` f64 short-circuit guard dropped, gate-blind → **FIX** (mandatory; money-parity regression on a red line)

**Finding (Breaker).** The old `apply_tax` opens with `if subtotal == 0 || tax_rate <= 0.0 ||
!tax_rate.is_finite() { return Ok(0); }` (`pricing.rs:50`). The proposal's bare adapter one-liner
`apply_tax(subtotal, round_f64_to_i64(tax_rate*1e6), incl)` drops it. Negative rate (from the
untrusted nullable `tax_rate numeric`, read `unwrap_or(0.0)` at `pg.rs:335`) → negative `rate_micro`
→ negative `Lek` → `Err` → **5xx** where OLD returned `Ok(0)`. `+Infinity` → `round_f64_to_i64(INF·1e6)
= i64::MAX` → `checked_mul` overflow → **5xx**. No existing vector covers negative/non-finite rate,
so the money-parity gate is blind.

**Disposition: FIX — a domain-split guard (NOT accept-risk, NOT deferred).** The guard is placed by
domain, resolving the "which of two options" question in favor of **both, each where it can actually
see the failure**:

- **Core (chosen for the sign/zero arm):** `if subtotal == 0 || rate_micro <= 0 { return Ok(0); }`
  as the first line of `domain::kernel::pricing::apply_tax`. Chosen over a shell-only `tax_rate<=0.0`
  guard because the i64 guard protects **every** future caller — not just today's adapter — from a
  negative `rate_micro` reaching `checked_mul`. The core's "trust the caller" posture is deliberately
  not relied on for the sign invariant: one i64 branch closes the class permanently and catches a
  hypothetical `rate_micro < 0` from any caller a future spec adds.
- **Shell adapter (necessary for the non-finite arm):** `if !tax_rate.is_finite() { return Ok(0); }`
  BEFORE conversion. This arm *cannot* move to the core: `+Infinity` maps to a **positive** `i64::MAX`,
  which the `rate_micro <= 0` core guard passes straight through into overflow. Non-finite has no
  faithful i64 image, so it must be caught in the float domain. The float `tax_rate <= 0.0` arm is
  intentionally not duplicated in the shell — it is subsumed by the core `rate_micro <= 0`.

Together the two arms reproduce OLD `Ok(0)` for negative rate, zero rate, ±Infinity, and NaN.

**Final signatures (unchanged shape; guard is a body prepend):**
```
// core:  pub fn apply_tax(subtotal: i64, rate_micro: i64, price_includes_tax: bool) -> Result<i64, MoneyError>
//        { if subtotal == 0 || rate_micro <= 0 { return Ok(0); }  … }
// shim:  pub fn apply_tax(subtotal: i64, tax_rate: f64, price_includes_tax: bool) -> Result<i64, MoneyError>
//        { if !tax_rate.is_finite() { return Ok(0); }
//          domain::kernel::pricing::apply_tax(subtotal, round_f64_to_i64(tax_rate*1e6), price_includes_tax) }
```

**New byte-parity vectors (added to proposal §6 — OLD `Ok(0)` MUST equal NEW):** core
`apply_tax(1000,-200000,_)=Ok(0)`, `(1000,-1,_)=Ok(0)`, `(1000,0,_)=Ok(0)`, `(0,75000,_)=Ok(0)`,
inclusive-branch variant; shim `adapter::apply_tax(1000, f64::INFINITY|NEG_INFINITY|NAN|-0.2, _)=Ok(0)`.
These are the red→green proof for the exact behavior the integerization was most likely to lose.

*Owner:* System Architect (design) → implementer (0b-1). Proposal updated: §4 decision paragraph
("Tax-guard split"), §4 core-sig comment + shim adapter body, §6 guard vectors, file-plan step 2.

---

## MEDIUM

### M1 — `HashMap`/`HashSet` = first `RandomState` entropy source in the entropy-free core → **FIX** (cheap, removes the problem entirely)

**Finding (Breaker).** `compute_order_pricing` constructs `group_counts: HashMap` (`pricing.rs:233`)
and `seen: HashSet` (`pricing.rs:239`) inside the core. `HashMap::new()` seeds `RandomState` from OS
entropy — the crate's first ambient-entropy read, violating core Law 2, unguarded by
`disallowed-methods`. Output is seed-independent today (lookup-by-key only), but that is incidental
and unenforced; a future iterating edit would go native↔wasm-divergent silently.

**Disposition: FIX — replace with `BTreeMap`/`BTreeSet`.** Ordered, entropy-free, `no RandomState`;
identical API surface (`.get`/`.entry`/`.insert`/`.contains`); std, no new dep; `O(log n)` on
handful-of-groups carts is immaterial (BOE unchanged). Bonus: deterministic iteration order if a
future edit ever iterates the map — closes the latent divergence class, not just today's instance.
Not accept-risk: it is a one-import + two-type-name change that eliminates the violation outright.

*Owner:* implementer (0b-1). Proposal updated: file-plan step 2 (exact BTreeMap/BTreeSet diff),
step 8 (Cargo note corrected: `BTree*` not `Hash*`).

### M2 — `disallowed-types` does not catch inferred-float literals; §9 RED proof over-claims → **FIX (doc precision)** (not a real hole in this move)

**Finding (Breaker).** Gate 2 lints named `f64`/`f32` type positions, not an inferred-float
expression (`let r = 0.1 + 0.2;`). The §9 RED proof exercises only the named-signature path, so the
headline "any float is a hard build failure" is narrower than stated.

**Disposition: FIX = correct the wording, add an honest scope caveat.** Not a present gap: core has
zero float-literal code today and this move adds none; the finding is over-claim in the *document*,
not a defect in the *design*. §9 now states the true bound: named-type `f64` is a hard build failure
(the real leak vector — a caller re-adding `tax_rate: f64` or a float field); an inferred-float
intermediate is not caught by Gate 2 alone, but any such float re-entering the i64 money path needs a
named `as i64` cast that `clippy::as_conversions` (denied workspace-wide) trips — doubly-fenced. Not
blocking; document-accuracy only.

*Owner:* System Architect. Proposal updated: §9 "Scope caveat" bullet under the purity gate.

---

## LOW

### L1 — R1 (sub-meter tier divergence) ships un-gated → **DEFER-FLAG** (accept-risk with a named re-visit trigger; agreed with Counsel)

**Finding (Breaker + Counsel §5).** The ≤3-dp analysis is correct; the sole divergence is a >3-dp
`max_distance_km` with a delivery within 0.5 m — schema-reachable (`numeric`, no scale cap) but
operationally unreachable. The compensating ≤3-dp control is "not blocking this step," risking an
accepted-and-forgotten flag. Counsel confirms grep finds `max_distance_km` in NO owner-facing `*.tsx`
editor.

**Disposition: DEFER-FLAG (accept-risk, condition-bound).** Grep confirms no tier-author UI exists →
R1 is not merely unreachable, it is **dead/unreachable today**, so the compensating control would be
ceremony guarding a surface that does not exist. Recorded as a defer-flag, not a silent accept:
*Owner + re-visit condition:* System Architect flags this to whichever future spec first ships a
tier-author UI (or any non-engineer path to set `max_distance_km`); the ≤3-dp validation / `CHECK
(scale(max_distance_km) <= 3)` becomes a **Definition-of-Done line for that spec** — it must not ship
the editor without it. Until then the risk stays dormant. **MISSING (deferred, explicit):** the DB
CHECK is intentionally NOT landed in 0b-1, to avoid an orphaned constraint no surface yet needs.
Re-visit trigger: the instant a tier-editor spec opens.

*Owner:* System Architect (flag-holder) → future tier-editor spec (DoD carrier). Proposal updated:
§10 R1 rewritten as defer-flag with re-visit condition.

### L2 — dual same-name structs (i64 core vs f64 shim `DeliveryTier`/`FeeLocation`) → **ACCEPT (with doc-fix)** (cheap; no workspace lint warranted)

**Finding (Breaker).** Two same-named money-adjacent structs coexist; a glob import grabbing
`domain::DeliveryTier` where the f64 shim shape was meant is a silent 1000×-scale (km-vs-m) bug. No
current break (pg.rs uses explicit `super::pricing::` paths).

**Disposition: ACCEPT with doc-fix.** A workspace lint just for this is over-engineering (no glob
import exists; explicit paths throughout). Instead: an explicit warning doc-comment on **each of the
four** types — the two core i64 types and the two shim f64 types — each naming its counterpart's full
path and the 1000×-scale hazard, so a reader/future refactorer cannot confuse them. Accept the
coexistence; the doc-comments are the compensating control.

*Owner:* implementer (0b-1). Proposal updated: §4 core-sig block (doc-comments on core
`DeliveryTier`/`FeeLocation`) and shim-sig block (doc-comments on shim `DeliveryTier`/`FeeLocation`).

### L3 — `shifts.rs` independent unrounded `distance_km(...)*1000.0` (geofence) → **ACCEPT-RISK** (explicitly out-of-scope; do not touch `shifts.rs`)

**Finding (Breaker).** `shifts.rs:896` does a second km→m conversion (`*1000.0`, not
`round_f64_to_i64`) with a different rounding convention. It stays in the shell, untouched → no
regression, but the §3 "single meter convention" claim is money-path-only.

**Disposition: ACCEPT-RISK, out-of-scope.** That path is a courier geofence ping — **not money**
(selects no fee, touches no `Lek`), not in 0b-1's scope. `shifts.rs` is left AS-IS (file-plan step 7
already keeps it untouched). The §3 claim is narrowed to "single meter convention *on the money
path*"; geofence meters are a separate non-money concern, no guardrail owed. Recorded as R5.

*Owner:* future geo-distance-unification spec (none scheduled, none needed for money parity).
Proposal updated: §10 new R5.

---

## Counsel ETHICAL-STOP disposition

**Grounded ETHICAL-STOPs: 0.** Counsel §2 finds zero grounded red-line crossings — server-
authoritative money preserved (`Lek` i64, server computes), "схема багата, рантайм мінімальний"
respected (pure move), no PII in core, identical `round_f64_to_i64`. Nothing to revise or escalate to
a human on ethical grounds. Confirmed unchanged after this RESOLVE round: the H1 guard fix
*strengthens* money-parity (it restores the exact OLD `Ok(0)` behavior the extraction risked losing),
so no new red-line surface is introduced.

**Counsel Open Question (§5) — "who authors a delivery tier / who carries R1?" — ANSWERED.** Grep
confirms no owner-facing tier editor exists today → R1 is dead/unreachable, not merely rare. Answer
chosen: neither land an orphan DB CHECK now (option a) nor leave R1 unowned. Instead **option (b),
sharpened**: R1 is a condition-bound defer-flag (L1) whose compensating control is pinned as a
Definition-of-Done line on the future tier-editor spec, with System Architect as the flag-holder who
carries the memory. This removes the "accepted-and-forgotten" failure mode Counsel warned about — the
constraint is owed *to a named future event*, not to an unstaffed role.

**Counsel non-blocking advice.** R2/Option 4A (moving `PricingError.code` to `domain::ErrorCode`,
deleting pg.rs `pricing_code`): kept as CHOSEN and already named plainly in §4/§10-R2 as a deliberate
scope-widening on a red-line file (not silent creep) — Counsel's ask satisfied. 4B remains the
documented minimal-diff fallback.

---

## HARD EXIT check

- Unresolved CRITICAL: **0** (none raised).
- Unresolved HIGH: **0** (H1 → FIX, byte-parity restored + new vectors).
- Unresolved MED: **0** (M1 → FIX BTree*, M2 → doc-precision FIX).
- Unresolved LOW: **0** (L1 → defer-flag with re-visit trigger, L2 → accept+doc-fix, L3 →
  accept-risk out-of-scope).
- Unresolved ETHICAL-STOP: **0** (none grounded; open question answered).
- Back-of-envelope: **still holds** — every disposition is a pure-code / doc change; zero new runtime,
  queries, connections, or deps (BTree* and both guards are std, O(log n)/O(1) branches on tiny
  inputs). §2 connection + numeric-domain budgets unchanged.

**HARD EXIT: reached.**
