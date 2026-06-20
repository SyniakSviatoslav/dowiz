import React from 'react';
import { useI18n } from '@deliveryos/ui';

/**
 * Public front-door hero for /start. An authored line-art swan — dowiz's mark —
 * gliding on dark water: amber underglow, a mirrored reflection, expanding
 * ripples, and a self-drawing entrance. Pure SVG + CSS (no WebGL/Three.js — the
 * cinematic swan is a separate fast-follow). Every colour is a brand token, so it
 * re-skins with the active theme. All motion is gated behind
 * prefers-reduced-motion: reduce → a fully-drawn, still swan.
 */
export function SwanHero() {
  const { t } = useI18n();

  const steps: { icon: string; label: string }[] = [
    { icon: 'ti ti-camera', label: t('start.step_upload', 'Upload your menu') },
    { icon: 'ti ti-sparkles', label: t('start.step_read', 'We read it with AI') },
    { icon: 'ti ti-broadcast', label: t('start.step_live', 'Go live in minutes') },
  ];

  return (
    <section className="dz-hero" aria-labelledby="dz-hero-title">
      <style>{CSS}</style>

      <div className="dz-hero-art" aria-hidden="true">
        <div className="dz-hero-glow" />
        <svg viewBox="0 0 240 200" role="presentation" focusable="false">
          {/* ripples on the water */}
          <ellipse className="dz-ripple" cx="100" cy="144" rx="46" ry="8.5" />
          <ellipse className="dz-ripple r2" cx="100" cy="144" rx="46" ry="8.5" />
          <ellipse className="dz-ripple r3" cx="100" cy="144" rx="46" ry="8.5" />

          {/* mirrored body reflection below the waterline (no neck → no thin thread) */}
          <g className="dz-reflection" transform="translate(0,280) scale(1,-1)">
            <path className="dz-swan-line" d="M52 138 C 48 112, 74 100, 104 102 C 130 104, 142 116, 140 132" />
          </g>

          {/* waterline */}
          <line x1="26" y1="140" x2="208" y2="140" stroke="var(--brand-border)" strokeWidth="2" strokeLinecap="round" opacity="0.65" />

          {/* the swan: full body on the water + S-neck rising to the head */}
          <g className="dz-swan-group">
            <path className="dz-swan-line dz-draw" pathLength={1} d="M52 138 C 48 112, 74 100, 104 102 C 130 104, 142 116, 140 132" />
            <path className="dz-swan-line dz-draw d2" pathLength={1} d="M136 132 C 124 110, 148 102, 150 84 C 151 70, 147 60, 159 57" />
            <path className="dz-swan-line dz-wing" d="M70 132 C 82 112, 108 108, 126 122" opacity="0.6" />
            <polygon className="dz-swan-beak" points="166,56 182,61 166,64" />
            <circle className="dz-swan-line dz-head" cx="160" cy="57" r="7" />
            <circle className="dz-swan-eye" cx="162" cy="55" r="1.6" />
          </g>
        </svg>
      </div>

      <h1 id="dz-hero-title" className="dz-hero-title dz-reveal">
        {t('start.hero_title', 'Your menu, online tonight.')}
      </h1>
      <p className="dz-hero-sub dz-reveal rv2">
        {t('start.hero_sub', 'Snap a photo of your menu — we read it, build your storefront, and you’re taking orders. No code, no wait.')}
      </p>

      <ol className="dz-hero-steps dz-reveal rv3">
        {steps.map((s, i) => (
          <li key={i}>
            <span className="dz-step-ic"><i className={s.icon} aria-hidden="true" /></span>
            <span>{s.label}</span>
          </li>
        ))}
      </ol>
    </section>
  );
}

const CSS = `
.dz-hero{position:relative;text-align:center;padding:4px 4px 8px}
.dz-hero-art{position:relative;width:100%;max-width:300px;margin:0 auto;aspect-ratio:6/5}
.dz-hero-art svg{position:relative;width:100%;height:100%;display:block;overflow:visible;z-index:1}
.dz-hero-glow{position:absolute;left:50%;top:46%;width:60%;height:56%;transform:translate(-50%,-50%);
  border-radius:50%;background:radial-gradient(circle,var(--brand-primary) 0%,transparent 68%);
  opacity:.2;filter:blur(10px);z-index:0}
.dz-swan-line{fill:none;stroke:var(--brand-text);stroke-width:5;stroke-linecap:round;stroke-linejoin:round}
.dz-head{fill:var(--brand-bg)}
.dz-swan-beak{fill:var(--brand-primary)}
.dz-swan-eye{fill:var(--brand-text)}
.dz-ripple{fill:none;stroke:var(--brand-primary);stroke-width:2;opacity:0;transform-box:fill-box;transform-origin:center}
.dz-reflection{opacity:.15}

.dz-hero-title{font-family:var(--brand-font-heading);color:var(--brand-text);
  font-size:clamp(26px,7.4vw,34px);line-height:1.08;font-weight:800;letter-spacing:-.02em;margin:2px 0 0}
.dz-hero-sub{color:var(--brand-text-muted);font-size:15px;line-height:1.5;max-width:32ch;margin:10px auto 0}

.dz-hero-steps{display:flex;gap:8px;justify-content:center;margin:20px auto 2px;padding:0;list-style:none;max-width:340px}
.dz-hero-steps li{flex:1;display:flex;flex-direction:column;align-items:center;gap:7px;
  font-size:11.5px;line-height:1.25;color:var(--brand-text-muted)}
.dz-step-ic{width:38px;height:38px;border-radius:50%;display:grid;place-items:center;
  background:var(--brand-primary-light);color:var(--brand-primary);font-size:18px}

@media (prefers-reduced-motion: no-preference){
  .dz-swan-group{animation:dzGlide 6s ease-in-out infinite;transform-box:fill-box;transform-origin:center}
  .dz-draw{stroke-dasharray:1;stroke-dashoffset:1;animation:dzDraw 1.5s ease forwards}
  .dz-draw.d2{animation-delay:.28s}
  .dz-wing{opacity:0;animation:dzFade .6s ease .9s forwards}
  .dz-hero-glow{animation:dzBreathe 7s ease-in-out infinite}
  .dz-ripple{animation:dzRipple 4.5s ease-out infinite}
  .dz-ripple.r2{animation-delay:1.5s}
  .dz-ripple.r3{animation-delay:3s}
  .dz-reveal{opacity:0;transform:translateY(8px);animation:dzReveal .6s ease .2s forwards}
  .dz-reveal.rv2{animation-delay:.34s}
  .dz-reveal.rv3{animation-delay:.46s}
}
@keyframes dzGlide{0%,100%{transform:translateY(0)}50%{transform:translateY(-6px)}}
@keyframes dzDraw{to{stroke-dashoffset:0}}
@keyframes dzFade{to{opacity:.65}}
@keyframes dzBreathe{0%,100%{opacity:.16;transform:translate(-50%,-50%) scale(.97)}50%{opacity:.27;transform:translate(-50%,-50%) scale(1.04)}}
@keyframes dzRipple{0%{opacity:0;transform:scale(.45)}18%{opacity:.45}100%{opacity:0;transform:scale(1.35)}}
@keyframes dzReveal{to{opacity:1;transform:none}}
`;
