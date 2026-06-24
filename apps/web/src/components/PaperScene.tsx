import React, { useEffect, useRef, useState } from 'react';

/**
 * PaperScene — a real-time 3D "paper / comic-book (Moebius)" hero canvas.
 *
 * Path A of the Nomadic-Tribe redesign: a calm Moebius desert/journey vignette
 * built from PRIMITIVES only (layered dune ridges, a low gold sun, a tiny
 * caravan of cone+box silhouettes). All the "paper magic" lives in the
 * post-process, not in geometry detail:
 *   1. INK OUTLINE  — inverted-hull back-face pass (front-face culled), ink-coloured.
 *   2. CEL shading  — lighting quantised to 2–3 flat bands, no specular/PBR.
 *   3. PAPER pass   — procedural grain texture MULTIPLY over toon + screen grain.
 *   4. GRADE        — palette bias, slight desaturation, vignette, subtle CA.
 *
 * Hard rules honoured:
 *   - `three` is dynamic-imported INSIDE this file → it lands in its own chunk,
 *     never the main bundle. Caller additionally React.lazy-imports this module.
 *   - Renders the canvas ONLY when not SSR, WebGL available, and motion allowed.
 *     Any failure (init throw, lost context) → swap to `fallback`, never throw.
 *   - dpr capped at 2; render loop paused when tab hidden or canvas offscreen;
 *     full dispose (geometries/materials/textures/renderer) + cancelAF on unmount.
 *
 * Decorative → aria-hidden. The `fallback` carries the meaning (the SVG hero).
 */

// Brand "paper" palette. This scene legitimately needs the literal hex of the
// paper identity — it is the brand, and these are not theme-overridable here.
const PAPER = {
  sand: '#987654',
  teal: '#49C5B6',
  tealDeep: '#3EA094',
  gold: '#ECD06F',
  ink: '#241F1A',
  cream: '#F4ECDB',
} as const;

function webglAvailable(): boolean {
  try {
    const c = document.createElement('canvas');
    return !!(
      window.WebGLRenderingContext &&
      (c.getContext('webgl2') || c.getContext('webgl') || c.getContext('experimental-webgl'))
    );
  } catch {
    return false;
  }
}

function prefersReducedMotion(): boolean {
  try {
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  } catch {
    return false;
  }
}

// Perf budget: don't pull the ~189KB-gzip three chunk on data-saver or slow links —
// fall back to the lightweight SVG hero instead. (Network Information API; absent → render.)
function saveDataOrSlow(): boolean {
  try {
    const conn = (navigator as unknown as { connection?: { saveData?: boolean; effectiveType?: string } }).connection;
    if (!conn) return false;
    if (conn.saveData) return true;
    return typeof conn.effectiveType === 'string' && /(^|-)2g$/.test(conn.effectiveType);
  } catch {
    return false;
  }
}

export interface PaperSceneProps {
  /** Rendered for SSR / no-WebGL / reduced-motion / any runtime error. */
  fallback: React.ReactNode;
  className?: string;
}

export default function PaperScene({ fallback, className }: PaperSceneProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  // Start by assuming we CANNOT render; flip on only after every guard passes.
  const [useFallback, setUseFallback] = useState(true);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    if (prefersReducedMotion() || !webglAvailable() || saveDataOrSlow()) {
      setUseFallback(true);
      return;
    }

    let disposed = false;
    let cleanup: (() => void) | null = null;

    // Dynamic import keeps three in a separate chunk.
    import('three')
      .then((THREE) => {
        if (disposed || !hostRef.current) return;
        try {
          cleanup = initScene(THREE, hostRef.current);
          setUseFallback(false);
        } catch (err) {
          // Never throw — degrade to the fallback hero.
          // eslint-disable-next-line no-console
          console.warn('[PaperScene] init failed, using fallback', err);
          setUseFallback(true);
        }
      })
      .catch((err) => {
        // eslint-disable-next-line no-console
        console.warn('[PaperScene] three import failed, using fallback', err);
        setUseFallback(true);
      });

    return () => {
      disposed = true;
      try {
        cleanup?.();
      } catch {
        /* dispose must never throw */
      }
    };
  }, []);

  return (
    <div className={className} style={{ position: 'relative', width: '100%' }}>
      {/* The WebGL host. Decorative; the fallback carries meaning. */}
      <div
        ref={hostRef}
        aria-hidden="true"
        style={{
          width: '100%',
          aspectRatio: '6 / 5',
          maxWidth: 360,
          margin: '0 auto',
          display: useFallback ? 'none' : 'block',
        }}
      />
      {/* Fallback hero (also the meaningful content for a11y / no-WebGL). */}
      {useFallback ? fallback : null}
    </div>
  );
}

