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

const BONE = "#f2f1ec", INK = "#0b0b0c", HOT = "#ff4d1c", MUTE = "#6f6f6b", BONE_2 = "#e9e7e0";
// The fonts ship with the composition (public/fonts), so every machine renders the same glyphs.
const DISPLAY = '"Unbounded", "Manrope", system-ui, sans-serif';
const MONO = '"JetBrains Mono", "SF Mono", Menlo, Consolas, monospace';
const SANS = '"Manrope", system-ui, sans-serif';
const FONT_FACES = [
  ["Unbounded", "unbounded-latin.woff2"],
  ["Unbounded", "unbounded-latin-ext.woff2"],
  ["Unbounded", "unbounded-cyrillic.woff2"],
  ["Manrope", "manrope-latin.woff2"],
  ["Manrope", "manrope-latin-ext.woff2"],
  ["Manrope", "manrope-cyrillic.woff2"],
  ["JetBrains Mono", "jetbrains-mono.woff2"],
  ["EB Garamond", "EBGaramond.ttf"],
] as const;

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
  { start: 0, end: 2, kind: "graphic" },
  { start: 2, end: 4, kind: "graphic" },
  { start: 4, end: 7, kind: "graphic" },
  { start: 7, end: 10, kind: "screen", src: "store-loader-grid-dish.webm", from: 0.2 },
  { start: 10, end: 13, kind: "screen", src: "store-loader-grid-dish.webm", from: 5.2 },
  { start: 13, end: 16, kind: "graphic" },
  { start: 16, end: 18, kind: "graphic" },
  { start: 18, end: 20, kind: "screen", src: "admin-posts-approve.webm", from: 7.0 },
  { start: 20, end: 22, kind: "graphic" },
  { start: 22, end: 24, kind: "screen", src: "admin-assistant.webm", from: 8.5 },
  { start: 24, end: 26, kind: "screen", src: "admin-orders.webm", from: 4.9 },
  { start: 26, end: 28, kind: "graphic" },
  { start: 28, end: 30, kind: "graphic" },
];

