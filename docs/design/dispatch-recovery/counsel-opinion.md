# Counsel Opinion — Dispatch Auto-Recovery (B2) + Reconciliation Re-enable (B5)

- Role: Counsel (advisory). Aesthetics/strategy non-blocking. ETHICAL-STOP = friction, not verdict — pause + recorded human decision; human is final.
- Verdict in one line: **the engineering is clean and pattern-honest; one grounded ETHICAL-STOP on the failure tail — the design fixes the dead queue but the *escalation* it routes to is itself unconsumed, so it trades silent-failure-A (dead journal) for silent-failure-B (a maxed-out order no human is told about).**

---

## 1. Reasoning by lens (only what's load-bearing)

### Honesty / degradation (the main lens) — GROUNDED GAP

The proposal's core virtue is real: today the system *lies* (`courier-offer-sweep.ts:47` logs "→ re-offered", `bindingRelease` returns `reoffered:true`) while orders strand in an undrained table. Making re-dispatch real and renaming the signal to the honest "re-enqueued" (Q4) is exactly right — honesty restored at the *head* of the pipeline.

But the **tail** is not honest yet, and the proposal asserts it is. It repeatedly calls the terminal state "owner-visible" (`:83`, `:233`, `:269`) on the strength of `messageBus.publish(ORDER_DISPATCH_FAILED, …)` (`courier-dispatch.ts:68`). I grepped every consumer of that channel:

- **Publishers:** 1 (the worker).
- **Subscribers:** **0.** Not in `bootstrap/messaging.ts` (where every owner Telegram + customer push is wired — `ORDER_ASSIGNMENT_CREATED`, `ORDER_REJECTED`, `ORDER_STATUS` all have handlers; `ORDER_DISPATCH_FAILED` does not). Not in the web app (`apps/web/**` — zero matches). Only the publisher, the registry constant, a verify-orphans script, and docs reference it.

So `ORDER_DISPATCH_FAILED` is **published into the void.** And by the design's own choice (`:235`) the **customer order is "untouched"** — no status change, no push. Net terminal state under a genuine courier shortage:

- **Customer:** still sees whatever they last saw — `CONFIRMED` / "being prepared" / "on its way soon" — *indefinitely*, while no courier is coming and nothing will change it.
- **Owner:** receives **nothing** (no subscriber → no Telegram, no dashboard WS, no order flag).

This is the precise pathology the charter forbids — "fail HONESTLY; the customer must not be left with a false 'on its way' while nothing happens" — re-instantiated one layer down. The fix drains the dead *queue* and lands the failure on a dead *event*. That is the ground for the STOP below. It is **fully fixable and small** (wire the consumer); the STOP is friction asking that the failure-surfacing be in *this* design's scope and DoD, not assumed.

### Courier fairness (brief) — no dignity red line

The new `'assigned'` accept-timeout (Q3, default 5 min) expires an **auto-assigned, not-yet-accepted** binding — not work in progress. It never touches `'accepted'`/`'picked_up'`, so no courier mid-task is "yanked"; a courier with poor signal loses only an offer they hadn't committed to, and the order is re-dispatched (often back to a free courier). That is humane. `R-OPEN-1` (window must exceed the FE accept timer) is the right hygiene and already flagged — keep it gated. **Non-blocking note:** confirm a lapsed accept-timeout carries **no reliability penalty** to the courier (there is no scoring system today — keep it that way; an accept-timeout under bad signal must never silently become a courier-quality mark).

### A6 monitoring trim (8→4) — honest-but-narrowing; strategic flag

Concretely: `EXPECTED_WORKERS` (8) = `dispatcher, settlement-cron, dwell-monitor, anonymizer-retention, signal-raiser, liveness-checker, courier-stale_check, backup-hourly`; only the **first 4** emit P31 heartbeats (`workers.ts:98-103`). The **4 trimmed** are `signal-raiser, liveness-checker, courier-stale_check, backup-hourly`.

Trimming is *half* honest: those 4 genuinely never heartbeat, so A6 watching them = guaranteed-false DRIFT = the alert fatigue that re-triggers C3. Removing guaranteed noise is correct.

The other half is the concern: **two of the four are themselves safety nets.** `liveness-checker` *is* the live watcher of the other heartbeats — after the trim, nothing watches the watcher; its death goes unseen. `backup-hourly` death is only caught via `BACKUP_FAILED` (`messaging.ts:7`), which fires when the worker *runs and fails* — a worker that's *dead and never runs* emits nothing. So for these, the trim doesn't silence a false alarm, it converts a real signal into a blind spot. This is the Goodhart/convergence-theater shape: the monitor goes green because it **stopped looking**, not because the system got healthier.

The honest resolution of "the monitor names a worker that doesn't heartbeat" is to **add the heartbeat** (instrument all 8), not to **remove the name** (watch 4). At pilot, trim-to-4 is an acceptable stopgap *only if* the ADR records, per trimmed worker, its death-detection path — and `liveness-checker`/`backup-hourly` currently have none. Strategic/operability flag, not a red line.

