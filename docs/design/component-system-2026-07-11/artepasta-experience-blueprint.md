# ArtePasta — One Continuous Experience: Blueprint & Design System

> **Thesis:** the order lifecycle is *one unbroken sea*. You arrive on an open ocean, **dive
> through the surface** into the menu, choose, place the order, and the same sea **re-forms and
> develops** underneath the tracking view — its colour, amplitude and wave direction all carry the
> order's state. No hard page cuts; every stage is a transformation of the last.
>
> Reference artifact: **`artepasta-v4.html`** →
> https://claude.ai/code/artifact/f528e9af-6548-422b-a902-497dc56dd296
> Companion research recipe: [`sea-cinematic-recipes.md`](./sea-cinematic-recipes.md)
> Model routing for the agents that built this: [[model-routing-opus-reasoning-haiku-else-2026-07-12]]

Status: reference implementation is code-review + syntax verified (no headless WebGL renderer was
available to screenshot). GLSL is valid; if a shader fails to compile the body falls back to
`.no-gl` gradient — graceful degradation, never a crash.

---

## 0. The three acts and the two transitions between them

| Act | Surface | What the sea is doing | Enter via |
|---|---|---|---|
| **1 · Arrive** | 100vh hero, full-bleed sea | ambient developing ocean, `u_phase=0` (terracotta, calm) | page load |
| **2 · Choose** | menu sheet + detail + checkout | sea sits *underneath*, dimmed, `u_dive` relaxed to 0 | **Transition A: the dive** |
| **3 · Receive** | tracking view + courier map | sea returns, `u_phase` climbs 0→1 across 5 stages | **Transition B: the phase climb** |

**Transition A — the dive (hero → menu).** Tapping *View menu* does not scroll; it **plunges the
camera through the water surface** at the button's location, then the menu sheet surfaces from
below while you're "under." One JS-tweened scalar drives everything (see §1).

**Transition B — the phase climb (checkout → tracking).** Placing the order sets `u_phase` and lets
it lerp toward 1 over five stages. The sea doesn't restart — it *develops*: warmer, taller,
turning, brighter, exactly as an order progresses received → cooking → riding → arriving →
delivered (see §2).

Both transitions are **reduced-motion aware**: under `prefers-reduced-motion` the dive is replaced
by a plain `scrollIntoView`, and the render loop renders one static frame.

---

## 1. Transition A — the cinematic dive (`u_dive`)

The whole plunge is a **single tweened scalar `u_dive ∈ [0,1]`**, eased `easeInOutCubic`. Everything
else — zoom, plunge, refraction, ring-wipe, chromatic aberration, water grade, caustics — is
*shaped from `u_dive`* in the composite pass. This is the key idea from the research: don't animate
ten things, animate one and derive the rest.

### 1.1 The tween (JS)

```js
var dive={v:0,from:0,to:0,t0:0,dur:1,active:false}, diveClick=[0.5,0.46];
function ez(x){ return x<0.5 ? 4*x*x*x : 1 - Math.pow(-2*x+2,3)/2; }   // easeInOutCubic
function tweenDive(to,dur){ dive.from=dive.v; dive.to=to; dive.t0=time; dive.dur=dur; dive.active=true; }

window.__seaDive=function(cx,cy){                    // cx,cy = click point in 0..1, y-up
  eTarget=0.92;                                      // energy spike
  if(cx!==undefined){ diveClick[0]=cx; diveClick[1]=cy; }
  tweenDive(1.0, 1.05);                              // plunge  0 -> 1   (~1.05s)
  setTimeout(function(){ tweenDive(0.0, 0.75); }, 1180); // relax back, hidden under the menu
};
```

The button handler anchors the dive on the button centre and surfaces the sheet mid-dive:

```js
var r=this.getBoundingClientRect();
var cx=(r.left+r.width/2)/innerWidth, cy=1-(r.top+r.height/2)/innerHeight;
window.__seaDive(cx,cy);
setTimeout(()=>sheet.scrollIntoView({behavior:"smooth"}), 620);  // menu rises while you're under
```

Per-frame the tween advances in `step()` and feeds three composite uniforms:

