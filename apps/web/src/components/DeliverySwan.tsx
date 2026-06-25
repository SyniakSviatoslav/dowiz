import React from 'react';

/**
 * DeliverySwan — dowiz's signature mark reimagined as a COURIER. The authored
 * line-art swan (geometry borrowed verbatim from SwanHero's dz-swan-* paths)
 * carries a small inked parcel slung beneath its body, gliding across the hero
 * "sky" on a gentle eased arc with a slight bank/rotate. On entry it self-draws,
 * leaves a brief motion-line trail (speed shown as smears, not blur — the brief),
 * then settles into an ambient glide; the parcel sways with weight (Ghibli
 * ease-in-out, no bounce). Two soft paper clouds drift behind it for depth.
 *
 * Pure SVG + CSS. Every colour is a brand token (--ink / --brand-*), so it
 * re-skins with the active theme; the limited paper palette is preserved.
 * Decorative → aria-hidden. Layered with pointer-events:none so it never blocks
 * the live PaperScene canvas underneath, at z-index:4 so it clears the
 * ArtNouveauFrame's internal content layer (z-index:3) and composites above the
 * WebGL canvas. All motion is transform/opacity (GPU) and gated behind
 * prefers-reduced-motion → a still, fully-drawn perched swan, clouds at rest.
 *
 * Motion source: design-system tokens (--ease-out / --ease-in-out / --motion-*)
 * from @deliveryos/ui theme/tokens.css. No raw cubic-beziers inlined.
 */
export function DeliverySwan() {
  return (
    <div className="dz-dswan" aria-hidden="true">
      <style>{DSWAN_CSS}</style>

      {/* Atmosphere: two soft paper clouds drifting at different depths (parallax). */}
      <div className="dz-dswan-clouds">
        <svg className="dz-dswan-cloud c-far" viewBox="0 0 120 40" role="presentation" focusable="false">
          <path d="M8 30 C 8 18, 26 16, 32 24 C 38 12, 62 12, 66 24 C 80 18, 94 24, 90 30 Z" />
        </svg>
        <svg className="dz-dswan-cloud c-near" viewBox="0 0 120 40" role="presentation" focusable="false">
          <path d="M6 30 C 6 20, 22 18, 30 25 C 36 14, 58 14, 64 25 C 78 20, 92 25, 88 30 Z" />
        </svg>
      </div>

      {/* dz-dswan-fly: the arc travel + bank. Inner svg holds the bob + draw. */}
      <div className="dz-dswan-fly">
        <svg className="dz-dswan-svg" viewBox="44 46 150 142" role="presentation" focusable="false">
          <g className="dz-dswan-bob">
            {/* Motion-line speed trail behind the swan — short ink dashes that
                show during the fly-in then fade as it settles into the glide. */}
            <g className="dz-dswan-speed">
              <line x1="40" y1="92" x2="20" y2="92" />
              <line x1="44" y1="106" x2="18" y2="106" />
              <line x1="48" y1="120" x2="26" y2="120" />
            </g>

            {/* Parcel slung beneath the body: a tiny inked box on a sling line,
                hung from the swan's underside. Swings from the sling anchor. */}
            <g className="dz-dswan-parcel">
              {/* sling cords from underside of body down to the box */}
              <line className="dz-dswan-cord" x1="86" y1="134" x2="92" y2="156" />
              <line className="dz-dswan-cord" x1="110" y1="134" x2="104" y2="156" />
              {/* the parcel box + tie */}
              <rect className="dz-dswan-box" x="84" y="155" width="28" height="22" rx="3" />
              <line className="dz-dswan-tie" x1="98" y1="155" x2="98" y2="177" />
            </g>

            {/* Released parcel: periodically the swan lets one go — it drifts
                down on a soft arc and fades as it's "delivered", then a new one
                appears at the sling. Pure transform/opacity, motion-gated. */}
            <g className="dz-dswan-drop">
              <rect className="dz-dswan-box" x="85" y="156" width="24" height="19" rx="3" />
              <line className="dz-dswan-tie" x1="97" y1="156" x2="97" y2="175" />
            </g>

            {/* The swan — body + S-neck + head + beak + eye + wing.
                Geometry matches SwanHero's authored dz-swan paths. */}
            <g className="dz-dswan-bird">
              <path className="dz-dswan-line dz-dswan-draw" pathLength={1}
                d="M52 138 C 48 112, 74 100, 104 102 C 130 104, 142 116, 140 132" />
              <path className="dz-dswan-line dz-dswan-draw d2" pathLength={1}
                d="M136 132 C 124 110, 148 102, 150 84 C 151 70, 147 60, 159 57" />
              <path className="dz-dswan-line dz-dswan-wing" pathLength={1}
                d="M70 132 C 82 112, 108 108, 126 122" />
              <polygon className="dz-dswan-beak" points="166,56 182,61 166,64" />
              <circle className="dz-dswan-line dz-dswan-head" cx="160" cy="57" r="7" />
              <circle className="dz-dswan-eye" cx="162" cy="55" r="1.6" />
            </g>
          </g>
        </svg>
      </div>
    </div>
  );
}