### Other lenses (quick)
- **Idempotency / no-cascade:** the Q6 already-bound guard, `singletonKey: orderId`, and `order_active_uniq` backstop are a genuinely coherent triple — the design earns its "no double-offer" claim. Good.
- **Aesthetics / integrity:** Option C (durable journal + fold drain into the existing sweep) is the restrained, pattern-consistent choice; "schema rich, runtime minimal" honored (one `FORCE RLS` line, no new queue/pool). Elegant — see the steel-man caveat.
- **Reversibility:** strong — data/flag-driven, `FORCE RLS` safe to leave, recon unregisterable instantly. Low regret.

---

## 2. ETHICAL-STOP (1) — friction, human decides

**ETHICAL-STOP-1 — honest-failure to customer AND owner at the dispatch-exhaustion tail.**

- **Grounded line:** "tertia, not punishment / fail honestly — a stranded customer and the owner must not be left a false 'on its way' while nothing happens" (charter; deliver-v2 honest-degradation posture).
- **Why it grounds (not taste):** `ORDER_DISPATCH_FAILED` has **zero consumers** (verified across `apps/api` and `apps/web`), and the customer order is by design "untouched." The proposal's load-bearing claim "owner-visible" is therefore false against the current tree, and the customer-truth is absent entirely. The design's stated reason for existing — *stop silently stranding orders* — is only half-delivered: it moves the silence from the journal to the escalation event.
- **What the STOP asks (pause + recorded human decision — NOT a block):** before this ships, the operator records a decision that the terminal escalation is **surfaced to a human who can act** — minimally a wired `ORDER_DISPATCH_FAILED` → owner channel (Telegram-ops / dashboard WS), and a decision on the **customer's** honest state (see open question). Add these to the DoD; DoD item 3 today stops at "publishes … and deletes the row" — publishing into the void must not count as green.
- **Not permanent, human is final:** if the operator consciously decides the owner-alert wiring lands in an immediately-following PR and accepts a brief window, that is their call to record. Counsel only insists the gap be *seen and chosen*, not assumed-solved.

---

## 3. Non-blocking recommendations

**The single cleanest one:** make the failure honest at *both* ends in this same change — (a) subscribe `ORDER_DISPATCH_FAILED` in `bootstrap/messaging.ts` to the existing owner ops outbox (the `ORDER_ASSIGNMENT_CREATED` handler at `:72` is the template, claim-check clean), and (b) on exhaustion, transition the order to a customer-honest holding state (or fire a customer status push) rather than leaving it "untouched." One subscriber + one status decision closes the tail with the same restraint the rest of the design shows.

Secondary (A6): prefer **instrument the missing 4 heartbeats** over trimming to 4; if trimming now for pilot, record each trimmed worker's death-detection path in the ADR — and note explicitly that `liveness-checker` currently has none.

---

## 4. Steel-man of the rejected option

**Option B (direct pg-boss enqueue at each of the 4 INSERT sites), rejected for "losing the self-healing journal," deserves more credit.** B is the *structurally* honest answer to the exact bug being fixed. Today's pathology is a producer/consumer split — 4 producers, 0 consumers — where someone forgot to wire the drain. The chosen Option C **re-creates that same split** (table + producers + a separate pump pass) and its correctness depends, forever, on the pump staying wired; a future refactor that drops the fold-in pass silently reintroduces stranding, with no compile-time tether. B has **no such latent failure mode**: the enqueue lives in the same transaction as the event that needs it, so "a row with no job" becomes structurally impossible — you cannot forget a consumer that doesn't exist. C's cited cost (a job lost past pg-boss's own retries but before our max-attempts) is real but narrow, and pg-boss already owns retry/archive. So the true trade is **C's self-healing-journal vs B's can't-forget-the-consumer honesty** — and given that *forgetting the consumer is the very bug in front of us*, B's framing was stronger than its one-paragraph dismissal. C remains defensible (reuses the existing table + 4 producers, pattern-consistent, lower diff), but the ADR should name the C-specific risk it accepts: **the drain fold-in is now load-bearing and untethered — guard it with the DoD-1 integration test as a standing regression, so dropping it goes red.**

---

## 5. Open question nobody asked

**Who owns the customer's truth when the owner doesn't act?** The whole design ends at an owner alert (and that alert is currently unwired). It treats "customer order untouched" as neutral — but it is a choice. If the owner is asleep, overwhelmed, or simply doesn't see the alert, the design has **no backstop**: owner-inaction equals permanent customer silence. Should a dispatch-exhausted order, after a bounded owner grace window, auto-transition to a customer-honest terminal/holding state (on-hold / being-re-arranged / refund-path) — so the customer's truth never depends solely on a human noticing an alert that today no one receives?
