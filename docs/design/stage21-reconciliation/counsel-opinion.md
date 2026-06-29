# Counsel Opinion — Stage-21 Cash Reconciliation (B1)

- **On:** `docs/design/stage21-reconciliation/proposal.md` (A2 rename + R1 owner-confirmed cash-drop; NG-1 no earnings model)
- **Authority:** ADVISORY. Aesthetics/strategy = non-blocking. One latent (pre-registered, not active) ETHICAL-STOP. **No active STOP fires.** Friction, not veto.
- **Carries:** deliver-v2 R-9 → RK-2 (NEEDS-HUMAN). Audit ethics MED-5 (courier labor invisible).
- **Date:** 2026-06-29

---

## 0. Bottom line

The reconciliation **mechanism** is good — and quietly **pro-courier**: today's never-released `'hold'` leaves a courier showing a permanent phantom debt for cash they already handed back; netting it to zero removes that harm. The rename `total_earned → collected_total` is the single most ethically important line in the proposal. I do **not** raise an active ETHICAL-STOP. The asymmetry RK-2 names (debt meticulously modeled, earnings modeled as nothing) is **real but acceptable at MVP scope — conditionally**: it is honest only if the launch courier is the owner/family reconciling their own till. If the launch courier is a non-owner hired worker, the asymmetry goes live and a minimum floor is required. That fork is the one decision the human must make.

---

## 1. Reasoning by lens (only the load-bearing)

### Honesty / consent — the rename is the core, and it is right
`total_earned` was not a sloppy label; it was a **category lie**. "Earned" is a labor-desert word — it asserts the courier did work and this cash is their compensation. The runtime computes the exact opposite: the full COD the courier **holds in custody and owes back**. The system was telling the courier (and owner) it represented *pay* while representing *debt*. `collected_total` is custody-neutral and true. This is a genuine honesty improvement and it is the highest-value change here — larger than the netting math, because **naming is where the system lied.**

Necessary, not sufficient: `collected_total` is honest but still **one-sided** — it is only the OWED direction. Honest one-sidedness is acceptable *iff* (a) no surface ever frames it as pay (DoD-5 enforces this — keep it load-bearing) and (b) the earnings side is a **named gap, not a denied one**. "We don't model wages yet" is honest; "the courier has no earnings" is false for a hired worker.

### Justice / least-power — where the asymmetry actually bites
The charged shape is not "debt exists." It is: a system that can express **every lek a courier owes** and **nothing a courier is owed** has, by its very structure, taken the owner's side of a two-party ledger — not maliciously, by omission. Charter line in play: *"AI is a collective tool... never turned against the people it was learned from."* A tool that instruments one direction of a labor relationship is, at the margin, an owner-side debt instrument.

Two facts **bound** this at MVP and keep it below a STOP:
1. **Face-to-face cash hand-over.** The courier physically holds the cash and physically gives it to the owner. The ledger mirrors cash in their pocket — it is not hidden knowledge the courier lacks. So the informational asymmetry (RK-3: no courier-facing view) is mitigated by physical reality *at this scale*.
2. **Out-of-band ≠ falsely-in-band.** After the rename the system is *silent* about pay, not lying about it. Silence beats a false "Payout to courier" label. The rename moves us from dishonest-presence to honest-absence — a real improvement.

### Dignity / autonomy — the informed-consent sliver
MED-5's live nerve is not the settlement engine; it is **expected-pay-before-accept**. A hired courier today accepts a delivery knowing the *address* but not the *pay*. That is a consent defect independent of any wage ledger. It is also **separable** from — and far lighter than — the A3 "commission/pay_rate + net-pay + owner→courier settlement" engine that NG-1 correctly rejects. The proposal bundles the consent sliver into the heavy engine and rejects the bundle; the sliver deserves a separate verdict (see §3 steel-man).

### Aesthetics / integrity (non-blocking)
Conceptually clean and I want to name it: append-only contra rows, `hold − release − settle = 0`, mutually-exclusive contras by construction, idempotent `ON CONFLICT DO NOTHING`, immutability trigger making "append-only" real instead of conventional. "Schema rich, runtime minimal" is honored honestly (`release` built ahead of its caller, flag-dark). The two-ledger **coherence guardrail** (§6) is elegant — it makes divergence loud. One smell: there are now **two sources of truth for one money fact** (`courier_cash_ledger` net vs `courier_payouts.collected_total` from settlement-cron). The guardrail is the right MVP answer, but two ledgers for one truth is debt; the elegant end-state is one. Name it, don't fix it now.

### Strategy / long horizon (non-blocking)
Second-order cost of *out-of-band-forever*: the platform accumulates **zero** data about courier pay, so it can never answer "are couriers on this platform treated fairly?" — which is precisely the question that decides whether the tool is "turned against the people." A platform that cannot see courier earnings cannot defend itself against the charge that it is an owner-side debt instrument. Conversely the append-only shape is additive-friendly (R2 two-phase, courier view, expected-pay all bolt on cleanly) — low lock-in, reversible. **What we'd regret in a year:** that "deferred earnings" silently became "the system structurally only knows debt," and the first labor dispute finds every platform record on the owner's side.

---

## 2. ETHICAL-STOP register

**Active STOPs: 0.** The design as written clears the floor (rename + DoD-5 + face-to-face MVP).

