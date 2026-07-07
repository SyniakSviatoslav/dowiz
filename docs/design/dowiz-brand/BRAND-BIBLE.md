# DOWIZ BRAND BIBLE — "Warm Cosmo-Noir"
> The coherent, unique Dowiz brand: narration, vibe, ironical language, and design system.
> Authored 2026-07-07 (Fable reasoning) from web research on Cowboy Bebop, warm-noir web UI,
> and the Nomadic Tribe structural skeleton. Operator-directed. This is a tuning fork, not a cage.

---

## 0. THE ONE IDEA — THE HYBRID

Dowiz is a **hybrid service**, so **hybrid is the brand**. Two natures live in one body, always both,
never blended into grey mud:

| COLD — reptilian logic · the machine | WARM — god-blessed mission · the soul |
|---|---|
| The Rust kernel: deterministic, total, refuses to lie or let a door invent money | The mission: owner sovereignty, dignity, the anti-aggregator — a commons, not a cage |
| Event-sourced truth. Math. Security by construction. | Authentic craft. Jazz. Cowboy Bebop warmth. |
| **Bebop Noir** — mono readouts, teal signal, the 180ms *snap* | **Cosmo-Gothic cathedral** — serif headlines, amber bloom, the 48s *drift* |
| Warm near-black `#12100E`, tabular mono data | Warm amber lamp-pools, bone text, film grain |
| *"This action is irreversible. Confirm you know what you're doing."* | *"Quiet night. Nothing on the pass yet."* |

**The governing ratio (from research, non-negotiable):** ~90% muted warm-dark field + still composition
(the soul, the hold) and ~10% saturated, precise accent (the machine, the strike). One meaningful
saturated color per view. This single rule is the difference between elegant Bebop and gaudy retro
cosplay.

**The thesis made visible:** the *Horizon Drift* — a cold, precise machine (a stylish ship) drifting
warm, slow, and blessed across a lit horizon. See §7.

**Brand essence in one line:** *An orbital cathedral of a machine, run by people who have survived
worse and still take pride in the work — it refuses to lie to you, and it refuses to take a cut.*

**The tagline (operator-coined 2026-07-07):** **"Hybrid is a feature, not a bug."** It is the whole
thesis in five words — the cold machine and the warm mission are not a compromise to be resolved but
the *point*. Use it as a north-star line (about page, manifesto, the sacred-moment footer). It is also
a perfect specimen of the voice: dry, technical, true, quietly defiant.

---

## 1. WHERE THE BRAND LIVES (scope — read this first)

Dowiz has two worlds. **Do not confuse them.**

- **The dining room = the vendor storefront (`/s/:slug`).** White-label, per-tenant, warm and
  food-premium, customized by the vendor's own branding. **The Dowiz brand does NOT go here.** Untouched.
- **The engine room = every Dowiz-owned surface.** Landing, onboarding/claim, admin, courier, login,
  404, privacy, system messages, docs, demos, marketing. **This is where Warm Cosmo-Noir lives.**

Mechanically: the brand is the internal-brand layer `[data-skin="bebop"]` in
`packages/ui/src/theme/tokens.css` — tenant-locked, never overridable by a storefront theme.
It **replaces the retired Paper/Moebius skin** (operator decision 2026-07-07).

---

## 2. THE THREE PILLARS (the source code of the vibe)

Every surface must draw from all three. You may change the ratio (see §11 Modulation); you may not
drop one.

### 2.1 Cosmo-Gothic Cathedral — the seduction (the soul)
Terrifying elegance of deep space + the architectural weight of a cathedral. Vast negative space,
warm near-black, sharp warm-serif headlines, languid velvet motion, a subtle sense of the sacred
(non-denominational — "sanctuary," "built with devotion," never a named god). Emotional goal: **awe,
focus, a little reverence.**

