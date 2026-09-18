// The Sea -- a dispersive, developing ocean, drawn in WebGL2.
//
// This is the "Tide over Bedrock" direction, as the storefront's ambient
// layer: one continuous process rather than a set of events. As an order
// matures the sea DEVELOPS the way a wind sea does -- fetch-limited growth
// from weak scattered ripples to a strong aligned swell -- and its colour
// flows along the order's lifecycle, amber to teal to gold, never a jump.
// A touch is a Cauchy-Poisson packet: a ring that spreads at group velocity
// and decays as 1/sqrt(r). Every change is an ease toward a target, so
// nothing on the field ever snaps; the numbers that carry money are on the
// Sheet, and the Sheet never moves with the Sea.
//
// The physics vocabulary (seven wave components with deep-water dispersion
// w = sqrt(g k), a maturing spectrum, a directional spread that narrows as
// the sea aligns) is a rendering of the design language's ζ story, not a
// simulation anyone should read numbers from.
//
//   const sea = createTideSea();
//   sea.init(canvas)             // false when there is no WebGL2
//   sea.setPhase(0..1)           // where the order is in its life
//   sea.setEnergy(0..1)          // how much of it shows (checkout stillness)
//   sea.ripple(x, y, strength)   // a touch, in field units (-1..1)
//   sea.accent('anomaly'|'degrade', ms)
//   sea.setReducedMotion(bool)   // one still frame, no loop

/// Pixel ratio is capped: the field is soft by nature and a 3x phone would
/// spend its battery on detail nobody can see.
const DPR_CAP = 1.4;
/// A screen whose short side is under this many pixels is a phone and gets
/// the smaller particle budget.
const MOBILE_SHORT_SIDE_PX = 700;
const PARTICLES_PHONE = 32_000;
const PARTICLES_DESKTOP = 118_000;
const POINT_SIZE_PHONE = 1.05;
const POINT_SIZE_DESKTOP = 1.25;
/// The trail: how much of the previous frame survives. Reduced motion keeps
/// none, which turns the loop into one still frame.
const TRAIL_KEEP = 0.966;
/// Eases per frame toward each target: the phase moves slowly (a sea takes
/// its time), the energy a little faster.
const PHASE_EASE = 0.012;
const ENERGY_EASE = 0.03;
/// The resting energy: a low ember drift so the page breathes before
/// anything has happened.
const ENERGY_REST = 0.16;
const ENERGY_ANOMALY = 0.5;
const ENERGY_DEGRADE = 0.6;
/// How long an accent holds before the sea settles back.
const ACCENT_MS_DEFAULT = 3400;
/// Ripples: how many the shader keeps, how strong a touch may be, how long one lives.
const RIPPLE_SLOTS = 10;
const RIPPLE_MIN = 0.1;
const RIPPLE_MAX = 0.9;
/// The field-to-clip scale the vertex shader applies, needed to map a page
/// point back into field units.
const FIELD_X = 0.70;
const FIELD_Y = 0.92;
/// Modes the vertex shader understands.
const MODE_CALM = 0;
const MODE_ANOMALY = 4;
const MODE_DEGRADE = 5;
const MODES = { anomaly: MODE_ANOMALY, degrade: MODE_DEGRADE };