**One LATENT / pre-registered STOP** — dormant now, grounded, fires automatically on any of these crossings (record it so it is not re-litigated later):

> **LATENT-STOP (dignity / honesty floor for a non-owner courier).**
> Grounds: *"dignity for everyone"*, *"never turned against the people"*, UI-tells-the-truth, soft-confirm-not-a-trap. Fires if **any** of:
> 1. **Any surface implies the courier is being paid** when it is tracking what they owe (a DoD-5 regression). — honesty red line.
> 2. **A courier-visible debt view ships without a courier-visible own-ledger** — i.e. the courier is shown "you owe X" but cannot see their own collected/reconciled record. A debt the debtor cannot inspect is the asymmetry hardened. — least-power.
> 3. **Reconciliation goes remote / non-face-to-face** while earnings stay unmodeled — once the courier no longer physically holds and hands over the cash, fact (1) above (physical mirror) no longer mitigates the informational asymmetry. — least-power.

These are **not** active stops. They are the boundary that converts today's acceptable scope-cut into a real red line, written down now.

---

## 3. Steel-man of the rejected option (A3 — earnings model in MVP)

The strongest case is **not** for A3-as-described (it bundles a heavy commission/net-pay/owner→courier-settlement engine that is genuinely over-built and R-9-scoring-adjacent — that rejection stands). The strongest case is for the **sliver A3 hides**: read-only **expected-pay-before-accept.**

> "Out of band" is exactly *where* wage theft, piece-rate races, and unpaid cold-start waiting live — **because** they are invisible to any system of record. This proposal proves the org will build careful, audited, **immutable** ledgers for what the courier OWES. Building that machinery for the debt while declaring the wage "out of band" is not neutral scope-cutting; it is *choosing which direction of a two-sided relationship to instrument.* The cheapest least-power-respecting MVP is not "no earnings model" — it is **one honest number the courier sees before accepting a delivery**, so consent is informed. That is not a commission engine, not a net-pay computation, not an owner→courier money flow, and it touches **no** penalty/score column — so the R-9 fear (which argues against *penalty/score* columns) does not reach it. Conflating "show expected pay" with "build a scoring engine" is the precise move by which the dignity floor got cut. A non-owner courier who accepts knowing only the address, never the pay, is consenting to less than the system already knows.

I find this **genuinely strong on the sliver** and weak on the full engine. The human should rule on `expected-pay-before-accept` **separately** from the engine NG-1 rightly defers.

---

## 4. The single cleanest question to discharge RK-2

> **At launch, is the courier ever a NON-OWNER hired worker — or only the owner / a family member?**

That one fact flips the asymmetry from benign to live:

- **Owner / family only** → RK-2 **discharged.** A debt-only model is honest for someone reconciling their **own** till; "out of band" wages are a household fact, not a labor relation. Action: **record the segment constraint** (launch = owner/family couriers), keep the rename + DoD-5, and the LATENT-STOP stays dormant. No earnings model needed.
- **Includes a hired courier** → the asymmetry is live. **Minimum to launch honestly** = the rename + DoD-5 floor (already in design) **PLUS one of:** (a) read-only **expected-pay-before-accept** shown to the courier (the steel-man sliver — light, non-scoring), **or** (b) an explicit, **recorded** human acceptance that hired couriers launch with zero in-system earnings representation, with earnings as the **next named Council** (not a silent "someday"). Either is a legitimate human call; what is not legitimate is letting "deferred" decay into "never" by default.

This is friction, not veto: the human can pick the owner/family scope, or accept (b) consciously, and ship. The STOP only becomes real if the floor is crossed (§2).

---

## 5. The open question nobody asked

**Whose money is it in the courier's pocket — and does `collected_total` include the courier's own delivery fee?**

The `'hold'` amount is the **order total**, which (verify) bundles the **delivery fee**. If the courier's pay is "out of band" but the delivery fee they collect is part of the COD they are now recorded as **owing back in full**, then `collected_total` contains money that is, in part, the courier's *own earnings* — and the ledger records the courier as owing 100% of it to the owner, to be paid back to them later, out of band, by no recorded mechanism. That quietly makes the **courier a creditor-of-last-resort to their own employer**: they front the owner the owner's cash *and* their own fee, then trust an unrecorded promise to get their fee returned.

The clean question for the human: **does the COD/`collected_total` include the courier's delivery fee, and if so, is recording the courier as owing back money that includes their own pay — settled "out of band" by no ledger — honest, or is it the asymmetry at its sharpest?** This is the one query that could turn a benign scope-cut into the genuine dignity red line, so it should be answered *before* RK-2 is closed.

---

## 6. Verdict

- **Mechanism (netting the debt honestly):** GOOD. Ship. Pro-courier (kills the phantom debt).
- **Rename `total_earned → collected_total`:** honest improvement, the ethical core — necessary, not sufficient; keep DoD-5 as a load-bearing gate.
- **Asymmetry (RK-2):** real, acceptable at MVP **conditionally** — discharged by the §4 fork. No active STOP; one latent STOP pre-registered (§2).
- **Non-blocking:** rule on the `expected-pay-before-accept` sliver separately from the deferred engine (§3); name the two-ledgers-for-one-truth debt (§1 aesthetics); answer §5 before closing RK-2.