### 2.2 Bebop Noir — the machine (the reptilian logic)
High-tech, low-life, analog-worn. *Cassette Futurism*: the future is decades old — CRT readouts,
tungsten lamps, taped-together hardware, not glowing glass. Mono data streams, teal signal, tactile
mechanical controls, 35mm grain. Emotional goal: **tactility, reliability, cool.**
> Trap to avoid: cyberpunk. Bright glassy neon-on-black, HUD chrome, synthwave grids. Bebop is analog,
> dusty, quiet — the *opposite*. This is the #1 tell of a cheap imitation.

### 2.3 Ukrainian Irony — the attitude (resilience, NOT fatalism)
Gallows humor, anti-delusion, craft-pride under the cynicism. We joke because we've seen worse, then
we fix the generator. No toxic positivity, no corporate bullshit, no emojis-as-UI. But **not
nihilism** — there is defiance and pride: the machine *works*, and we're quietly proud of it. The joke
is *with* the operator (a fellow survivor), never *at* them. Emotional goal: **mutual respect, dry
amusement, defiant competence.**

---

## 3. COLOR

Warm cosmo-noir. Cool/warm near-black field, warm amber life, teal signal, one saturated accent at a
time. **Never `#000000` for surfaces** — the warmth is the whole point. Tokens live in the
`[data-skin="bebop"]` block. (Hexes are community-extracted from Bebop palettes + calibrated to the
described grade; re-verify contrast WITH the grain overlay before locking — see §12.)

### Noir field (the 90%)
| Token | Hex | Role |
|---|---|---|
| `--void` | `#12100E` | warm near-black — base canvas |
| `--hull` | `#1A1E1F` | teal-tinted charcoal — raised surface |
| `--slate` | `#232A2E` | slate-teal — elevated panel/modal |
| `--ink-night` | `#2A2F52` | deep indigo — optional night-exterior surface |

### Warm punctuation (the soul, ~10%)
| Token | Hex | Role |
|---|---|---|
| `--amber` | `#E8A544` | tungsten lamp — **primary interactive/brand** |
| `--amber-hi` | `#F0B95E` | hover |
| `--amber-deep` | `#C68530` | pressed/strong |
| `--rust` | `#B26850` | secondary warm accent |
| `--gold` | `#C79675` | muted decorative accent |
| `--ember` | `#FC3C04` | rare high-impact kicker ONLY |

### Signal (the machine)
| Token | Hex | Role |
|---|---|---|
| `--teal` | `#46B0A4` | success / alive / data-signal |
| `--cyan` | `#3FB7C4` | HUD glow — rare |
| `--magenta` | `#C8438F` | neon highlight — rare |

### Danger / neutrals / strokes
| Token | Hex | Role |
|---|---|---|
| `--blood` | `#E0543E` | danger (large + icon; never color-only) |
| `--blood-deep` | `#B9080E` | rare high-consequence danger |
| `--bone` | `#F2E9DB` | primary text (~15:1 AAA) |
| `--taupe` | `#BCB09C` | secondary text (~8:1 AA) |
| `--ash` | `#8F8474` | muted (AA large only, never small body) |
| `--card-paper` | `#E8DFD3` | aged-paper chips / light surfaces |
| `--hairline` | `#3A3128` | decorative dividers |
| `--stroke` | `#4B4034` | input borders / focus rails |

**Color laws:**
1. **Amber/rust/ember are never small body copy.** Chroma is for display, accents, CTAs. Body is
   bone/taupe. (The single biggest lever against "unreadable retro.")
2. **One saturated accent per view.** Everything else muted.
3. **Elevation = warmer + lighter surface step**, not shadow-only (`--void → --hull → --slate`).
4. **Never color-only status** (WCAG 1.4.1) — always pair with icon/label.
5. **Verify accents on the surface they sit on**, not just canvas (accent-on-surface drops ~1 step).
   Every glow needs a solid-outline fallback.

### Daylight courier variant
Warm-noir washes out in direct sun; couriers work outdoors. `[data-skin="bebop"][data-daylight="true"]`
(courier surface only) remaps to a high-contrast warm-light palette (paper `#EDE6D8`, ink `#1A1712`,
darkened amber `#A85F14`), grain off. Same identity, sun-readable. Toggle scoped to courier.

