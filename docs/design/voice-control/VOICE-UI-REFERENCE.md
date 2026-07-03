# Voice Control — UI + Animation Reference (MicFab visual language)

> Status: DESIGN RESEARCH (no product code). Date: 2026-07-03. Author register: design research
> for the storefront **MicFab** specified in `docs/design/voice-control/ui-spec.md` +
> `PHASE1-IMPLEMENTATION-PLAN.md` §3. Every screen below is a **real capture** taken with the
> agentic browser and saved to `./refs/`. No names or engagement numbers are invented — where a
> number appears it is attributed to its source (Dribbble now hides like/view counts behind login,
> so counts are cited only where a public search listing exposed them; authors were read from each
> shot page's `<meta>`).

**The operator's ask:** the voice UI + animation should "look **cool and clear**," with *"whisper
design"* named as the reference. **What "whisper design" most likely means** (investigated below): the
**OpenAI Whisper / ChatGPT Advanced Voice** aesthetic — the soft, luminous **blue orb / halo** that
morphs while the model listens and speaks. There is **no distinct design system literally named
"whisper"**; the strong, capturable match is the ChatGPT Advanced Voice orb (ref 02) and its ring/halo
lineage (refs 03–04). This document captures that lineage plus best-in-class voice-mic UIs, then
converts it into a **buildable, token-based, reduced-motion-safe spec** that honours the hard
constraints in ADR-0015 / ui-spec.md (**no idle pulse**, push-to-talk, dark + paper-skin auto,
WebGPU-gated, CSS-lean).

---

## Hard constraints this spec must satisfy (from ADR-0015 + ui-spec.md, verified 2026-07-03)

| Constraint | Source | Consequence for motion |
|---|---|---|
| **No idle animation** — a pulsing idle mic reads as "always listening" (surveillance perception R-E) | ui-spec §1, plan §3.1 | Motion begins **only after tap**. Idle = static button. |
| **Push-to-talk**, single utterance, no wake-word | plan §3.1 | One listening burst per tap; a new tap pre-empts (barge-in, plan §3.3). |
| **Mic stays a `<button>` with `ti-microphone`** (Tabler glyph) | ui-spec §1 | The visual must **keep a recognizable mic** for clarity — not replace it with an abstract blob. |
| **Token-only** colour: `--brand-primary` / `--color-on-primary` / `--elev-*`; paper-skin re-maps `--brand-*`→`--action` automatically | ui-spec §1 (tokens.css:445-479) | Zero MicFab-specific paper CSS; the halo colour is `color-mix(in srgb, var(--brand-primary) …)`. |
| **Motion tokens** `--motion-instant 80 / fast 150 / base 240 / slow 400`; easings `--ease-out (0.16,1,0.3,1)`, `--ease-in-out`, `--ease-soft`; **all `--motion-*`→0ms under `prefers-reduced-motion`** | tokens.css:19-27, 246-252; `packages/ui/src/lib/motion.ts` | Transitions use these tokens. **A looping keyframe duration is NOT a `--motion-*` token** → the loop must be gated behind `@media (prefers-reduced-motion: reduce)` **and** a `prefersReduced` JS check, not left to token zero-out alone. |
| **Reduced-motion**: motion is never the *only* signal — each state also has a text/aria change | ui-spec §7 | Every animated state has a static fallback + label + `aria-live`. |
| **56px FAB** (`--tap-critical`), bottom-right, `z-sticky`; unmounted while any modal open | ui-spec §1 | Halo must bloom **outside** the 56px disc without triggering layout/overflow — use a pseudo-element + `transform`, never width/height. |
| **WebGPU-gated** — MicFab absent (renders `null`) when unavailable, never greyed | ui-spec §1, plan §3.7 | No "disabled" visual state to design. |
| Repo **avoids heavy deps** — prefer CSS / Web Animations | task brief | Spec is **CSS keyframes + one rAF loop** for amplitude; framer-motion (already in `packages/ui`) is optional for chip/toast entrances only. **No WebGL/canvas** for the FAB. |

---

## 1. Reference gallery (real captures — `./refs/`)

> Method: captured via `agent-browser` in an isolated session. OpenAI's own domains
> (`openai.com`, `help.openai.com`, the retired `openai.fm`) are all behind a Cloudflare/Turnstile
> interactive challenge that does not auto-clear headless, so the canonical ChatGPT orb (ref 02) was
> captured from press coverage that embeds OpenAI's own product image. Authors verified from each
> page's `<meta name="description">`.

### 01 — Dribbble "voice assistant" field overview
`refs/01-dribbble-voice-search-grid.png` · <https://dribbble.com/search/voice-assistant>
The whole design field at a glance: the dominant shape languages are **glowing orbs/spheres** and
**concentric rings**, almost always on **dark** canvases with a single saturated accent (violet, blue,
cyan). Bars/waveforms are secondary. Confirms the "orb-or-ring on dark, one accent colour" convention.

### 02 — ChatGPT Advanced Voice "blue orb" (the "whisper" reference)
`refs/02-chatgpt-advanced-voice-blue-orb.png` · image is OpenAI's product shot, captured from
<https://www.aiforwardmarketer.com/the-blue-orb-that-just-might-change-everything/>
**What it is:** OpenAI's Advanced Voice mode — a soft **sky-blue-and-white cloud-like sphere** centred
on a white screen. **Why it's good / why it's the reference:** it is *the* mental model the operator
is pointing at — calm, premium, unmistakably "AI is listening" without a microphone icon or a hard
edge. The article documents the **state semantics** verbatim: *"A glowing blue orb means Advanced
Voice is active … A blue orb = Advanced Voice; a black circle = older standard voice mode,"* and
*"Instant responsiveness: it starts listening the second it finishes talking."*
- **Shape:** amorphous orb/blob (gaseous, soft-edged). **Motion:** slow internal churn + a bloom on
  voice; amplitude-reactive. **Colour:** monochrome blue→white gradient, luminous. **A11y:** the orb
  is the *only* state signal in ChatGPT — a gap our spec closes with labels + aria-live.
- **Caveat for us:** the pure blob **removes the mic glyph** → great mood, weaker "this is a button
  you press." We borrow the *glow/bloom*, not the glyph-removal.

### 03 — ChatGPT/Siri-style glowing **ring / halo**
`refs/03-chatgpt-style-ring-halo.png` · <https://dribbble.com/shots/23655866-AI-Voice-Assistant-for-ChatGPT>
(author "Krsn"). **Why it's good:** a **luminous blue-violet ring with a pale-cyan core glow** — the
same family as the ChatGPT orb but expressed as a **ring**, which is the single most important
reference for us: **a ring can wrap a persistent mic glyph.** Shape: ring/annulus. Motion: the ring
expands/breathes and reacts to audio. Colour: two-stop gradient ring (violet→blue) over a soft cyan
fill. This is the visual we adapt.

### 04 — Apple **Siri** "Ask Siri" glow-pill + mic
`refs/04-apple-siri-glow-pill.png` · <https://www.apple.com/apple-intelligence/> (redirected from
`/siri/`). **Why it's good:** Apple's current Siri is a **coloured light that blooms from the screen
edge / input bar**, and the "Ask Siri" field shows a **mic glyph inside a softly glowing pill** — a
mainstream, shipped example of *keeping an explicit mic affordance while adding an ambient glow*.
Shape: pill + edge-glow. Motion: a soft chromatic bloom on activation. Colour: multi-hue glow (we keep
it single-accent for brand coherence). Validates "mic glyph + glow" as production-grade, not just
Dribbble.

