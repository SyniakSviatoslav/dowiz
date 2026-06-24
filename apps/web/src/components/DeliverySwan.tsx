import React from 'react';

/**
 * DeliverySwan — dowiz's signature mark reimagined as a COURIER. The authored
 * line-art swan (geometry borrowed verbatim from SwanHero's dz-swan-* paths)
 * carries a small inked parcel slung beneath its body, gliding across the hero
 * "sky" on a gentle eased arc with a slight bank/rotate. On entry it self-draws
 * (stroke-dasharray, like dz-draw), then settles into an ambient glide loop; the
 * parcel sways with weight (Ghibli ease-in-out, no bounce).
 *
 * Pure SVG + CSS. Every colour is a brand token (--ink / --brand-*), so it
 * re-skins with the active theme; the limited paper palette is preserved.
 * Decorative → aria-hidden. Layered with pointer-events:none so it never blocks
 * the live PaperScene canvas underneath. All motion is transform/opacity (GPU)
 * and gated behind prefers-reduced-motion → a still, fully-drawn perched swan.
 *
 * Motion source: design-system tokens (--ease-out / --ease-in-out / --ease-soft
 * / --motion-base) from @deliveryos/ui theme/tokens.css. No raw cubic-beziers
 * inlined — the keyframes reference the token curves.
 */
export function DeliverySwan() {
  return (
    <div className="dz-dswan" aria-hidden="true">
      <style>{DSWAN_CSS}</style>
      {/* dz-dswan-fly: the arc travel + bank. Inner svg holds the bob + draw. */}
      <div className="dz-dswan-fly">
        <svg className="dz-dswan-svg" viewBox="44 46 150 142" role="presentation" focusable="false">
          <g className="dz-dswan-bob">
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
.dz-dswan{position:absolute;inset:0;pointer-events:none;z-index:2;overflow:hidden}
.dz-dswan-fly{position:absolute;top:4%;left:3%;width:52%;max-width:220px}
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

/* Default (reduced-motion-safe): perched, fully drawn, parcel at rest.
   stroke fully visible; no flight, no draw-in, no sway. */
.dz-dswan-fly{transform:translate(8%,2%)}
.dz-dswan-draw{stroke-dasharray:1;stroke-dashoffset:0}
.dz-dswan-wing{stroke-dasharray:1;stroke-dashoffset:0}

@media (prefers-reduced-motion: no-preference){
  /* self-draw the body & neck on entry, then the wing feathers in */
  .dz-dswan-draw{stroke-dashoffset:1;animation:dzDSwanDraw 1.5s var(--ease-out,cubic-bezier(.16,1,.3,1)) .9s forwards}
  .dz-dswan-draw.d2{animation-delay:1.12s}
  .dz-dswan-wing{stroke-dashoffset:1;animation:dzDSwanDraw .6s var(--ease-out,cubic-bezier(.16,1,.3,1)) 1.55s forwards}

  /* fly in from the left edge, then settle into a slow ambient glide arc
     across the sky with a gentle bank. ease-out for the entrance delivery,
     ease-in-out for the looping glide so it breathes naturally. */
  .dz-dswan-fly{
    opacity:0;
    transform:translate(-46%,14%) rotate(-7deg);
    animation:
      dzDSwanEnter 1.6s var(--ease-out,cubic-bezier(.16,1,.3,1)) .75s forwards,
      dzDSwanGlide 13s var(--ease-in-out,cubic-bezier(.65,0,.35,1)) 2.35s infinite;
  }
  /* gentle vertical bob of the whole rig (swan + parcel) */
  .dz-dswan-bob{transform-box:fill-box;transform-origin:center;
    animation:dzDSwanBob 5.5s var(--ease-in-out,cubic-bezier(.65,0,.35,1)) 2.35s infinite}
  /* the parcel sways with weight — pendulum from the sling anchor, no bounce */
  .dz-dswan-parcel{animation:dzDSwanSway 4.2s var(--ease-in-out,cubic-bezier(.65,0,.35,1)) 2.1s infinite}
}

/* delivery: arrive from the left and bank level */
@keyframes dzDSwanEnter{
  0%{opacity:0;transform:translate(-46%,14%) rotate(-7deg)}
  60%{opacity:1}
  100%{opacity:1;transform:translate(8%,2%) rotate(-2deg)}
}
/* ambient glide: a slow shallow arc to the right and back, banking with travel */
@keyframes dzDSwanGlide{
  0%{transform:translate(8%,2%) rotate(-2deg)}
  50%{transform:translate(40%,-3%) rotate(1.5deg)}
  100%{transform:translate(8%,2%) rotate(-2deg)}
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
`;
