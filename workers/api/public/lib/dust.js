// Gold dust -- the storefront's resting ambience, in a 2D canvas.
//
// A dark velvet paper needs to breathe without asking for attention: a few
// dozen motes of the venue's accent drift upward, sway a little, and
// twinkle, the way dust does in a shaft of light. It is cheap enough for the
// oldest phone in Durrës (one 2D canvas, sixty small circles) and it is not
// the Sea: the Sea, the developing ocean, is the order's own field and
// appears when there is an order to wait for.
//
//   const dust = createDust();
//   dust.init(canvas, { colour: '#c9a35a' });
//   dust.spark(x, y, n)      // a small rise of motes from a point (page px)
//   dust.setEnergy(0..1)     // how many motes are lit; checkout stills it
//   dust.setReducedMotion(bool)
//   dust.destroy()

/// Motes on screen, and how many more a phone leaves out.
const MOTES_DESKTOP = 70;
const MOTES_PHONE = 44;
const MOBILE_SHORT_SIDE_PX = 700;
/// Pixel ratio cap: soft dots do not need a 3x screen.
const DPR_CAP = 1.5;
/// A mote's radius in CSS pixels, and its climb per second.
const RADIUS_MIN = 0.6;
const RADIUS_MAX = 1.9;
const RISE_MIN = 6;
const RISE_MAX = 18;
/// Sway: amplitude in pixels and period in seconds.
const SWAY_PX = 14;
const SWAY_S_MIN = 5;
const SWAY_S_MAX = 11;
/// Twinkle period in seconds, and the floor of a mote's brightness.
const TWINKLE_S_MIN = 2.2;
const TWINKLE_S_MAX = 4.8;
const GLOW_MIN = 0.12;
/// A spark: how fast its motes rise and how long they live.
const SPARK_RISE = 90;
const SPARK_LIFE_S = 1.6;
/// The energy eases toward its target at this rate per second.
const ENERGY_EASE = 2.0;
/// Frame budget: no more than thirty a second; the dust is slow by nature.
const FRAME_MIN_MS = 33;

function hexToRgb(hex){
  const n = parseInt(String(hex).replace('#', ''), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

export function createDust(){
  let canvas = null, ctx = null, W = 0, H = 0, DPR = 1;
  let motes = [], sparks = [];
  let rgb = [201, 163, 90];
  let energy = 1, energyTarget = 1;
  let last = 0, raf = 0, running = false, reduced = false;

  function seed(m){
    m.x = Math.random() * W; m.y = Math.random() * H;
    m.r = RADIUS_MIN + Math.random() * (RADIUS_MAX - RADIUS_MIN);
    m.rise = RISE_MIN + Math.random() * (RISE_MAX - RISE_MIN);
    m.swayS = SWAY_S_MIN + Math.random() * (SWAY_S_MAX - SWAY_S_MIN);
    m.twS = TWINKLE_S_MIN + Math.random() * (TWINKLE_S_MAX - TWINKLE_S_MIN);
    m.ph = Math.random() * Math.PI * 2;
    return m;
  }
  function alloc(){
    DPR = Math.min(devicePixelRatio || 1, DPR_CAP);
    W = canvas.clientWidth; H = canvas.clientHeight;
    canvas.width = Math.max(1, Math.floor(W * DPR)); canvas.height = Math.max(1, Math.floor(H * DPR));
    ctx.setTransform(DPR, 0, 0, DPR, 0, 0);
    const n = Math.min(innerWidth, innerHeight) < MOBILE_SHORT_SIDE_PX ? MOTES_PHONE : MOTES_DESKTOP;
    motes = Array.from({ length: n }, () => seed({}));
  }
  function draw(t, dt){
    ctx.clearRect(0, 0, W, H);
    energy += (energyTarget - energy) * Math.min(1, ENERGY_EASE * dt);
    const lit = Math.round(motes.length * energy);
    for (let i = 0; i < lit; i++) {
      const m = motes[i];
      m.y -= m.rise * dt;
      if (m.y < -m.r * 2) { seed(m); m.y = H + m.r * 2; }
      const x = m.x + Math.sin(t / m.swayS + m.ph) * SWAY_PX;
      const glow = GLOW_MIN + (1 - GLOW_MIN) * (0.5 + 0.5 * Math.sin(t / m.twS * Math.PI * 2 + m.ph));
      ctx.fillStyle = `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${glow.toFixed(3)})`;
      ctx.beginPath(); ctx.arc(x, m.y, m.r, 0, Math.PI * 2); ctx.fill();
    }
    sparks = sparks.filter(s => s.age < SPARK_LIFE_S);
    for (const s of sparks) {
      s.age += dt;
      const a = 1 - s.age / SPARK_LIFE_S;
      s.y -= SPARK_RISE * dt; s.x += s.vx * dt;
      ctx.fillStyle = `rgba(${rgb[0]},${rgb[1]},${rgb[2]},${(a * 0.9).toFixed(3)})`;
      ctx.beginPath(); ctx.arc(s.x, s.y, s.r * (0.6 + a * 0.6), 0, Math.PI * 2); ctx.fill();
    }
  }
  function frame(ts){
    if (!running) return;
    if (ts - last >= FRAME_MIN_MS) {
      const dt = last ? Math.min(0.1, (ts - last) / 1000) : 0;
      last = ts; draw(ts / 1000, dt);
    }
    if (reduced) { running = false; return; }
    raf = requestAnimationFrame(frame);
  }
  const start = () => { if (running) return; running = true; last = 0; raf = requestAnimationFrame(frame); };
  const stop = () => { running = false; cancelAnimationFrame(raf); };
  const onResize = () => alloc();
  const onVisible = () => { if (document.hidden) stop(); else if (!reduced) start(); };

  return {
    init(el, { colour } = {}){
      canvas = el; ctx = canvas.getContext('2d');
      if (!ctx) return false;
      if (colour) rgb = hexToRgb(colour);
      alloc();
      addEventListener('resize', onResize);
      document.addEventListener('visibilitychange', onVisible);
      start();
      return true;
    },
    setColour(colour){ rgb = hexToRgb(colour); },
    setEnergy(e){ energyTarget = Math.max(0, Math.min(1, e)); },
    spark(x, y, n){
      const r = canvas.getBoundingClientRect();
      for (let i = 0; i < n; i++) sparks.push({ x: x - r.left + (Math.random() - 0.5) * 24, y: y - r.top, vx: (Math.random() - 0.5) * 40, r: RADIUS_MIN + Math.random() * RADIUS_MAX, age: Math.random() * 0.3 });
      if (reduced) { running = false; start(); }
    },
    setReducedMotion(on){ reduced = !!on; if (reduced) { stop(); running = true; raf = requestAnimationFrame(frame); } else start(); },
    destroy(){ stop(); removeEventListener('resize', onResize); document.removeEventListener('visibilitychange', onVisible); },
  };
}
