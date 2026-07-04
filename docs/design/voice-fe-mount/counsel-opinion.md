# Counsel Opinion — Voice FE Mount (ADR-DRAFT-voice-fe-mount / proposed ADR-0021)

> Register: Counsel — good · beauty · wisdom. Advisory; the human is final. Date: 2026-07-03.
> Reviews `docs/design/voice-fe-mount/proposal.md` + `docs/design/voice-fe-mount/ADR-DRAFT-voice-fe-mount.md`.
> Verdict in one line: **a disciplined, honest, true-dark mount that crosses zero grounded red line —
> its ethical weight is not in what it does but in what mounting a not-yet-real feature does to a gate
> I myself insisted on.** No live ETHICAL-STOP. One conditional watch-line (B1a). STOP-1 (worker voice)
> untouched and still deferred. Friction stays proportional — mostly I am affirming, not blocking.

This mount inherits ADR-0015's ethics envelope, which I examined across three rounds and cleared for
Phase 0/1 (`docs/design/voice-control/counsel-opinion.md`, "PHASE 0/1 ETHICS: CLEAR"). I do **not**
re-litigate that here. I examine only what the *mount* newly puts on the board.

---

## 1. Reasoning by lens (only what is load-bearing)

### Ethics / honesty — the Charter fit, and the Potemkin-mic seam
Against the standing Ethics Charter this is clean: no military/warfare/surveillance-for-harm surface;
strictly additive (touch stays whole → coerces no one); zero PII egress in this PR (the MockEngine is
scripted text — no `getUserMedia`, no audio, no model fetch, no network). "AI as a collective tool that
serves everyone" is not offended by a read-only menu helper. So the Charter question the task poses
answers **yes** — a read-only, client-only, dark voice layer is consistent with it.

One honest seam the proposal under-names. This mount renders a **MicFab that presents as listening but
cannot hear** — `createMockVoiceEngine` synthesizes `onPermissionGranted → onTranscribing → onProposal`
from a scripted source (`useVoiceControl.ts:87,99,103`), and that source is dead outside `DEV`/test (R2).
A mic-shaped affordance backed by a mock is a small untruth. It is *bounded* — the flag is OFF in prod
and ON is a staging/E2E artifact — so it is not a live crossing of honest-UI. But it must never face a
real user: a person tapping a listening-looking mic that is a scripted no-op is being shown a capability
that does not exist. The launch gate (PR-4 real engine + the `/api/public/voice-config` hot-kill) already
forecloses this by construction; I only ask that "**ON is never a public storefront until PR-4**" be an
*asserted* line, not an assumed one — the same true-dark bundle assertion the proposal already plans
(§9) plus the existing R2-A fail-closed config guardrail together carry it. Name the Potemkin-mic
explicitly so no one flips the flag for a demo and calls it live.

### Aesthetics / conceptual integrity — this is the strong part
The mount is a model of "schema rich, runtime ruthlessly minimal" (a closed decision, handoff §6): the
seams already exist (DI deps → gate → port → hook → UI); the mount only *wires* them, adds **one** guarded
lazy line to a 99.x%ile hotspot, and keeps the whole subsystem quarantined in `VoiceMount` behind a
statically-pruned `false && import(...)`. The safety property is again the *shape of the graph* — engine
write-incapable, gate the sole sink, adapter the sole `@deliveryos/voice` importer — not a runtime check
that can rot. Choosing the most boring seam at every fork (existing `lazy()` pattern, existing
`?checkout=1` deep-link, existing `useToast`/`i18n-add`) is genuine elegance, not the seductive kind.
The one new load-bearing invariant ("pages reach voice only via the adapter") is being promoted from
prose to an eslint guardrail (R7/§8) — prose is not a gate, and the proposal knows it. No friction here.

### Strategy / long horizon — the mount is a commitment ratchet
The sequencing truth from my ADR-0015 opinion is unchanged and the proposal honours it: voice does not
touch the money/dispatch/RLS path where launch is actually NO-GO (B1/B2/B3), and it stays dark behind the
recorded demand gate (R4/R-J). Designing-and-mounting-while-cheap is defensible. But note the second-order
effect the proposal frames purely as engineering (drift-vs-cost): **a mounted, 95%-built feature is no
longer psychologically speculative.** It becomes "almost done, just needs the real engine," and that
gravity quietly re-frames voice from *demand-gated* to *in-flight*. The mount is technically reversible
(flag/revert, zero data) — but reversibility of *bytes* is not reversibility of *organizational momentum*.
This is the exact shape of the operator's own open accountability item (handoff §9: patience vs attachment
"indistinguishable from inside without a pre-committed trigger"). See the open question (§5) — this is
where it bites.