```js
if(dive.active){ var dk=(time-dive.t0)/dive.dur; if(dk>=1){dk=1;dive.active=false;} dive.v=dive.from+(dive.to-dive.from)*ez(dk); }
// COMP uniforms:  u_dive = dive.v ; u_time = time ; u_click = diveClick
```

### 1.2 What `u_dive` drives, in the composite pass (`dv = u_dive`)

Read like a timeline of the plunge; each line is one visible beat:

| Beat | Range of `dv` | Effect | Shader term |
|---|---|---|---|
| Camera push-in, then settle | 0.1→0.62→1.0 | zoom toward the click point | `zoom = 1 + 0.32·smoothstep(.1,.62,dv) − 0.24·smoothstep(.62,1,dv)` |
| Downward plunge | 0.1→0.62 | UVs sink toward the click | `duv.y −= 0.16·smoothstep(.1,.62,dv)` |
| Refraction through the "membrane" | peaks ~0.45 | sample the trail-FBO luminance gradient, bend UVs | `refr = ∇luma · 6 · pls(dv,.05,.45,1)` |
| Ring-wipe from the button | expands with `dv` | a travelling sine front from the click | `front = d − dv·1.3 ; ring = sin(front·26)·…` |
| Chromatic aberration | peaks ~0.45 | R/B split radially, strongest at the surface | `ca = (0.004 + 0.02·|ring|)·pls(dv,.25,.45,.7)` |
| Inverted Beer–Lambert grade | 0.35→1.0 | absorb **blue first** → water stays warm terracotta, never turns sea-blue | `transm = exp(−depth·vec3(1.0,1.6,2.6))` |
| "Through the membrane" darken pulse | 0.40–0.66 | brief plunge into darkness at the surface break | `water *= mix(1, 0.42, pls(dv,.40,.52,.66))` |
| Textureless caustics | 0.45→0.72 | 3-octave animated light dapples underwater | `caus(duv)` (no texture, pure trig) |
| Reversibility guarantee | dv=0 | composite is **byte-identical to the original** | `col = mix(base, water, smoothstep(0,0.4,dv))` |

`pls(x,a,b,c) = smoothstep(a,b,x)·(1−smoothstep(b,c,x))` — a ramp-up-then-fall pulse. That last row
matters: at `dv=0`, `col = base = grade(scene)`, so the effect is *fully reversible* and the ambient
hero is untouched.

> **Design guard we hit and fixed:** naïvely `mix(deep, scene·tint, transm)` tints the composite even
> at `dv=0` (because `transm=1` there). The fix is to compute `base` (the plain grade) and `water`
> separately, then cross-fade them by `smoothstep(0,0.4,dv)` — so the water grade only exists once
> you're actually diving.
>
> **Double-zoom guard:** the shader owns the whole plunge. Do **not** also `transform:scale()` the
> canvas in CSS — the `.dive` overlay element is inert (`opacity:0; pointer-events:none`). Two zoom
> sources compound and also produce transient dark corners.

---

## 2. Transition B — state → wave "ETA continuum" (`u_phase`)

The single most-requested fix: **make the state change visible.** `u_phase ∈ [0,1]` lerps slowly
(`phase += (phaseTarget−phase)·0.022`) and is set per stage by `window.__seaPhase(p)`. Three
independent wave properties travel with it so the change is unmistakable:

| Property | At `phase=0` (received) | At `phase=1` (delivered) | Vertex-shader term |
|---|---|---|---|
| **Amplitude** | ×0.5 | ×1.95 | `amp = breath·mix(.30,1.20,z)·(1+en·.6)·mix(0.5,1.95,grow)` |
| **Direction** | 0 rad | ~0.9 rad rotation | `ang = grow·0.9; wp = rot(ang)·wp` — the whole field **turns** |
| **Gradient** | terracotta `#57291C→#D6803F` | gold `#A3592A→#FFD375` | `c1 = mix(.34,.16,.11 → .64,.35,.16, grow); c2 = mix(.84,.50,.25 → 1,.83,.46, grow)` |

`grow = smoothstep(0,1,u_phase)`. Amplitude nearly **quadruples**, the wave field **rotates ~50°**,
and the entire colour ramp travels **terracotta → gold** — so each stage transition reads at a
glance, not as a subtle tint. Energy (`u_energy`) adds a transient swell on each add-to-cart
(`__seaSwell` → `eTarget=0.5`, decays back to 0.14).

