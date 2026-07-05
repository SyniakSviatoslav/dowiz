# Voice Control — UI Spec (MicFab, state machine, confirmation surface)

> Status: PROPOSED (design-time; **NO production code** — a spec artifact). Date: 2026-06-30.
> Companion to `docs/design/voice-control/proposal.md` + `docs/adr/0015-voice-control.md`.
> Author register: System Architect (does it work / scale / hold) with emil-design-eng /
> motion-system polish applied. **This spec adds NO behaviour the hardened design forbids** — it
> only describes the pixels of: confirm-then-execute (§6 of the proposal), READ_ONLY-vs-STATEFUL,
> dietary-touch-only (C1/R2-B), money-has-no-voice-grammar, fail-closed runtime gating (R2-A/R2-E),
> and the actor-anonymous / zero-egress invariants (C-1/R2-C). Any conflict → the proposal/ADR wins.

**Grounding (verified against the live tree, 2026-06-30):**
- Storefront menu route `/s/:slug/*` — `apps/web/src/main.tsx:49`; page `apps/web/src/pages/client/MenuPage.tsx`.
- **Cart is NOT a bottom-right FAB** — the cart trigger sits in the **sticky top header**
  (`apps/web/src/routes/ClientLayout.tsx:144-187`) and opens a `ResponsiveDialog` at
  `z-modal-backdrop` (300). The bottom-right corner is therefore **free**.
- **Compare-bar** is **bottom-center**: `fixed left-1/2 -translate-x-1/2 z-sticky`, max-w-md,
  `bottom: calc(env(safe-area-inset-bottom,0px) + 5.5rem)` (`MenuPage.tsx:1086-1112`).
- **Detail product modal** = `z-modal` (400); **checkout** is a bottom-sheet over the menu (400).
- **Z-scale** (`packages/ui/src/theme/tokens.css:199-203`): dropdown 100 · sticky 200 ·
  modal-backdrop 300 · modal 400 · toast 500.
- **Tap tokens** (`tokens.css:11-13`): `--tap-min:44px` · `--tap-courier:48px` · `--tap-critical:56px`.
- **Motion tokens** (`tokens.css:20-25`, zeroed under reduced-motion at `:378-379`):
  `--motion-fast:150ms` · `--motion-base:240ms` · `--ease-out: cubic-bezier(0.16,1,0.3,1)`.
- **Paper-skin** (`tokens.css:445-479`) **re-maps `--brand-*` automatically** under
  `[data-skin="paper"]` → if the MicFab uses `--brand-*` tokens it inherits the paper palette with
  **zero MicFab-specific paper CSS**. `--color-on-primary` exists for text on `--brand-primary`.
- **i18n**: single-source flat-key catalog `packages/ui/src/lib/i18n-catalog.ts`; add keys **only**
  via `scripts/i18n-add.ts` (sq/en/uk together); parity gated by `scripts/i18n-parity`.

---

## 1. MicFab — the entry affordance

**Placement (collision-proof — stated exactly).** Bottom-right corner, the one fixed slot nothing
else occupies:

```
position: fixed;
right:  calc(env(safe-area-inset-right,  0px) + 1rem);
bottom: calc(env(safe-area-inset-bottom, 0px) + 1rem);
z-index: var(--z-sticky);   /* 200 — Tailwind: z-sticky */
```

- **Size:** `--tap-critical` (56px) round — it is the safety-critical entry to a confirm-gated
  action, so it gets the largest tap token, not `--tap-min`.
- **Why no collision (the geometry, on a 375px viewport):** the cart lives in the **top** header
  (different edge). The **compare-bar** is bottom-center at `bottom 5.5rem`; a 56px FAB at
  `bottom 1rem` reaches up to ~`4.5rem` → a **≥1rem vertical clearance** to the compare-bar's lower
  edge, and they never share a row. **Reserved-lane rule:** the compare-bar owns the
  `5.5rem` band, the MicFab owns the `1rem` band; if either grows, the MicFab yields upward, never
  the compare-bar (the compare flow is mid-task, the mic is ambient).
- **Hard hide (render `null`, NOT z-bury) whenever a higher surface is open:** detail product modal
  (`detailProduct`), cart dialog, compare **panel** (full), or checkout bottom-sheet. The MicFab is
  `z-sticky` (200) — below every dialog — but we **unmount** it while any is open so it cannot leak
  into a focus order or an aria tree behind a backdrop. It **coexists** only with the compare-*bar*
  (spatially separated, above).
