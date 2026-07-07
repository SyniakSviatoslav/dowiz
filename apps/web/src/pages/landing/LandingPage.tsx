import React, { useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, useScroll, useSpring } from 'framer-motion';
import { useI18n, LanguageSwitcher } from '@deliveryos/ui';
import { HorizonDrift } from './HorizonDrift.js';
import { CityPopRadio } from './CityPopRadio.js';
import './landing.css';

// Two jazz curves (mirror --ease-jazz-in / --ease-jazz-snap in tokens.css).
const JAZZ_IN: [number, number, number, number] = [0.23, 1, 0.32, 1];

/**
 * Dowiz entry point — "the Cowboy Bebop title sequence, as a storefront pitch."
 * Warm cosmo-noir (data-skin="bebop") · Nomadic skeleton (stage + corner HUD +
 * gated sessions) · Horizon Drift ambient · Ukrainian dry-wit copy. The engine
 * room, sold honestly. Money/CTA copy stays plain; brand moments carry the wit.
 */
export function LandingPage() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const scrollRef = useRef<HTMLDivElement>(null);

  const { scrollYProgress } = useScroll({ container: scrollRef });
  const progress = useSpring(scrollYProgress, { stiffness: 120, damping: 30, mass: 0.4 });

  // Reveal: staggered entrance on scroll-into-view, syncopated (uneven delays).
  const Reveal = ({ children, delay = 0, y = 24 }: { children: React.ReactNode; delay?: number; y?: number }) => (
    <motion.div
      initial={{ opacity: 0, y }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ root: scrollRef, amount: 0.4, once: true }}
      transition={{ duration: 0.56, ease: JAZZ_IN, delay }}
    >
      {children}
    </motion.div>
  );

  const doors: Array<[string, string]> = [
    [t('lp.door_web', 'Web'), t('lp.door_web_v', 'A link that is yours')],
    [t('lp.door_qr', 'QR'), t('lp.door_qr_v', 'On the table, on the door')],
    [t('lp.door_tg', 'Telegram'), t('lp.door_tg_v', 'Order inside the chat')],
    [t('lp.door_ig', 'Instagram'), t('lp.door_ig_v', 'From the bio, to the pass')],
    [t('lp.door_wa', 'WhatsApp'), t('lp.door_wa_v', 'Where they already talk')],
    [t('lp.door_kiosk', 'Kiosk'), t('lp.door_kiosk_v', 'The counter, unmanned')],
  ];

  return (
    <div className="lp-root" data-skin="bebop">
      {/* right-edge scroll progress */}
      <div className="lp-progress"><motion.div className="lp-progress__fill" style={{ scaleY: progress, height: '100%' }} /></div>

      {/* HUD — corner-pinned chrome */}
      <div className="lp-hud" aria-hidden="false">
        <a href="/" className="lp-logo">dowiz</a>
        <div className="lp-hud-tr"><LanguageSwitcher /></div>
        <div className="lp-hud-bl">{t('lp.hud_tag', 'Sovereign ordering — est. void')}</div>
      </div>

      {/* Easter egg: subtle "88.2 FM" chip → Japanese city-pop radio (press R) */}
      <CityPopRadio />

      <div className="lp-scroll" ref={scrollRef}>
        {/* ── SESSION 01 — THE PITCH ── */}
        <section className="lp-session">
          <HorizonDrift />
          <div style={{ maxWidth: '1180px', margin: '0 auto', width: '100%' }}>
            <Reveal>
              <span className="lp-eyebrow">{t('lp.s1_eyebrow', 'Session 01 — The Pitch')}</span>
            </Reveal>
            <Reveal delay={0.08}>
              <h1 className="lp-title">
                {t('lp.s1_title_a', 'Your kitchen.')}<br />
                {t('lp.s1_title_b', 'Your customers.')}<br />
                <em>{t('lp.s1_title_c', 'Your money.')}</em>
              </h1>
            </Reveal>
            <Reveal delay={0.16}>
              <p className="lp-lede">{t('lp.s1_lede', 'Novel concept, we know. Dowiz is sovereign ordering for independent kitchens — no middleman, no percentage claws, no platform holding your customers hostage.')}</p>
            </Reveal>
            <Reveal delay={0.26}>
              <div style={{ display: 'flex', gap: '14px', marginTop: '32px', flexWrap: 'wrap' }}>
                <button className="lp-cta lp-cta--primary" onClick={() => navigate('/claim')}>
                  {t('lp.cta_claim', 'Claim your storefront')}
                </button>
                <button className="lp-cta lp-cta--ghost" onClick={() => navigate('/start')}>
                  {t('lp.cta_menu', 'Upload your menu')}
                </button>
              </div>
            </Reveal>
          </div>
          <div className="lp-scrollhint">
            <span className="lp-scrollhint__rail" />
            {t('lp.scroll', 'Scroll')}
          </div>
        </section>

        {/* ── SESSION 02 — 0% ── */}
        <section className="lp-session" style={{ background: 'linear-gradient(180deg, #12100e, #1a1512)' }}>
          <div style={{ maxWidth: '1180px', margin: '0 auto', width: '100%', display: 'grid', gap: '24px' }}>
            <Reveal><span className="lp-eyebrow">{t('lp.s2_eyebrow', 'Session 02 — The Cut')}</span></Reveal>
            <Reveal delay={0.1}><div className="lp-figure">0%</div></Reveal>
            <Reveal delay={0.2}>
              <p className="lp-lede" style={{ maxWidth: '40ch' }}>
                {t('lp.s2_lede', 'Commission: zero. That number is not a typo. Aggregators take a quarter of every plate. We take none — the sale is yours, whole.')}
              </p>
            </Reveal>
          </div>
        </section>

        {/* ── SESSION 03 — OWN YOUR DATA ── */}
        <section className="lp-session" style={{ background: 'linear-gradient(180deg, #1a1512, #12100e)' }}>
          <div style={{ maxWidth: '1180px', margin: '0 auto', width: '100%' }}>
            <Reveal><span className="lp-eyebrow">{t('lp.s3_eyebrow', 'Session 03 — What’s Yours')}</span></Reveal>
            <Reveal delay={0.1}>
              <h2 className="lp-title" style={{ fontSize: 'clamp(36px, 6vw, 84px)' }}>
                {t('lp.s3_title', 'Your customers know')}<br /><em>{t('lp.s3_title_em', 'your name, not ours.')}</em>
              </h2>
            </Reveal>
            <Reveal delay={0.18}>
              <p className="lp-lede">{t('lp.s3_lede', 'Names, numbers, orders — every record is yours, in your hands, exportable, never locked behind a login you don’t control. The platform is a conduit, not a landlord.')}</p>
            </Reveal>
          </div>
        </section>

        {/* ── SESSION 04 — MANY DOORS, ONE HUB ── */}
        <section className="lp-session" style={{ background: 'linear-gradient(180deg, #12100e, #16130f)' }}>
          <div style={{ maxWidth: '1180px', margin: '0 auto', width: '100%' }}>
            <Reveal><span className="lp-eyebrow">{t('lp.s4_eyebrow', 'Session 04 — Many Doors')}</span></Reveal>
            <Reveal delay={0.1}>
              <h2 className="lp-title" style={{ fontSize: 'clamp(36px, 6vw, 84px)', marginBottom: '0.5em' }}>
                {t('lp.s4_title', 'Every entrance.')} <em>{t('lp.s4_title_em', 'One pass.')}</em>
              </h2>
            </Reveal>
            <Reveal delay={0.18}>
              <p className="lp-lede" style={{ marginBottom: '28px' }}>{t('lp.s4_lede', 'Wherever a hungry human already is — send them a door. Every order lands on the same kitchen screen.')}</p>
            </Reveal>
            <Reveal delay={0.26}>
              <div className="lp-doors">
                {doors.map(([k, v], i) => (
                  <div className="lp-door" key={i}>
                    <div className="lp-door__k">{k}</div>
                    <div className="lp-door__v">{v}</div>
                  </div>
                ))}
              </div>
            </Reveal>
          </div>
        </section>

        {/* ── SESSION 05 — THE SOVEREIGN CORE ── */}
        <section className="lp-session" style={{ background: 'linear-gradient(180deg, #16130f, #12100e)' }}>
          <div style={{ maxWidth: '1180px', margin: '0 auto', width: '100%' }}>
            <Reveal><span className="lp-eyebrow">{t('lp.s5_eyebrow', 'Session 05 — The Machine')}</span></Reveal>
            <Reveal delay={0.1}>
              <h2 className="lp-title" style={{ fontSize: 'clamp(32px, 5vw, 72px)', marginBottom: '0.6em' }}>
                {t('lp.s5_title', 'Cold logic.')} <em>{t('lp.s5_title_em', 'Warm mission.')}</em>
              </h2>
            </Reveal>
            <div className="lp-features">
              {[
                [t('lp.f1_h', 'Deterministic core'), t('lp.f1_p', 'Every order runs through one auditable Rust engine. It cannot invent a price or lose a sale. It simply refuses to lie.')],
                [t('lp.f2_h', 'Yours to keep'), t('lp.f2_p', 'Event-sourced and exportable. Walk away any time and take everything with you. Sovereignty is built in, not promised.')],
                [t('lp.f3_h', 'Hybrid by design'), t('lp.f3_p', 'Cold reptilian precision united with an authentic mission. Hybrid is a feature, not a bug.')],
              ].map(([h, p], i) => (
                <Reveal key={i} delay={0.16 + i * 0.08}>
                  <div>
                    <div className="lp-feature__h">{h}</div>
                    <p className="lp-feature__p">{p}</p>
                  </div>
                </Reveal>
              ))}
            </div>
          </div>
        </section>

        {/* ── END CARD — SEE YOU SPACE COWBOY ── */}
        <section className="lp-session" style={{ background: 'radial-gradient(120% 90% at 50% 120%, rgba(232,165,68,0.16), #0c0b0a 62%)' }}>
          <div className="lp-endcard" style={{ maxWidth: '900px', margin: '0 auto' }}>
            <Reveal>
              <div className="lp-endcard__kicker">{t('lp.end_kicker', 'See you, space cowboy')}</div>
            </Reveal>
            <Reveal delay={0.1}>
              <div className="lp-endcard__line">{t('lp.end_line', 'Ready to own')}<br /><em style={{ color: 'var(--amber)', fontStyle: 'italic' }}>{t('lp.end_line_em', 'your channel?')}</em></div>
            </Reveal>
            <Reveal delay={0.2}>
              <div style={{ display: 'flex', gap: '14px', justifyContent: 'center', marginTop: '36px', flexWrap: 'wrap' }}>
                <button className="lp-cta lp-cta--primary" onClick={() => navigate('/claim')}>{t('lp.cta_claim', 'Claim your storefront')}</button>
                <button className="lp-cta lp-cta--ghost" onClick={() => navigate('/start')}>{t('lp.cta_menu', 'Upload your menu')}</button>
              </div>
            </Reveal>

            {/* footer */}
            <footer className="lp-footer" style={{ marginTop: '96px', textAlign: 'left' }}>
              <div className="lp-footer__grid">
                <div>
                  <div className="lp-footer__h">{t('lp.f_product', 'Product')}</div>
                  <a href="/start">{t('lp.cta_menu', 'Upload your menu')}</a><br />
                  <a href="/claim">{t('lp.cta_claim', 'Claim your storefront')}</a>
                </div>
                <div>
                  <div className="lp-footer__h">{t('lp.f_company', 'Company')}</div>
                  <a href="/privacy">{t('lp.f_privacy', 'Privacy')}</a><br />
                  <a href="https://github.com/dowiz" target="_blank" rel="noreferrer noopener">{t('lp.f_source', 'Source')}</a>
                </div>
                <div>
                  <div className="lp-footer__h">{t('lp.f_credit_h', 'Design')}</div>
                  <span style={{ color: 'var(--ash)' }}>{t('lp.f_credit', 'Structure inspired by Nomadic Tribe by makemepulse. Skinned in warm cosmo-noir. Kudos to the originals —')} <a href="https://www.makemepulse.com/case-study/nomadic-tribe" target="_blank" rel="noreferrer noopener">makemepulse</a>.</span>
                </div>
              </div>
              <div className="lp-footer__legal">
                <span className="lp-mono">© 2026 dowiz</span> — {t('lp.f_legal', 'Hybrid is a feature, not a bug. Built with devotion; held together by spite. Yours, not ours.')}
              </div>
            </footer>
          </div>
        </section>
      </div>
    </div>
  );
}
