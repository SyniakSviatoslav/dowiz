# Sea Cinematic Recipes — Dive Transition, Stylized Delivery Map, Immersive Framing

**Context.** A full-screen WebGL2 Gerstner/dispersive ocean for the "Artë Pasta" storefront:
fixed canvas, additive point particles + a trail feedback FBO + a composite pass, graded warm
(terracotta → gold). A landing hero sits over the sea; "View menu" currently just scrolls to a
clean menu sheet. This document specifies three concrete, self-contained recipes (raw
WebGL2/Canvas2D/CSS only, everything inline — **no external libraries**, Artifact-CSP-safe):

1. A cinematic **"dive into the sea"** transition (hero → menu, reversible on scroll-up).
2. A stylized, procedural **delivery map** driven by an ETA parameter.
3. **Full-screen / no-vignette / not-top-down** framing fixes.

All three reuse the existing FBOs and add at most **one** extra fullscreen pass, so they hold 60 fps.

---

## 0. Design principles distilled from the research

- **One scalar drives the cinematic.** Award WebGL sites (Active Theory's Hydra-driven scene
  transitions) get their "cool factor" from *transitions between states*, not new geometry
  ([Webby / Active Theory](https://www.webbyawards.com/crafted-with-code/active-theory/)). Codrops'
  transition tutorials always reduce the effect to a single `progress` uniform (0→1) that both
  warps UVs and `mix()`es states
  ([Creative WebGL Image Transitions](https://tympanus.net/codrops/2019/11/05/creative-webgl-image-transitions/)).
  We follow that: **`uDive` 0→1** is the only value JS animates; everything else (submerge dip,
  chromatic split, caustics, zoom) is *derived in the shader* from `uDive`. That makes the whole
  effect trivially **reversible** (run `uDive` back to 0) and scroll-scrubbable.
- **Click-anchored radial refraction.** Codrops' GSAP ripple reveal stores the click point in
  `uMouse`, expands a `smoothstep` radial mask, and rides a `uRippleProgress` keyframed `0→1→0`
  wave — exactly the "wipe from the button" we want
  ([GSAP ripples/reveals](https://tympanus.net/codrops/2025/10/08/how-to-animate-webgl-shaders-with-gsap-ripples-reveals-and-dynamic-blur-effects/)).
  We replace GSAP with a ~10-line RAF tween.
- **Warm underwater, not blue.** Real water absorbs long wavelengths first, going blue with depth
  ([gameidea stylized water](https://gameidea.org/2026/02/01/creating-a-stylized-3d-water-shader/)).
  We deliberately **invert** the Beer–Lambert coefficients (absorb *blue* fastest) so "submerged"
  reads as a darkened terracotta, keeping brand continuity.
- **Reduced motion means fade, not freeze-frame.** The safe fallback for a big plunge is a plain
  opacity crossfade, applied opt-in via `prefers-reduced-motion`
  ([MDN](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion),
  [Smashing](https://www.smashingmagazine.com/2021/10/respecting-users-motion-preferences/)).

---

## 1. The "Dive Into The Sea" transition (hero → menu)

### 1.1 What the user sees (single timeline, ~1.6 s)

The transition adds **one fullscreen "dive" pass** after the existing composite. It samples the
composited sea color and the **trail FBO** (the trail's luminance *is* the water membrane we
refract through). All timing below is derived from `uDive`; the JS only tweens `uDive: 0→1` with
`easeInOutCubic` over ~1600 ms.

| Phase | `uDive` | ~ms | What happens |
|---|---|---|---|
| **A — Wind-up** (anticipation) | 0 → 0.10 | 0–160 | Sea energy ramps: amplitude ×1.0→1.6, chop up, particle spawn ↑. Hero copy scales `1→0.96` + fades. A radial ripple is *seeded* at the click point (`uClick`). |
| **B — Plunge** | 0.10 → 0.45 | 160–720 | Camera pushes in: composite UV zoom `1.0→1.35` toward `uClick`; vertical field shift downward. Radial refraction wipe expands outward from the button. |
| **C — Break the surface** | 0.45 → 0.62 | 720–1000 | Luminance **dips** to ~0.4 then recovers (the dark "membrane" moment). Chromatic aberration **peaks** here. Color grade lerps warm → warm-underwater. Caustics fade in. |
| **D — Rise into menu** | 0.62 → 1.0 | 1000–1600 | Grade lerps underwater → warm; zoom eases back `1.35→1.08`; caustics settle to a whisper. The **menu sheet surfaces from below** (CSS: `translateY(8vh→0)`, `opacity 0→1`, `filter: blur(12px→0)`). |

Because C's dip and B's zoom are *shaping functions of `uDive`*, scroll-up simply drives `uDive`
back down and the whole thing plays in reverse — menu sinks, membrane re-forms, sea relaxes.

### 1.2 The dive fragment shader (extends the composite pass)

```glsl
#version 300 es
precision highp float;
in  vec2 vUv;
out vec4 frag;

uniform sampler2D uScene;   // existing composited sea color (this frame)
uniform sampler2D uTrail;   // existing trail feedback FBO (our "water membrane")
uniform float uTime;
uniform float uDive;        // 0..1  — the ONLY animated value
uniform vec2  uClick;       // button center in UV (0..1), y-up

// ---- shaping curves derived from uDive (no extra JS) ----
float pulse(float x,float a,float b,float c){          // ramp a->b, fall b->c
  return smoothstep(a,b,x) * (1.0 - smoothstep(b,c,x));
}
// procedural caustics: 3 cheap octaves of interfering sines (Renou-style, no texture)
float caustics(vec2 p){
  p *= 6.0; float t = uTime*0.4; float v = 0.0, amp = 0.6;
  for(int i=0;i<3;i++){
    vec2 q = p + vec2(sin(t+float(i)*1.7), cos(t*1.3+float(i)*2.1));
    v += pow(0.5+0.5*sin(q.x)*sin(q.y), 8.0) * amp;
    p *= 1.7; amp *= 0.65;
  }
  return v;
}

void main(){
  // 1) camera push toward the click point
  float zoom   = 1.0 + 0.35*smoothstep(0.1,0.62,uDive)          // in
                     - 0.27*smoothstep(0.62,1.0,uDive);          // settle back
  vec2 uv = (vUv - uClick)/zoom + uClick;
  uv.y += 0.18 * smoothstep(0.1,0.62,uDive);                     // plunge downward

  // 2) refraction through the trail "membrane" (gradient of trail luminance)
  float ripplePhase = pulse(uDive, 0.05, 0.45, 1.0);            // 0->1->0
  vec2  px = vec2(1.0)/vec2(textureSize(uTrail,0));
  float tl = texture(uTrail, uv+vec2(-px.x,0)).r, tr = texture(uTrail, uv+vec2(px.x,0)).r;
  float tb = texture(uTrail, uv+vec2(0,-px.y)).r, tt = texture(uTrail, uv+vec2(0,px.y)).r;
  vec2  refr = vec2(tr-tl, tt-tb) * 6.0 * ripplePhase;

  // 3) radial ring wipe emanating from the button
  vec2  dv = uv - uClick; float d = length(dv);
  float front = d - uDive*1.3;                                   // expanding wavefront
  float ring  = sin(front*28.0) * smoothstep(0.22,0.0,abs(front)) * ripplePhase;
  uv += refr + normalize(dv+1e-4) * ring * 0.03;

  // 4) chromatic aberration, peaking as we cross the surface (~uDive 0.45)
  float ca = (0.004 + 0.02*abs(ring)) * pulse(uDive,0.25,0.45,0.7);
  vec2  cad = normalize(dv+1e-4) * ca;
  vec3  col = vec3(
    texture(uScene, uv + cad).r,
    texture(uScene, uv       ).g,
    texture(uScene, uv - cad).b
  );

  // 5) WARM underwater grade — absorb BLUE fastest (inverted Beer–Lambert)
  float depth = smoothstep(0.35,1.0,uDive);
  vec3  transm   = exp(-depth * vec3(1.0, 1.6, 2.6));           // r keeps, b dies
  vec3  waterTint= vec3(0.90, 0.58, 0.36);                       // terracotta cast
  vec3  deep     = vec3(0.05, 0.035, 0.03);                      // near-black warm
  col = mix(deep, col*waterTint, transm);

  // 6) submerge darken pulse (the dark "through the membrane" beat)
  col *= mix(1.0, 0.40, pulse(uDive,0.40,0.52,0.66));

  // 7) caustics bloom once under the surface
  float cAmt = smoothstep(0.45,0.72,uDive) * (1.0 - 0.5*smoothstep(0.88,1.0,uDive));
  col += cAmt * caustics(uv) * vec3(1.0, 0.78, 0.5) * 0.5;

  frag = vec4(col, 1.0);
}
```

**Why it's cheap:** one extra fullscreen quad, no new geometry, no FBO readback, 3-octave
caustics, ≤4 texture taps for the trail gradient + 3 for the RGB split. It reuses `uScene`
(composite) and `uTrail` you already produce. The `dFdx/dFdy` area-ratio caustics from the
literature are more physical but cost derivatives on the whole quad; the interfering-sine version
above is the standard cheap substitute
([Renou](https://medium.com/@martinRenou/real-time-rendering-of-water-caustics-59cda1d74aa),
[Evan Wallace](https://medium.com/@evanwallace/rendering-realtime-caustics-in-webgl-2a99a29a0b2c)).

### 1.3 Driving it — the JS (self-contained, replaces GSAP)

```js
const easeInOutCubic = k => k<.5 ? 4*k*k*k : 1 - Math.pow(-2*k+2,3)/2;

let dive = 0, diveTarget = 0, tweening = false;
function tweenDive(to, ms, done){
  const from = dive, t0 = performance.now(); diveTarget = to; tweening = true;
  (function loop(now){
    const k = Math.min(1,(now-t0)/ms);
    dive = from + (to-from)*easeInOutCubic(k);            // set uDive uniform in render()
    if(k<1) requestAnimationFrame(loop); else { tweening=false; done&&done(); }
  })(performance.now());
}

viewMenuBtn.addEventListener('click', (e)=>{
  const r = canvas.getBoundingClientRect();
  uClick = [ (e.clientX-r.left)/r.width, 1 - (e.clientY-r.top)/r.height ]; // y-up UV
  energyBoost = 1.6;                                      // ramp Gerstner amplitude/chop
  document.body.classList.add('diving');                 // triggers CSS menu rise (below)
  tweenDive(1, 1600, ()=> menu.scrollIntoView({behavior:'auto'}));
});
```

The render loop each frame sets `uDive = dive`, `uClick = uClick`, and eases `energyBoost` back to
1 (`energyBoost += (1-energyBoost)*0.03`) so the sea calms after the plunge.

### 1.4 Menu entrance + surface color grade (CSS, compositor-only)

```css
.menu-sheet{
  transform: translateY(8vh); opacity: 0; filter: blur(12px);
  transition: transform .7s cubic-bezier(.16,1,.3,1),   /* power3.out "surfacing" */
              opacity  .5s ease-out,
              filter   .6s ease-out;
  will-change: transform, opacity, filter;
}
body.diving .menu-sheet{ transform: none; opacity: 1; filter: none; transition-delay: .9s; }

/* a warm→deep→warm scrim over the DOM, synced to the shader's submerge beat */
.dive-scrim{ position:fixed; inset:0; pointer-events:none; opacity:0;
  background: radial-gradient(120% 120% at 50% 60%, #2a120a 0%, #0b0503 70%); }
body.diving .dive-scrim{ animation: submerge 1.6s ease both; }
@keyframes submerge{ 0%{opacity:0} 45%{opacity:.55} 62%{opacity:.7} 100%{opacity:0} }
```

Transform / opacity / filter are compositor-friendly, so the DOM half never blocks the GL frame.

### 1.5 Reverse on scroll-up (scrubbed)

Because the whole effect is a pure function of `uDive`, wire it to scroll when the menu is up and
the user returns toward the top:

```js
addEventListener('scroll', ()=>{
  if(tweening) return;                                   // don't fight an active tween
  // menuTop = px scrolled into the menu; map the last ~1 viewport to uDive 1..0
  const p = Math.min(1, Math.max(0, menu.scrollTop / innerHeight));
  dive = p;                                              // scrub: near top -> re-submerge -> hero
  document.body.classList.toggle('diving', dive > 0.05);
});
```

Scrolling back up re-runs C→A in reverse: the menu sinks (`translateY` grows), the membrane
re-forms, caustics fade, the hero re-emerges. For a discrete "scroll-up snaps back to hero" gesture,
call `tweenDive(0, 1200)` instead.

### 1.6 Graceful fallback + `prefers-reduced-motion`

```js
const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
const gl2ok  = !!canvas.getContext('webgl2');

if(reduce || !gl2ok){
  viewMenuBtn.addEventListener('click', ()=>{
    // no plunge: 300ms opacity crossfade, sea held static, then reveal menu
    document.body.classList.add('reduced');   // CSS: .reduced .menu-sheet{transition:opacity .3s}
    menu.scrollIntoView({behavior: reduce ? 'auto' : 'smooth'});
  }, {capture:true});
}
```

```css
@media (prefers-reduced-motion: reduce){
  .menu-sheet{ transform:none; filter:none; transition: opacity .3s linear; }
  .dive-scrim{ display:none; }
}
```

Reduced-motion users get a plain fade (the vestibular-safe choice per
[Piccalilli](https://piccalil.li/blog/some-practical-examples-of-view-transitions-to-elevate-your-ui/)
and [Smashing](https://www.smashingmagazine.com/2021/10/respecting-users-motion-preferences/)); if
WebGL2 or the context is lost, the `.diving` class is never added, `uDive` stays 0, and the CTA
degrades to the existing scroll. Also register a `webglcontextlost` guard that pins `uDive=0`.

---

## 2. Stylized procedural delivery map (Canvas2D, ETA-driven)

Goal: a delivery-tracking view that reads as a **map** — streets, a route, a moving courier, a
destination pin — but rendered in the **generative-sea aesthetic** (dark warm ground, additive
glow, gentle aquatic shimmer), fully **procedural** (no tiles, no network), and driven by a single
**`eta ∈ [0,1]`** progress value. This mirrors Just Eat Takeaway's redesign findings: put ETA/status
prominent at the top, keep the map/animated illustration central, show a *predicted route* not just
a dot, and cut clutter — "animation conveys progress emotionally"
([JET UX](https://medium.com/jetux/redesigning-our-global-order-tracking-experience-1f0fd7c91418)).

### 2.1 Determinism (same address → same city)

```js
function hashStr(s){ let h=2166136261; for(let i=0;i<s.length;i++){ h^=s.charCodeAt(i); h=Math.imul(h,16777619);} return h>>>0; }
function mulberry32(a){ return ()=>{ a|=0; a=a+0x6D2B79F5|0; let t=Math.imul(a^a>>>15,1|a);
  t=t+Math.imul(t^t>>>7,61|t)^t; return((t^t>>>14)>>>0)/4294967296; }; }
const rnd = mulberry32(hashStr(addressString));       // deterministic per address
```

Seeding from the address hash means the generated "city" is stable across reloads/sessions — the
deterministic tensor-field philosophy of ProbableTrain's
[City Generator](https://probabletrain.itch.io/city-generator), reduced to what we actually need.

### 2.2 Street grid — a rotated, jittered grid + a few boulevards

Two street classes. **Minor:** a grid rotated by a small deterministic angle, each line given a
low-frequency sinusoidal wobble so nothing is dead-straight. **Major:** 2–3 curved "boulevards" as
gentle beziers. (Perlin/tiling/jitter are the standard cheap procedural street primitives per the
[procedural-city survey](https://www.citygen.net/files/images/Procedural_City_Generation_Survey.pdf).)

```js
function buildStreets(w,h){
  const ang = (rnd()-0.5)*0.5;                          // city rotation ±0.25 rad
  const gap = 46 + rnd()*22;                            // block size
  const streets = [];
  const rot = (x,y)=>({x:(x-w/2)*Math.cos(ang)-(y-h/2)*Math.sin(ang)+w/2,
                       y:(x-w/2)*Math.sin(ang)+(y-h/2)*Math.cos(ang)+h/2});
  for(let gx=-2; gx<w+gap; gx+=gap){                    // verticals
    const pts=[], j=rnd()*6.28, amp=3+rnd()*6;
    for(let y=-10;y<=h+10;y+=18) pts.push(rot(gx+Math.sin(y*0.02+j)*amp, y));
    streets.push({pts, major: rnd()<0.18});
  }
  for(let gy=-2; gy<h+gap; gy+=gap){                    // horizontals
    const pts=[], j=rnd()*6.28, amp=3+rnd()*6;
    for(let x=-10;x<=w+10;x+=18) pts.push(rot(x, gy+Math.sin(x*0.02+j)*amp));
    streets.push({pts, major: rnd()<0.18});
  }
  return streets;
}
```

### 2.3 Palette + the additive-glow line (of-a-piece with the ocean)

Use `globalCompositeOperation='lighter'` — the **same additive blending** as the sea's particles —
so streets read as light on dark water, not ink on paper. Two-pass stroke = wide dim halo + thin
bright core = the "soft glowing line" look.

```js
const PAL = {
  bg0:'#160b06', bg1:'#0a0503',                         // deep espresso ground
  minorHalo:'rgba(198,98,58,0.10)',  minorCore:'rgba(226,150,92,0.34)',   // terracotta
  majorHalo:'rgba(240,180,110,0.14)',majorCore:'rgba(248,206,140,0.62)',  // gold
  route:'#ffcf8a', courier:'#ffe0b0', pin:'#ff7a45', label:'#f3e2cf'
};
function glowLine(ctx, pts, halo, core, wHalo, wCore){
  ctx.globalCompositeOperation='lighter'; ctx.lineJoin='round'; ctx.lineCap='round';
  ctx.beginPath(); ctx.moveTo(pts[0].x,pts[0].y);
  for(let i=1;i<pts.length;i++) ctx.lineTo(pts[i].x,pts[i].y);
  ctx.lineWidth=wHalo; ctx.strokeStyle=halo; ctx.stroke();     // pass 1: halo
  ctx.lineWidth=wCore; ctx.strokeStyle=core; ctx.stroke();     // pass 2: core
}
```

**Cache the static grid once** to an offscreen canvas (it's deterministic — draw it a single time),
then each frame only redraw the dynamic layer (route progress, courier, pin, shimmer). This is what
keeps the map at 60 fps.

### 2.4 The route — a bezier from restaurant → address

Canvas2D has no `getPointAtLength`, so evaluate the cubic parametrically
([MDN bezierCurveTo](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/bezierCurveTo)).
The two control points are pushed perpendicular to the straight line for a graceful arc.

```js
function bez(p0,p1,p2,p3,t){ const u=1-t,uu=u*u,uuu=uu*u,tt=t*t,ttt=tt*t;
  return { x: uuu*p0.x+3*uu*t*p1.x+3*u*tt*p2.x+ttt*p3.x,
           y: uuu*p0.y+3*uu*t*p1.y+3*u*tt*p2.y+ttt*p3.y }; }
function bezTan(p0,p1,p2,p3,t){ const u=1-t;              // heading, for courier orientation
  return { x:3*u*u*(p1.x-p0.x)+6*u*t*(p2.x-p1.x)+3*t*t*(p3.x-p2.x),
           y:3*u*u*(p1.y-p0.y)+6*u*t*(p2.y-p1.y)+3*t*t*(p3.y-p2.y) }; }

// restaurant fixed lower-left; destination deterministic upper-right quadrant
const A = {x:w*0.22, y:h*0.74};
const B = {x:w*(0.6+rnd()*0.22), y:h*(0.2+rnd()*0.18)};
const mx=(A.x+B.x)/2, my=(A.y+B.y)/2, dx=B.x-A.x, dy=B.y-A.y, len=Math.hypot(dx,dy);
const nx=-dy/len, ny=dx/len, bow=len*0.28;               // perpendicular bow
const C1={x:mx+nx*bow*0.6-dx*0.15, y:my+ny*bow*0.6-dy*0.15};
const C2={x:mx+nx*bow      +dx*0.15, y:my+ny*bow      +dy*0.15};
```

Draw the route in two states: a **dim full-length guide** (0→1) plus the **bright traveled
portion** (0→`eta`) — the "route prediction" JET recommends.

```js
function drawRoute(ctx, eta){
  const guide=[], trav=[]; const N=64;
  for(let i=0;i<=N;i++){ const t=i/N, p=bez(A,C1,C2,B,t); guide.push(p); if(t<=eta) trav.push(p); }
  glowLine(ctx, guide, 'rgba(255,207,138,0.05)','rgba(255,207,138,0.18)', 10, 2); // dashed-feel guide
  if(trav.length>1) glowLine(ctx, trav, 'rgba(255,207,138,0.20)', PAL.route, 12, 2.6);
}
```

### 2.5 Courier marker advancing with ETA + pulsing pin

```js
function drawCourier(ctx, eta, time){
  // comet trail: a few samples behind the head, fading
  for(let k=6;k>=1;k--){ const t=Math.max(0,eta-k*0.012), p=bez(A,C1,C2,B,t);
    ctx.globalCompositeOperation='lighter'; ctx.beginPath();
    ctx.arc(p.x,p.y, 3.2-k*0.3, 0,6.28);
    ctx.fillStyle=`rgba(255,224,176,${0.06*(7-k)})`; ctx.fill(); }
  const p = bez(A,C1,C2,B,eta), tan = bezTan(A,C1,C2,B,eta);
  const ang = Math.atan2(tan.y,tan.x);
  // pulsing halo ring
  const pr = 8 + 3*Math.sin(time*3);
  const g = ctx.createRadialGradient(p.x,p.y,0,p.x,p.y,pr*2);
  g.addColorStop(0,'rgba(255,224,176,0.5)'); g.addColorStop(1,'rgba(255,224,176,0)');
  ctx.fillStyle=g; ctx.beginPath(); ctx.arc(p.x,p.y,pr*2,0,6.28); ctx.fill();
  // head + heading chevron
  ctx.fillStyle=PAL.courier; ctx.beginPath(); ctx.arc(p.x,p.y,4,0,6.28); ctx.fill();
  ctx.save(); ctx.translate(p.x,p.y); ctx.rotate(ang);
  ctx.beginPath(); ctx.moveTo(6,0); ctx.lineTo(1,-2.4); ctx.lineTo(1,2.4); ctx.closePath();
  ctx.fillStyle=PAL.courier; ctx.fill(); ctx.restore();
  return p;
}
function drawPin(ctx, time){
  for(let i=0;i<3;i++){                                    // expanding sonar rings
    const ph=(time*0.6+i/3)%1, r=6+ph*26;
    ctx.globalCompositeOperation='lighter'; ctx.beginPath(); ctx.arc(B.x,B.y,r,0,6.28);
    ctx.strokeStyle=`rgba(255,122,69,${0.35*(1-ph)})`; ctx.lineWidth=2; ctx.stroke();
  }
  ctx.fillStyle=PAL.pin; ctx.beginPath(); ctx.arc(B.x,B.y,5,0,6.28); ctx.fill();
}
```

### 2.6 Binding to the ETA parameter + smoothing

`eta` is the order progress, computed from the tracking state
(`eta = 1 - minutesRemaining/minutesTotal`, clamped 0..1). Lerp the *displayed* value toward the
target so status jumps glide rather than snap:

```js
let shownEta = 0;
function frame(now){
  const time = now/1000;
  shownEta += (targetEta - shownEta) * 0.05;             // smooth catch-up
  ctx.globalCompositeOperation='source-over';
  ctx.drawImage(streetCache, 0, 0);                       // cached static grid
  drawRoute(ctx, shownEta);
  const cp = drawCourier(ctx, shownEta, time);
  drawPin(ctx, time);
  seaShimmer(ctx, time);                                  // §2.7
  positionLabels(cp);                                     // §2.8 (DOM labels follow courier/pin)
  if(shownEta > 0.995) arrivalBurst(ctx, time);           // pin flares on arrival
  requestAnimationFrame(frame);
}
```

When `targetEta` reaches 1 the courier lands on the pin and `arrivalBurst` flares — the emotional
"it's here" beat.

### 2.7 Aquatic shimmer (ties the map to the sea)

Two cheap tricks give streets the sea's living quality without re-stroking every line each frame:

1. **Flowing light along roads** — animate `lineDashOffset` on a sparse set of bright dashes over the
   major boulevards so light *streams* down them like a current.
2. **Breathing drift** — translate the whole dynamic layer by `sin(time*0.3)*2px` vertically and
   pass a couple of slow, wide additive radial gradients across the canvas (a "caustic" wash) so
   brightness undulates like the ocean composite above it.

```js
function seaShimmer(ctx,t){
  ctx.globalCompositeOperation='lighter';
  for(let i=0;i<2;i++){
    const x=(0.5+0.5*Math.sin(t*0.2+i*2))*w, y=(0.5+0.5*Math.cos(t*0.16+i))*h;
    const g=ctx.createRadialGradient(x,y,0,x,y,w*0.5);
    g.addColorStop(0,'rgba(255,190,120,0.05)'); g.addColorStop(1,'rgba(255,190,120,0)');
    ctx.fillStyle=g; ctx.fillRect(0,0,w,h);
  }
}
```

Optional deeper tie-in: render this map canvas **over the live sea composite** at ~0.9 opacity so
the actual ocean breathes faintly through the streets — the map literally floats on the sea.

### 2.8 Address / courier label styling

Per JET, ETA/status live prominently at the top; the map's own labels stay minimal to avoid clutter.
Two small floating plates, positioned in the DOM to follow the courier head and the pin:

```css
.map-label{
  position:absolute; transform:translate(-50%,-140%);
  font: 600 11px/1.3 ui-monospace, "SFMono-Regular", monospace;
  letter-spacing:.08em; text-transform:uppercase; color:#f3e2cf;
  padding:6px 9px; border-radius:8px;
  background:rgba(20,10,7,0.55); backdrop-filter:blur(6px);
  border:1px solid rgba(240,180,110,0.25);
  box-shadow:0 4px 18px rgba(0,0,0,.4), 0 0 12px rgba(255,140,80,.15);
  white-space:nowrap; pointer-events:none;
}
.map-label::after{                             /* leader line down to the point */
  content:""; position:absolute; left:50%; top:100%; width:1px; height:12px;
  background:linear-gradient(rgba(240,180,110,.6), transparent);
}
.map-label .eta{ font-size:15px; letter-spacing:0; color:#ffcf8a; }   /* big number */
.map-label--dest{ transform:translate(-50%,-160%); }
```

Content: courier plate → `MARCO · 6 MIN` (name + `<span class="eta">` for the number); destination
plate → the street line of `addressString`, uppercased, one line. Warm cream on a blurred smoked
plate with a hairline gold border keeps it unmistakably part of the Artë Pasta world.

---

## 3. Full-screen, no vignette, not-top-down framing

Three changes to the field mapping + composite make the sea *envelop* the viewer instead of reading
as a flat top-down plane in a dark box.

### 3.1 Kill the dark corners

- **Remove the radial vignette multiply** in the composite pass (`col *= smoothstep(radius, 0.0, dist_from_center)` or similar). If you want edge shaping at all, *lift* the edges warm rather than darken them.
- **Set the clear color to the warm deep base** (`gl.clearColor(0.055,0.035,0.03,1)`), never black — any pixel the field doesn't cover then reads as deep water, not void.
- **Size the canvas edge-to-edge with viewport units that survive mobile chrome:**
  ```css
  #sea{ position:fixed; inset:0; width:100vw; height:100dvh; height:100svh; display:block; }
  ```
  and size the drawing buffer to `Math.round(innerWidth*dpr) × Math.round(innerHeight*dpr)` on
  resize, `dpr = Math.min(devicePixelRatio, 2)`. Letterbox black corners are almost always an
  aspect/vignette artifact, not the shader.

### 3.2 Give the field a horizon + fake perspective (stop it reading top-down)

Instead of sampling the Gerstner sum in linear screen UV, project screen-Y onto a **ground plane**
so wavelength foreshortens toward a horizon — the standard "fake perspective ocean" that makes a 2D
field feel like you're *on* the water, not above it. FBM/multi-octave detail then never repeats
([three.js ocean](https://threejs.org/examples/webgl_shaders_ocean.html),
[jbouny/ocean](https://github.com/jbouny/ocean)).

```glsl
// vUv in [0,1]; horizon a bit above center so we look "across" the sea
const float HORIZON = 0.62;
float toHorizon = HORIZON - vUv.y;                 // >0 below horizon (visible water)
float depth = 1.0 / max(0.05, toHorizon);          // grows toward the horizon line
vec2  world = vec2( (vUv.x-0.5) * depth, depth + uTime*0.15 );  // ground-plane coords + drift
// sample your existing Gerstner/dispersive sum in `world` instead of vUv:
float h = gerstnerSum(world);
```

Then blend the far water into a **warm haze/sky** so the plane's far edge is never visible:

```glsl
vec3 haze = mix(vec3(0.9,0.55,0.3), vec3(0.18,0.09,0.06), smoothstep(0.0,0.25, vUv.y-HORIZON));
float fog = smoothstep(0.0, 0.14, toHorizon);      // 0 at horizon -> 1 nearer camera
vec3 sea  = mix(haze, seaColor(h, world), fog);    // waves dissolve into haze up top
```

Above `HORIZON`, draw the haze gradient directly (warm gold at the horizon line → deeper terracotta
toward the top) so the upper screen is atmosphere, not a hard plane boundary.

### 3.3 Depth cues that sell "you're in it"

- **Two-layer parallax:** sum a *far* compressed field and a *near* large-amplitude field with
  slightly opposing horizontal drift; the disparity reads as depth.
- **Foreground-biased particles:** scale additive point size/speed by `depth` (bigger, faster near
  the bottom = closer to you) so the spray belongs to a 3D volume.
- **Gentle camera bob:** offset `HORIZON` by `sin(uTime*0.2)*0.01` and add a whisper of the §1
  chromatic split at the very edges — the frame breathes like a held breath underwater.
- **Warm rim, not dark vignette:** a soft bottom glow (sun on the near water) plus the horizon haze
  replaces corner darkening — the composition stays bright edge-to-edge while still having a focal
  center.

Net effect: no black corners, a real horizon, foreshortened waves, and parallax — the sea now
surrounds the hero rather than sitting under it like a tabletop.

---

## Sources

- Active Theory / Hydra scene transitions — [Webby: Active Theory](https://www.webbyawards.com/crafted-with-code/active-theory/)
- Click ripple reveal, `uMouse`/`uRippleProgress` keyframes, GSAP timeline — [Codrops: Animate WebGL Shaders with GSAP — Ripples, Reveals, Blur](https://tympanus.net/codrops/2025/10/08/how-to-animate-webgl-shaders-with-gsap-ripples-reveals-and-dynamic-blur-effects/)
- Fullscreen plane, `progress` uniform, UV-displacement + `mix()` transitions — [Codrops: Creative WebGL Image Transitions](https://tympanus.net/codrops/2019/11/05/creative-webgl-image-transitions/)
- Refraction/lens & wavy sine-refraction shaders — [Codrops: Progressively Enhanced WebGL Lens Refraction](https://tympanus.net/codrops/2023/10/10/progressively-enhanced-webgl-lens-refraction/), [Codrops: Dissecting a Wavy Shader](https://tympanus.net/codrops/2025/10/25/dissecting-a-wavy-shader-sine-refraction-and-serendipity/)
- Real-time caustics (area ratio, procedural) — [Martin Renou](https://medium.com/@martinRenou/real-time-rendering-of-water-caustics-59cda1d74aa), [Evan Wallace](https://medium.com/@evanwallace/rendering-realtime-caustics-in-webgl-2a99a29a0b2c), [Maxime Heckel](https://blog.maximeheckel.com/posts/caustics-in-webgl/)
- Beer–Lambert absorption / underwater tint (`exp(-thickness*absorption)`, red-first absorption) — [gameidea: Stylized 3D Water Shader](https://gameidea.org/2026/02/01/creating-a-stylized-3d-water-shader/)
- Immersive ocean / horizon / FBM (avoid flat top-down) — [three.js Ocean example](https://threejs.org/examples/webgl_shaders_ocean.html), [jbouny/ocean](https://github.com/jbouny/ocean)
- Order-tracking UX (ETA top, route prediction, de-clutter, animation-as-progress) — [Just Eat Takeaway UX: Redesigning Order Tracking](https://medium.com/jetux/redesigning-our-global-order-tracking-experience-1f0fd7c91418)
- Deterministic procedural streets (tensor fields / Perlin / tiling) — [ProbableTrain City Generator](https://probabletrain.itch.io/city-generator), [Procedural City Generation Survey (PDF)](https://www.citygen.net/files/images/Procedural_City_Generation_Survey.pdf)
- Cubic bezier point/tangent in Canvas2D (no native `getPointAtLength`) — [MDN: bezierCurveTo](https://developer.mozilla.org/en-US/docs/Web/API/CanvasRenderingContext2D/bezierCurveTo)
- Reduced-motion = crossfade fallback, opt-in motion-safe — [MDN: prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion), [Smashing: Respecting Motion Preferences](https://www.smashingmagazine.com/2021/10/respecting-users-motion-preferences/), [Piccalilli: View Transitions](https://piccalil.li/blog/some-practical-examples-of-view-transitions-to-elevate-your-ui/)