The five stages (`STAGES[]`) each pin a `phase`, advancing on a 4.2s timer:

| Stage | eye label | phase | courier |
|---|---|---|---|
| received | "order placed" | 0.05 | at restaurant |
| cooking | "in the kitchen" | 0.30 | leaves ~here |
| courier | "on the way" | 0.58 | mid-route |
| arriving | "almost there" | 0.85 | near address |
| delivered | "buon appetito" | 1.00 | at address |

---

## 3. Render pipeline (how the effects are physically produced)

WebGL2, one canvas `#sea`, three shader programs, ping-pong RGBA8 FBO for the feedback trail:

```
each frame:
  1. FADE pass  → fb[d]     : previous trail × 0.963, sampled with a 0.9986 zoom-in (slow inward drift)
  2. PARTICLE   → fb[d]     : additive GL_POINTS, COUNT = 84000 desktop / 26000 mobile,
                             positions computed procedurally in the vertex shader from gl_VertexID hash
                             (no attribute buffers — pure GPU), blendFunc(ONE,ONE)
  3. COMP pass  → screen    : tonemap + the entire §1 dive, reading fb[d] as BOTH scene and membrane
  swap si <-> d
```

- **Waves:** 5 Gerstner/trochoidal octaves, deep-water dispersion `ω = √(g·k)` (`G=0.30`), directions
  `D0..D4`, plus Cauchy–Poisson touch ripples (`u_rip[8]`, pointer-down injects a ring).
- **Full-screen framing (no top-down, no dark corners):** particle field widened to
  `b.x = hx·mix(1.5,2.2,z)` and `b.y = mix(1.18,−1.24, pow(z,1.02))` so every depth band spans the
  frame; NDC squash `gl_Position = vec4(p.x·0.73, p.y, 0, 1)`. The old vignette is removed. The grade
  adds only a gentle warm lift toward the bottom, not a corner darken.
- **Single-texture dive:** the research recipe used two textures (uScene + uTrail); we fold both roles
  onto `u_src` (the trail FBO) to keep the pipeline at ≤1 composite pass / 60fps without a refactor.

---

## 4. Design system — reused-and-reskinned real dowiz storefront

Acts 2–3 are **not hand-made components** — they mirror the real `/s/artepasta` anatomy (captured in
`storefront-shots/NOTES.md`), re-skinned to the light "sea" brand. The dead compare button was
omitted; detail/checkout CTAs were made active (the live demo disables them) because this artifact
demonstrates the intended ordering flow.

### 4.1 Tokens (authoritative for the re-skin)

```css
:root{
  --paper:#FAF6F0;  --card:#F2EEE8;  --line:#DEDBD5;         /* cream page · card · hairline */
  --ink:#231C15;    --ink-soft:#6A5D49;  --faint:#9A8B71;    /* text ramp */
  --terra:#C24E2C;  --terra-deep:#9E3C20; --parm:#CE9E45;    /* warm accents */
  --red:#C21A1F;    /* brand red — prices + active tab ONLY (matches real page) */
  --stage:#0C0704;  /* the deep sea ground behind the hero/maps */
  --sans: ui-rounded, "SF Pro Rounded", system-ui, …;        /* stands in for Nunito/Quicksand */
  --serif:"Iowan Old Style", Palatino, Georgia, serif;
  --ease:cubic-bezier(0.32,0.72,0,1); --tide:cubic-bezier(0.37,0,0.63,1);
  --spectral:linear-gradient(90deg in oklch, #9E3C20,#C24E2C,#E8A544,#CE9E45,#C24E2C);
}
```

Cards: radius 16px, `--card` fill, `--line` border. Prices and the active category tab are the **only**
uses of `--red`. Dark theme is fully specified (`prefers-color-scheme` + `data-theme` override in both
directions). Currency is **ALL** (Albanian Lek) throughout; header has a cosmetic `L · ALL` +
`SQ | EN` toggle for parity with the real chrome.

### 4.2 Menu (mirrors real anatomy)

- **Counted category tabs** — `Të gjitha 10 / Pasta 5 / Antipasti 2 / Secondi 1 / Dessert 2`.
- **Filter row** — search `Kërko` + sort chips `Çmimi ↑` (price), `Më shumë proteina` (protein),
  `Kalori ↑` (calories). `applyList()` = category filter → text search over name+desc+ingredients →
  sort. All live.
