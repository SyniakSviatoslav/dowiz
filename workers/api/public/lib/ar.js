// See the dish on your own table, at its real size.
//
// WHAT THIS IS FOR, and it is not novelty. The question a customer actually has
// about a photograph of food is HOW BIG IS IT. "Large set" means nothing; a
// picture of a plate means nothing without something beside it for scale. This
// puts the dish on the table in front of them at the size it will arrive.
//
// NO 3D MODELS. A restaurant does not have them and will never make them.
// What it does have, because the menu importer and the photo upload put them
// there, is a photograph and a measurement. So the dish is rendered as a
// textured quad lying flat on a detected surface, sized from the real diameter
// the owner typed. A photograph at the correct scale answers the question; a
// wrong 3D model answers it wrongly.
//
// NO 3D LIBRARY EITHER. three.js is around 600 KB and would be fetched by every
// customer opening a menu, for one textured rectangle. The same reasoning that
// moved maplibre off the courier's first paint applies harder here: this is
// hand-written WebGL, about two hundred lines, loaded only when someone asks
// for it.
//
// ABSENT WHERE UNSUPPORTED. WebXR's immersive-ar is Android Chrome and a
// handful of headsets; iOS Safari has no WebXR. `supported()` answers honestly
// and the caller shows no button at all rather than one that fails on tap.

export async function supported() {
  try {
    return Boolean(navigator.xr) && await navigator.xr.isSessionSupported('immersive-ar');
  } catch { return false; }
}

const VERT = `
attribute vec2 aCorner;
uniform mat4 uProj, uView, uModel;
varying vec2 vUv;
void main() {
  // The quad lies FLAT: its local x/z span the plane, y is zero, so it rests on
  // the surface rather than standing up facing the camera like a billboard.
  vUv = aCorner * vec2(0.5, -0.5) + 0.5;
  gl_Position = uProj * uView * uModel * vec4(aCorner.x, 0.0, aCorner.y, 1.0);
}`;

const FRAG = `
precision mediump float;
uniform sampler2D uTex;
uniform float uAlpha;
varying vec2 vUv;
void main() {
  vec4 c = texture2D(uTex, vUv);
  // A soft round vignette, so a rectangular photograph reads as a plate on the
  // table rather than as a floating postcard.
  float d = distance(vUv, vec2(0.5));
  float edge = smoothstep(0.5, 0.42, d);
  gl_FragColor = vec4(c.rgb, c.a * edge * uAlpha);
}`;

function compile(gl, type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
    throw new Error('shader: ' + gl.getShaderInfoLog(s));
  }
  return s;
}

/// A 4x4 translation, column-major as GL wants it.
function translation(x, y, z) {
  return new Float32Array([1,0,0,0, 0,1,0,0, 0,0,1,0, x,y,z,1]);
}
function scaled(m, s) {
  const o = m.slice();
  o[0] = s; o[5] = s; o[10] = s;
  return o;
}

/// Start an AR session showing one dish.
///
/// `sizeCm` is the dish's real widest dimension. It is REQUIRED: without it
/// there is nothing to show that a photograph alone does not already show, and
/// guessing a size would answer the customer's question wrongly, which is worse
/// than not answering it.
export async function show({ imageUrl, sizeCm, overlay, onEnd, onStatus }) {
  if (!(sizeCm > 0)) throw new Error('no size');

  // The texture is loaded BEFORE the session starts. Entering AR and then
  // staring at an empty room while a photo downloads is the worst order to do
  // this in.
  onStatus?.('loading');
  const img = await new Promise((res, rej) => {
    const i = new Image();
    i.crossOrigin = 'anonymous';
    i.onload = () => res(i);
    i.onerror = () => rej(new Error('image'));
    i.src = imageUrl;
  });

  const canvas = document.createElement('canvas');
  const gl = canvas.getContext('webgl', { xrCompatible: true, alpha: true, antialias: true });
  if (!gl) throw new Error('no webgl');

  const prog = gl.createProgram();
  gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, VERT));
  gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, FRAG));
  gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error('link');
  gl.useProgram(prog);

  const buf = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  // Two triangles, corners at ±1; the model matrix scales them to real metres.
  gl.bufferData(gl.ARRAY_BUFFER,
    new Float32Array([-1,-1, 1,-1, -1,1, -1,1, 1,-1, 1,1]), gl.STATIC_DRAW);
  const aCorner = gl.getAttribLocation(prog, 'aCorner');
  gl.enableVertexAttribArray(aCorner);
  gl.vertexAttribPointer(aCorner, 2, gl.FLOAT, false, 0, 0);

  const tex = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, img);

  const uProj = gl.getUniformLocation(prog, 'uProj');
  const uView = gl.getUniformLocation(prog, 'uView');
  const uModel = gl.getUniformLocation(prog, 'uModel');
  const uAlpha = gl.getUniformLocation(prog, 'uAlpha');
  gl.enable(gl.BLEND);
  gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

  const opts = { requiredFeatures: ['hit-test'] };
  if (overlay) { opts.optionalFeatures = ['dom-overlay']; opts.domOverlay = { root: overlay }; }
  const session = await navigator.xr.requestSession('immersive-ar', opts);
  session.updateRenderState({ baseLayer: new XRWebGLLayer(session, gl) });

  const local = await session.requestReferenceSpace('local');
  const viewer = await session.requestReferenceSpace('viewer');
  const hitSource = await session.requestHitTestSource({ space: viewer });

  // Half the real size, in metres: the quad's corners are at ±1, so this is the
  // scale that makes it span `sizeCm` edge to edge.
  const halfMetres = (sizeCm / 100) / 2;

  let placed = null;        // the matrix where the customer put it
  let reticle = null;       // where it would land right now

  const tap = () => { if (reticle) { placed = reticle; onStatus?.('placed'); } };
  session.addEventListener('select', tap);

  session.addEventListener('end', () => {
    try { hitSource.cancel(); } catch {}
    try { gl.deleteTexture(tex); gl.deleteBuffer(buf); gl.deleteProgram(prog); } catch {}
    onEnd?.();
  });

  onStatus?.('scanning');
  session.requestAnimationFrame(function frame(_t, xrFrame) {
    session.requestAnimationFrame(frame);
    const pose = xrFrame.getViewerPose(local);
    if (!pose) return;

    const hits = xrFrame.getHitTestResults(hitSource);
    if (hits.length) {
      const p = hits[0].getPose(local);
      if (p) {
        reticle = p.transform.matrix;
        if (!placed) onStatus?.('ready');
      }
    }

    const layer = session.renderState.baseLayer;
    gl.bindFramebuffer(gl.FRAMEBUFFER, layer.framebuffer);
    // The camera feed is the background; clearing colour would paint over it.
    gl.clear(gl.DEPTH_BUFFER_BIT);

    const target = placed || reticle;
    if (!target) return;

    for (const view of pose.views) {
      const vp = layer.getViewport(view);
      gl.viewport(vp.x, vp.y, vp.width, vp.height);
      gl.uniformMatrix4fv(uProj, false, view.projectionMatrix);
      gl.uniformMatrix4fv(uView, false, view.transform.inverse.matrix);
      gl.uniformMatrix4fv(uModel, false, scaled(target, halfMetres));
      // Not yet placed: shown faint, so it reads as a preview following the
      // surface rather than as something already on the table.
      gl.uniform1f(uAlpha, placed ? 1.0 : 0.45);
      gl.drawArrays(gl.TRIANGLES, 0, 6);
    }
  });

  return session;
}
