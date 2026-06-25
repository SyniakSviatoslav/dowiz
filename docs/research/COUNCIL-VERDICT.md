# Triadic Council Verdict — what to borrow/adapt NOW at MVP

Convened on the OSS-teardown research (`SYNTHESIS.md` S5). Three seats — Architect (proposes),
Breaker (proves it breaks), Counsel (strategy/ethics) — each grounded against the **real codebase**.
Companion: `COUNCIL-architect-mvp-borrow.md`. Council deliberates; the human decides.

## Headline correction to the research
The S5 decision log was **optimistic about the codebase**. Grounded against source, the council found:
1. **Most "MVP ADAPT/surfacing" items already ship** (Architect): `CartFAB` with count+total
   (`packages/ui/.../client/CartFAB.tsx`); inline ModifierSheet with radio/checkbox + live delta
   (`MenuPage.tsx:404,419`); 86/stop-list owner toggle (`MenuManagerPage.tsx:229`); sold-out greying
   (`ProductCard.tsx:56`); courier accept/decline (`TasksPage.tsx:70,83`); owner accept/reject + prep
   display (`admin/OrderCard.tsx:196`). These are **not** do-now work.
2. **The one code-BORROW is invalid** (Breaker C2): `SYNTHESIS.md:8`'s stack ("Zustand + TanStack
   Query v5; shadcn/ui") is **aspirational** — `apps/web` has NO `@tanstack/react-query`, NO
   `react-hook-form`, NO `cva`, NO shadcn, NO zustand; it uses raw `apiClient`/`fetch`. S5 #17/#18/#19
   rest on a false premise → the "clean v3→v5 port" is a new-framework adoption, not a lift.
3. **The two schema "gaps" are dead-columns-now** (all three seats): `modifier_groups.display_type`
   is cosmetic — server already validates min/max/required without it (`orders.ts:494`), and the
   public `.strict()` contract is bypassed (the route serves raw plpgsql jsonb without `.parse()`,
   `menu.ts:43`). `order_status_history.{comment,notify}` bolts onto a table the happy path never
   writes (only the auto-cancel worker writes it, `order-timeout-sweep.ts:84`) and `notify` couples
   to the still-unbuilt Telegram subsystem.
4. **The "cheap timestamp-driven stepper" is not cheap** (Breaker C1): per-transition `*_at` are
   mostly NOT written (`ready_at` is never written anywhere; `updateOrderStatus` stamps only
   `confirmed_at`+`delivered_at`, `orderStatusService.ts:66`), and the customer endpoint returns only
   `status`+`created_at` (`customer/orders.ts:30`). A timestamp-filled stepper = a backend
   instrumentation project, not a UI borrow.

## DO NOW — the converged do-now set (3 items; schema/contract-ready, additive, UI-mostly)
Ranked by value-per-effort (Counsel) and verified actionable (Architect/Breaker). All must read
`var(--brand-*)` and survive `derivePalette` so they cohere with the just-shipped cinematic media.

1. **Owner new-order alert — make it iOS-correct + persistent + honest.** [#1, unanimous]
   It is BUILT but BROKEN: `DashboardPage.tsx:42` `useSound`, WS `order.created` arrives — but
   `useSound` (`lib/hooks.ts:30`) lazily `new Audio().play()` on the WS event (not a user gesture)
   and swallows the autoplay rejection → silent no-op on iOS Safari; and it's one-shot, not
   persistent. Bounded work: AudioContext unlock-on-first-gesture, loop/persistence, and an honest
   armed/blocked indicator (Counsel: a *silent false promise of an alert is worse than no alert*).
   Highest value in a 77%-cash market where a missed order = a lost sale + churn.
2. **Order-status stepper — correctness fix (status-driven, not timestamp-driven).**
   `OrderProgress` hard-codes 5 steps, **drops CONFIRMED, has no PICKED_UP/pickup branch**
   (`OrderProgress.tsx:7`; consumer `OrderStatusPage.tsx:436`) → it misrepresents pickup orders today.
   Fix it to honestly reflect the real 10-state machine (CONFIRMED in; `READY→PICKED_UP` pickup branch;
   terminal styling for REJECTED/CANCELLED), driven off `status` (already exposed). UI-only, cheap.
   The richer timestamp-filled version is DEFERRED (needs the backend instrumentation in Breaker C1).
3. **Surface venue `busy` state.** The contract already carries `status: open|closed|busy`
   (`public/menu.ts:53`) but the client collapses it to `isOpen===false` (`MenuPage.tsx:634`) → `busy`
   never reaches the eater. Surface a `busy` chip/banner. Near-free; high operator-trust (teardown §F3).

## DO NOT DO NOW — deferred (unanimous, with reasons)
- **R3 code BORROW (#18/#19)** — `apps/web` lacks every assumed framework; ADAPT the *idea* later
  (colocated `zod.safeParse` on our atoms), never lift the code. No NOTICE entry.
- **`display_type` + `status_history.{comment,notify}`** — dead columns / unbuilt-subsystem coupling.
  Add each only when its consumer (a quantity-modifier; the Telegram-notif worker) is built.
- **Timestamp-driven stepper, live courier map (#24), schedule/mealtime engine (#4), totals ledger
  (#10), PSP seam (#12)** — Counsel: the schedule engine is the trap (R4 oversells "most actionable";
  `SCHEDULED` is scaffold). Single COD path + fixed integer columns are the right invariants — protect them.
- **ModifierSheet rebuild (#21)** — already exists inline; refactoring it out of the 99th-pctile-churn
  `MenuPage.tsx` that *just* absorbed the media modal (Breaker H4) is a regression collision, not a borrow.

## Ethical gates the human should set BEFORE any code (Counsel — friction, not veto)
- 🔶 If `status_history.notify` is ever added: notifications must be a **downstream reader** of history,
  never a participant in the transition critical section (the 10-state machine + `assertTransition()`
  anti-race were just stabilised).
- 🔶 Any borrowed cart/checkout UI must compute **nothing** financial client-side (money is server-side
  integer-cents, just stabilised).
- 🔶 RLS work = read-and-verify-first, change-last; a cross-tenant-read-still-fails test gates any change.
- 🟡 The alert must degrade to a visible persistent banner when audio is blocked (no silent false promise).
- ⛔ CI deploy is red (pre-existing import-endpoint hang); per Ship Discipline prove on staging-direct,
  and don't declare done against a red pipeline.

## Open business tension (council cannot resolve — human's call)
**Couriers day-one, or pickup-first?** If pickup-first, the entire courier cluster (#23/#24/#7) defers and
the do-now set shrinks to the 3 above + lean on `READY→PICKED_UP`. This reshapes the MVP and is a
business decision, not a research one.

## One crisp recommendation
Do the **COD trust-loop cluster** in order: **(1) owner alert iOS-fix → (2) stepper correctness →
(3) busy-state surfacing** — all brand-token-native. It is the smallest set that makes a cash order
**complete reliably, visibly, and honestly** for one Durrës venue. Explicitly **do not** build the
schedule engine, the live courier map, the totals ledger, or lift the R3 code.