const ease = Easing.bezier(0.2, 0.8, 0.2, 1);
const rise = (frame: number, delay = 0, ms = 360) => {
  const f = Math.max(0, frame - delay * FPS), n = (ms / 1000) * FPS;
  const k = interpolate(f, [0, n], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  return { opacity: k, transform: `translateY(${(1 - k) * 24}px)`, filter: `blur(${(1 - k) * 6}px)` };
};

/// Global camera: the whole picture kicks on the three hits (0.9 s, 4 s, 28 s) and settles.
const HITS = [0.9, 4.0, 28.0];
const Camera: React.FC<{ children?: React.ReactNode }> = ({ children }) => {
  const frame = useCurrentFrame();
  let dx = 0, dy = 0, sc = 1;
  for (const h of HITS) {
    const f = frame - h * FPS;
    if (f >= 0 && f < 16) { const k = Math.exp(-f / 4); dx += Math.sin(f * 2.7) * 9 * k; dy += Math.cos(f * 3.1) * 7 * k; sc += 0.018 * k; }
  }
  return <AbsoluteFill style={{ transform: `translate(${dx}px, ${dy}px) scale(${sc})`, transformOrigin: "50% 50%" }}>{children}</AbsoluteFill>;
};

/// Per-shot camera: a slow push-in with a little drift, so nothing is ever static.
const Shot: React.FC<{ index: number; children?: React.ReactNode }> = ({ index, children }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const k = frame / durationInFrames;
  const dir = index % 2 ? 1 : -1;
  return <AbsoluteFill style={{ transform: `scale(${1 + 0.035 * k}) translate(${dir * 6 * k}px, ${-4 * k}px)`, transformOrigin: index % 3 === 0 ? "50% 45%" : "50% 55%" }}>{children}</AbsoluteFill>;
};

/// A flash and a shockwave on a beat: white-gold bloom for six frames, one ring expanding out.
const Impact: React.FC<{ at?: number; x?: number; y?: number; strength?: number }> = ({ at = 0, x = W / 2, y = H / 2, strength = 1 }) => {
  const frame = useCurrentFrame();
  const f = frame - at * FPS;
  if (f < 0 || f > 26) return null;
  const flash = Math.max(0, 1 - f / 6) * 0.45 * strength;
  const ring = interpolate(f, [0, 26], [0, 1], { extrapolateRight: "clamp", easing: Easing.out(Easing.cubic) });
  return (
    <>
      <AbsoluteFill style={{ pointerEvents: "none", background: `radial-gradient(60% 40% at ${(x / W) * 100}% ${(y / H) * 100}%, rgba(255,77,28,${flash * 0.35}), transparent 70%)` }} />
      <div style={{ position: "absolute", left: x - 400 * ring, top: y - 400 * ring, width: 800 * ring, height: 800 * ring, borderRadius: "50%", border: `${3 * (1 - ring) + 1}px solid rgba(11,11,12,${(1 - ring) * 0.7 * strength})` }} />
    </>
  );
};

const Stage: React.FC<{ children?: React.ReactNode }> = ({ children }) => (
  <AbsoluteFill style={{ background: BONE }}>
    {children}
  </AbsoluteFill>
);

const Headline: React.FC<{ text: string; size?: number; mono?: boolean; color?: string; top?: number; delay?: number }> = ({ text, size = 104, mono = false, color = INK, top = 180, delay = 0 }) => {
  const frame = useCurrentFrame();
  const words = text.split(" ");
  const displaySize = size * 0.8;
  return (
    <div style={{ position: "absolute", left: 72, right: 72, top, textAlign: "left", fontFamily: DISPLAY, fontSize: displaySize, fontWeight: 800, lineHeight: 1.0, letterSpacing: "-0.035em", color, textWrap: "balance" as any }}>
      {words.map((w, i) => <span key={i} style={{ display: "inline-block", whiteSpace: "pre", ...rise(frame, delay + i * 0.07, 420) }}>{w}{i < words.length - 1 ? " " : ""}</span>)}
    </div>
  );
};

const Caption: React.FC<{ text: string; top?: number; delay?: number }> = ({ text, top = 1080, delay = 0.12 }) => {
  const frame = useCurrentFrame();
  return <div style={{ position: "absolute", left: 60, right: 60, top, textAlign: "center", fontFamily: MONO, fontSize: 26, letterSpacing: "0.12em", textTransform: "uppercase", color: HOT, ...rise(frame, delay) }}>{text}</div>;
};

const Subtitle: React.FC<{ text: string }> = ({ text }) => {
  const frame = useCurrentFrame();
  const k = interpolate(frame, [4, 12], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  return (
    <div style={{ position: "absolute", left: 0, right: 0, bottom: 260, display: "flex", justifyContent: "center", opacity: k }}>
      <div style={{ maxWidth: 940, padding: "12px 20px", borderRadius: 12, background: INK, color: BONE, fontFamily: SANS, fontWeight: 500, fontSize: 30, lineHeight: 1.3, textAlign: "center" }}>{text}</div>
    </div>
  );
};

/// A phone on the stage: the clip inside a 44px-radius frame with a glass sweep.
const Phone: React.FC<{ src?: string; from?: number; push?: number; rise?: boolean; children?: React.ReactNode }> = ({ src, from = 0, push = 0.03, rise: rises = false, children }) => {
  const frame = useCurrentFrame();
  const { durationInFrames } = useVideoConfig();
  const scale = 1 + push * interpolate(frame, [0, durationInFrames], [0, 1], { extrapolateRight: "clamp" });
  const sp = rises ? spring({ frame, fps: FPS, config: { damping: 18, stiffness: 60 } }) : 1;
  const y = interpolate(sp, [0, 1], [900, 0]), ry = interpolate(sp, [0, 1], [-14, -3]);
  const sweep = interpolate(frame, [0, durationInFrames], [-40, 140]);
  const w = 570, h = 1235;
  return (
    <div style={{ position: "absolute", left: (W - w) / 2, top: 320, width: w, height: h, transform: `translateY(${y}px) perspective(2200px) rotateX(6deg) rotateY(${ry}deg) scale(${scale})`, transformOrigin: "50% 60%" }}>
      <div style={{ position: "absolute", inset: 0, borderRadius: 44, overflow: "hidden", background: INK, boxShadow: "0 40px 80px rgba(11,11,12,.25), 0 0 0 2px rgba(11,11,12,.06)" }}>
        {src ? <OffthreadVideo muted src={staticFile(`promo/${src}`)} trimBefore={Math.round(from * FPS)} style={{ width: "100%", height: "100%", objectFit: "cover" }} /> : children}
        <div style={{ position: "absolute", inset: 0, background: `linear-gradient(115deg, transparent ${sweep - 14}%, rgba(255,255,255,.10) ${sweep}%, transparent ${sweep + 14}%)`, pointerEvents: "none" }} />
      </div>
    </div>
  );
};

/// Shot 6, drawn in the storefront's own palette: the tracking sheet, first the ink ocean while the
/// order is being made, then (on the beat at 1.5 s) the map — venue, door, and the courier gliding.
/// Headless WebGL on the render box rasterises nothing, so the real sheet cannot be captured here.
const PAPER = "#0f1c1a", CARD = "#17302b", LINE = "#274640", MUTED = "rgba(241,232,216,.55)";
// The venue's own brand inside the phone: its seal gold, cream ink and serif. The promo chrome never uses them.
const S_GOLD = "#c9a35a", S_INK = "#f1e8d8";
const S_SERIF = '"EB Garamond", "Iowan Old Style", Palatino, Georgia, serif';
const Ocean: React.FC = () => {
  const frame = useCurrentFrame();
  const waves = [0, 1, 2, 3, 4, 5];
  return (
    <svg width={540} height={300} viewBox="0 0 540 300" style={{ position: "absolute", left: 0, top: 0 }}>
      <defs>
        <linearGradient id="ink" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stopColor="#12302c" /><stop offset="1" stopColor="#061110" /></linearGradient>
      </defs>
      <rect width={540} height={300} fill="url(#ink)" />
      {waves.map(i => {
        const amp = 8 + i * 3, y0 = 90 + i * 34, sp = 0.9 + i * 0.25, ph = frame / FPS * sp + i;
        let d = `M0 ${y0}`;
        for (let x = 0; x <= 540; x += 20) d += ` L${x} ${y0 + Math.sin(x / 70 + ph) * amp + Math.sin(x / 31 - ph * 1.7) * amp * 0.35}`;
        d += " L540 300 L0 300 Z";
        return <path key={i} d={d} fill={`rgba(${10 + i * 6}, ${40 + i * 9}, ${38 + i * 8}, ${0.55})`} stroke={i % 2 ? S_GOLD : "rgba(201,163,90,.35)"} strokeWidth={i % 2 ? 1.2 : 0.7} strokeOpacity={0.5 + 0.1 * Math.sin(ph * 2)} />;
      })}
      {Array.from({ length: 18 }, (_, i) => {
        const x = (i * 131) % 540, y = 40 + ((i * 71) % 220), tw = 0.5 + 0.5 * Math.sin(frame / 6 + i);
        return <circle key={i} cx={x} cy={y} r={1.2 + tw} fill={S_GOLD} opacity={0.25 + 0.5 * tw} />;
      })}
    </svg>
  );
};
const InkMap: React.FC<{ t: number }> = ({ t }) => {
  // the route from the venue (bottom left) to the door (top right); the courier rides it
  const venue = { x: 110, y: 240 }, door = { x: 430, y: 70 };
  const mid = { x: 300, y: 250 };
  const k = Math.min(1, Math.max(0, (t - 0.1) / 1.3));
  const bez = (a: number) => { const u = 1 - a; return { x: u * u * venue.x + 2 * u * a * mid.x + a * a * door.x, y: u * u * venue.y + 2 * u * a * mid.y + a * a * door.y }; };
  const c = bez(k * 0.78);
  const streets = [[0, 60, 540, 40], [0, 130, 540, 150], [0, 210, 540, 190], [0, 270, 540, 285], [70, 0, 90, 300], [180, 0, 160, 300], [260, 0, 280, 300], [360, 0, 350, 300], [470, 0, 490, 300]];
  return (
    <svg width={540} height={300} viewBox="0 0 540 300" style={{ position: "absolute", left: 0, top: 0 }}>
      <rect width={540} height={300} fill={PAPER} />
      <path d="M0 300 L0 250 Q60 235 90 300 Z" fill="#12312e" />
      <path d="M450 0 L540 0 L540 120 Q500 90 470 60 Q445 30 450 0 Z" fill="#12312e" />
      {streets.map(([x1, y1, x2, y2], i) => <line key={i} x1={x1} y1={y1} x2={x2} y2={y2} stroke={LINE} strokeWidth={i < 4 ? 3 : 2} />)}
      <path d={`M${venue.x} ${venue.y} Q${mid.x} ${mid.y} ${door.x} ${door.y}`} fill="none" stroke={S_GOLD} strokeWidth={2} strokeDasharray="6 6" strokeOpacity={0.85} />
      <circle cx={venue.x} cy={venue.y} r={16} fill={S_GOLD} /><text x={venue.x} y={venue.y + 6} textAnchor="middle" fontFamily={S_SERIF} fontSize={18} fill="#1a1408">ド</text>
      <g transform={`translate(${door.x} ${door.y})`}><circle r={15} fill="#1d5f7a" stroke="#7fc4dd" strokeWidth={2} /><path d="M-7 3 L0 -5 L7 3 V9 H-7 Z" fill="#e8f4f8" /></g>
      <g transform={`translate(${c.x} ${c.y})`}><circle r={17} fill="#0f8f83" stroke="#9de7dc" strokeWidth={2} /><circle r={5} fill="#e6fff9" /><circle r={26} fill="none" stroke="#0f8f83" strokeOpacity={0.5 - 0.5 * ((t * 1.5) % 1)} strokeWidth={2} transform={`scale(${1 + ((t * 1.5) % 1) * 0.8})`} /></g>
    </svg>
  );
};
const TrackingScreen: React.FC = () => {
  const frame = useCurrentFrame(); const t = frame / FPS;
  const riding = t >= 1.5;
  const step = riding ? 5 : 3;
  const dots = [1, 2, 3, 4, 5, 6];
  const eta = riding ? (t > 2.6 ? "3–5 min" : "6–9 min") : "12–18 min";
  const Chip: React.FC<{ children: React.ReactNode }> = ({ children }) => <div style={{ padding: "10px 16px", borderRadius: 22, background: CARD, color: S_INK, fontFamily: SANS, fontSize: 16, fontWeight: 600, letterSpacing: "0.02em" }}>{children}</div>;
  return (
    <div style={{ position: "absolute", inset: 0, background: PAPER, fontFamily: SANS, color: S_INK }}>
      <div style={{ position: "absolute", left: 0, right: 0, top: 0, height: 4, background: `linear-gradient(90deg, ${S_GOLD}, #e8c77a)` }} />
      <div style={{ position: "absolute", left: 28, top: 44, fontFamily: MONO, fontSize: 12, letterSpacing: "0.22em", color: S_GOLD }}>ORDER · #PROMO0001 · DUBIN & SUSHI</div>
      <div style={{ position: "absolute", left: 22, right: 22, top: 84, height: 88, borderRadius: 16, overflow: "hidden", background: CARD, borderLeft: `4px solid ${S_GOLD}` }}>
        <div style={{ position: "absolute", left: 24, top: 16, fontFamily: S_SERIF, fontSize: 32, color: S_INK }}>{riding ? "On the way" : "Being prepared"}</div>
        <div style={{ position: "absolute", left: 26, top: 62, display: "flex", gap: 10 }}>{dots.map(d => <div key={d} style={{ width: 10, height: 10, borderRadius: 5, background: d <= step ? S_GOLD : "transparent", border: `1.5px solid ${d <= step ? S_GOLD : MUTED}`, opacity: d === step ? 0.7 + 0.3 * Math.sin(frame / 3) : 1 }} />)}</div>
      </div>
      <div style={{ position: "absolute", left: 28, top: 190, fontFamily: MONO, fontSize: 12, letterSpacing: "0.2em", color: S_GOLD }}>STEP {step} OF 6 · UP NEXT: {riding ? "DELIVERED" : "READY"}</div>
      <div style={{ position: "absolute", left: 28, right: 28, top: 214, height: 2, background: LINE }}><div style={{ width: `${(step / 6) * 100}%`, height: 2, background: S_GOLD }} /></div>
      <div style={{ position: "absolute", left: 28, top: 236, display: "flex", gap: 10 }}><Chip>◷ {eta}</Chip><Chip>ALL 1,800</Chip><Chip>Cash</Chip></div>
      <div style={{ position: "absolute", left: 22, right: 22, top: 300, height: 300, borderRadius: 16, overflow: "hidden", border: `1px solid ${LINE}` }}>
        {riding ? <InkMap t={t - 1.5} /> : <Ocean />}
        {!riding && <div style={{ position: "absolute", left: 0, right: 0, top: 118, textAlign: "center", fontFamily: S_SERIF, fontSize: 30, color: S_INK, textShadow: "0 2px 12px rgba(0,0,0,.6)" }}>Being made for you</div>}
      </div>
      <div style={{ position: "absolute", left: 28, top: 612, fontFamily: MONO, fontSize: 11, letterSpacing: "0.2em", color: MUTED }}>{riding ? "● VENUE   ● YOU   ● COURIER" : "● VENUE   ● YOU"}</div>
      <div style={{ position: "absolute", left: 28, top: 660, fontFamily: MONO, fontSize: 12, letterSpacing: "0.22em", color: S_GOLD }}>WHAT PEOPLE SAY</div>
      <div style={{ position: "absolute", left: 28, right: 28, top: 686, fontFamily: S_SERIF, fontSize: 19, lineHeight: 1.35, color: S_INK }}>“Very tasty rolls — if you want them ‘like in Ukraine’, this is the place.”</div>
      <div style={{ position: "absolute", left: 28, top: 770, fontFamily: MONO, fontSize: 12, color: S_GOLD }}>★★★★★ <span style={{ color: MUTED }}>Volo SLD</span></div>
      <div style={{ position: "absolute", left: 28, right: 28, bottom: 36, height: 56, borderRadius: 28, background: CARD, display: "grid", placeItems: "center", fontFamily: MONO, fontSize: 14, letterSpacing: "0.2em", color: S_INK }}>DONE</div>
    </div>
  );
};

/// The counter of the pain: −25 → −30 → −35, one tick per beat, shaking.
const Counter: React.FC = () => {
  const frame = useCurrentFrame();
  const steps = [-25, -30, -35];
  const i = Math.min(steps.length - 1, Math.floor(frame / (FPS * 0.66)));
  const jit = Math.sin(frame * 2.3) * 2;
  const tick = (frame % Math.round(FPS * 0.66)) / (FPS * 0.66);
  return (
    <div style={{ position: "absolute", left: 0, right: 0, top: 640, textAlign: "center", fontFamily: MONO, fontSize: 260, fontWeight: 700, color: HOT, transform: `translate(${jit}px, ${-jit}px) scale(${1 + 0.05 * Math.max(0, 1 - tick * 4)})`, letterSpacing: "-0.04em" }}>{steps[i]}%</div>
  );
};

/// The orange zero: flips in on the first frame.
const Zero: React.FC = () => {
  const frame = useCurrentFrame();
  const flip = interpolate(frame, [0, 8], [90, 0], { extrapolateRight: "clamp", easing: ease });
  return (
    <div style={{ position: "absolute", left: 0, right: 0, top: 600, textAlign: "center", perspective: 1200 }}>
      <div style={{ display: "inline-block", fontFamily: MONO, fontSize: 400, fontWeight: 700, letterSpacing: "-0.08em", transform: `rotateX(${flip}deg)`, color: HOT }}>0%</div>
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
      <svg width={W} height={H} viewBox={`0 0 ${W} ${H}`} style={{ position: "absolute", left: 0, top: 0 }}>
        {labels.map((l, i) => {
          const a = -Math.PI / 2 + (i * 2 * Math.PI) / labels.length;
          const k = spring({ frame: frame - i * 4, fps: FPS, config: { damping: 14, stiffness: 90 } });
          const x = cx + Math.cos(a) * r * k, y = cy + Math.sin(a) * r * k;
          const pulse = ((frame - i * 4) / 24) % 1;
          return <g key={l}><line x1={x} y1={y} x2={cx} y2={cy} stroke={INK} strokeWidth={1.5} strokeOpacity={0.25} /><circle cx={x + (cx - x) * pulse} cy={y + (cy - y) * pulse} r={5} fill={HOT} opacity={k * (1 - pulse)} /></g>;
        })}
      </svg>
      {labels.map((l, i) => {
        const a = -Math.PI / 2 + (i * 2 * Math.PI) / labels.length;
        const k = spring({ frame: frame - i * 4, fps: FPS, config: { damping: 14, stiffness: 90 } });
        const x = cx + Math.cos(a) * r * k, y = cy + Math.sin(a) * r * k;
        return (
          <div key={l} style={{ position: "absolute", left: x - 90, top: y - 26, width: 180, textAlign: "center", opacity: k }}>
            <div style={{ width: 16, height: 16, borderRadius: 8, background: HOT, margin: "0 auto 8px" }} />
            <div style={{ fontFamily: MONO, fontSize: 22, letterSpacing: "0.12em", color: INK, textTransform: "uppercase" }}>{l}</div>
          </div>
        );
      })}
      <div style={{ position: "absolute", left: cx - 70, top: cy - 70, width: 140, height: 140, borderRadius: 70, background: HOT, display: "grid", placeItems: "center", fontFamily: DISPLAY, fontSize: 72, color: BONE }}>d</div>
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
        <g fill="none" stroke={INK} strokeWidth={6} strokeLinecap="round" strokeLinejoin="round" style={{ strokeDasharray: 2200, strokeDashoffset: 2200 * draw }}>
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
        const isOne = i % 2 === 1;
        const digitColor = settle < 1 ? MUTE : (isOne ? HOT : MUTE);
        return <div key={i} style={{ position: "absolute", left: x, top: y, fontFamily: MONO, fontSize: 28, color: digitColor, opacity: 0.25 + 0.5 * settle }}>{ch}</div>;
      })}
    </>
  );
};