const HASH = 'precision highp float;\nfloat hash11(float p){ p=fract(p*0.1031); p*=p+33.33; p*=p+p; return fract(p);}';
const VERT = '#version 300 es\n' + HASH + '\n' + [
  'uniform float u_time,u_energy,u_dpr,u_size,u_alpha,u_phase; uniform int u_mode; uniform vec2 u_res;',
  'uniform vec4 u_rip[10]; uniform int u_ripn;',
  'out vec4 v_col;',
  // continuous colour along the order lifecycle (0..1): many states, no jumps
  'vec3 statusHue(float p){ vec3 a=vec3(0.95,0.68,0.30),b=vec3(0.90,0.52,0.30),c=vec3(0.92,0.44,0.32),d=vec3(0.24,0.68,0.64),e=vec3(0.26,0.74,0.80),f=vec3(0.46,0.86,0.80),g=vec3(0.98,0.80,0.46);',
  ' if(p<0.18)return mix(a,b,p/0.18); if(p<0.36)return mix(b,c,(p-0.18)/0.18); if(p<0.55)return mix(c,d,(p-0.36)/0.19);',
  ' if(p<0.72)return mix(d,e,(p-0.55)/0.17); if(p<0.88)return mix(e,f,(p-0.72)/0.16); return mix(f,g,(p-0.88)/0.12); }',
  'void main(){',
  ' float id=float(gl_VertexID);',
  ' float z=hash11(id*0.00097); float hx=hash11(id*0.0011)*2.0-1.0; float jit=hash11(id*0.0017+3.1)*2.0-1.0;',
  ' vec2 b; b.x=hx*mix(0.62,1.85,z); b.y=mix(0.70,-1.22,pow(z,1.22))+jit*0.024;',
  ' float grow=smoothstep(0.0,1.0,u_phase);',                                       // fetch-limited growth
  ' float breath=0.85+0.15*sin(u_time*0.10+1.3);',
  ' float degrade=(u_mode==5)?clamp(u_energy,0.0,1.0):0.0;',
  ' float amp=breath*mix(0.30,1.30,z)*(1.0-0.72*degrade)*mix(0.40,1.55,grow);',
  ' float warp=sin(b.x*2.1+u_time*0.18)*0.5+sin(b.y*2.6-u_time*0.13)*0.5;',
  ' vec2 wp=vec2(b.x*1.10+warp*0.08, z*3.3+warp*0.05);',
  ' float ang=grow*0.55; wp=mat2(cos(ang),-sin(ang),sin(ang),cos(ang))*wp;',      // the sea aligns as it grows
  ' amp*=0.70+0.30*sin(b.x*0.8+u_time*0.11)*cos(b.y*0.6-u_time*0.075+warp);',
  ' float en=u_energy; float dx=0.0,hgt=0.0,phz; const float G=0.30;',
  ' vec2 D0=vec2(0.19,0.982),D1=vec2(0.52,0.854),D2=vec2(-0.34,0.940),D3=vec2(0.74,0.672),D4=vec2(0.06,0.998),D5=vec2(-0.62,0.785),D6=vec2(0.90,0.436);',
  ' float mat=mix(0.75,1.4,grow), chp=mix(1.15,0.55,grow);',                         // long swell up, chop down
  ' float k;',
  ' k=1.6; phz=dot(D0,wp)*k-u_time*sqrt(G*k);     dx+=0.42*0.150*amp*mat*cos(phz)*D0.x; hgt+=0.150*amp*mat*sin(phz);',
  ' k=2.6; phz=dot(D1,wp)*k-u_time*sqrt(G*k)+1.7; dx+=0.38*0.095*amp*cos(phz)*D1.x; hgt+=0.095*amp*sin(phz);',
  ' k=4.0; phz=dot(D2,wp)*k-u_time*sqrt(G*k)+3.1; dx+=0.34*0.058*amp*cos(phz)*D2.x; hgt+=0.058*amp*sin(phz);',
  ' k=6.1; phz=dot(D3,wp)*k-u_time*sqrt(G*k)+0.6; dx+=0.29*0.035*amp*cos(phz)*D3.x; hgt+=0.035*amp*sin(phz);',
  ' k=9.2; phz=dot(D4,wp)*k-u_time*sqrt(G*k)+4.4; dx+=0.24*0.020*amp*chp*cos(phz)*D4.x; hgt+=0.020*amp*chp*sin(phz);',
  ' k=13.5;phz=dot(D5,wp)*k-u_time*sqrt(G*k)+2.2; hgt+=0.011*amp*chp*sin(phz);',
  ' k=19.0;phz=dot(D6,wp)*k-u_time*sqrt(G*k)+5.5; hgt+=0.007*amp*chp*sin(phz);',
  ' if(u_mode==4){ dx+=(hash11(id+floor(u_time*10.0))*2.0-1.0)*0.016*en; }',
  // a touch: a dispersive packet, envelope at group velocity, 1/sqrt(r) decay
  ' for(int i=0;i<10;i++){ if(i>=u_ripn) break; vec4 R=u_rip[i]; float age=u_time-R.z;',
  '   if(R.w>0.001 && age>0.0 && age<3.6){ float d=distance(b,R.xy); float kk=9.0; float cg=0.5*sqrt(0.30/kk); float front=cg*age*3.0;',
  '     float omega=sqrt(0.30*kk+kk*kk*kk*0.0004); float pk=exp(-pow((d-front)/(0.15+age*0.05),2.0));',
  '     float ring=sin(d*kk-age*omega*3.0)*pk/sqrt(d+0.35)*exp(-age*1.05)*R.w; hgt+=ring*0.10; dx+=(d>1e-4?(b.x-R.x)/d:0.0)*ring*0.045; } }',
  ' vec2 p; p.x=b.x+dx*mix(0.35,0.9,z)*0.6; p.y=b.y+hgt*mix(0.11,0.26,z);',
  ' float t=clamp(hgt*1.7+0.5,0.0,1.0);',
  ' vec3 hue=statusHue(u_phase);',
  ' vec3 col=mix(vec3(0.03,0.075,0.09), hue, t);',                                    // troughs deep, crests in the order's colour
  ' if(u_mode==4 && hash11(id*1.3)<0.40) col=vec3(0.86,0.30,0.60);',                  // anomaly: solo magenta
  ' col=mix(vec3(0.03,0.075,0.09), col, mix(0.34,1.0,z));',                           // deep-water absorption
  ' col=mix(col, vec3(0.42,0.40,0.36), degrade);',
  ' float crest=smoothstep(0.28,0.96,t);',
  ' float aB=(0.075+crest*0.5)*(0.80+0.20*breath)*(1.0+en*0.35)*mix(0.5,1.0,z);',
  ' v_col=vec4(col, aB*u_alpha);',
  ' gl_Position=vec4(p.x*0.70, p.y*0.92, 0.0, 1.0);',
  ' gl_PointSize=(0.75+crest*2.3+en*1.0)*u_dpr*u_size*mix(0.5,1.35,z); }',
].join('\n');
const FRAG = '#version 300 es\nprecision highp float;\nin vec4 v_col; out vec4 o;\nvoid main(){ vec2 d=gl_PointCoord-0.5; float r2=dot(d,d); if(r2>0.25) discard; float a=exp(-r2*8.0)*v_col.a; o=vec4(v_col.rgb*a,a); }';
const QUAD = '#version 300 es\nprecision highp float;\nvoid main(){ vec2 v[3]=vec2[3](vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0)); gl_Position=vec4(v[gl_VertexID],0.0,1.0); }';
const FADE = '#version 300 es\nprecision highp float;\nuniform sampler2D u_prev; uniform vec2 u_res; uniform float u_fade; out vec4 o;\nvoid main(){ vec2 uv=gl_FragCoord.xy/u_res; vec2 c=uv-0.5; uv=0.5+c*0.9986; o=texture(u_prev,uv)*u_fade; }';
const COMP = '#version 300 es\nprecision highp float;\nuniform sampler2D u_src; uniform vec2 u_res; uniform float u_t,u_grow; out vec4 o;\n' +
  'void main(){ vec2 uv=gl_FragCoord.xy/u_res; vec3 c=texture(u_src,uv).rgb; c=c/(c+vec3(0.64)); c=pow(c,vec3(0.85));' +
  ' c+=vec3(0.06,0.045,0.022)*smoothstep(0.50,0.98,uv.y)*0.65;' +
  ' c+=vec3(0.03,0.028,0.02)*smoothstep(0.5,0.0,uv.y)*0.5;' +
  ' float glint=exp(-pow((uv.x-0.5)/0.22,2.0))*(0.5+0.5*sin(uv.y*30.0-u_t*2.2+sin(uv.x*8.0)));' +
  ' c+=vec3(0.95,0.78,0.46)*glint*smoothstep(0.22,0.7,uv.y)*(0.04+u_grow*0.03);' +
  ' float vig=smoothstep(1.45,0.30,length((uv-0.5)*vec2(1.02,1.2))); c*=mix(0.62,1.0,vig);' +
  ' o=vec4(c+vec3(0.02,0.022,0.024),1.0); }';