const DSWAN_CSS = `
/* z-index:4 clears the ArtNouveauFrame's internal content layer (z-index:3) so the
   swan + clouds composite ABOVE the WebGL canvas; pointer-events:none keeps drag alive. */
.dz-dswan{position:absolute;inset:0;pointer-events:none;z-index:4;overflow:hidden}

/* ── Atmosphere: soft paper clouds ── */
.dz-dswan-clouds{position:absolute;inset:0;overflow:hidden}
.dz-dswan-cloud{position:absolute;display:block;height:auto;fill:var(--paper-raised,var(--brand-surface-raised))}
.dz-dswan-cloud.c-far{top:10%;width:30%;max-width:120px;opacity:.45}
.dz-dswan-cloud.c-near{top:24%;width:24%;max-width:96px;opacity:.6}

/* ── The flying swan ── */
.dz-dswan-fly{position:absolute;top:5%;left:2%;width:42%;max-width:172px}
.dz-dswan-svg{width:100%;height:auto;display:block;overflow:visible}

.dz-dswan-line{fill:none;stroke:var(--ink,var(--brand-text));stroke-width:5;
  stroke-linecap:round;stroke-linejoin:round}
.dz-dswan-head{fill:var(--paper-surface,var(--brand-bg))}
.dz-dswan-beak{fill:var(--brand-primary)}
.dz-dswan-eye{fill:var(--ink,var(--brand-text))}
.dz-dswan-wing{opacity:.55}

/* parcel: inked box hung on cords */
.dz-dswan-cord{stroke:var(--ink,var(--brand-text));stroke-width:2;stroke-linecap:round;opacity:.7}
.dz-dswan-box{fill:var(--paper-raised,var(--brand-surface-raised));
  stroke:var(--ink,var(--brand-text));stroke-width:3}
.dz-dswan-tie{stroke:var(--brand-primary);stroke-width:2.5;stroke-linecap:round}
.dz-dswan-parcel{transform-box:fill-box;transform-origin:98px 134px}

/* motion-line speed trail */
.dz-dswan-speed line{stroke:var(--ink,var(--brand-text));stroke-width:3;stroke-linecap:round;opacity:0}

/* released parcel (the "delivery" beat) — hidden at rest */
.dz-dswan-drop{transform-box:fill-box;transform-origin:97px 165px;opacity:0}

/* Default (reduced-motion-safe): perched, fully drawn, parcel at rest, no trail,
   clouds static, swan settled in the upper-left sky clear of the sun. */
.dz-dswan-fly{transform:translate(2%,2%)}
.dz-dswan-draw{stroke-dasharray:1;stroke-dashoffset:0}
.dz-dswan-wing{stroke-dasharray:1;stroke-dashoffset:0}

@media (prefers-reduced-motion: no-preference){
  /* self-draw the body & neck on entry, then the wing feathers in */
  .dz-dswan-draw{stroke-dashoffset:1;animation:dzDSwanDraw 1.5s var(--ease-out,cubic-bezier(.16,1,.3,1)) .9s forwards}
  .dz-dswan-draw.d2{animation-delay:1.12s}
  .dz-dswan-wing{stroke-dashoffset:1;animation:dzDSwanDraw .6s var(--ease-out,cubic-bezier(.16,1,.3,1)) 1.55s forwards}

  /* fly in from the left edge, then settle into a slow ambient glide arc that
     stays in the upper-left sky (clear of the sun). ease-out entrance,
     ease-in-out for the looping glide so it breathes naturally. */
  .dz-dswan-fly{
    opacity:0;
    transform:translate(-50%,12%) rotate(-7deg);
    animation:
      dzDSwanEnter 1.6s var(--ease-out,cubic-bezier(.16,1,.3,1)) .75s forwards,
      dzDSwanGlide 14s var(--ease-in-out,cubic-bezier(.65,0,.35,1)) 2.35s infinite;
  }
  /* gentle vertical bob of the whole rig (swan + parcel) */
  .dz-dswan-bob{transform-box:fill-box;transform-origin:center;
    animation:dzDSwanBob 5.5s var(--ease-in-out,cubic-bezier(.65,0,.35,1)) 2.35s infinite}
  /* the parcel sways with weight — pendulum from the sling anchor, no bounce */
  .dz-dswan-parcel{animation:dzDSwanSway 4.2s var(--ease-in-out,cubic-bezier(.65,0,.35,1)) 2.1s infinite}
  /* speed trail flashes in during the fly-in, then fades for the calm glide */
  .dz-dswan-speed line{animation:dzDSwanTrail 1.9s var(--ease-out,cubic-bezier(.16,1,.3,1)) .75s forwards}
  .dz-dswan-speed line:nth-child(2){animation-delay:.82s}
  .dz-dswan-speed line:nth-child(3){animation-delay:.9s}

  /* clouds drift slowly across the sky at different speeds (parallax depth) */
  .dz-dswan-cloud.c-far{left:-32%;animation:dzDSwanCloud 64s linear infinite}
  .dz-dswan-cloud.c-near{left:-26%;animation:dzDSwanCloud 44s linear 6s infinite}

  /* the delivery beat: release a parcel that drifts down + away and fades, on a
     slow loop offset from the glide so it reads as an occasional drop-off */
  .dz-dswan-drop{animation:dzDSwanDrop 11s var(--ease-in-out,cubic-bezier(.65,0,.35,1)) 5s infinite}
}

/* delivery: arrive from the left and bank level into the upper-left sky */
@keyframes dzDSwanEnter{
  0%{opacity:0;transform:translate(-50%,12%) rotate(-7deg)}
  60%{opacity:1}
  100%{opacity:1;transform:translate(2%,2%) rotate(-2deg)}
}
/* ambient glide: a shallow arc kept to the left of the sun, banking with travel */
@keyframes dzDSwanGlide{
  0%{transform:translate(2%,2%) rotate(-2deg)}
  50%{transform:translate(20%,-4%) rotate(1deg)}
  100%{transform:translate(2%,2%) rotate(-2deg)}
}
@keyframes dzDSwanBob{
  0%,100%{transform:translateY(0)}
  50%{transform:translateY(-3.5px)}
}
/* pendulum sway of the slung parcel — eased, no bounce */
@keyframes dzDSwanSway{
  0%,100%{transform:rotate(-5deg)}
  50%{transform:rotate(5deg)}
}
/* self-draw the ink line: pathLength=1 → reveal the stroke from hidden to full */
@keyframes dzDSwanDraw{to{stroke-dashoffset:0}}
/* speed trail: dashes streak in behind the swan, then fade */
@keyframes dzDSwanTrail{0%{opacity:0;transform:translateX(10px)}35%{opacity:.5}100%{opacity:0;transform:translateX(-4px)}}
/* clouds drift left→right across and beyond the panel, then loop */
@keyframes dzDSwanCloud{from{transform:translateX(0)}to{transform:translateX(520%)}}
/* released parcel: appears at the sling, drifts down + slightly right on a soft
   arc with a little tumble, then fades as it's delivered. One beat per loop. */
@keyframes dzDSwanDrop{
  0%{opacity:0;transform:translate(0,0) rotate(-4deg)}
  10%{opacity:1;transform:translate(2px,6px) rotate(2deg)}
  46%{opacity:1;transform:translate(14px,86px) rotate(20deg)}
  66%{opacity:0;transform:translate(22px,128px) rotate(30deg)}
  100%{opacity:0;transform:translate(22px,128px) rotate(30deg)}
}
`;