### 05 — Yasir Ekinci — **"Push-to-talk AI Voice Assistant"** (most on-point)
`refs/05-yasir-push-to-talk-mic-glow.png` ·
<https://dribbble.com/shots/23342957-Push-to-talk-AI-Voice-Assistant-Visual-UI-Styling>
Author: **Yasir Ekinci**. **Per a public Dribbble search listing (search-sourced, 2026-07): ~98 likes,
~28.4k views.** **Why it's the closest match:** it is literally a **push-to-talk** concept — a
**microphone glyph sitting in a warm radial glow** (peach/gold light blooming out of the mic) over a
soft blurred aura. Shape: **mic glyph + radial bloom** (exactly our recommendation). Motion: the glow
pulses/breathes from the mic while held. Colour: warm cream/gold core → violet surround. Demonstrates
that "mic + halo bloom" reads as premium and clearly as *press-and-hold to talk*.

### 06 — Milkinside — iridescent **glass orb** (idle sphere)
`refs/06-milkinside-glass-orb.png` ·
<https://dribbble.com/shots/20422981-AI-Voice-assistant-motion-for-Milkinside>
Author: **Yuriy Izmaylov** (Milkinside — Gleb Kuznetsov's studio, the reference studio for AI-orb
motion). **Why it's good:** the gold standard for a **liquid-glass sphere with an internal flowing
mobius/ribbon** — the "expensive orb." Shape: 3D glass sphere. Motion: slow internal fluid flow.
Colour: pearlescent blue/lilac/mint on light. **Caveat:** achieving this needs shader/WebGL or a heavy
Lottie → **out of budget for a 56px CSS FAB.** Cited as the aspirational ceiling and to justify *why
we deliberately do not chase a full orb.*