const EndCard: React.FC = () => {
  const frame = useCurrentFrame();
  const k = spring({ frame, fps: FPS, config: { damping: 12, stiffness: 120 } });
  const sparks = Array.from({ length: 16 }, (_, i) => i);
  return (
    <>
      <Impact at={0.05} y={750} strength={1.2} />
      {sparks.map(i => {
        const a = (i / sparks.length) * Math.PI * 2, t = Math.min(1, Math.max(0, (frame - 2) / 22));
        const d = 60 + t * 260;
        return <div key={i} style={{ position: "absolute", left: W / 2 + Math.cos(a) * d - 3, top: 750 + Math.sin(a) * d - 3, width: 6, height: 6, borderRadius: 3, background: HOT, opacity: (1 - t) * 0.9 }} />;
      })}
      <div style={{ position: "absolute", left: W / 2 - 110, top: 640, width: 220, height: 220, borderRadius: 110, transform: `scale(${k})`, background: HOT, display: "grid", placeItems: "center", fontFamily: DISPLAY, fontSize: 130, color: BONE }}>d</div>
      <div style={{ position: "absolute", left: 0, right: 0, top: 900, textAlign: "center", fontFamily: DISPLAY, fontSize: 150, fontWeight: 800, letterSpacing: "-0.05em", color: INK, ...rise(frame, 0.2) }}>dowiz</div>
      <div style={{ position: "absolute", left: 0, right: 0, top: 1060, textAlign: "center", fontFamily: MONO, fontSize: 30, letterSpacing: "0.22em", color: HOT, textTransform: "uppercase", ...rise(frame, 0.35) }}>dowiz.org</div>
    </>
  );
};