- **z-index relationship (explicit):** page cards < **MicFab = compare-bar = `z-sticky` (200)** <
  disclosure BottomSheet (`z-modal-backdrop` 300) < cart/detail/checkout (`z-modal` 400) <
  **confirm chip + READ_ONLY toast (`z-toast` 500)**. The confirm chip sits *above* the FAB that
  spawned it; because the MicFab is unmounted while any modal is open, the `z-toast` chip never
  fights an open modal.

**Icon + idle affordance.** Tabler glyph `ti ti-microphone` (matches the existing `ti ti-*`
iconography, e.g. `MenuPage.tsx:1094`). **No idle animation** — a pulsing mic while idle reads as
"always listening" (the exact surveillance perception R-E warns of); idle is a calm, static button.
Affordance weight = a single round surface with `--elev-3` shadow and an `aria-label` (no persistent
tooltip). Motion happens **only** after an explicit tap (the listening state, §2).

**Token mapping (dark default + paper-skin, no arbitrary Tailwind):**

| Element | Token (dark storefront default) | Under `[data-skin="paper"]` |
|---|---|---|
| FAB surface | `background: var(--brand-primary)` | auto → `--action` (#246B61) |
| FAB glyph | `color: var(--color-on-primary)` | auto → `--action-ink` |
| Resting shadow | `box-shadow: var(--elev-3)` | inherits soft canonical `--elev-*` |
| Focus ring | `focus-visible:ring-2 ring-[var(--brand-primary)]` (dark) | paper uses the global `:focus-visible` outline `2px var(--action)` |
| Listening ring | `var(--brand-primary)` at 35% via `color-mix` | auto via `--brand-primary`→`--action` |

Because every value is a `--brand-*` / `--elev-*` / `--color-on-primary` token, the **paper skin
needs zero MicFab-specific CSS** — the re-map at `tokens.css:468-479` carries it.

**Render predicate (the whole gate — absent, never greyed-disabled).** The MicFab renders **iff
ALL** hold; if any fails it is **absent** (returns `null`), never a disabled grey button (a greyed
mic invites "why is this off?" support load and implies a capability we won't deliver):

1. **Build-flag true-dark:** `import.meta.env.VITE_VOICE_CONTROL_ENABLED === 'true'` (else the
   engine module is never even in the bundle — §9/L2 of the proposal).
2. **Runtime config says enabled AND not killed:** `GET /api/public/voice-config?slug=:slug`
   (`cache:'no-store'`, SW-exempt) returned `{ enabled: true }`. **Fail-closed:** fetch reject /
   `!res.ok` / non-JSON / `enabled !== true` ⇒ absent (R2-A).
3. **Capability floor passed:** WebGPU adapter present **AND** the bounded warmup probe completed in
   its time/OOM budget (§2.2/M2). No WebGPU (iOS Safari, most social in-app WebViews) ⇒ absent.
4. **User pref not "off":** the persistent storefront voice setting (§5) is not `off`.
5. **Secure context + getUserMedia supported.** Else absent.

Order: 1 short-circuits at build (true-dark), then 2 (one fail-closed round-trip), then 3 (probe,
async — the FAB may appear a beat late, which is fine; it is additive). A returning visitor gets the
runtime kill **instantly** because the `/api/` config path is never SW-cached (R2-A).

---

## 2. State machine (visual)

One finite machine. Every node has a visual; **no node dead-ends** (every terminal/error offers a
recovery affordance and leaves touch fully working). Live regions per §7.

```
        (render predicate §1 passes)
                  │
                IDLE ──tap──▶ [first-ever tap] DISCLOSURE SHEET (§5)
                  │                    │  "Not now" → back to IDLE (no-op default)
                  │                    │  "Use voice" → persist pref=on, continue ▼
                  ▼                    ▼
            PERMISSION-REQUEST  (native getUserMedia prompt)
                  │  denied → ERROR:mic-denied
                  ▼  granted
              LISTENING  (mic-active, reduced-motion-safe pulse + live partial transcript)
                  │  silence/cap → TRANSCRIBING
                  ▼
            TRANSCRIBING  (spinner + "…")
                  │
            INTENT-PROPOSAL  (matcher result)
              ├─ READ_ONLY (filter/sort/search/macro/compare/read-order/go-checkout)
              │      → AUTO-APPLY → APPLIED + non-modal toast w/ Undo (§3)
              ├─ STATEFUL (add-to-cart — the ONLY stateful intent)
              │      → CONFIRMATION CHIP (§3) ──confirm──▶ APPLIED
              │                                └─cancel/timeout──▶ IDLE (no write)
              ├─ low-confidence / no-match → ERROR:no-match (did-you-mean / re-prompt)
              ├─ ambiguous (ties) → DISAMBIGUATION chips (§3) → re-enter proposal
              └─ dietary-named category / excluded kind → silently DROPPED → ERROR:no-match copy
                     (never auto-applies a dietary read — R2-B; never a "we ignored a safety
                      request" message that itself implies safety handling)
            APPLIED → returns to IDLE
```

**Per-state visual (tokens, motion):**

| State | Visual | Motion |
|---|---|---|
| **idle** | static `ti-microphone` FAB, `--brand-primary` | none |
| **permission-request** | FAB dims to 60% while native prompt is up; no custom overlay (the browser owns this surface) | none |
| **listening** | FAB swaps glyph to a live state; an expanding ring (`color-mix(in srgb, var(--brand-primary) 35%, transparent)`) + a small **live partial-transcript pill** above the FAB (`aria-live="polite"`) | pulse `var(--motion-base)` `var(--ease-out)`, looped; **reduced-motion → no pulse**, a static filled ring + a "Listening…" text label instead (`tokens.css:378-379` zeroes the duration; the component also checks `prefersReduced` like `MenuPage.tsx:480`) |
| **transcribing** | FAB shows an indeterminate spinner; transcript pill shows the frozen final words + "…" | spinner respects reduced-motion (opacity fade, not spin, when reduced) |
| **intent-proposal (READ_ONLY)** | menu visibly updates (filter/sort applied) + a **toast w/ Undo** (§3) | toast slides in `--motion-fast`; reduced-motion → fade |
| **intent-proposal (STATEFUL)** | **confirmation chip** (§3) above the FAB; menu unchanged until confirm | chip appears `--motion-fast`/`--ease-out`; reduced-motion → fade |
| **applied** | brief check pulse on the FAB; toast/chip dismisses | `--motion-fast`; reduced-motion → instant |

**Error matrix (each = what the user sees + the recovery; nothing dead-ends):**

| Error | What the user sees | Recovery affordance |
|---|---|---|
| **Mic permission denied** | FAB stops; a one-line inline pill `voice.err.mic_denied` ("Voice needs mic access — use touch") | FAB stays present but **does not re-prompt in a loop**; tapping again re-opens the native prompt once; **touch/keyboard fully intact** |
| **Model fetch fails / offline** (first use, 30 s timeout) | pill `voice.err.model_offline` ("Voice unavailable offline") + a **Retry** affordance | Retry button re-attempts the R2 GET; **no spinner-forever**; page never blocked |
| **Low-confidence / no-match transcript** | pill `voice.err.no_match` ("Didn't catch that") + optional **did-you-mean** chips when near-matches exist | re-prompt by tapping the FAB; **never auto-executes a STATEFUL action on low confidence** |
| **Ambiguous match** (two items within edit distance) | **disambiguation chips** (§3) listing the candidates | tap a chip to pick; **never guess-executes** |
| **Offline / unavailable** (control-plane GET fails) | **MicFab simply absent** (fail-closed, §1) — there is nothing to see | use touch (unaffected) |
| **Unsupported device** (no WebGPU / probe fails / insecure context) | **MicFab absent** (not greyed) | use touch; no message (we don't advertise an absent feature) |
| **Kill-switch flips mid-session** (operator `VOICE_KILL`) | on the **next** config check (bootstrap / next activation) the FAB goes **absent**; an in-flight listening session is aborted cleanly with `voice.err.unavailable` | use touch; engine import + model fetch are gated on `{enabled:true}` so nothing new loads |
| **Worker crash / inference timeout (~8 s)** | pill `voice.err.try_again` ("Didn't catch that — try again or use touch"); worker terminated + respawned | tap to retry; **main thread unaffected** (isolation) |

---

## 3. The confirmation surface (safety-critical)

**This is the C-2 / STOP-2 guardrail in pixels.** It applies to the **STATEFUL add-to-cart** intent
only (the sole stateful intent in active scope, §6 of the proposal).

**What it shows.** A non-modal chip anchored above the MicFab, at `z-toast` (500), echoing the
*parsed* intent before any write:

```
┌────────────────────────────────────────────┐
│  Shto 2× Sufllaqe?   /   Add 2× Sufllaqe?    │   ← parsed {qty, item}, from the proposal
│  [  Cancel  ]            [  Confirm  ]        │   ← EQUAL-weight affordances
└────────────────────────────────────────────┘
```

- **EQUAL affordance weight (assert this — C-2 / STOP-2).** Confirm and Cancel use the **same
  button class**: identical size (`--tap-min` 44px min), identical `background: var(--brand-surface-raised)`,
  identical `border: 1px solid var(--brand-border)`, identical `color: var(--brand-text)`. **Neither
  is a bright `--brand-primary` CTA against a greyed ghost.** A check (`ti-check`) / x (`ti-x`) glyph
  may differentiate them, but **never a color-weight or size asymmetry**. A re-introduced lopsided
  hierarchy is a soft dark-pattern that a passing functional-decline test must NOT license — the spec
  carries a visual-equality assertion (same computed `background`, `border-width`, `min-height`,
  `font-weight` for both buttons) into the §10 CI lane (proposal §10 / Counsel C-2).
- **Confirm runs the handler exactly once; Cancel writes nothing.** The chip is the only place a
  STATEFUL proposal can reach `addItem` (`CartProvider.tsx:89`), and only on an explicit human tap —
  the proposal is `consumed-once` (proposal §6 idempotency).
- **Timeout = fail-safe Cancel.** If no decision within ~12 s the chip dismisses as **Cancel** (no
  write). A safety surface defaults to *not acting*.
- **Esc / outside-tap = Cancel** (explicit, not a silent confirm).

**Which intents get a confirm chip vs auto-apply (consistent with proposal §6):**

| Intent | Class | Surface |
|---|---|---|
| filter (non-dietary) / sort / search / macro-lens / compare-toggle | READ_ONLY | **auto-apply** + toast w/ Undo |
| "read my order" (reads own cart + total) | READ_ONLY | **auto-apply** → read-back panel (§4) |
| "go to checkout" (navigation) | READ_ONLY | **auto-apply** → route change |
| **add-to-cart** | **STATEFUL** | **confirmation chip** (above) |
| dietary/allergen-named category or filter | **excluded (no `kind`)** | **dropped, touch-only** — no chip, no auto-apply |
| place-order / pay / checkout field write / order-finalization | **excluded (no `kind`)** | **no voice UI at all** |

**Explicit, in the spec (so it cannot be re-opened by UI drift):**
- **Dietary / allergen intents have NO voice UI** — not even a confirm chip. A chip that echoes a
  mis-parsed allergen is *itself* a trusted safety assertion off a noisy channel (C1). Allergen chips
  + dietary-named categories stay **touch-only** (R2-B): the voice match is **dropped** before any
  surface renders.
- **Checkout writes / place-order / payment have NO voice UI at all** — no chip, no field focus, no
  "say your address". The money path has no voice grammar by construction (proposal §6).

**READ_ONLY toast w/ Undo (auto-apply surfaces).** Reuse the existing `ToastManager`
(`packages/ui/src/components/molecules/ToastManager.tsx`, `z-toast`, `aria-live`): non-modal, shows
e.g. "Sorted by price" + an **Undo** action that re-calls the prior setter value (filter/sort/search
are reversible — `MenuPage.tsx:211-240`). Undo is the reversibility affordance that lets auto-apply
be safe.

**Disambiguation chips.** A horizontal row of `--tap-min` chips (item names), `--brand-surface-raised`
/ `--brand-border`, anchored like the confirm chip. Picking one re-enters the proposal flow (a
non-dietary pick auto-applies; an add-to-cart pick shows the confirm chip).

---

## 4. Checkout READ_ONLY UI ("read my order")

Two READ_ONLY intents, no field entry:

**"Read my order" — an accessible read-back panel.**
- A panel (or inline region) rendering the user's **own client-side cart lines + total** — the same
  data the cart dialog already holds (`ClientLayout.tsx:187-257`), no new network call, no new PII
  egress, no cross-tenant surface.
- Wrapped in an **`aria-live="polite"`** region (with a `role="status"`) so a screen reader actually
  **announces** the lines + total when voice triggers it — the announcement is the point; the panel
  is the visual mirror. Format per line: "{qty}× {name} — {formatMoney}", then "Total — {formatMoney}"
  (reuse `formatMoney` + `activeCurrency`, as in `ClientLayout.tsx:240-249`).
- No write, no checkout field, reversible (it only reads). Dismissible by tap/Esc.

**"Go to checkout" — navigation.**
- A route change to the checkout bottom-sheet (the existing `setCheckoutOpen(true)` /
  checkout-over-menu primitive, `ClientLayout.tsx:271`+). UI-local, reversible (closing returns to
  the menu, cart preserved).
- **No field-entry UI** — address / phone / notes are **not voiceable** and get **no voice surface**.
  Voice can *navigate to* checkout; the human fills the fields by touch/keyboard.

---

## 5. Disclosure sheet + persistent setting

**First-mic-tap one-time disclosure (on-device).** A `BottomSheet`
(`packages/ui/src/components/molecules/BottomSheet.tsx`, `z-modal-backdrop` 300) shown on the **first
ever** MicFab tap, in sq/en/uk:

> "Processed on your device. Audio never leaves your phone. No recording is kept."
> `[ Not now / use touch ]`     `[ Use voice ]`

- **Equal affordance weight (C-2, same rule as §3):** "Use voice" and "Not now / use touch" share
  the **same button class** — same size (`--tap-min`), same `--brand-surface-raised` background, same
  `--brand-border`, same `--brand-text`. **"Not now" is NOT a small grey ghost** against a bright
  primary. The spec carries the same computed-style equality assertion into CI (proposal §8/C-2).
- **"Not now" is the no-op default** — it leaves the mic **unactivated** and touch fully functional;
  it does **not** activate the engine, request the mic, or fetch the model. (Guardrail: the decline
  test asserts the engine is never imported and touch still works — proposal §10.)
- **"Use voice"** persists the pref (below), then continues into the permission-request state (§2).
- No always-listening, no wake-word (FORBIDDEN).

**Persistent on/off setting.** Backed by the existing storefront prefs store (the same
`safeStorage` `dos_menu_prefs_${slug}` family used at `MenuPage.tsx:212-251`): add a `voice:
'on'|'off'` field (absent = "not yet decided" → disclosure shows on first tap).

- **Where it lives:** a small **"Voice" toggle in the storefront menu controls/preferences row** —
  the same surface that already hosts sort / macro-lens / search (`MenuPage`), so it sits with the
  other per-storefront UI prefs the user already manages. The toggle is shown **only when the §1
  render predicate passes** (flag on + not killed + WebGPU-capable) — otherwise there is nothing to
  toggle and the row omits it (consistent with "absent, not greyed").
- Flipping it `off` removes the MicFab immediately (predicate clause 4) and aborts any session;
  flipping it `on` re-shows it. Re-opening the disclosure copy is available from the toggle's info
  affordance (so the on-device privacy statement is always re-readable).

---

## 6. i18n — new keys (add via `scripts/i18n-add.ts`, sq/en/uk, parity gated)

All under the `voice.*` namespace. Add each with `pnpm exec tsx scripts/i18n-add.ts <key> "<en>"
"<sq>" "<uk>"` (never hand-edit derived `messages` — proposal §3.1). Parity gate
(`scripts/i18n-parity`) **fails** any half-translated key, and per proposal §4.2 a locale that has
not passed its own dangerous-misfire bound ships **dark even though the strings exist**.

| Key | en (reference) |
|---|---|
| `voice.fab_label` | "Order by voice" |
| `voice.listening` | "Listening…" |
| `voice.transcribing` | "Getting that…" |
| `voice.confirm_add` | "Add {{qty}}× {{item}}?" |
| `voice.confirm` | "Confirm" |
| `voice.cancel` | "Cancel" |
| `voice.applied` | "Done" |
| `voice.undo` | "Undo" |
| `voice.did_you_mean` | "Did you mean?" |
| `voice.read_order_title` | "Your order" |
| `voice.read_order_total` | "Total" |
| `voice.go_checkout` | "Go to checkout" |
| `voice.disclosure_body` | "Processed on your device. Audio never leaves your phone. No recording is kept." |
| `voice.disclosure_use` | "Use voice" |
| `voice.disclosure_decline` | "Not now / use touch" |
| `voice.setting_label` | "Voice" |
| `voice.err.mic_denied` | "Voice needs mic access — use touch instead" |
| `voice.err.model_offline` | "Voice unavailable offline" |
| `voice.err.no_match` | "Didn't catch that" |
| `voice.err.try_again` | "Didn't catch that — try again or use touch" |
| `voice.err.unavailable` | "Voice is unavailable right now" |
| `voice.retry` | "Retry" |

(sq/en/uk must all be supplied at add-time; the table lists the en reference only. The `sq` strings
are the hardest ASR-adjacent copy and the default locale — author them first.)

---

## 7. Accessibility + motion (voice is ADDITIVE, never the only path)

- **Voice never the only way to do anything.** Every voice action has a pre-existing touch/keyboard
  equivalent (add via the card `[menu-item-add]` button, sort/filter via their controls, checkout via
  the cart button). Voice adds an input device; it removes none.
- **Full keyboard path:** the MicFab is a real `<button>` (Tab-reachable, Enter/Space activates).
  The confirm chip, disambiguation chips, toast Undo, disclosure buttons, and the read-back panel are
  all keyboard-operable; **Esc cancels** the chip / closes the read-back (fail-safe = no write).
- **Live regions:**
  - partial transcript pill (listening) → `aria-live="polite"` (incremental, non-interruptive).
  - intent proposal / confirm chip → `aria-live="polite"` **and** focus is moved to the chip when it
    appears (so a keyboard/SR user lands on the decision). Equal-weight buttons → focus the chip
    **container** (labelled by `voice.confirm_add`), then Tab cycles Cancel ↔ Confirm; neither is a
    default-Enter "primary" (a STATEFUL write must be a deliberate choice, not an accidental Enter).
  - read-back panel → `role="status"` + `aria-live="polite"` (§4).
  - errors → the error pill is `aria-live="assertive"` (the user needs to know voice failed).
- **Focus management on dismiss:** closing the chip / panel returns focus to the MicFab (no focus
  loss to `<body>`).
- **prefers-reduced-motion:** the listening pulse and chip/toast slides are gated by
  `prefersReduced` (component check, mirroring `MenuPage.tsx:480`) **and** by the token zero-out
  (`tokens.css:378-379` sets `--motion-fast/base: 0ms`). Reduced-motion listening = a **static filled
  ring + "Listening…" label**, no animation. No motion is ever the *only* signal of a state change —
  each state also has a text/aria change.
- **Contrast + tap targets:** FAB = `--tap-critical` (56px); all chip/toast/disclosure controls
  ≥ `--tap-min` (44px). Colour pairs use `--brand-primary`/`--color-on-primary` (dark) and the
  paper-skin re-map's AA-cleared `--action`/`--action-ink` — no arbitrary hex, so AA is inherited
  from the token system, not re-derived per component.

---

## 8. Phase-0 transcribe overlay (dev-only — FORBIDDEN on public `/s/:slug`)

The minimal Phase-0 quality-probe UI: a **dev-only transcribe overlay** that surfaces the **raw
transcript** (plus confidence + the matched intent) so the §4.2 gate corpus can be eyeballed.

- **Visual (deliberately un-polished):** a fixed bottom strip, monospace, showing
  `transcript · conf=0.xx · intent={action,target}`. No token investment — it is an instrument, not a
  product surface.
- **Gated behind `import.meta.env.VITE_VOICE_TRANSCRIBE_DEBUG`** so it **tree-shakes out** of any
  build without the flag.
- **FORBIDDEN on any public deployed `/s/:slug` serving real customers (R2-C) — enforced by the
  three machine-checks already specified in proposal §8.1, NOT by this prose:**
  1. **Bundle-absence (red→green):** CI greps the built storefront chunks and **fails** if the
     overlay / any transcript-surfacing symbol survives when the flag is not `true`.
  2. **Deploy-arg assertion:** every public deploy target (prod + the public staging `/s/:slug`) must
     carry `VITE_VOICE_TRANSCRIBE_DEBUG=false`; the deploy **fails** if a public target sets it true.
  3. **Separate research build/host:** the debug build is a distinct target deployed **only** to a
     non-public / throwaway research host; the CI env-matrix check forbids it from the prod/staging-
     public lanes.
- **The transcript shown by the overlay is ephemeral component state** (the same in-memory string the
  confirm UI uses), **never logged, never sent to the server** (proposal §8). The overlay only
  *renders* what is already in the tab; it adds no egress — the guardrails ensure that render path
  cannot exist in a customer build.

---

## 9. Open UI items (carried, not invented here)

- **Idle hint vs no-hint:** this spec chooses **no idle animation** (anti-surveillance, R-E). A
  one-time, dismissible first-render coachmark for the FAB is a possible future polish — **deferred**,
  not added (YAGNI; the disclosure sheet already explains it on first tap).
- **Where exactly the persistent toggle renders** (§5) is specified as "the menu controls/preferences
  row"; the precise control placement is a Phase-1 build decision inside that row, not a new surface.
- **TTS audio confirm read-back** (the accessibility-earning path, R-L) is **out of scope** here —
  this spec is text-confirm only, consistent with the proposal's "convenience for capable devices"
  framing. Earning the accessibility framing is a separate, human-gated future change.
