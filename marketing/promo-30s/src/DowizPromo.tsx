// dowiz × Dubin & Sushi — the 30-second keynote cut.
//
// One idea per shot, a phone floating on true black, enormous serif type,
// a burned-in subtitle on every shot, the music (when on disk) under all of
// it. Every timing below is in seconds and mirrors
// docs/marketing/promo-dubin-sushi-30s.en.md; the beat grid nudges them once
// the licensed track exists.

import React from "react";
import {
  AbsoluteFill,
  Audio,
  continueRender,
  delayRender,
  Easing,
  OffthreadVideo,
  Sequence,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";

export const FPS = 30;
export const DURATION_S = 30;
const W = 1080, H = 1920;

const GOLD = "#c9a35a", INK = "#f1e8d8", RED = "#DC2626", TEAL = "#0D9488";
// The fonts ship with the composition (public/fonts), so every machine renders the same glyphs.
const SERIF = '"EB Garamond", "Iowan Old Style", Palatino, Georgia, serif';
const MONO = '"JetBrains Mono", "SF Mono", Menlo, Consolas, monospace';
const SANS = '"DM Sans", system-ui, -apple-system, "Segoe UI", Roboto, sans-serif';
const FONT_FACES = [["EB Garamond", "EBGaramond.ttf"], ["JetBrains Mono", "JetBrainsMono.ttf"], ["DM Sans", "DMSans.ttf"]] as const;

/// Declares the three faces and holds the render until Chrome has loaded them.
const Fonts: React.FC = () => {
  const [handle] = React.useState(() => delayRender("fonts"));
  React.useEffect(() => {
    const faces = FONT_FACES.map(([family, file]) => new FontFace(family, `url(${staticFile(`fonts/${file}`)})`, { weight: "100 900" }));
    Promise.all(faces.map(f => f.load())).then(loaded => { loaded.forEach(f => document.fonts.add(f)); continueRender(handle); }).catch(() => continueRender(handle));
  }, [handle]);
  return null;
};

type Lang = "en" | "uk" | "sq";
export type PromoProps = { lang: Lang; music: string | null; musicInSeconds: number };

const T: Record<Lang, { title: string[]; sub: string[] }> = {
  en: {
    title: ["Giving away a third of every order?", "per-order fees", "no per-order fees · no tariffs · no commission", "Your own mini-app", "Your brand. Your customers. Your orders.", "They watch it come.", "One hub for every order", "Autoposting, on your approval", "Post-quantum security", "Local AI. Data never leaves the venue.", "Keep 100% of every order", "Installs as an app", "Launch your fee-free mini-app"],
    sub: ["Aggregators take a cut of every order you sell.", "Twenty-five to thirty-five percent, every day.", "dowiz charges nothing per order. One flat subscription.", "Your venue gets its own app, on its own domain.", "Your colours, your seal, your menu — and an order in a few taps.", "Customers watch the courier come, in real time.", "Storefront, phone, WhatsApp, Instagram, partner APIs — one order log.", "dowiz drafts posts about your menu; you approve, it publishes.", "Post-quantum encryption protects your data and your customers'.", "The AI assistant runs on your own server. Nothing is sent away.", "Every order's profit stays with you.", "It installs on the phone like any app. No store needed.", "Launch your fee-free mini-app at dowiz.org."],
  },
  uk: {
    title: ["Віддаєш третину кожного замовлення?", "тарифи за замовлення", "без тарифів за замовлення · без комісій", "Твій власний міні-додаток", "Твій бренд. Твої клієнти. Твої замовлення.", "Вони бачать, як воно їде.", "Один хаб для всіх замовлень", "Автопостинг з вашого схвалення", "Постквантова безпека", "Локальний ШІ. Дані не виходять за поріг.", "Зберігай 100% кожного замовлення", "Встановлюється як додаток", "Запусти свій міні-додаток без тарифів"],
    sub: ["Агрегатори беруть частку з кожного вашого замовлення.", "Від двадцяти п'яти до тридцяти п'яти відсотків, щодня.", "dowiz не бере нічого за замовлення. Одна фіксована підписка.", "Ваш заклад отримує власний додаток на власному домені.", "Ваші кольори, ваша печатка, ваше меню — і замовлення в кілька дотиків.", "Клієнти бачать, як їде кур'єр, у реальному часі.", "Вітрина, телефон, WhatsApp, Instagram, API партнерів — один журнал замовлень.", "dowiz пише пости про ваше меню; ви схвалюєте, воно публікує.", "Постквантове шифрування захищає ваші дані й дані клієнтів.", "ШІ-помічник працює на вашому сервері. Нічого не надсилається.", "Прибуток з кожного замовлення лишається вам.", "Встановлюється на телефон як звичайний додаток. Без магазину.", "Запустіть свій міні-додаток без тарифів на dowiz.org."],
  },
  sq: {
    title: ["Jep një të tretën e çdo porosie?", "tarifa për porosi", "pa tarifa për porosi · pa komisione", "Mini-aplikacioni yt", "Marka jote. Klientët e tu. Porositë e tua.", "E shohin duke ardhur.", "Një qendër për të gjitha porositë", "Autopostim me miratimin tuaj", "Siguri post-kuantike", "AI lokal. Të dhënat nuk dalin nga lokali.", "Mbaj 100% të çdo porosie", "Instalohet si aplikacion", "Nis mini-aplikacionin tënd pa tarifa"],
    sub: ["Agregatorët marrin një pjesë nga çdo porosi.", "Nga 25 në 35 përqind, çdo ditë.", "dowiz s'merr asgjë për porosi. Një abonim fiks.", "Lokali juaj merr aplikacionin e vet, në domenin e vet.", "Ngjyrat, vula, menyja juaj — dhe porosi me pak prekje.", "Klientët shohin korrierin duke ardhur, në kohë reale.", "Vitrina, telefoni, WhatsApp, Instagram, API — një regjistër porosish.", "dowiz shkruan postime për menynë; ju miratoni, ai boton.", "Kriptimi post-kuantik mbron të dhënat tuaja dhe të klientëve.", "Asistenti AI punon në serverin tuaj. Asgjë nuk dërgohet.", "Fitimi i çdo porosie mbetet me ju.", "Instalohet si çdo aplikacion. Pa dyqan.", "Nisni mini-aplikacionin pa tarifa në dowiz.org."],
  },
};

/// The thirteen shots: start, end (seconds), and the source clip's in-point.
const SHOTS: { start: number; end: number; kind: "broll" | "graphic" | "screen"; src?: string; from?: number }[] = [
  { start: 0, end: 2, kind: "broll", src: "h1-craft-box.mp4" },
  { start: 2, end: 4, kind: "graphic" },
  { start: 4, end: 7, kind: "graphic" },
  { start: 7, end: 10, kind: "screen", src: "store-loader-grid-dish.webm", from: 0.2 },
  { start: 10, end: 13, kind: "screen", src: "store-loader-grid-dish.webm", from: 5.2 },
  { start: 13, end: 16, kind: "screen", src: "track-ocean-map.webm", from: 5.6 },
  { start: 16, end: 18, kind: "graphic" },
  { start: 18, end: 20, kind: "screen", src: "admin-posts-approve.webm", from: 7.0 },
  { start: 20, end: 22, kind: "graphic" },
  { start: 22, end: 24, kind: "screen", src: "admin-assistant.webm", from: 8.5 },
  { start: 24, end: 26, kind: "screen", src: "admin-orders.webm", from: 4.9 },
  { start: 26, end: 28, kind: "broll", src: "h4-hand-phone.mp4" },
  { start: 28, end: 30, kind: "graphic" },
];

const ease = Easing.bezier(0.2, 0.8, 0.2, 1);
const rise = (frame: number, delay = 0, ms = 360) => {
  const f = Math.max(0, frame - delay * FPS), n = (ms / 1000) * FPS;
  const k = interpolate(f, [0, n], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  return { opacity: k, transform: `translateY(${(1 - k) * 24}px)`, filter: `blur(${(1 - k) * 6}px)` };
};

const Stage: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <AbsoluteFill style={{ background: "radial-gradient(60% 18% at 50% 92%, rgba(201,163,90,.14), transparent 70%), #000" }}>
    {children}
    <AbsoluteFill style={{ pointerEvents: "none", background: "radial-gradient(120% 90% at 50% 50%, transparent 60%, rgba(0,0,0,.55) 100%)" }} />
  </AbsoluteFill>
);

const Headline: React.FC<{ text: string; size?: number; mono?: boolean; color?: string; top?: number; delay?: number }> = ({ text, size = 104, mono = false, color = INK, top = 180, delay = 0 }) => {
  const frame = useCurrentFrame();
  return (
    <div style={{ position: "absolute", left: 72, right: 72, top, textAlign: "center", fontFamily: mono ? MONO : SERIF, fontSize: size, lineHeight: 1.08, letterSpacing: mono ? "0.02em" : "-0.02em", color, textWrap: "balance" as any, ...rise(frame, delay) }}>{text}</div>
  );
};

const Caption: React.FC<{ text: string; top?: number; delay?: number }> = ({ text, top = 1080, delay = 0.12 }) => {
  const frame = useCurrentFrame();
  return <div style={{ position: "absolute", left: 60, right: 60, top, textAlign: "center", fontFamily: MONO, fontSize: 24, letterSpacing: "0.24em", textTransform: "uppercase", color: GOLD, ...rise(frame, delay) }}>{text}</div>;
};

const Subtitle: React.FC<{ text: string }> = ({ text }) => {
  const frame = useCurrentFrame();
  const k = interpolate(frame, [4, 12], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  return (
    <div style={{ position: "absolute", left: 0, right: 0, bottom: 260, display: "flex", justifyContent: "center", opacity: k }}>
      <div style={{ maxWidth: 940, padding: "12px 20px", borderRadius: 8, background: "rgba(0,0,0,.62)", color: "#fff", fontFamily: SANS, fontSize: 30, lineHeight: 1.3, textAlign: "center" }}>{text}</div>
    </div>
  );
};

/// A phone on the stage: the clip inside a 44px-radius frame with a glass sweep.
const Phone: React.FC<{ src: string; from: number; push?: number; rise?: boolean }> = ({ src, from, push = 0.03, rise: rises = false }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const scale = 1 + push * interpolate(frame, [0, durationInFrames], [0, 1], { extrapolateRight: "clamp" });
  const y = rises ? interpolate(spring({ frame, fps: FPS, config: { damping: 18, stiffness: 60 } }), [0, 1], [900, 0]) : 0;
  const sweep = interpolate(frame, [0, durationInFrames], [-40, 140]);
  const w = 570, h = 1235;
  return (
    <div style={{ position: "absolute", left: (W - w) / 2, top: 320, width: w, height: h, transform: `translateY(${y}px) perspective(2200px) rotateX(6deg) scale(${scale})`, transformOrigin: "50% 60%" }}>
      <div style={{ position: "absolute", inset: 0, borderRadius: 44, overflow: "hidden", background: "#0b1717", boxShadow: "0 40px 90px rgba(0,0,0,.6), 0 0 0 2px rgba(255,255,255,.08), inset 0 0 0 1px rgba(255,255,255,.05)" }}>
        <OffthreadVideo muted src={staticFile(`promo/${src}`)} trimBefore={Math.round(from * FPS)} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
        <div style={{ position: "absolute", inset: 0, background: `linear-gradient(115deg, transparent ${sweep - 12}%, rgba(255,255,255,.10) ${sweep}%, transparent ${sweep + 12}%)`, pointerEvents: "none" }} />
      </div>
    </div>
  );
};

/// The counter of the pain: −25 → −30 → −35, one tick per beat, shaking.
const Counter: React.FC = () => {
  const frame = useCurrentFrame();
  const steps = [-25, -30, -35];
  const i = Math.min(steps.length - 1, Math.floor(frame / (FPS * 0.66)));
  const jit = Math.sin(frame * 2.3) * 2;
  return <div style={{ position: "absolute", left: 0, right: 0, top: 640, textAlign: "center", fontFamily: SANS, fontSize: 260, fontWeight: 700, color: RED, transform: `translate(${jit}px, ${-jit}px)`, letterSpacing: "-0.04em" }}>{steps[i]}%</div>;
};

/// The gold zero: flips in on the first frame, then the foil breathes.
const Zero: React.FC = () => {
  const frame = useCurrentFrame();
  const flip = interpolate(frame, [0, 8], [90, 0], { extrapolateRight: "clamp", easing: ease });
  const pos = (frame / (FPS * 6.4)) * 200;
  return (
    <div style={{ position: "absolute", left: 0, right: 0, top: 600, textAlign: "center", perspective: 1200 }}>
      <div style={{ display: "inline-block", fontFamily: SANS, fontSize: 440, fontWeight: 700, letterSpacing: "-0.06em", transform: `rotateX(${flip}deg)`, backgroundImage: `linear-gradient(135deg, #6b4f1c 0%, ${GOLD} 28%, #f3dea0 50%, ${GOLD} 72%, #6b4f1c 100%)`, backgroundSize: "200% 200%", backgroundPosition: `${pos}% 50%`, WebkitBackgroundClip: "text", backgroundClip: "text", color: "transparent" }}>0</div>
    </div>
  );
};

/// Five sources fly in to one node on the beats.
const Hub: React.FC = () => {
  const frame = useCurrentFrame();
  const labels = ["storefront", "phone", "WhatsApp", "Instagram", "API"];
  const cx = W / 2, cy = 900, r = 330;
  return (
    <>
      {labels.map((l, i) => {
        const a = -Math.PI / 2 + (i * 2 * Math.PI) / labels.length;
        const k = spring({ frame: frame - i * 4, fps: FPS, config: { damping: 14, stiffness: 90 } });
        const x = cx + Math.cos(a) * r * k, y = cy + Math.sin(a) * r * k;
        return (
          <div key={l} style={{ position: "absolute", left: x - 90, top: y - 26, width: 180, textAlign: "center", opacity: k }}>
            <div style={{ width: 16, height: 16, borderRadius: 8, background: GOLD, margin: "0 auto 8px", boxShadow: `0 0 18px ${GOLD}` }} />
            <div style={{ fontFamily: MONO, fontSize: 22, letterSpacing: "0.12em", color: INK, textTransform: "uppercase" }}>{l}</div>
          </div>
        );
      })}
      <div style={{ position: "absolute", left: cx - 70, top: cy - 70, width: 140, height: 140, borderRadius: 70, background: `radial-gradient(circle, #f3dea0, ${GOLD} 60%, #6b4f1c)`, boxShadow: `0 0 ${40 + 30 * Math.abs(Math.sin(frame / 9))}px ${GOLD}`, display: "grid", placeItems: "center", fontFamily: SERIF, fontStyle: "italic", fontSize: 72, color: "#1a1408" }}>d</div>
    </>
  );
};

/// A lock drawn from a lattice, noise digits freezing around it.
const Lock: React.FC = () => {
  const frame = useCurrentFrame();
  const draw = interpolate(frame, [0, 22], [1, 0], { extrapolateRight: "clamp", easing: ease });
  const digits = Array.from({ length: 34 }, (_, i) => i);
  return (
    <>
      <svg width={W} height={H} style={{ position: "absolute", left: 0, top: 0 }} viewBox={`0 0 ${W} ${H}`}>
        <g fill="none" stroke={GOLD} strokeWidth={5} strokeLinecap="round" strokeLinejoin="round" style={{ strokeDasharray: 2200, strokeDashoffset: 2200 * draw }}>
          <rect x={380} y={860} width={320} height={260} rx={28} />
          <path d="M440 860 V760 a100 100 0 0 1 200 0 V860" />
          <path d="M380 930 L540 860 L700 930 M380 1050 L540 980 L700 1050 M540 860 V1120" />
        </g>
      </svg>
      {digits.map(i => {
        const seed = (i * 9301 + 49297) % 233280 / 233280;
        const x = 120 + seed * 840, y = 560 + ((i * 7919) % 700);
        const settle = Math.min(1, Math.max(0, (frame - i) / 16));
        const ch = settle < 1 ? String((frame * (i + 3)) % 10) : (i % 2 ? "1" : "0");
        return <div key={i} style={{ position: "absolute", left: x, top: y, fontFamily: MONO, fontSize: 28, color: GOLD, opacity: 0.25 + 0.5 * settle }}>{ch}</div>;
      })}
    </>
  );
};

const EndCard: React.FC = () => {
  const frame = useCurrentFrame();
  const k = spring({ frame, fps: FPS, config: { damping: 12, stiffness: 120 } });
  return (
    <>
      <div style={{ position: "absolute", left: W / 2 - 110, top: 640, width: 220, height: 220, borderRadius: 110, transform: `scale(${k})`, background: `linear-gradient(135deg, #6b4f1c, ${GOLD} 40%, #f3dea0 55%, ${GOLD})`, boxShadow: `0 30px 80px rgba(201,163,90,.35)`, display: "grid", placeItems: "center", fontFamily: SERIF, fontStyle: "italic", fontSize: 130, color: "#1a1408" }}>d</div>
      <div style={{ position: "absolute", left: 0, right: 0, top: 900, textAlign: "center", fontFamily: SERIF, fontSize: 120, letterSpacing: "0.12em", color: INK, ...rise(frame, 0.2) }}>dowiz</div>
      <div style={{ position: "absolute", left: 0, right: 0, top: 1060, textAlign: "center", fontFamily: MONO, fontSize: 30, letterSpacing: "0.22em", color: GOLD, textTransform: "uppercase", ...rise(frame, 0.35) }}>dowiz.org</div>
    </>
  );
};

/// A generated insert when the file exists; a stand-in when it does not yet.
const Broll: React.FC<{ src: string; fallback: React.ReactNode }> = ({ src, fallback }) => {
  const [ok, setOk] = React.useState<boolean | null>(null);
  React.useEffect(() => { fetch(staticFile(`promo/${src}`), { method: "HEAD" }).then(r => setOk(r.ok)).catch(() => setOk(false)); }, [src]);
  if (ok) return <OffthreadVideo muted src={staticFile(`promo/${src}`)} style={{ width: "100%", height: "100%", objectFit: "cover", filter: "saturate(.8) contrast(1.05)" }} />;
  return <>{fallback}</>;
};

/// Banknotes lifting and burning, for the first shot's stand-in and overlay.
const Notes: React.FC = () => {
  const frame = useCurrentFrame();
  return (
    <>
      {Array.from({ length: 20 }, (_, i) => {
        const t = Math.max(0, frame - 12 - i * 1.5) / 24;
        const x = 240 + (i * 37) % 600, y = 1100 - t * 900, o = Math.max(0, 1 - t);
        return <div key={i} style={{ position: "absolute", left: x, top: y, width: 120, height: 56, borderRadius: 6, background: `linear-gradient(90deg, #2e6b3f, #4f9a5b)`, opacity: o, transform: `rotate(${(i * 23) % 60 - 30 + t * 40}deg)`, boxShadow: `0 0 ${20 * t}px ${RED}` }} />;
      })}
      <div style={{ position: "absolute", left: 300, top: 1040, width: 480, height: 300, borderRadius: 24, background: "linear-gradient(180deg, #4a3a25, #2b2116)", boxShadow: "0 40px 80px rgba(0,0,0,.7)" }} />
    </>
  );
};

export const DowizPromo: React.FC<PromoProps> = ({ lang, music, musicInSeconds }) => {
  const t = T[lang] ?? T.en;
  return (
    <Stage>
      <Fonts />
      {music ? <Audio src={staticFile(`promo/${music}`)} trimBefore={Math.round(musicInSeconds * FPS)} volume={f => interpolate(f, [DURATION_S * FPS - 18, DURATION_S * FPS], [1, 0], { extrapolateLeft: "clamp" })} /> : null}
      {SHOTS.map((s, i) => (
        <Sequence key={i} from={s.start * FPS} durationInFrames={(s.end - s.start) * FPS}>
          {i === 0 && <Broll src={s.src!} fallback={<Notes />} />}
          {i === 0 && <Headline text={t.title[0]} size={92} top={220} />}
          {i === 1 && <><Counter /><Caption text={t.title[1]} top={1000} /></>}
          {i === 2 && <><Zero /><Caption text={t.title[2]} top={1120} /></>}
          {i === 3 && <><Phone src={s.src!} from={s.from!} rise /><Headline text={t.title[3]} top={120} delay={0.3} /></>}
          {i === 4 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[4]} size={72} top={120} /></>}
          {i === 5 && <><Phone src={s.src!} from={s.from!} push={0.04} /><Headline text={t.title[5]} size={96} top={130} /></>}
          {i === 6 && <><Hub /><Headline text={t.title[6]} size={88} top={220} /></>}
          {i === 7 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[7]} size={80} top={120} /></>}
          {i === 8 && <><Lock /><Headline text={t.title[8]} size={92} top={220} /><Caption text="ML-KEM-768 · ML-DSA-65" top={1220} delay={0.5} /></>}
          {i === 9 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[9]} size={68} top={120} /></>}
          {i === 10 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[10]} size={84} top={120} /></>}
          {i === 11 && <><Broll src={s.src!} fallback={<Phone src="store-loader-grid-dish.webm" from={0.2} rise />} /><Headline text={t.title[11]} size={92} top={120} delay={0.2} /></>}
          {i === 12 && <><EndCard /><Caption text={t.title[12]} top={1140} delay={0.5} /></>}
          <Subtitle text={t.sub[i]} />
        </Sequence>
      ))}
    </Stage>
  );
};