/// Shot 1, drawn: a craft box on the black stage, its lid closing in the first half second;
/// on the hit (0.9 s) banknotes lift from under the lid, catch, and break into ash and embers.
const KRAFT = "linear-gradient(180deg, #a5824f 0%, #8a6a3d 55%, #6d5230 100%)";
const Notes: React.FC = () => {
  const frame = useCurrentFrame();
  const lid = interpolate(frame, [0, 16], [-62, 0], { extrapolateRight: "clamp", easing: ease });
  const HIT = 27; // 0.9 s
  const notes = Array.from({ length: 14 }, (_, i) => i);
  const ash = Array.from({ length: 60 }, (_, i) => i);
  return (
    <>
      <div style={{ position: "absolute", left: 250, top: 1000, width: 580, height: 330, perspective: 1600, perspectiveOrigin: "50% 0%" }}>
        <div style={{ position: "absolute", left: 0, top: 90, width: 580, height: 240, borderRadius: 18, background: KRAFT, boxShadow: "0 50px 90px rgba(0,0,0,.75), inset 0 -30px 60px rgba(0,0,0,.35)" }} />
        <div style={{ position: "absolute", left: 20, top: 118, width: 540, height: 2, background: "rgba(0,0,0,.35)" }} />
        <div style={{ position: "absolute", left: 200, top: 190, width: 180, height: 44, borderRadius: 4, border: "1px solid rgba(0,0,0,.28)", display: "grid", placeItems: "center", fontFamily: MONO, fontSize: 18, letterSpacing: "0.3em", color: "rgba(0,0,0,.55)" }}>SUSHI</div>
        <div style={{ position: "absolute", left: -6, top: 60, width: 592, height: 70, borderRadius: 14, background: "linear-gradient(180deg, #b6925c, #8f6f41)", transformOrigin: "50% 0%", transform: `rotateX(${lid}deg)`, boxShadow: "0 18px 30px rgba(0,0,0,.5)" }} />
      </div>
      {notes.map(i => {
        const t = Math.max(0, frame - HIT - i * 1.3) / 30;
        if (t <= 0) return null;
        const burn = Math.min(1, Math.max(0, (t - 0.35) / 0.5));
        const seed = ((i * 2654435761) >>> 0) % 1000 / 1000;
        const x = 280 + seed * 500 + Math.sin(t * 6 + i) * 30, y = 1060 - t * (700 + seed * 220);
        const o = Math.max(0, 1 - Math.max(0, t - 0.7) / 0.3);
        return (
          <div key={i} style={{ position: "absolute", left: x, top: y, width: 150, height: 68, borderRadius: 6, opacity: o, transform: `rotate(${seed * 70 - 35 + t * 60}deg) scale(${1 - burn * 0.35})`, background: `linear-gradient(90deg, hsl(${140 - burn * 130}, 45%, ${32 - burn * 10}%), hsl(${140 - burn * 130}, 55%, ${44 - burn * 12}%))`, boxShadow: `inset 0 0 0 4px rgba(255,255,255,.14), 0 0 ${30 * burn}px rgba(255,77,28,${burn})` }}>
            <div style={{ position: "absolute", left: 58, top: 17, width: 34, height: 34, borderRadius: 17, border: "2px solid rgba(255,255,255,.28)" }} />
          </div>
        );
      })}
      {ash.map(i => {
        const t = Math.max(0, frame - HIT - 8 - (i % 12) * 1.2) / 34;
        if (t <= 0) return null;
        const ember = i % 4 === 0;
        const x = 330 + ((i * 53) % 460) + Math.sin(t * 9 + i) * 40, y = 980 - t * 900 - (i % 5) * 20;
        return <div key={i} style={{ position: "absolute", left: x, top: y, width: ember ? 6 : 9, height: ember ? 6 : 9, borderRadius: ember ? 3 : 2, background: ember ? `hsl(${20 - t * 20}, 100%, ${65 - t * 30}%)` : "#3a3128", opacity: Math.max(0, (ember ? 1 : 0.7) - t), boxShadow: ember ? `0 0 ${10 * (1 - t)}px ${HOT}` : "none", transform: `rotate(${t * 300}deg)` }} />;
      })}
    </>
  );
};