---

## 4. TYPOGRAPHY — the hybrid three-register system

The aesthetic lives in the **display** (retro-future warmth) and the **mono** (analog-technical
texture). The UI sans is **boring on purpose** so data stays legible. All faces are open-license.

| Register | Face (token) | Use | Pillar |
|---|---|---|---|
| Display serif | **Fraunces** → `--font-display` | Headlines, hero, section titles. Warm, high-contrast, softly "wonky" (Bookman/Cheltenham lineage — Bebop's end-card serif register). | Cathedral / soul |
| Condensed caps | **Oswald** (→ Bebas Neue) `--font-condensed` | Eyebrows, "SESSION" labels, kickers. Bebop title-card impact. | Bebop / machine |
| UI sans | **DM Sans** `--brand-font-body` | All body, forms, dense data. Disappears. | neutral spine |
| Mono | **Space Mono** (→ JetBrains Mono) `--font-mono` | Prices, order IDs, timestamps, gauges, terminal states. The machine's CRT voice. | Bebop / machine |

**Type laws:** headings serif; body sans; **numbers/money/IDs in mono with `tabular-nums`**;
eyebrows/labels in condensed uppercase with `0.14em` tracking. **Never** pixel/arcade fonts
(Press Start 2P, Orbitron) for UI — instant cyberpunk/amateur. Reserve any deco face for a logotype
moment only.

> Font-loading note (Pass 2): Fraunces/Oswald/Space Mono must be added to the `googleFontsHref`
> allowlist (`packages/ui/src/lib/fonts.ts`) or self-hosted. JetBrains Mono + DM Sans + DM Serif
> Display already ship as fallbacks, so the dormant skin degrades gracefully today.

---

## 5. SPACE & LAYOUT — the Nomadic Tribe skeleton (skin-stripped)

We take Nomadic Tribe's **structure**, not its style. Mined from its production source. The spine:

**One full-bleed stage + a minimal corner-pinned HUD + a linear gated sequence + one micro-interaction
per section + a share/CTA card.** No deep menu.

- **Stage:** full-viewport (`100vh`) sections, CSS scroll-snap, each full-bleed.
- **HUD convention:** overlay chrome corner-pinned at a **consistent 40px inset** — top-center
  advance/nav, bottom-right utility toggle (theme/lang/sound), bottom-left contextual label, a thin
  progress indicator (vertical bar or top scroll line), captions capped at ~500px, left-aligned,
  placed contextually near their focal element.
- **Poster frame:** framed screens (hero, end card) use a centered, aspect-aware, `max-width ~1180px`
  composition with generous negative-space margins — bordered "panels" (comic-page logic), not a fluid
  12-column grid. One dominant CTA per screen.
- **Reusable component set (build once):** `<Stage>`/`<Section>` (enter-reveal hook), `<Button>`
  (bordered frame + hover), `<ProgressBar>`, `<HudCorner>` slots, `<Caption>` (letter-stagger,
  contextual position), `<Cursor>` (hint-carrying, desktop), `<InstructionCard>` (mobile, bottom-left),
  `<OrientationGate>`+`<Fallback>`, `<EndCard>` (headline + share + CTA).
- **Base unit:** 4px (existing `--space-*`). The **"scarred grid"** (Ukrainian irony): deliberate,
  subtle misalignment — a cropped border, an off-beat offset, one flickering character — so it looks
  lived-in and repaired, never sterile.
- **Rhythm to reuse:** `gate → repeated [scene + one required micro-interaction + a caption] → payoff/
  share card`.
- **Responsive:** same narrative spine; swap the input model (scroll ↔ swipe) and hint surface
  (cursor ↔ instruction card) per device; orientation/capability gates protect the experience.

Full structural blueprint: research report archived; landing implementation is Pass 2.

---

## 6. MOTION — jazz as logic

Bebop was built on a *jazz structure* (episodes are "sessions"). Motion is **content-driven tempo**:
punchy syncopation for action, long eased dwells for calm. **The hold** — most of the screen is still;
motion is a deliberate accent.

**Two curves cover ~90% (tokens in the skin):**
- `--ease-jazz-in: cubic-bezier(0.23, 1, 0.32, 1)` — entrances/reveals. Fast attack, long decel,
  slight overshoot. `--dur-reveal 480ms` content, `--dur-banner 900ms` hero. Enter from
  `translate3d(0, 8px, 0)` + opacity.
- `--ease-jazz-snap: cubic-bezier(0.32, 0.72, 0, 1)` — interactions. `--dur-snap 180ms` (150–220ms)
  for hovers, presses, dropdowns, modals.

**The signature move — "low-power idle → sharp engage":** resting elements breathe slowly (a pilot-
light amber glow over 3–4s `ease-in-out`, a barely-drifting gradient — the "2 a.m. bar" idle). On
interaction they *snap* to the tight curve. The contrast between languid idle and sharp engage **is
the cool** — and it is the hybrid (soul at rest, machine on strike).

**Syncopation:** irregular stagger (e.g. `0 / 40 / 120 / 160ms`), not a metronome. One "hero" element
takes the solo (overshoot curve); neighbors comp (calmer curve). **Call-and-response:** every user
action gets a rhythmically-timed answer.

**Laws:** ease-out entrances, ease-in exits; animate `transform`/`opacity` only; nothing > ~1s;
stagger 30–60ms; springs only for organic motion, never utility controls. **Reduced-motion:** set
`animation/transition-duration: 0.01ms` on everything incl. pseudo-elements (non-zero so
`animationend` still fires); kill ambient loops and freeze grain.

---

## 7. THE SIGNATURE ANIMATION — "Horizon Drift" (spec; built Pass 2)

The thesis in one shot: a **cold, stylish machine drifting warm and slow across a distant, lit
horizon.** Landing hero backdrop + a subtler version behind admin login.

- **Parallax layers (back→front):** (1) star/dust field, near-static slow twinkle; (2) a warm horizon
  band — amber lamp-glow bleeding up from below (the "god-lit" line); (3) **the ship** — a clean
  silhouette with one warm window-glow and a faint anamorphic (horizontal) flare, drifting across on a
  `--dur-ambient 48s` linear loop; (4) foreground haze/grain drifting the opposite way (depth).
- **Grade:** muted teal-charcoal sky, one amber horizon, one lit window. Obeys the 90/10 rule.
- **Technique:** SVG/CSS layers, `transform: translate3d`-only, GPU-composited; `.stage-surface` opts
  the layer out of the global grain (has its own). Warm radial bloom behind the ship; vignette on the
  frame. Pause when offscreen (IntersectionObserver).
- **Reduced-motion:** freeze to one composed still frame (ship mid-horizon) — still beautiful, zero
  motion.
- **Perf budget:** must hold 60fps on mid-tier Android; if it drops, reduce layers, never framerate.
  No WebGL required (Nomadic's 23MB was the *ceiling*, not the target — asset discipline is the lesson).

---

## 8. TEXTURE — felt, not seen

- **35mm film grain:** low-opacity (`--grain-opacity 0.05`) SVG `feTurbulence` (`fractalNoise`,
  `numOctaves ≤ 3`), static data-URI on one fixed `::after` overlay (already in the skin), `soft-light`
  blend. Opts out over `.data-surface`/`.stage-surface`. Frozen under reduced-motion.
- **Anamorphic flare:** horizontal light smears on highlights — never radial star-bursts.
- **Warm bloom:** soft warm-tinted radial glow behind amber accents; low-contrast falloff, never on text.
- **Vignette:** single radial darkening at frame edges — cheap noir payoff, focuses the eye.
- **Scanlines:** only on genuine readout surfaces, opacity ≤ 0.06; never over body text (moiré).

**Caveats:** every texture on one composited `pointer-events:none` layer; measure text contrast *with*
overlays on; grain off in daylight-courier; respect `prefers-reduced-motion` / `prefers-reduced-transparency`.

---

## 9. VOICE & TONE — the narration

> **PROJECT-WIDE CANON (operator directive 2026-07-07).** This voice is not landing-page dressing —
> it is the **single, brand-consistent Dowiz narration for the entire project, forever.** Every
> user-visible string on every surface uses it: landing, admin, courier, onboarding, empty/loading/
> success/error states, toasts, emails, push, the Telegram bot, receipts, **and all sales & marketing**
> (outreach, demos, decks, social, docs). The "texting vibe" — humanized, dry, a real person who has
> seen some things — is the default. New copy that reads like corporate SaaS is a bug. The only
> exception is the stakes rule (law 7): money/auth/security copy stays plain and clear in all three
> languages. When writing ANY string anywhere in the codebase, match this register.

The narrator is a **battle-hardened, dry co-pilot** who respects you, refuses to lie or condescend,
and takes quiet pride when the machine works. Cool, a little tired, defiant. The bar (research):
*"if the only thing you can say about a writing style is that it's witty, it's probably flat."* Wit
rides on substance; it never replaces the information. Reference register: **Mailchimp-grade** — clear,
genuine, dry humor, your most competent friend — dialed toward cooler and more ironical.

**The seven laws:**
1. **State reality.** If it's broken, say so.
2. **Substance first, wit as garnish** (the last 10%). Every string still does its job.
3. **Respect intelligence.** Don't explain obvious UI.
4. **Dry > cutesy.** Understatement and punctuation. **No emojis. No exclamation-mark cheer.**
5. **The joke is with the operator, never at them.** Camaraderie of survivors.
6. **Subtle sacred, non-denominational.** Occasional reverent, cathedral-flavored words —
   *"sanctuary," "sovereign," "built with devotion," "your kitchen, your altar"* — never a named
   religion. (Operator decision; Albania is religiously mixed.)
7. **Match tone to stakes — THE HARD RULE.** Dry wit on brand moments (empty/success/onboarding/hero).
   **Plain, zero-joke, reassuring clarity on the money, auth, and security path** — payments, refunds,
   failures, account. In *all three languages*. A joke on a failed payment is a bug.

**Trilingual:** full dry wit in **sq / en / uk** (operator decision). Author the canon in English,
then adapt sq/uk *preserving the irony* (not literal translation) — Ukrainian carries the darkest
humor most naturally; Albanian is the live market, keep it sharp but never alienating. Money/error/
security copy: plain and identical-in-clarity across all three.

---

## 10. MICROCOPY LIBRARY (canonical EN; sq/uk adapt in-tone, Pass 2)

**Brand moments — full dry wit:**
| State | Copy |
|---|---|
| Landing hero | "Your kitchen. Your customers. Your money. Novel concept, we know." |
| Sub-hero | "Commission: 0%. That number is not a typo." |
| Onboarding welcome | "Link established. Let's get your kitchen off the leash." |
| Empty — no orders | "Quiet night. Nothing on the pass yet." |
| Empty — no menu | "The menu's empty. Even the void needs a starter." |
| Save success | "Saved. Back to work." |
| Order placed (owner view) | "Order's in. No middleman took a cut. Strange feeling, isn't it." |
| Loading | "Working. The machine doesn't rush — neither should you." |
| Offline | "Connection's gone. Orders are queued. They'll survive; we built them to." |
| 404 | "This page doesn't exist. Neither did half the promises other platforms made you." |
| Generic non-money error | "Something broke. Not your fault this time — probably. We're on it." |
| Sacred moment (footer/about) | "Built with devotion. Held together by spite. Yours, not ours." |

**Money / auth / security — plain, zero wit (all languages):**
| State | Copy |
|---|---|
| Payment failed | "Payment didn't go through. Your card wasn't charged. Try again or use another card." |
| Refund issued | "Refund sent. It may take 3–5 business days to appear." |
| Auth failed | "Wrong email or password. Try again." |
| Session expired | "You've been signed out. Sign in to continue." |
| Destructive confirm | "This deletes it for good. No undo. Confirm you know what you're doing." |

---

## 11. MODULATION — the mixing desk (per surface)

Slide the pillar faders without losing the soul. Ratio guide (machine : soul : irony):

| Surface | Turn UP | Turn DOWN | Why |
|---|---|---|---|
| **Landing / marketing** | Cathedral + Bebop (seduce; Horizon Drift) | Irony harshness → mysterious, not hostile | First contact; awe converts |
| **Onboarding / claim** | Cathedral warmth + light irony | Bebop density | Lower the barrier, warm welcome |
| **Admin dashboard** | Bebop + Irony (mono data, terminal calm, dry labels) | Cathedral velvet (strip to raw metal) | A working tool; density + honesty |
| **Courier** | Bebop tactile + daylight variant | Grain/vignette/heavy motion (safety, sun) | One-handed, outdoors, high-stakes |
| **Money / checkout / security** | Cathedral precision (cold, quiet) | Irony to ~zero | High stakes; plain clarity wins |
| **Errors / system** | Irony (brand) OR plain (money) — see §9 law 7 | — | Depends on stakes |

**Critical-path rule:** retro/aesthetic for *brand moments*; **plain, high-clarity logic for the
critical path** (auth, cart, payment, order status). Never let aesthetic consistency override
functional clarity.

---

## 12. ANTI-PATTERNS (red lines — how this looks cheap)

1. Sliding into **cyberpunk** (glassy neon, HUD chrome, synthwave grids). #1 tell.
2. **Full monochrome-amber "CRT terminal"** everything — gimmick + unreadable. Amber is 10%.
3. **Style over function** — retro chrome burying the critical path.
4. **Glow without a solid-outline fallback** (fails WCAG).
5. **Pure `#000` + saturated neon** — vibrates, reads cheap. Use warm near-blacks.
6. **Motion everywhere / bouncy springs on utility / >1s choreography** — kills the hold.
7. **Arcade/pixel fonts as UI.**
8. **Texture cranked too high** — visible grain over body text, heavy scanlines, moiré.
9. **Costume, not system** — retro as a skin over generic components instead of tokens.
10. **Perf debt from effects** — the vibe dies at 30fps.
11. **Wit on the money path** — a joke on a failed payment.
12. **Emojis as UI. Toxic positivity. Corporate buzzwords.** Not our voice.

---

## 13. GOVERNANCE

- **Single source of truth:** the `[data-skin="bebop"]` block in `packages/ui/src/theme/tokens.css` +
  this bible. Collapse any stray token copies into it (design-system unification task).
- **Tenant lock:** internal brand is FIXED — never storefront-overridable.
- **Every new token** carries a role comment + contrast note. **Every hex** stays inside `:root`/skin
  blocks — never in component files (`.agents/rules/design-system.md`).
- **Contrast:** verify each text/accent pair in an APCA/WCAG checker **with grain overlay applied**
  before locking. The bible's hexes are extracted/calibrated, not studio-exact — sample Blu-ray stills
  if frame-exact values are ever needed.
- **The final directive (when lost in a decision):** *Would a chain-smoking bounty hunter use this
  while waiting out a blackout on a 22nd-century orbital station — and would it still take their money
  honestly?* If yes, ship it.

---

## 14. IMPLEMENTATION STATE

- ✅ **Pass 1 (this doc):** `[data-skin="bebop"]` token block written (DORMANT — no consumer flipped,
  nothing visibly changed) + this bible. Paper/Moebius scheduled for deletion in Pass 2.
- ⏳ **Pass 2 (after operator review):** flip `paperSkinAttr()` → bebop, delete Paper block + helpers,
  add fonts to allowlist, build the landing on the Nomadic skeleton with Horizon Drift, author sq/uk
  microcopy, migrate admin/courier + daylight toggle, visual-regression proof (Playwright vs staging).
- **Blast radius for Pass 2** (mapped): `tokens.css` (delete paper), `paperSkin.ts`, `index.ts` export,
  `PaperIllustration`, `AdminRoutes`/`CourierRoutes`/`ClientLayout`, `TasksPage`, `main.tsx` (404),
  `PrivacyPage`, `css-comment-integrity.test.ts`.