### User dignity / autonomy — preserved, and the consent surface goes live here
Confirm-then-execute keeps the human as the one who acts; voice is offered, never imposed; every touch
path stays whole. Newly relevant: the mount is the **first place the consent/disclosure surface becomes
live pixels**. The built UI honours it structurally — `declineDisclosure` (`useVoiceControl.ts:177-182`)
touches no engine/gate (its whole body is two dispatches, by design), voice is off-by-default
(`decideTapAction` → `show-disclosure` only when `voicePref` is undefined), and the sheet appears *after*
a gesture already made. That is the honest shape of consent I cleared in round 2 (STOP-2 dissolved). The
mount does not regress it — but it does *instantiate* it, so my carried-forward C-2 constraint (decline
visually equal to "Use voice") stops being a spec promise and becomes a rendered thing. See §3.

### Accessibility / justice — not re-introduced by this mount, still owed at launch
The distributional inversion I flagged for ADR-0015 (the WebGPU capability floor routes benefit toward
wealthier/younger devices and away from the cheap-phone elderly-poor for whom voice would matter most)
is a **PR-4 property, not a mount property** — the MockEngine has no WebGPU gate. So this mount excludes
no one new, and the accessibility framing remains honestly *dropped* (ADR-0015), not falsely re-claimed.
The mount owes only WCAG-AA on the *new interactive pixels it lights up* (disclosure sheet, confirm chip,
disambiguation chips, MicFab `aria-label`, reduced-motion) — the ui-spec already specifies these; a
non-blocking reminder, not a finding.

### Epistemic — mock-green is wiring-proof, not product-proof
The load-bearing unexamined assumption is the same one ADR-0015 already names ("green CI ≠ IRA passed"):
a Playwright pass against a deterministic MockProvider proves the *plumbing*, and nothing about whether a
real person speaking Albanian into a real mic is understood. That is fine **as long as no one mistakes
the one for the other.** The risk is that a repo full of green voice E2E manufactures a false "voice
works" confidence that softens the demand/feasibility gates. The cure is a label, not a veto (§3).

---

## 2. ETHICAL-STOP(s) — grounded red lines only

**Live crossings in this dark mount: ZERO.** I walked each grounded line against what the mount actually
*does* (dark by default; ON = staging/E2E; MockEngine scripted text, no mic, no egress; READ_ONLY +
confirm-gated add; closed-venue fixed fail-closed; server authoritative):
human-in-loop/zero-autopilot ✓ · server-authoritative ✓ · zero-PII-to-AI/claim-check ✓ (trivially — no
audio exists) · soft-confirm-not-a-trap ✓ (decline path structurally honest) · anonymize-not-delete
N/A (zero persisted state) · a11y additive ✓ · schema-rich/runtime-minimal ✓ (exemplary) · trigger =
first real paid order ✓ (stays dark, demand-gated). I am **not** manufacturing friction where none is
earned. This is consistent with my ADR-0015 verdict.

### WATCH-LINE (conditional — not a STOP unless descoped) — B1a is load-bearing, not polish
- **Grounded line:** "сервер-авторитетний" + honest-UI. The current voice `addToCart`
  (`handlers.ts:58-88`) checks `available`/`hasRequiredModifiers` but **not** `orderingDisabled`, while
  every tap path gates on it (`MenuPage.tsx:451,691,707`, `orderingDisabled = isClosed || isPreview`).
  I confirmed both in source.