/// Shot 12, drawn: a home screen with the venue's icon among muted neighbours; a tap, the icon
/// grows to fill the glass, and the storefront's loader takes over.
const HomeScreen: React.FC<{ tapAt?: number }> = ({ tapAt = 0.55 }) => {
  const frame = useCurrentFrame();
  const tap = Math.max(0, frame - tapAt * FPS);
  const ripple = interpolate(tap, [0, 14], [0, 1], { extrapolateRight: "clamp" });
  const open = interpolate(tap, [10, 34], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp", easing: ease });
  const icons = Array.from({ length: 20 }, (_, i) => i);
  const w = 570, h = 1235, cell = (w - 2 * 46) / 4;
  const ours = 9;
  return (
    <div style={{ position: "absolute", left: (W - w) / 2, top: 320, width: w, height: h, transform: "perspective(2200px) rotateX(6deg)", transformOrigin: "50% 60%" }}>
      <div style={{ position: "absolute", inset: 0, borderRadius: 44, overflow: "hidden", background: "#ffffff", boxShadow: "0 40px 80px rgba(11,11,12,.25), 0 0 0 2px rgba(11,11,12,.06)" }}>
        <div style={{ position: "absolute", left: 0, right: 0, top: 22, textAlign: "center", fontFamily: SANS, fontSize: 22, fontWeight: 600, color: INK }}>9:41</div>
        <div style={{ position: "absolute", left: 46, top: 140, width: w - 92, display: "grid", gridTemplateColumns: "repeat(4, 1fr)", rowGap: 34, opacity: 1 - open }}>
          {icons.map(i => {
            const mine = i === ours;
            return (
              <div key={i} style={{ display: "grid", justifyItems: "center", gap: 8 }}>
                <div style={{ width: cell - 34, height: cell - 34, borderRadius: 22, background: mine ? INK : `hsl(${(i * 47) % 360}, 12%, ${18 + (i % 3) * 4}%)`, boxShadow: mine ? `0 0 0 3px ${HOT}` : "inset 0 0 0 1px rgba(255,255,255,.05)", display: "grid", placeItems: "center", fontFamily: DISPLAY, fontSize: 34, color: mine ? BONE : HOT, letterSpacing: "-0.02em", position: "relative" }}>
                  {mine ? "D&S" : ""}
                  {mine && ripple > 0 && <div style={{ position: "absolute", left: "50%", top: "50%", width: 40, height: 40, marginLeft: -20, marginTop: -20, borderRadius: 20, border: `3px solid ${HOT}`, transform: `scale(${1 + ripple * 3})`, opacity: 1 - ripple }} />}
                </div>
                <div style={{ fontFamily: SANS, fontSize: 15, color: mine ? INK : "rgba(11,11,12,.35)" }}>{mine ? "Dubin & Sushi" : "\u00a0"}</div>
              </div>
            );
          })}
        </div>
        <div style={{ position: "absolute", left: 40, right: 40, bottom: 24, height: 108, borderRadius: 30, background: "rgba(255,255,255,.06)", opacity: 1 - open }} />
        {open > 0 && (
          <div style={{ position: "absolute", inset: 0, opacity: open, transform: `scale(${0.4 + 0.6 * open})`, transformOrigin: "62% 42%" }}>
            <OffthreadVideo muted src={staticFile("promo/store-loader-grid-dish.webm")} style={{ width: "100%", height: "100%", objectFit: "cover" }} />
          </div>
        )}
      </div>
    </div>
  );
};

export const DowizPromo: React.FC<PromoProps> = ({ lang, music, musicInSeconds }) => {
  const t = T[lang] ?? T.en;
  return (
    <Stage>
      <Fonts />
      <Camera>
      {music ? <Audio src={staticFile(`promo/${music}`)} trimBefore={Math.round(musicInSeconds * FPS)} volume={f => interpolate(f, [DURATION_S * FPS - 18, DURATION_S * FPS], [1, 0], { extrapolateLeft: "clamp" })} /> : null}
      {SHOTS.map((s, i) => (
        <Sequence key={i} from={s.start * FPS} durationInFrames={(s.end - s.start) * FPS}>
          <Shot index={i}>
          {i === 0 && <><Notes /><Impact at={0.9} y={1100} strength={0.8} /></>}
          {i === 0 && <Headline text={t.title[0]} size={92} top={220} />}
          {i === 1 && <><Counter /><Caption text={t.title[1]} top={1000} /></>}
          {i === 2 && <><Zero /><Impact at={0} y={820} strength={1.3} /><Caption text={t.title[2]} top={1120} /></>}
          {i === 3 && <><Phone src={s.src!} from={s.from!} rise /><Headline text={t.title[3]} top={120} delay={0.3} /></>}
          {i === 4 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[4]} size={72} top={120} /></>}
          {i === 5 && <><Phone push={0.04}><div style={{ position: "absolute", inset: 0, transform: "scale(1.0556)", transformOrigin: "0 0" }}><div style={{ position: "absolute", left: 0, top: 0, width: 540, height: 1170 }}><TrackingScreen /></div></div></Phone><Headline text={t.title[5]} size={96} top={130} /></>}
          {i === 6 && <><Hub /><Headline text={t.title[6]} size={88} top={220} /></>}
          {i === 7 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[7]} size={80} top={120} /></>}
          {i === 8 && <><Lock /><Impact at={0.75} y={990} strength={0.7} /><Headline text={t.title[8]} size={92} top={220} /><Caption text="ML-KEM-768 · ML-DSA-65" top={1220} delay={0.5} /></>}
          {i === 9 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[9]} size={68} top={120} /></>}
          {i === 10 && <><Phone src={s.src!} from={s.from!} /><Headline text={t.title[10]} size={84} top={120} /></>}
          {i === 11 && <><HomeScreen /><Headline text={t.title[11]} size={92} top={120} delay={0.2} /></>}
          {i === 12 && <><EndCard /><Caption text={t.title[12]} top={1140} delay={0.5} /></>}
          </Shot>
          <Subtitle text={t.sub[i]} />
        </Sequence>
      ))}
      </Camera>
    </Stage>
  );
};