### 07 — "Vox" — dark-UI purple **glass orb** (dark-theme match)
`refs/07-vox-dark-ui-orb.png` ·
<https://dribbble.com/shots/27151481-Vox-AI-Voice-Assistant-App-Dark-UI-Interaction>
Author: **Koushik Sarkar**. **Why it's good:** a **glossy iridescent purple orb on a near-black dark
UI** — the closest to our **dark storefront default**. Shows how a single-accent orb + soft glow reads
on dark with dark surrounding panels (our exact canvas). Shape: orb. Motion: gloss/rotation. Colour:
violet on near-black. Confirms the accent-on-dark treatment; the surrounding chip/panel styling maps
to our `--brand-surface-raised`.

### 08 — "Voice AI Sphere Idle Animation" — warm sphere on black
`refs/08-voiceai-warm-sphere-idle.png` ·
<https://dribbble.com/shots/25985520-Voice-AI-Sphere-Idle-Animation>
Author: **Doruk Kavcioglu**. **Why it's good:** a **warm orange/cream gaseous sphere on pure black** —
the *warm* palette that our **paper-skin** (`--action` #246B61 teal + warm cream) leans toward, proving
the halo language survives a warm-accent re-map, not just cool blues. Shape: soft sphere. Motion: slow
gaseous drift (an "idle" loop — which we deliberately **omit** per no-idle-pulse, but the *listening*
bloom borrows its softness).

**Field synthesis (what the 8 references agree on):** dark canvas · one saturated accent · **soft
luminous bloom** as the "active/listening" signal · amplitude-reactivity is the shared wow-factor ·
the *premium* end is a full orb (heavy) and the *clear + shippable* end is a **mic glyph wrapped in a
glowing ring/halo** (refs 03, 04, 05).

---

## 2. Recommended visual language for the dowiz MicFab

### Decision: **a persistent mic glyph wrapped in a bloom of concentric "halo" rings** (ring, not orb, not bars)

> One line: **"The mic stays a mic; a halo of light blooms out of it the moment you tap, and the halo
> breathes with your voice."**

**Ring vs orb vs waveform-bars — decided against the constraints:**

| Option | For | Against | Verdict |
|---|---|---|---|
| **Ring / halo** (refs 03, 04, 05) | Keeps the `ti-microphone` glyph → maximal **clarity** ("this is a mic button"); bloom = the **cool**; pure CSS (`transform`/`opacity` on a pseudo-element, no layout); trivially **amplitude-reactive**; one accent colour → works dark + paper via `color-mix(--brand-primary)`; static-ring reduced-motion fallback is natural | Slightly less "wow" than a full 3D orb | **CHOSEN** — the only option that is cool **and** clear **and** lean **and** reduced-motion-honest simultaneously |
| **Orb / blob** (refs 02, 06, 07, 08) | Highest "premium/AI" mood; the literal "whisper" look | **Removes the mic glyph** → weaker button affordance; a convincing orb needs WebGL/Lottie (**breaks the no-heavy-deps rule**); a glowing orb *at rest* implies "alive/listening" — collides with **no-idle** | Rejected as the FAB primary; its **glow** is borrowed into the halo |
| **Waveform bars** | Unambiguous "audio/listening"; naturally amplitude-reactive | Reads as "recording/audio-editor," crowds/hides the mic glyph in 56px; less calm | Rejected as primary; used as a **secondary** accent inside the transcript pill (below) |

**Anatomy (buildable, 56px FAB):**
```
.mic-fab (button, 56px round)         ← --tap-critical, background var(--brand-primary), --elev-3
  ├─ .mic-fab__glyph  (ti-microphone) ← color var(--color-on-primary); swaps glyph per state
  ├─ .mic-fab__halo   (::before)      ← the amplitude ring (single reactive ring; scale+opacity via --amp)
  └─ .mic-fab__bloom  (::after)       ← the looping "breath" ring(s) that expand+fade (listening only)
```
- Colour: halo/bloom = `color-mix(in srgb, var(--brand-primary) 35%, transparent)` (ui-spec §1). Under
  `[data-skin="paper"]` `--brand-primary` auto-remaps to `--action` → **zero paper-specific CSS.**
- The halo/bloom live on **pseudo-elements sized larger than the 56px disc** and are
  `pointer-events:none; overflow:visible` — they never affect layout, the compare-bar clearance
  (ui-spec §1), or the tap target.
- Everything animates with **`transform` + `opacity` only** (compositor-cheap; safe at 60fps on mid
  phones — the WebGPU floor already guarantees a capable device).

---

## 3. Per-state animation spec

States are the finite machine of ui-spec §2 / plan §3.2:
`IDLE → DISCLOSURE → PERMISSION → LISTENING → TRANSCRIBING → PROPOSAL → {APPLIED | CONFIRMING | ERROR} → IDLE`.
Durations/easings are the repo tokens (`tokens.css` / `motion.ts`). **Amp-reactive?** = does it respond
to live mic amplitude. **Reduced-motion** = the mandatory static fallback (motion is never the only cue).

| State | What animates | Duration / easing (tokens) | Amp-reactive? | Reduced-motion fallback |
|---|---|---|---|---|
| **idle** | **Nothing.** Static `ti-microphone`, `--brand-primary`, `--elev-3`. (Press feedback only: `:active` `scale(0.97)` = `scalePress`, `--motion-instant` / `--ease-out`.) | 80ms on press only | No | Identical (already static) — no change needed. |
| **permission-request** | FAB `opacity 1→0.6` while the native prompt is up (browser owns its surface). | `--motion-fast` 150 / `--ease-soft` | No | opacity transition = 0ms → instant dim; still legible. |
| **listening** | **`.mic-fab__bloom`**: 1–2 concentric rings `scale(1)→scale(1.7)` + `opacity .35→0`, looping ~1800ms staggered (the calm "breath"). **`.mic-fab__halo`**: a solid inner ring whose `scale` + `opacity` are driven by `--amp` (the live "reacts to your voice"). Glyph → filled/active mic. A **partial-transcript pill** slides up above the FAB (`aria-live="polite"`). | bloom loop 1800ms `--ease-out` (custom loop dur, **gated by `@media reduce`** — see note); halo follows `--amp` via rAF (~16ms); pill = `--motion-fast`/`--ease-out` | **Yes** — halo `scale = 1 + var(--amp)*0.22`, `opacity = .25 + var(--amp)*.5` | **No bloom, no rАF.** Static filled ring at `--brand-primary` 35% + visible **"Listening…"** label + the pill. (Loop keyframe is disabled by the media query; `--amp` frozen.) |
| **transcribing** | Bloom **stops**; glyph → indeterminate **spinner** (rotating `conic-gradient` ring, 900ms linear loop). Pill freezes final words + "…". | spinner 900ms linear (gated by `@media reduce`) | No | Spinner replaced by an **opacity breathe** on the "…" **or** just static **"Getting that…"** text (no spin). |
| **proposal · READ_ONLY** (auto-apply) | Menu visibly updates; reuse `ToastManager` — toast slides in from right (`toastIn`, spring/`--motion-fast`) with an **Undo**. | `--motion-fast` 150 / `--ease-out` (or `toastIn` spring) | No | opacity-only appear → instant; `aria-live` polite announces "Sorted by price · Undo". |
| **proposal · STATEFUL → CONFIRMING** (the safety chip) | Confirm chip appears above the FAB: `opacity 0→1`, `scale .95→1`, `translateY 8→0` (`scaleIn`). Focus moves to the chip container. **Both buttons animate identically** (equal-weight — no button "pops"; a motion asymmetry would be a soft dark-pattern, ui-spec §3/C-2). | `--motion-base` 240 / `--ease-out` | No | opacity-only / instant; focus still moves to chip; equal-weight preserved. |
| **applied** | Glyph → `ti-check`; **one** confirm bloom ring (single `scale`+`fade`, not looped); chip/toast dismiss (`scaleOut`); focus returns to FAB; then → idle. | `--motion-base` 240 / `--ease-out` | No | Glyph swap to check + **"Done"** text/aria, no scale, instant dismiss. |
| **error** | Inline pill fades/slides up (`slideUp`, `--motion-fast`), `aria-live="assertive"`; glyph returns to idle mic. **No shake / no red flash** (a shake reads as blame + is not reduced-motion-friendly; the spec's error pills are neutral). Recovery affordance (Retry / did-you-mean chips) per ui-spec §2 matrix. | `--motion-fast` 150 / `--ease-out` | No | Pill appears instantly; assertive aria fires; recovery affordance present. |

**Amplitude-reactivity mechanism (the one rAF loop — cheap, no WebGL):**
```
1 AnalyserNode on the mic MediaStream (the engine already opens it).
loop (requestAnimationFrame, listening only, prefersReduced === false):
  getFloatTimeDomainData → RMS → smooth (e.g. amp = amp*0.8 + rms*0.2) → clamp 0..1
  micFabEl.style.setProperty('--amp', amp.toFixed(3))
CSS then maps --amp onto the halo ring's transform/opacity (above). Stop the loop + clear --amp on
exit / reduced-motion.
```
This is the shared "reacts to your voice" wow-factor from refs 02/03/05, delivered by **one CSS
custom property** — no canvas, no dep.

**Reduced-motion note (important, non-obvious):** the `--motion-*` tokens zero out under
`prefers-reduced-motion` (tokens.css:246), which handles all the *transition*-based states for free.
But the **looping** listening bloom + the transcribing spinner use **keyframe animations whose
durations are not `--motion-*` tokens** — so they must be **explicitly disabled** in a
`@media (prefers-reduced-motion: reduce)` block **and** gated by the component's `prefersReduced` check
(mirroring `MenuPage.tsx:480`). Do not rely on token zero-out for the loops.

**Buildable CSS sketch (illustrative — not product code):**
```css
.mic-fab { position: relative; width: var(--tap-critical); height: var(--tap-critical);
  border-radius: 9999px; background: var(--brand-primary); color: var(--color-on-primary);
  box-shadow: var(--elev-3); --amp: 0; }
.mic-fab:active { transform: scale(.97); transition: transform var(--motion-instant) var(--ease-out); }

/* reactive halo — always present in listening, driven by --amp */
.mic-fab[data-state="listening"]::before {
  content:""; position:absolute; inset:-8px; border-radius:inherit; pointer-events:none;
  border: 2px solid color-mix(in srgb, var(--brand-primary) 35%, transparent);
  transform: scale(calc(1 + var(--amp) * .22));
  opacity: calc(.25 + var(--amp) * .5);
}
/* looping "breath" bloom — listening only, disabled under reduced motion */
.mic-fab[data-state="listening"]::after {
  content:""; position:absolute; inset:0; border-radius:inherit; pointer-events:none;
  box-shadow: 0 0 0 0 color-mix(in srgb, var(--brand-primary) 35%, transparent);
  animation: mic-bloom 1800ms var(--ease-out) infinite;
}
@keyframes mic-bloom {
  0%   { transform: scale(1);   opacity:.35; }
  100% { transform: scale(1.7); opacity:0;   }
}
@media (prefers-reduced-motion: reduce) {
  .mic-fab[data-state="listening"]::after { animation: none; }           /* no loop */
  .mic-fab[data-state="listening"]::before{ transform:none; opacity:.35;} /* static ring */
  .mic-fab[data-state="transcribing"] .spinner { animation: none; }
}
```

---

## 4. "Clarity" checklist — each state is unambiguous

Voice UI's biggest failure mode is *"is it listening? did it hear me? did it do something?"* Each state
must answer that with **≥2 independent cues** (glyph + text/aria + motion/static-visual). Motion is
never the sole cue (ui-spec §7).

| State | Glyph cue | Text / aria cue | Visual (motion or static) cue | Passes "unambiguous"? |
|---|---|---|---|---|
| **idle** | `ti-microphone` (outline) | `aria-label` `voice.fab_label` "Order by voice" | static disc, `--elev-3` | ✓ clearly a press-to-talk button, at rest |
| **listening** | filled/active mic | `voice.listening` "Listening…" + partial-transcript pill `aria-live="polite"` | breathing bloom + amp-reactive ring (or static ring under reduced) | ✓ obviously live + reacting to me |
| **transcribing** | spinner | `voice.transcribing` "Getting that…" + frozen words + "…" | spin (or breathe) | ✓ "it heard me, it's thinking" |
| **proposal · READ_ONLY** | mic returns | toast text ("Sorted by price") + `aria-live` polite + **Undo** | menu visibly changed | ✓ "it did X; I can undo" |
| **proposal · CONFIRMING** | chip: `ti-check` / `ti-x` on equal buttons | `voice.confirm_add` "Add 2× Sufllaqe?" `aria-live` polite; focus lands on chip | chip present above FAB; menu **unchanged** | ✓ "nothing happened yet — I must choose" |
| **applied** | `ti-check` pulse | `voice.applied` "Done" (aria) | one confirm bloom; chip dismiss | ✓ "the action completed" |
| **error** | idle mic / `ti-microphone-off` for mic-denied | error pill `aria-live="assertive"` (`voice.err.*`) + recovery (Retry / did-you-mean) | neutral pill (no red flash, no shake) | ✓ "voice failed; here's how to recover / touch still works" |

**Cross-cutting clarity rules:**
- **Never colour-code state with red/green** (colour-blind + it implies money/danger semantics we don't
  own) — state is carried by **glyph + label**, colour stays the single `--brand-primary` family.
- **The confirm chip is the only asymmetry-free surface** — equal weight is a *clarity* rule as much as
  a safety rule: a bright "Confirm" vs grey "Cancel" would falsely signal "the safe default is to
  proceed." Both buttons identical (ui-spec §3, computed-style equality asserted in CI / G11).
- **Every terminal state offers a recovery affordance and leaves touch fully working** — no dead-ends
  (ui-spec §2).
- **Text labels are i18n keys** already reserved in ui-spec §6 (`voice.listening`, `voice.transcribing`,
  `voice.applied`, `voice.err.*`, …) in sq/en/uk — the clarity copy exists, don't hardcode.

---

## 5. Build notes

- **Tech:** CSS keyframes + `transform`/`opacity` + one `requestAnimationFrame` amplitude loop that
  writes a single `--amp` custom property. **No WebGL, no canvas, no Lottie, no new dep.**
  framer-motion (already in `packages/ui`, see `src/lib/motion.ts`) may be used for the **chip/toast
  entrances** (`scaleIn` / `toastIn`) if convenient, but the FAB halo should stay plain CSS so it is
  trivially reduced-motion-gated and paper-skin-inherited.
- **Tokens only:** durations `--motion-instant/fast/base/slow`, easings `--ease-out/in-out/soft`,
  colour `--brand-primary` / `--color-on-primary` / `--elev-*`, size `--tap-critical`/`--tap-min`.
  The `motion.ts` barrel already exports matching `duration` / `ease` values for any framer usage —
  import from there, never inline a cubic-bezier.
- **Paper-skin:** because every colour is a `--brand-*` token, `[data-skin="paper"]` re-maps it to the
  warm `--action` palette automatically (tokens.css:468-479) — **no MicFab-specific paper CSS**, and
  ref 08 confirms the halo language survives a warm accent.
- **Reduced motion:** gate the two *loops* (bloom, spinner) behind `@media (prefers-reduced-motion:
  reduce)` **and** the `prefersReduced` JS check; the token zero-out covers the rest.
- **Do NOT** add an idle bloom "to look alive" — it is the single forbidden animation (surveillance
  perception). Motion starts on tap, ends when the utterance resolves.

---

## Appendix — capture provenance

- Tool: `agent-browser` (CDP), isolated session (`--session voiceref`) to avoid collision with other
  agents' shared "default" session.
- 8 real screenshots in `./refs/` (01–08). OpenAI first-party pages (`openai.com`, `help.openai.com`,
  `openai.fm`) were **all Cloudflare/Turnstile-gated** and could not be captured directly; ref 02 uses
  OpenAI's own product image as embedded in press coverage. Dribbble now hides like/view/save counts
  behind login — the only engagement number cited (ref 05, ~98 likes / ~28.4k views) comes from a
  public Dribbble **search** listing (2026-07); all authors were read from each shot's page `<meta>`.
  No names or numbers are invented.