- **Why it is a trust issue, not merely a bug** (the task asks precisely this): a closed/preview venue
  that silently *accepts* a voice add is a UI that lies about orderability. The server still refuses at
  checkout (ledger #65 `409 VENUE_CLOSED`) so **no money is extracted** — which is exactly why this is
  a *trust/dignity* fault, not a money-red-line breach: the harm is a broken promise and wasted human
  effort (a cart the customer cannot submit), landing on the customer and, at preview, on a not-yet-onboarded
  owner. Honest-UI means the client must not affirm what the server will deny.
- **Disposition:** the proposal **already fixes this** at the correct layer (B1a: `orderingDisabled` dep,
  checked first in `addToCart`, fail-closed to `onNoMatch`, gate rebuilt via `useMemo` for liveness). The
  fix is fail-closed and mirrors the tap path — the right posture. **Therefore no live STOP.** This is a
  *watch-line*: **B1a is not optional polish. If it is ever descoped, deferred past this PR, or the
  liveness rebuild dropped, this becomes a grounded ETHICAL-STOP on honest-UI.** Hold it as a
  must-pass exit criterion (the DoD unit test "add on closed venue → no `addItem`" is the proof) —
  and, per defense-in-depth, keep it *at the sink*, never behind a "hide the MicFab when closed" hack
  (correctly rejected as B1b — hiding the affordance is not the same as refusing the write).

### STOP-1 (worker/courier voice) — untouched, still deferred, out of scope here
This mount is client-storefront only; admin/courier voice is removed from the active build. STOP-1 (the
labour-surveillance gradient) neither hardens nor erodes here — it remains a recorded human gate at the
future Phase-3/4 boundary. Noted only so the record shows I checked it against this change and it is
**not** implicated.

---

## 3. Non-blocking advice (aesthetic / strategic — take or leave)

- **Add a decline-path arm to the mount E2E.** The consent sheet is now live pixels (§1 dignity). The
  DoD proof (§DoD) tests tap → transcript → confirm → cart line, and the closed-venue rejection — but
  **not** the disclosure decline. Since the mount *instantiates* the STOP-2-dissolving affordance, assert
  it here: tapping "Not now" leaves the mic unactivated and touch fully working, and the two choices are
  **visually equivalent in weight** (my carried-forward C-2: not a bright "Use voice" against a greyed,
  small "Not now"). A passing *functional* decline test must not license a *lopsided* hierarchy. Cheap,
  high-leverage, and it is where a dark-pattern would be born if it ever were.
- **Label the mock-green honestly.** Carry ADR-0015's "green CI ≠ product works" onto this mount's E2E in
  one line: *this proves wiring + gate, not that voice understands anyone.* One sentence forecloses the
  epistemic trap (§1) at zero cost.
- **Name the Potemkin-mic constraint as an asserted line, not an assumption.** "`VITE_VOICE_ENABLED=true`
  is a staging/E2E artifact; a real user never meets the mock mic — PR-4 real engine is the precondition
  for any public ON." The true-dark bundle assertion + R2-A fail-closed config already carry the mechanics;
  this just makes the *intent* explicit so no one flips the flag for a demo and calls it live.
- **Toast-Undo defer is fine — but keep it honestly scoped.** v1 plain toast for READ_ONLY (user-reversible)
  is adequate and I would not gold-plate it. Just do not let "Undo deferred" drift silently onto any future
  STATEFUL path; add-to-cart is already confirm-gated (the human tap *is* the undo point), so the defer is
  correct precisely because it never touches a stateful apply.

---

## 4. Steel-man of a rejected option — A2 ("don't mount until PR-4")

The proposal rejects A2 briskly ("leaves the FE unexercised → ledger-#68 drift"). The steel-man is
stronger than the one-line rejection admits, and it is worth stating in full because it names a real cost
the chosen path carries:

**A2's core virtue is refusing to build a mic-shaped lie.** Mounting a listening affordance whose only
engine is a deterministic mock creates a *live but hollow* surface — and hollow surfaces are exactly where
the pathologies this project guards against breed: fake-green (E2E passes against a scripted mock and
"voice works" enters the team's mental model), convergence-theater (a mounted FAB *looks* like progress on
a feature no one has yet asked for), and the commitment ratchet in §1 (the mount manufactures momentum
toward a launch the demand gate has not authorized). A2 says: **the cleanest design is the feature not
mounted until there is a real thing behind it** — which is the very "schema rich, runtime minimal /
cleanest design is the feature not built" discipline the project holds as doctrine. A2 also *dissolves*
two seams A1 accepts: it never renders a consent sheet that asks about a capture that does not happen, and
it never risks a flag-flip Potemkin mic. And strategically, A2 keeps mount-wiring attention off the board
until demand + B1/B2/B3 are answered — no engineering hours spent presenting done-ness on a detour.

**Where A1 nonetheless earns its place — but only if it adopts A2's discipline.** The drift risk is real:
~1,800 lines of unexercised FE (ledger #68) rot, and re-verifying the mount later under launch pressure is
worse than locking the seams now with a deterministic harness while PR-4 becomes a pure port swap. That is
a legitimate answer. But A1 only stays honest if it *absorbs A2's worry rather than dismissing it*: the
mount is defensible **provided** (a) the mock-green is explicitly labeled wiring-proof-not-product-proof,
(b) the Potemkin mic is asserted-never-public, and (c) the demand gate's freeness is protected against the
mount's own momentum (§5). A2 is not wrong that mounting has a cost; A1 is right that the cost is payable —
*if* it is paid, not waved away. The proposal currently waves; the three asks above make it pay.

---

## 5. The open question nobody asked

**Does dark-mounting voice change the honesty of the demand gate — or, put plainly: after we have built
and mounted 95% of this, will the human "do we launch voice?" decision still be free, or will "we already
built it, just flip it on" become the gravity that turns R-J into a rubber stamp?**

Every question asked of this proposal is about *safety* and *cost* — is it dark enough, testable enough,
cheap enough, reversible enough. No one has asked what the mount does to the *gate it is deferred behind*.
I insisted on that demand gate (ADR-0015 §5); the mount is now the single largest sunk cost pointed at
overriding it. This is not a red line and not a reason to stop — it is the operator's own unresolved
accountability item made concrete (handoff §9: patience vs attachment, indistinguishable from inside
without a **pre-committed trigger**). So the question to put to the human is not "is the mount safe" — it
manifestly is. It is: **before we mount, will we write down — now, while unattached — the condition under
which we flip this ON (a real demand signal, ranked against the first paid order), and the condition under
which we delete it unmounted?** A gate defended only by good intentions after the thing is built is not a
gate; it is a formality wearing one. Answer the pre-commitment before the mount, not after — so that when
PR-4 lands, the decision to launch is made by a human who is still free to say no.

---

**MOUNT ETHICS: CLEAR (dark).** No live ETHICAL-STOP. B1a is a load-bearing watch-line (STOP only if
descoped). The residual weight is not inside the mount — it is the mount's momentum against a gate — and
that is a human decision, exactly where a human, not Counsel, is meant to be final.