function shader(gl, type, src){
  const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) { console.warn('tide-sea:', gl.getShaderInfoLog(s)); return null; }
  return s;
}
function program(gl, vs, fs){
  const v = shader(gl, gl.VERTEX_SHADER, vs), f = shader(gl, gl.FRAGMENT_SHADER, fs);
  if (!v || !f) return null;
  const p = gl.createProgram(); gl.attachShader(p, v); gl.attachShader(p, f); gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) { console.warn('tide-sea:', gl.getProgramInfoLog(p)); return null; }
  return p;
}

export function createTideSea(){
  let canvas = null, gl = null;
  let pParts, pFade, pComp, uP = {}, uF = {}, uC = {};
  const texs = [], fbos = [];
  let W = 1, H = 1, DPR = 1, COUNT = PARTICLES_DESKTOP, SIZE = POINT_SIZE_DESKTOP;
  let src = 0, mode = MODE_CALM, modeUntil = 0;
  let energy = ENERGY_REST, energyTarget = ENERGY_REST, phase = 0, phaseTarget = 0;
  let t0 = null, time = 0, raf = 0, running = false, reduced = false;
  const rip = new Float32Array(RIPPLE_SLOTS * 4); let ripW = 0;

  function alloc(){
    W = Math.max(1, Math.floor(canvas.clientWidth * DPR)); H = Math.max(1, Math.floor(canvas.clientHeight * DPR));
    canvas.width = W; canvas.height = H;
    for (let i = 0; i < 2; i++) {
      gl.bindTexture(gl.TEXTURE_2D, texs[i]);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, W, H, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE); gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, fbos[i]); gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, texs[i], 0);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  }

  function frame(ts){
    if (!running) return;
    if (t0 === null) t0 = ts;
    time = (ts - t0) / 1000;
    phase += (phaseTarget - phase) * PHASE_EASE;
    energy += (energyTarget - energy) * ENERGY_EASE;
    if (ts > modeUntil && mode !== MODE_CALM) { mode = MODE_CALM; energyTarget = ENERGY_REST; }
    const dst = src ^ 1;
    gl.viewport(0, 0, W, H); gl.disable(gl.BLEND);
    gl.bindFramebuffer(gl.FRAMEBUFFER, fbos[dst]); gl.useProgram(pFade);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, texs[src]);
    gl.uniform1i(uF.u_prev, 0); gl.uniform2f(uF.u_res, W, H); gl.uniform1f(uF.u_fade, reduced ? 0 : TRAIL_KEEP);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE); gl.useProgram(pParts);
    gl.uniform1f(uP.u_time, time); gl.uniform1f(uP.u_energy, energy); gl.uniform1f(uP.u_dpr, DPR); gl.uniform1f(uP.u_size, SIZE);
    gl.uniform1f(uP.u_alpha, 0.95); gl.uniform1f(uP.u_phase, phase);
    gl.uniform1i(uP.u_mode, mode); gl.uniform2f(uP.u_res, W, H); gl.uniform4fv(uP.u_rip, rip); gl.uniform1i(uP.u_ripn, RIPPLE_SLOTS);
    gl.drawArrays(gl.POINTS, 0, COUNT);
    gl.disable(gl.BLEND); gl.bindFramebuffer(gl.FRAMEBUFFER, null); gl.useProgram(pComp);
    gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, texs[dst]);
    gl.uniform1i(uC.u_src, 0); gl.uniform2f(uC.u_res, W, H); gl.uniform1f(uC.u_t, time); gl.uniform1f(uC.u_grow, phase);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    src = dst;
    if (reduced) { running = false; return; }
    raf = requestAnimationFrame(frame);
  }
  function start(){ if (running) return; running = true; t0 = null; raf = requestAnimationFrame(frame); }
  function stop(){ running = false; cancelAnimationFrame(raf); }

  return {
    /// True when the field is drawing; false leaves the page's own gradient.
    init(el){
      if (gl) return true;
      canvas = el;
      gl = canvas.getContext('webgl2', { antialias: false, alpha: false, premultipliedAlpha: false, powerPreference: 'high-performance' });
      if (!gl) return false;
      pParts = program(gl, VERT, FRAG); pFade = program(gl, QUAD, FADE); pComp = program(gl, QUAD, COMP);
      if (!pParts || !pFade || !pComp) { gl = null; return false; }
      gl.bindVertexArray(gl.createVertexArray());
      texs.push(gl.createTexture(), gl.createTexture()); fbos.push(gl.createFramebuffer(), gl.createFramebuffer());
      const phone = Math.min(innerWidth, innerHeight) < MOBILE_SHORT_SIDE_PX;
      DPR = Math.min(devicePixelRatio || 1, DPR_CAP);
      COUNT = phone ? PARTICLES_PHONE : PARTICLES_DESKTOP; SIZE = phone ? POINT_SIZE_PHONE : POINT_SIZE_DESKTOP;
      alloc();
      for (const n of ['u_time', 'u_energy', 'u_dpr', 'u_size', 'u_alpha', 'u_phase', 'u_mode', 'u_res', 'u_rip', 'u_ripn']) uP[n] = gl.getUniformLocation(pParts, n);
      uF = { u_prev: gl.getUniformLocation(pFade, 'u_prev'), u_res: gl.getUniformLocation(pFade, 'u_res'), u_fade: gl.getUniformLocation(pFade, 'u_fade') };
      uC = { u_src: gl.getUniformLocation(pComp, 'u_src'), u_res: gl.getUniformLocation(pComp, 'u_res'), u_t: gl.getUniformLocation(pComp, 'u_t'), u_grow: gl.getUniformLocation(pComp, 'u_grow') };
      addEventListener('resize', alloc);
      document.addEventListener('visibilitychange', () => { if (document.hidden) stop(); else if (!reduced) start(); });
      start();
      return true;
    },
    setPhase(p){ phaseTarget = Math.max(0, Math.min(1, p)); },
    setEnergy(e){ energyTarget = Math.max(0, Math.min(1, e)); },
    /// A touch at a page point, or at a random point when none is given.
    ripple(x, y, strength){
      if (!gl) return;
      const fx = Number.isFinite(x) ? x : (Math.random() * 2 - 1) / FIELD_X;
      const fy = Number.isFinite(y) ? y : (Math.random() * 2 - 1) / FIELD_Y;
      const o = ripW * 4;
      rip[o] = fx; rip[o + 1] = fy; rip[o + 2] = time; rip[o + 3] = Math.max(RIPPLE_MIN, Math.min(RIPPLE_MAX, strength));
      ripW = (ripW + 1) % RIPPLE_SLOTS;
      if (reduced) { running = false; start(); }
    },
    /// A page point in pixels → field units, for a ripple under a finger.
    fieldPoint(clientX, clientY){
      const r = canvas.getBoundingClientRect();
      return { x: ((clientX - r.left) / r.width * 2 - 1) / FIELD_X, y: (1 - 2 * (clientY - r.top) / r.height) / FIELD_Y };
    },
    accent(kind, ms = ACCENT_MS_DEFAULT){
      const m = MODES[kind]; if (!m) return;
      mode = m; modeUntil = performance.now() + ms;
      energyTarget = m === MODE_ANOMALY ? ENERGY_ANOMALY : ENERGY_DEGRADE;
    },
    setReducedMotion(on){ reduced = !!on; if (reduced) { stop(); running = true; raf = requestAnimationFrame(frame); } else start(); },
    destroy(){ stop(); },
  };
}
