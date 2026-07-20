/*
 * particle-cloud.js — hand-rolled WebGL2 particle field.
 *
 * Event-visual vocabulary (see docs/design/particle-cloud-2026-07-11):
 *   order_created     → amber burst      (a new order ignites)
 *   courier_assigned  → teal stream      (a courier is pulled into the flow)
 *   delivered         → gold bloom       (terminal success bloom)
 *   dispatch_failed   → blood turbulence (assignment failed to land)
 *   pending_aging     → slow ember drift (order aging while unassigned)
 *
 * No three.js / OGL. No build step, no deps. Plain ES module.
 *
 * API:
 *   const cloud = createParticleCloud();
 *   cloud.init(canvas);                       // singleton canvas, idempotent
 *   cloud.burst(status, count?);             // emit an event burst
 *   cloud.setReducedMotion(bool);            // honour prefers-reduced-motion
 *   cloud.setPalette(status, [r,g,b]);       // optional per-event override
 *   cloud.setPointer(x, y);                   // pointer force (0..1 viewport)
 *   cloud.resize();                           // manual resize
 *   cloud.dispose();
 */
/* eslint-disable local/no-hardcoded-string -- GLSL source + engine strings, not user-facing copy; no i18n runtime in a plain-JS module */
/* eslint-disable max-params -- pushParticle is a per-frame hot path; explicit primitives avoid per-particle allocation */
// Top-level binding so the ESM `export` below resolves (export is hoisted/static).
var createParticleCloud;
(function (root) {
  'use strict';

  // ---- event vocabulary: palette (0..1 rgb) + energy + behaviour --------
  var VOCAB = {
    order_created:    { color: [1.00, 0.64, 0.13], energy: 1.00, burst: 1.4, swirl: 0.2 },
    courier_assigned: { color: [0.18, 0.85, 0.78], energy: 0.78, burst: 1.0, swirl: 1.6 },
    delivered:        { color: [1.00, 0.82, 0.27], energy: 0.92, burst: 1.8, swirl: 0.5 },
    dispatch_failed:  { color: [0.80, 0.10, 0.16], energy: 1.10, burst: 1.2, swirl: 3.4 },
    pending_aging:    { color: [0.95, 0.45, 0.20], energy: 0.30, burst: 0.5, swirl: 0.3 }
  };

  var MAX = 4096;            // hard particle cap (ring buffer)
  var VERT = [
    '#version 300 es',
    'layout(location=0) in vec2 a_seed;',   // xy seed + life packed below
    'layout(location=1) in vec4 a_state;',  // x,y,vx,vy
    'layout(location=2) in vec4 a_meta;',   // life,maxLife,r,g
    'uniform vec2 u_res;',
    'uniform float u_energy;',
    'out float v_life;',
    'out vec3 v_col;',
    'void main(){',
    '  float life = a_meta.x / max(a_meta.y, 0.001);',
    '  v_life = life;',
    '  v_col = vec3(a_meta.z, a_meta.w, 1.0);',
    '  vec2 p = a_state.xy / u_res * 2.0 - 1.0;',
    '  p.y = -p.y;',
    '  gl_Position = vec4(p, 0.0, 1.0);',
    '  gl_PointSize = (2.0 + life * 6.0 * u_energy);',
    '}'
  ].join('\n');

  var FRAG = [
    '#version 300 es',
    'precision mediump float;',
    'in float v_life;',
    'in vec3 v_col;',
    'out vec4 outColor;',
    'void main(){',
    '  vec2 d = gl_PointCoord - 0.5;',
    '  float r = dot(d,d);',
    '  if(r > 0.25) discard;',
    '  float a = (1.0 - r*4.0) * v_life;',
    '  outColor = vec4(v_col, a);',
    '}'
  ].join('\n');

  function compile(gl, type, src) {
    var s = gl.createShader(type);
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      throw new Error('shader: ' + gl.getShaderInfoLog(s));
    }
    return s;
  }

  createParticleCloud = function () {
    var canvas = null, gl = null, prog = null, raf = 0, running = false;
    var reduced = false;
    var px = -1, py = -1;          // pointer in pixels
    var W = 0, H = 0, dpr = 1;
    var n = 0;                     // live particle count
    var head = 0;                  // ring buffer write head
    var last = 0;
    var energy = 1.0;              // global energy damped over time

    // attribute CPU buffers
    var seed = new Float32Array(MAX * 2);
    var state = new Float32Array(MAX * 4);
    var meta = new Float32Array(MAX * 4);
    var bufs = null;

    function resize() {
      if (!canvas) return;
      dpr = Math.min(root.devicePixelRatio || 1, 2);
      W = Math.max(1, Math.floor(canvas.clientWidth * dpr));
      H = Math.max(1, Math.floor(canvas.clientHeight * dpr));
      if (canvas.width !== W || canvas.height !== H) {
        canvas.width = W; canvas.height = H;
      }
      if (gl) gl.viewport(0, 0, W, H);
    }

    function rebuildBuffers() {
      bufs = {
        seed: gl.createBuffer(),
        state: gl.createBuffer(),
        meta: gl.createBuffer()
      };
      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.seed);
      gl.bufferData(gl.ARRAY_BUFFER, seed, gl.DYNAMIC_DRAW);
      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.state);
      gl.bufferData(gl.ARRAY_BUFFER, state, gl.DYNAMIC_DRAW);
      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.meta);
      gl.bufferData(gl.ARRAY_BUFFER, meta, gl.DYNAMIC_DRAW);
    }

    function pushParticle(x, y, vx, vy, life, maxLife, r, g) {
      var i = head;
      head = (head + 1) % MAX;
      if (n < MAX) n++;
      var i2 = i * 2, i4 = i * 4;
      seed[i2] = Math.random(); seed[i2 + 1] = Math.random();
      state[i4] = x; state[i4 + 1] = y; state[i4 + 2] = vx; state[i4 + 3] = vy;
      meta[i4] = life; meta[i4 + 1] = maxLife; meta[i4 + 2] = r; meta[i4 + 3] = g;
    }

    function burst(status, count) {
      if (!gl) return;
      var v = VOCAB[status];
      if (!v) return;
      var c = count || 120;
      c = Math.min(c, MAX - n > 0 ? MAX : 0);
      if (reduced) c = Math.ceil(c * 0.25);   // respect motion preference
      var cx = W * 0.5, cy = H * 0.5;
      energy = Math.min(1.6, energy + v.energy * 0.5);
      for (var k = 0; k < c; k++) {
        var ang = Math.random() * 6.2831853;
        var spd = (0.4 + Math.random()) * v.burst * 60 * dpr;
        var vx = Math.cos(ang) * spd;
        var vy = Math.sin(ang) * spd;
        // swirl bias for courier/dispatch — tangential component
        if (v.swirl > 1.0) {
          vx += -Math.sin(ang) * v.swirl * 40 * dpr;
          vy += Math.cos(ang) * v.swirl * 40 * dpr;
        }
        var life = (0.6 + Math.random() * 0.8) * 1000;
        pushParticle(cx, cy, vx, vy, life, life, v.color[0], v.color[1]);
      }
    }

    function setPalette(status, rgb) {
      if (VOCAB[status]) VOCAB[status].color = rgb;
    }

    function setReducedMotion(b) { reduced = !!b; }

    function setPointer(x, y) {
      px = x * (canvas ? canvas.clientWidth : 1);
      py = y * (canvas ? canvas.clientHeight : 1);
    }

    function step(dt) {
      var damp = Math.pow(0.92, dt / 16.6);
      for (var i = 0; i < n; i++) {
        var i4 = i * 4;
        var x = state[i4], y = state[i4 + 1];
        var vx = state[i4 + 2], vy = state[i4 + 3];
        // pointer repulsion force
        if (px >= 0 && !reduced) {
          var dx = x - px * dpr, dy = y - py * dpr;
          var d2 = dx * dx + dy * dy;
          if (d2 < 9000 * dpr * dpr) {
            var f = (1 - d2 / (9000 * dpr * dpr)) * 0.6;
            var inv = 1 / (Math.sqrt(d2) + 0.001);
            vx += dx * inv * f * 600 * dpr;
            vy += dy * inv * f * 600 * dpr;
          }
        }
        vx *= damp; vy *= damp;
        x += vx * dt / 16.6;
        y += vy * dt / 16.6;
        var life = meta[i4] - dt;
        state[i4] = x; state[i4 + 1] = y;
        state[i4 + 2] = vx; state[i4 + 3] = vy;
        meta[i4] = life;
        if (life <= 0) {
          // swap-remove with last live particle
          var j = n - 1;
          if (j !== i) {
            var j4 = j * 4, j2 = j * 2;
            seed[i * 2] = seed[j2]; seed[i * 2 + 1] = seed[j2 + 1];
            state[i4] = state[j4]; state[i4 + 1] = state[j4 + 1];
            state[i4 + 2] = state[j4 + 2]; state[i4 + 3] = state[j4 + 3];
            meta[i4] = meta[j4]; meta[i4 + 1] = meta[j4 + 1];
            meta[i4 + 2] = meta[j4 + 2]; meta[i4 + 3] = meta[j4 + 3];
          }
          n--; i--;
        }
      }
      energy *= Math.pow(0.95, dt / 16.6);
    }

    function draw() {
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
      if (n === 0) return;
      gl.useProgram(prog);
      gl.uniform2f(gl.getUniformLocation(prog, 'u_res'), W, H);
      gl.uniform1f(gl.getUniformLocation(prog, 'u_energy'), energy);

      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.seed);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, seed.subarray(0, n * 2));
      gl.enableVertexAttribArray(0);
      gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.state);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, state.subarray(0, n * 4));
      gl.enableVertexAttribArray(1);
      gl.vertexAttribPointer(1, 4, gl.FLOAT, false, 0, 0);

      gl.bindBuffer(gl.ARRAY_BUFFER, bufs.meta);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, meta.subarray(0, n * 4));
      gl.enableVertexAttribArray(2);
      gl.vertexAttribPointer(2, 4, gl.FLOAT, false, 0, 0);

      gl.enable(gl.BLEND);
      gl.blendFunc(gl.SRC_ALPHA, gl.ONE);   // additive glow
      gl.drawArrays(gl.POINTS, 0, n);
    }

    function frame(t) {
      if (!running) return;
      if (!last) last = t;
      var dt = Math.min(64, t - last);
      last = t;
      resize();
      step(dt);
      draw();
      // stop the loop when idle to save battery, auto-resume on burst
      if (n === 0 && energy < 0.02) {
        running = false; raf = 0; return;
      }
      raf = root.requestAnimationFrame(frame);
    }

    function ensureRunning() {
      if (!running) {
        running = true; last = 0;
        raf = root.requestAnimationFrame(frame);
      }
    }

    function init(cv) {
      if (!cv) throw new Error('particle-cloud: canvas required');
      if (canvas === cv && gl) return;       // idempotent singleton
      canvas = cv;
      gl = canvas.getContext('webgl2', { alpha: true, premultipliedAlpha: false, antialias: false });
      if (!gl) { console.warn('particle-cloud: WebGL2 unavailable'); return; }
      var vs = compile(gl, gl.VERTEX_SHADER, VERT);
      var fs = compile(gl, gl.FRAGMENT_SHADER, FRAG);
      prog = gl.createProgram();
      gl.attachShader(prog, vs); gl.attachShader(prog, fs);
      gl.linkProgram(prog);
      if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
        throw new Error('link: ' + gl.getProgramInfoLog(prog));
      }
      resize();
      rebuildBuffers();
      gl.disable(gl.DEPTH_TEST);
      var onMove = function (e) {
        var r = canvas.getBoundingClientRect();
        setPointer((e.clientX - r.left) / r.width, (e.clientY - r.top) / r.height);
      };
      var onLeave = function () { px = -1; py = -1; };
      canvas.addEventListener('pointermove', onMove);
      canvas.addEventListener('pointerleave', onLeave);
      ensureRunning();
    }

    function dispose() {
      running = false;
      if (raf) root.cancelAnimationFrame(raf);
      raf = 0; n = 0; head = 0;
      if (gl && prog) {
        gl.deleteProgram(prog); prog = null;
        if (bufs) { gl.deleteBuffer(bufs.seed); gl.deleteBuffer(bufs.state); gl.deleteBuffer(bufs.meta); }
      }
      gl = null; canvas = null;
    }

    return {
      init,
      burst,
      setReducedMotion,
      setPalette,
      setPointer,
      resize,
      dispose,
      VOCAB
    };
  }

  // ESM named export (works as `import { createParticleCloud }` in type:module).
  // Guarded global fallback for classic <script> (non-module) usage.
  if (typeof globalThis !== 'undefined') globalThis.createParticleCloud = createParticleCloud;
  if (typeof window !== 'undefined') window.createParticleCloud = createParticleCloud;
})(typeof self !== 'undefined' ? self : (typeof globalThis !== 'undefined' ? globalThis : this));

// ESM default export target — must be a top-level statement (not inside the IIFE).
export { createParticleCloud };