- **Dish cards** — chef badge, veg/hot tags, `NNN kcal` + `Ng proteina`, price `NNN ALL`, quick
  `+ shto` (add) with `stopPropagation`, whole card taps into detail.
- **Data** — 10 dishes each with `p`(price), `kcal`, `pr`(protein), `veg/hot/chef`, `ing[]`
  (ingredient chips) and `sq` (italic Albanian ingredient line).

### 4.3 Product detail (modal desktop / bottom-sheet mobile)

Fills: name, `NNN ALL`, English desc, **italic Albanian ingredient line** (`sq`), ingredient chips
(`ÇFARË PËRMBAN`), nutrition (kcal + protein + veg), quantity stepper, active *add* CTA that folds
qty into the cart. Escape closes; backdrop click closes.

### 4.4 Checkout sheet

Aggregates the cart into line items, subtotal, **address** (with pin) + entrance/floor note + phone
(OTP hint), **Cash on Delivery is the only payment** (red line — never scored, never a card field), a
sea-tinted mini-map with the address pin, and `Vendos porosinë` → closes → `startTracking()`.

---

## 5. Courier + address map (Act 3, stylized to the sea)

Procedural Canvas 2D, **not** Google Maps — of a piece with the ocean:

- **Ground:** `#170d06 → #26130a` gradient; "streets" are faint **flowing currents** (animated sine
  rows + slightly skewed verticals), warm `rgba(206,116,58,·)`.
- **Route:** a cubic-bézier `RP0(restaurant) → RP3(address)`; remaining leg dim-dashed, travelled leg
  bright gold with glow.
- **Courier position from order state:** `rt = clamp((phase−0.30)/(0.92−0.30))` — the courier *leaves*
  at the cooking stage and *arrives* near delivered, so the map and the sea share one clock.
- **Markers:** restaurant dot (`#F4C25A`), **pulsing address pin** that brightens on approach
  (`near = clamp((phase−0.7)/0.3)`), courier dot with a warm glow and a following `🛵 Andi` label.
- A static mini-version of the same map appears in the checkout sheet (`drawCoMap`).

---

## 6. Porting into the real dowiz codebase (for the code session)

1. **Sea component** = one self-contained `<canvas>` + the three shaders (§3). Expose an imperative
   API: `seaDive(cx,cy)`, `seaPhase(0..1)`, `seaSwell()`. It owns its RAF; pause on `visibilitychange`.
2. **Dive is a route transition primitive**, not a menu-only trick: call `seaDive()` on any
   hero→content navigation anchored at the trigger element's centre; surface the destination at
   ~600ms.
3. **`seaPhase` binds to real order status** (received/cooking/courier/arriving/delivered) from the
   orders channel — replace the 4.2s demo timer with the live status stream. Same for courier `rt`
   (bind to real courier telemetry when available; fall back to the phase-derived estimate).
4. **Tokens above are the re-skin contract** — do not invent new `--brand-*`; map onto the existing
   storefront token set. Prices + active tab keep `--red`; everything else warm-neutral.
5. **Reduced-motion + no-GL fallbacks are mandatory** and already specified — keep them.
6. **COD stays the only payment**; courier is **never scored/rated** (hard red line).

---

## 7. Verification & open items

- ✅ JS `node --check` clean; div tags balanced 60/60; all 10 key functions defined once
  (`startTracking, openCheckout, closeCheckout, openDetail, closeDetail, startMap, stopMap, drawMap,
  drawCoMap, tweenDive`); 3 canvases; reduced-motion guard present.
- ⚠️ **Not** visually rendered — no headless WebGL here. If the dive reads too weak/strong, tune
  `tweenDive` duration and the §1.2 `dv` breakpoints; if the underwater grade is too dark, soften the
  Beer–Lambert vector `vec3(1.0,1.6,2.6)` or the darken pulse `mix(1,0.42,…)`.
- Companion: [`sea-cinematic-recipes.md`](./sea-cinematic-recipes.md) holds the full research recipe
  (the four dive phases A/B/C/D, the separate-texture variant, the stylized-map recipe).

*Built with Opus for the reasoning/research agents, Haiku for mechanical work, per the operator's
model-routing rule.*