/**
 * Builds the scene and returns a disposer. Kept as a plain function (not a hook)
 * so the whole thing can be wrapped in try/catch by the caller.
 */
type ThreeNS = typeof import('three');

function initScene(THREE: ThreeNS, host: HTMLElement): () => void {
  const W = () => host.clientWidth || 360;
  const H = () => host.clientHeight || 300;

  const renderer = new THREE.WebGLRenderer({
    antialias: false,
    alpha: true,
    powerPreference: 'low-power',
  });
  const dpr = Math.min(window.devicePixelRatio || 1, 2); // cap dpr
  renderer.setPixelRatio(dpr);
  renderer.setSize(W(), H());
  renderer.setClearColor(0x000000, 0); // transparent — page bg shows through
  host.appendChild(renderer.domElement);
  renderer.domElement.style.width = '100%';
  renderer.domElement.style.height = '100%';
  renderer.domElement.style.display = 'block';

  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(38, W() / H(), 0.1, 100);
  camera.position.set(0, 1.1, 9);
  camera.lookAt(0, 0.6, 0);

  const c = (hex: string) => new THREE.Color(hex);

  // ── Lights (cel shading uses the light direction; bands come from the shader) ──
  const sunLight = new THREE.DirectionalLight(0xffffff, 1.0);
  sunLight.position.set(-3, 4, 5);
  scene.add(sunLight);
  scene.add(new THREE.AmbientLight(0xffffff, 0.35));

  // Track all disposables for a clean teardown.
  const geometries: import('three').BufferGeometry[] = [];
  const materials: import('three').Material[] = [];
  const textures: import('three').Texture[] = [];
  const track = <T extends object>(obj: T): T => {
    if ((obj as any).isBufferGeometry) geometries.push(obj as any);
    else if ((obj as any).isMaterial) materials.push(obj as any);
    else if ((obj as any).isTexture) textures.push(obj as any);
    return obj;
  };

  // ── (3) Paper-grain texture: procedural canvas-2D noise, tiling ──────────────
  const paperTex = makePaperTexture(THREE);
  textures.push(paperTex);

  // ── (2) CEL material factory — quantises N·L into 2–3 flat bands, no specular ─
  function celMaterial(color: import('three').Color): import('three').ShaderMaterial {
    const m = new THREE.ShaderMaterial({
      uniforms: {
        uColor: { value: color },
        uLightDir: { value: sunLight.position.clone().normalize() },
        uPaper: { value: paperTex },
        uAmbient: { value: 0.4 },
        uInk: { value: c(PAPER.ink) },
        uPx: { value: dpr },
      },
      vertexShader: /* glsl */ `
        varying vec3 vNormal;
        varying vec2 vUv;
        void main() {
          vNormal = normalize(normalMatrix * normal);
          vUv = uv;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
      `,
      fragmentShader: /* glsl */ `
        uniform vec3 uColor;
        uniform vec3 uLightDir;
        uniform float uAmbient;
        uniform sampler2D uPaper;
        uniform vec3 uInk;
        uniform float uPx;
        varying vec3 vNormal;
        varying vec2 vUv;

        // One family of parallel ink strokes in screen space. Returns ~0 ON a
        // stroke, ~1 between strokes (dpr-normalised so spacing is stable).
        float strokeMask(vec2 fc, float angle, float freq, float thick) {
          vec2 d = vec2(cos(angle), sin(angle));
          float v = abs(fract(dot(fc, d) * freq) - 0.5) * 2.0;
          return smoothstep(thick, thick + 0.10, v);
        }

        void main() {
          float ndl = max(dot(normalize(vNormal), normalize(uLightDir)), 0.0);
          // 3 flat bands — no smooth ramp, no specular.
          float band = ndl > 0.66 ? 1.0 : (ndl > 0.33 ? 0.72 : 0.5);
          vec3 lit = uColor * (uAmbient + (1.0 - uAmbient) * band);

          // (2b) Moebius cross-hatch: ink strokes accumulate in the shadow bands.
          // The mid band gets one diagonal family; the darkest band crosses a
          // second family for true cross-hatch. Strokes lay ink, not pure black,
          // so the drawn texture stays warm.
          vec2 fc = gl_FragCoord.xy / max(uPx, 1.0);
          float hatch = 0.0;
          if (band < 0.95) hatch += 1.0 - strokeMask(fc, 0.70, 0.045, 0.42);
          if (band < 0.60) hatch += 1.0 - strokeMask(fc, -0.70, 0.045, 0.42);
          hatch = clamp(hatch, 0.0, 1.0);
          lit = mix(lit, uInk, hatch * 0.16);

          // (3) MULTIPLY procedural paper grain over the toon colour.
          vec3 paper = texture2D(uPaper, vUv * 3.0).rgb;
          lit *= mix(vec3(1.0), paper, 0.35);
          gl_FragColor = vec4(lit, 1.0);
        }
      `,
    });
    return track(m);
  }

  // ── (1) INK OUTLINE — inverted hull: back-faces expanded along normal, ink ───
  function inkMaterial(width: number): import('three').ShaderMaterial {
    const m = new THREE.ShaderMaterial({
      uniforms: { uInk: { value: c(PAPER.ink) }, uWidth: { value: width } },
      side: THREE.BackSide, // front-face culled → only the expanded hull shows
      vertexShader: /* glsl */ `
        uniform float uWidth;
        void main() {
          vec3 p = position + normal * uWidth;
          gl_Position = projectionMatrix * modelViewMatrix * vec4(p, 1.0);
        }
      `,
      fragmentShader: /* glsl */ `
        uniform vec3 uInk;
        void main() { gl_FragColor = vec4(uInk, 1.0); }
      `,
    });
    return track(m);
  }

  // Helper: add a mesh + its inverted-hull ink outline (slightly varied width).
  function inkMesh(
    geo: import('three').BufferGeometry,
    color: import('three').Color,
    inkWidth: number,
  ): import('three').Group {
    track(geo);
    const g = new THREE.Group();
    g.add(new THREE.Mesh(geo, celMaterial(color)));
    g.add(new THREE.Mesh(geo, inkMaterial(inkWidth)));
    return g;
  }

  // ── Parallax layers ──────────────────────────────────────────────────────────
  const bg = new THREE.Group(); // sky / sun / far dunes
  const mid = new THREE.Group(); // mid dunes
  const fg = new THREE.Group(); // near dune + caravan
  scene.add(bg, mid, fg);

  // Animated-uniform handles set up below (sun halo + drifting dust motes).
  let haloUniforms: { uTime: { value: number } } | null = null;
  let moteUniforms: { uTime: { value: number } } | null = null;

  // Low gold sun (a flat circle so it reads as a printed disc, not a sphere).
  {
    const sunGeo = new THREE.CircleGeometry(1.5, 48);
    track(sunGeo);
    const sunMat = track(
      new THREE.MeshBasicMaterial({ color: c(PAPER.gold) }),
    ) as import('three').MeshBasicMaterial;
    const sun = new THREE.Mesh(sunGeo, sunMat);
    sun.position.set(-1.4, 1.7, -8);
    bg.add(sun);

    // Soft printed halo behind the disc — a larger additive ring whose alpha
    // falls off radially, so the sun reads as warm light bleeding into paper
    // rather than a hard sticker. Gently breathes via uTime.
    const haloGeo = track(new THREE.CircleGeometry(3.6, 48));
    const haloMat = track(
      new THREE.ShaderMaterial({
        uniforms: { uColor: { value: c(PAPER.gold) }, uTime: { value: 0 } },
        transparent: true,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
        vertexShader: /* glsl */ `
          varying vec2 vUv;
          void main(){ vUv = uv; gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }
        `,
        fragmentShader: /* glsl */ `
          precision mediump float;
          uniform vec3 uColor; uniform float uTime;
          varying vec2 vUv;
          void main(){
            float d = distance(vUv, vec2(0.5));
            // soft radial falloff + a slow breathing of the bloom radius
            float breathe = 0.5 + 0.5 * sin(uTime * 0.5);
            float a = smoothstep(0.5, 0.04, d) * (0.18 + 0.07 * breathe);
            gl_FragColor = vec4(uColor, a);
          }
        `,
      }),
    ) as import('three').ShaderMaterial;
    const halo = new THREE.Mesh(haloGeo, haloMat);
    halo.position.set(-1.4, 1.7, -8.1); // just behind the disc
    bg.add(halo);
    haloUniforms = haloMat.uniforms as { uTime: { value: number } };
  }

  // ── Dust motes: a sparse field of warm specks drifting up through the sky.
  //   Pure GPU — positions animate in the vertex shader from a per-point seed,
  //   so the CPU never loops over them. Soft round alpha, additive, no depth write.
  {
    const COUNT = 46;
    const positions = new Float32Array(COUNT * 3);
    const seeds = new Float32Array(COUNT); // phase/speed seed per mote
    for (let i = 0; i < COUNT; i++) {
      // deterministic-ish spread across the upper scene volume
      positions[i * 3] = (Math.random() - 0.5) * 14;
      positions[i * 3 + 1] = Math.random() * 4.2 - 0.5;
      positions[i * 3 + 2] = -6 + Math.random() * 6;
      seeds[i] = Math.random();
    }
    const moteGeo = track(new THREE.BufferGeometry());
    moteGeo.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    moteGeo.setAttribute('aSeed', new THREE.BufferAttribute(seeds, 1));
    const moteMat = track(
      new THREE.ShaderMaterial({
        uniforms: { uTime: { value: 0 }, uColor: { value: c(PAPER.cream) }, uPx: { value: dpr } },
        transparent: true,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
        vertexShader: /* glsl */ `
          attribute float aSeed;
          uniform float uTime; uniform float uPx;
          varying float vTw;
          void main(){
            vec3 p = position;
            float s = aSeed;
            // slow upward drift that wraps, plus a lateral sway
            p.y += mod(uTime * (0.06 + s * 0.05) + s * 6.0, 5.0);
            p.y = mod(p.y + 1.0, 5.0) - 1.0;
            p.x += sin(uTime * 0.2 + s * 6.28) * 0.3;
            vTw = 0.5 + 0.5 * sin(uTime * (1.0 + s) + s * 10.0); // twinkle
            vec4 mv = modelViewMatrix * vec4(p, 1.0);
            gl_PointSize = (2.0 + s * 3.5) * uPx * (6.0 / -mv.z);
            gl_Position = projectionMatrix * mv;
          }
        `,
        fragmentShader: /* glsl */ `
          precision mediump float;
          uniform vec3 uColor; varying float vTw;
          void main(){
            float d = distance(gl_PointCoord, vec2(0.5));
            float a = smoothstep(0.5, 0.0, d) * (0.10 + 0.16 * vTw);
            gl_FragColor = vec4(uColor, a);
          }
        `,
      }),
    ) as import('three').ShaderMaterial;
    const motes = new THREE.Points(moteGeo, moteMat);
    motes.position.set(0, 0, 0);
    mid.add(motes);
    moteUniforms = moteMat.uniforms as { uTime: { value: number } };
  }

  // Dune ridge factory — a low-poly plane bent into a soft ridge silhouette.
  function dune(width: number, height: number, z: number, color: import('three').Color, ink: number) {
    const geo = new THREE.PlaneGeometry(width, height, 24, 1);
    const pos = geo.attributes.position as import('three').BufferAttribute;
    for (let i = 0; i < pos.count; i++) {
      const x = pos.getX(i);
      const y = pos.getY(i);
      // Ridge profile on the top edge; flat bottom.
      const ridge = Math.cos((x / width) * Math.PI) * 0.5 + Math.sin(x * 1.3) * 0.12;
      pos.setY(i, y + (y > 0 ? ridge : 0));
    }
    geo.computeVertexNormals();
    const g = inkMesh(geo, color, ink);
    g.position.set(0, -1.2, z);
    return g;
  }

  bg.add(dune(22, 3.2, -7, c(PAPER.sand), 0.03));
  mid.add(dune(20, 3.0, -3.5, c(PAPER.tealDeep), 0.04));
  fg.add(dune(18, 3.4, -0.5, c(PAPER.teal), 0.05));

  // Tiny caravan silhouettes on the mid ridge: cone "tents" + box bodies.
  {
    const caravan = new THREE.Group();
    const tentGeo = new THREE.ConeGeometry(0.32, 0.6, 5);
    const bodyGeo = new THREE.BoxGeometry(0.34, 0.36, 0.34);
    const figs: Array<[number, number, number]> = [
      [-1.0, 0, 0.95],
      [0.0, 0, 1.0],
      [1.1, 0, 0.9],
    ];
    figs.forEach(([x, , s]) => {
      const top = inkMesh(tentGeo, c(PAPER.ink), 0.02);
      top.scale.setScalar(s);
      top.position.set(x, 0.3 * s, 0);
      const base = inkMesh(bodyGeo, c(PAPER.sand), 0.02);
      base.scale.setScalar(s);
      base.position.set(x, -0.05 * s, 0);
      caravan.add(top, base);
    });
    caravan.position.set(0.4, -0.55, 0.2);
    mid.add(caravan);
  }

  // ── (1+4) Post-process: render scene to a target, then a fullscreen pass that
  //   adds screen-space grain, vignette, desaturation/palette bias + subtle CA. ─
  const target = new THREE.WebGLRenderTarget(W() * dpr, H() * dpr, {
    minFilter: THREE.LinearFilter,
    magFilter: THREE.LinearFilter,
  });
  textures.push(target.texture);

  const postScene = new THREE.Scene();
  const postCam = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const postGeo = track(new THREE.PlaneGeometry(2, 2));
  const postMat = track(
    new THREE.ShaderMaterial({
      uniforms: {
        uScene: { value: target.texture },
        uPaper: { value: paperTex },
        uTime: { value: 0 },
        uRes: { value: new THREE.Vector2(W(), H()) },
        uCream: { value: c(PAPER.cream) },
      },
      vertexShader: /* glsl */ `
        varying vec2 vUv;
        void main() { vUv = uv; gl_Position = vec4(position.xy, 0.0, 1.0); }
      `,
      fragmentShader: /* glsl */ `
        precision highp float;
        uniform sampler2D uScene;
        uniform sampler2D uPaper;
        uniform float uTime;
        uniform vec2 uRes;
        uniform vec3 uCream;
        varying vec2 vUv;

        float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453); }

        void main() {
          vec2 uv = vUv;
          // (4) subtle chromatic aberration — grows toward the edges.
          vec2 dir = uv - 0.5;
          float ca = dot(dir, dir) * 0.010;
          float r = texture2D(uScene, uv - dir * ca).r;
          float g = texture2D(uScene, uv).g;
          float b = texture2D(uScene, uv + dir * ca).b;
          vec4 col = texture2D(uScene, uv);
          vec3 rgb = vec3(r, g, b);

          // Where the scene is transparent (alpha ~0), fall back to cream paper.
          rgb = mix(uCream, rgb, col.a);

          // (4) slight desaturation toward a faded-print look.
          float luma = dot(rgb, vec3(0.299, 0.587, 0.114));
          rgb = mix(vec3(luma), rgb, 0.82);
          // mild contrast / palette bias toward cream.
          rgb = mix(rgb, rgb * rgb * (3.0 - 2.0 * rgb), 0.18);
          rgb = mix(rgb, uCream, 0.05);

          // (3) screen-space paper grain MULTIPLY + animated print noise.
          vec3 paper = texture2D(uPaper, uv * (uRes / 256.0)).rgb;
          rgb *= mix(vec3(1.0), paper, 0.22);
          float n = hash(uv * uRes + uTime) * 0.06 - 0.03;
          rgb += n;

          // (4) vignette.
          float vig = smoothstep(0.95, 0.35, length(dir) * 1.25);
          rgb *= mix(0.78, 1.0, vig);

          gl_FragColor = vec4(clamp(rgb, 0.0, 1.0), 1.0);
        }
      `,
    }),
  ) as import('three').ShaderMaterial;
  postScene.add(new THREE.Mesh(postGeo, postMat));

  // ── Motion state: pointer parallax + scroll + hold/drag, eased toward target ─
  const pointer = { x: 0, y: 0 }; // target, -1..1
  const eased = { x: 0, y: 0 }; // smoothed
  const drag = { active: false, x: 0, y: 0, startX: 0, startY: 0, ox: 0, oy: 0 };
  let scrollOff = 0;

  const el = renderer.domElement;
  el.style.touchAction = 'pan-y';

  function setPointerFromEvent(e: PointerEvent) {
    const rect = el.getBoundingClientRect();
    pointer.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    pointer.y = ((e.clientY - rect.top) / rect.height) * 2 - 1;
  }
  const onPointerMove = (e: PointerEvent) => {
    if (drag.active) {
      // Dragging nudges parallax further, with damping at the bounds.
      const dx = (e.clientX - drag.startX) / el.clientWidth;
      const dy = (e.clientY - drag.startY) / el.clientHeight;
      drag.x = Math.max(-1.4, Math.min(1.4, drag.ox + dx * 2));
      drag.y = Math.max(-1.0, Math.min(1.0, drag.oy + dy * 2));
    } else {
      setPointerFromEvent(e);
    }
  };
  const onPointerDown = (e: PointerEvent) => {
    drag.active = true;
    drag.startX = e.clientX;
    drag.startY = e.clientY;
    drag.ox = drag.x;
    drag.oy = drag.y;
    try {
      el.setPointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
  };
  const onPointerUp = (e: PointerEvent) => {
    drag.active = false;
    try {
      el.releasePointerCapture(e.pointerId);
    } catch {
      /* ignore */
    }
  };
  const onPointerLeave = () => {
    if (!drag.active) {
      pointer.x = 0;
      pointer.y = 0;
    }
  };
  const onScroll = () => {
    const rect = el.getBoundingClientRect();
    // -1..1 as the hero scrolls through the viewport.
    scrollOff = Math.max(-1, Math.min(1, (rect.top + rect.height / 2) / window.innerHeight - 0.5));
  };

  el.addEventListener('pointermove', onPointerMove);
  el.addEventListener('pointerdown', onPointerDown);
  el.addEventListener('pointerup', onPointerUp);
  el.addEventListener('pointercancel', onPointerUp);
  el.addEventListener('pointerleave', onPointerLeave);
  window.addEventListener('scroll', onScroll, { passive: true });

  // ── Loop control: pause when hidden (tab) or offscreen (IntersectionObserver) ─
  let raf = 0;
  let running = false;
  let visible = true;
  let onScreen = true;
  const clock = new THREE.Clock();

  // Ghibli-weighty ease (easeOutExpo-ish via frame-rate-independent lerp).
  const ease = (a: number, b: number, t: number) => a + (b - a) * (1 - Math.pow(1 - t, 3));

  function frame() {
    if (!running) return;
    const dt = Math.min(clock.getDelta(), 0.05);
    const t = clock.elapsedTime;

    // Target parallax = pointer (or drag) + a little scroll + gentle ambient drift.
    const targetX = (drag.active ? drag.x : pointer.x) + Math.sin(t * 0.25) * 0.12;
    const targetY = (drag.active ? drag.y * 0.6 : pointer.y * 0.6) + scrollOff * 0.4;

    // Spring toward target with weighty easing (no bounce).
    const k = 1 - Math.pow(0.0015, dt); // ~ critically-damped feel
    eased.x = ease(eased.x, targetX, k);
    eased.y = ease(eased.y, targetY, k);

    // Layer offsets: far moves least, near moves most.
    bg.position.x = -eased.x * 0.15;
    bg.position.y = -eased.y * 0.06;
    mid.position.x = -eased.x * 0.45;
    mid.position.y = -eased.y * 0.16;
    fg.position.x = -eased.x * 0.9;
    fg.position.y = -eased.y * 0.28;

    // Gentle ambient breathing of the camera.
    camera.position.y = 1.1 + Math.sin(t * 0.4) * 0.05;
    camera.lookAt(0, 0.6, 0);

    (postMat.uniforms.uTime as { value: number }).value = t;
    if (haloUniforms) haloUniforms.uTime.value = t;
    if (moteUniforms) moteUniforms.uTime.value = t;

    // Pass 1: scene → target. Pass 2: target → screen with the grade pass.
    renderer.setRenderTarget(target);
    renderer.render(scene, camera);
    renderer.setRenderTarget(null);
    renderer.render(postScene, postCam);

    raf = requestAnimationFrame(frame);
  }

  function start() {
    if (running || !visible || !onScreen) return;
    running = true;
    clock.start();
    raf = requestAnimationFrame(frame);
  }
  function stop() {
    running = false;
    if (raf) cancelAnimationFrame(raf);
    raf = 0;
  }

  const onVisibility = () => {
    visible = document.visibilityState !== 'hidden';
    visible && onScreen ? start() : stop();
  };
  document.addEventListener('visibilitychange', onVisibility);

  const io = new IntersectionObserver(
    (entries) => {
      onScreen = entries.some((e) => e.isIntersecting);
      visible && onScreen ? start() : stop();
    },
    { threshold: 0.05 },
  );
  io.observe(host);

  // Resize handling.
  const onResize = () => {
    const w = W();
    const h = H();
    renderer.setSize(w, h);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
    target.setSize(w * dpr, h * dpr);
    (postMat.uniforms.uRes as { value: import('three').Vector2 }).value.set(w, h);
  };
  let ro: ResizeObserver | null = null;
  if (typeof ResizeObserver !== 'undefined') {
    ro = new ResizeObserver(onResize);
    ro.observe(host);
  } else {
    window.addEventListener('resize', onResize);
  }

  // Recover gracefully if the GL context is lost.
  const onContextLost = (e: Event) => {
    e.preventDefault();
    stop();
  };
  el.addEventListener('webglcontextlost', onContextLost as EventListener);

  start();

  // ── Disposer: cancel loop, remove listeners, dispose all GPU resources ───────
  return function dispose() {
    stop();
    document.removeEventListener('visibilitychange', onVisibility);
    window.removeEventListener('scroll', onScroll);
    if (ro) ro.disconnect();
    else window.removeEventListener('resize', onResize);
    io.disconnect();
    el.removeEventListener('pointermove', onPointerMove);
    el.removeEventListener('pointerdown', onPointerDown);
    el.removeEventListener('pointerup', onPointerUp);
    el.removeEventListener('pointercancel', onPointerUp);
    el.removeEventListener('pointerleave', onPointerLeave);
    el.removeEventListener('webglcontextlost', onContextLost as EventListener);

    geometries.forEach((g) => g.dispose());
    materials.forEach((m) => m.dispose());
    textures.forEach((t) => t.dispose());
    target.dispose();
    renderer.dispose();
    if (renderer.domElement.parentNode === host) host.removeChild(renderer.domElement);
  };
}

/** Procedural paper-grain texture via a small canvas-2D noise pass (tiling). */
function makePaperTexture(THREE: ThreeNS): import('three').Texture {
  const size = 256;
  const cv = document.createElement('canvas');
  cv.width = cv.height = size;
  const ctx = cv.getContext('2d')!;
  // Warm cream base.
  ctx.fillStyle = PAPER.cream;
  ctx.fillRect(0, 0, size, size);
  // Speckle grain.
  const img = ctx.getImageData(0, 0, size, size);
  const d = img.data;
  for (let i = 0; i < d.length; i += 4) {
    const n = (Math.random() - 0.5) * 28;
    d[i] = Math.max(0, Math.min(255, (d[i] ?? 0) + n));
    d[i + 1] = Math.max(0, Math.min(255, (d[i + 1] ?? 0) + n));
    d[i + 2] = Math.max(0, Math.min(255, (d[i + 2] ?? 0) + n));
  }
  ctx.putImageData(img, 0, 0);
  // A few faint fibre streaks for a hand-pressed-paper feel.
  ctx.globalAlpha = 0.05;
  ctx.strokeStyle = PAPER.sand;
  for (let i = 0; i < 40; i++) {
    ctx.beginPath();
    const y = Math.random() * size;
    ctx.moveTo(0, y);
    ctx.lineTo(size, y + (Math.random() - 0.5) * 8);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;

  const tex = new THREE.CanvasTexture(cv);
  tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
  return tex;
}
