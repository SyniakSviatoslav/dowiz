// A minimal DOM for the ui tests: enough HTML parsing, selector matching and
// event dispatch to render a component's string, find things in it, and run a
// binder against it. NOT a browser: no layout, no CSS, no CSP. What it checks
// is structure and behaviour -- the parts a string test cannot see.
//
// Supported selectors: tag, #id, .class, [attr], [attr="v"], :not([attr]) and
// :not([attr="v"]) compounds, the descendant combinator, and comma lists.

const VOID = new Set(['area', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'source', 'wbr']);
const ENT = { amp: '&', lt: '<', gt: '>', quot: '"', '#39': "'", apos: "'" };
const decode = s => s.replace(/&(amp|lt|gt|quot|#39|apos);/g, (_, e) => ENT[e]);
const enc = s => s.replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

class Node {
  constructor(doc){ this.ownerDocument = doc; this.parentNode = null; this.childNodes = []; this._l = {}; }
  appendChild(n){ if (n.parentNode) n.remove(); n.parentNode = this; this.childNodes.push(n); return n; }
  remove(){ const p = this.parentNode; if (p) { p.childNodes = p.childNodes.filter(c => c !== this); this.parentNode = null; } }
  contains(n){ for (let x = n; x; x = x.parentNode) if (x === this) return true; return false; }
  get textContent(){ return this.childNodes.map(c => c.textContent).join(''); }
  set textContent(v){ this.childNodes = []; this.appendChild(new Text(this.ownerDocument, String(v))); }
  addEventListener(t, f){ (this._l[t] ||= []).push(f); }
  removeEventListener(t, f){ this._l[t] = (this._l[t] || []).filter(x => x !== f); }
  dispatchEvent(e){
    e.target ||= this;
    for (let n = this; n && !e._stop; n = n.parentNode) for (const f of [...(n._l[e.type] || [])]) f.call(n, e);
    return !e.defaultPrevented;
  }
}
class Text extends Node {
  constructor(doc, s){ super(doc); this.data = s; }
  get textContent(){ return this.data; }
  get outerHTML(){ return enc(this.data); }
}
export class Element extends Node {
  constructor(doc, tag){ super(doc); this.tagName = tag.toUpperCase(); this.attributes = new Map(); }
  getAttribute(k){ return this.attributes.has(k) ? this.attributes.get(k) : null; }
  setAttribute(k, v){ this.attributes.set(k, String(v)); }
  removeAttribute(k){ this.attributes.delete(k); }
  hasAttribute(k){ return this.attributes.has(k); }
  get id(){ return this.getAttribute('id') || ''; }
  set id(v){ this.setAttribute('id', v); }
  get className(){ return this.getAttribute('class') || ''; }
  set className(v){ this.setAttribute('class', v); }
  get classList(){
    const get = () => this.className.split(/\s+/).filter(Boolean), set = a => { this.className = a.join(' '); };
    return { contains: c => get().includes(c), add: (...c) => set([...new Set([...get(), ...c])]),
             remove: (...c) => set(get().filter(x => !c.includes(x))),
             toggle: (c, on) => { const has = get().includes(c); const want = on ?? !has; want ? set([...get().filter(x => x !== c), c]) : set(get().filter(x => x !== c)); return want; } };
  }
  get dataset(){
    const el = this;
    return new Proxy({}, { get: (_, k) => el.getAttribute('data-' + String(k).replace(/[A-Z]/g, m => '-' + m.toLowerCase())) ?? undefined });
  }
  get hidden(){ return this.hasAttribute('hidden'); }
  set hidden(v){ v ? this.setAttribute('hidden', '') : this.removeAttribute('hidden'); }
  get disabled(){ return this.hasAttribute('disabled'); }
  set disabled(v){ v ? this.setAttribute('disabled', '') : this.removeAttribute('disabled'); }
  get children(){ return this.childNodes.filter(c => c instanceof Element); }
  get innerHTML(){ return this.childNodes.map(c => c.outerHTML).join(''); }
  set innerHTML(html){ this.childNodes = []; for (const n of parse(this.ownerDocument, html)) this.appendChild(n); }
  get outerHTML(){
    const a = [...this.attributes].map(([k, v]) => v === '' ? ` ${k}` : ` ${k}="${enc(v)}"`).join('');
    const tag = this.tagName.toLowerCase();
    return VOID.has(tag) ? `<${tag}${a}>` : `<${tag}${a}>${this.innerHTML}</${tag}>`;
  }
  focus(){ this.ownerDocument.activeElement = this; }
  closest(sel){ for (let n = this; n instanceof Element; n = n.parentNode) if (matches(n, sel)) return n; return null; }
  matches(sel){ return matches(this, sel); }
  querySelectorAll(sel){ const out = []; walk(this, n => { if (n !== this && matches(n, sel, this)) out.push(n); }); return out; }
  querySelector(sel){ return this.querySelectorAll(sel)[0] || null; }
}
function walk(n, f){ for (const c of n.childNodes) if (c instanceof Element) { f(c); walk(c, f); } }

function parse(doc, html){
  const root = new Element(doc, '#frag');
  let cur = root, i = 0;
  const re = /<!--[\s\S]*?-->|<\/([a-zA-Z0-9-]+)\s*>|<([a-zA-Z0-9-]+)((?:\s+[^\s=>\/]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+))?)*)\s*\/?>/g;
  let m;
  while ((m = re.exec(html))) {
    if (m.index > i) cur.appendChild(new Text(doc, decode(html.slice(i, m.index))));
    i = re.lastIndex;
    if (m[0].startsWith('<!--')) continue;
    if (m[1]) { for (let n = cur; n !== root; n = n.parentNode) if (n.tagName === m[1].toUpperCase()) { cur = n.parentNode; break; } continue; }
    const el = new Element(doc, m[2]);
    for (const a of m[3].matchAll(/([^\s=>\/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?/g))
      el.setAttribute(a[1], decode(a[2] ?? a[3] ?? a[4] ?? ''));
    cur.appendChild(el);
    if (!VOID.has(m[2].toLowerCase())) cur = el;
  }
  if (i < html.length) cur.appendChild(new Text(doc, decode(html.slice(i))));
  const kids = root.childNodes; for (const k of kids) k.parentNode = null;
  return kids;
}

function simple(el, s){
  const nots = [];
  s = s.replace(/:not\(([^)]*)\)/g, (_, x) => { nots.push(x); return ''; });
  const tag = s.match(/^[a-zA-Z][a-zA-Z0-9-]*/);
  if (tag && el.tagName !== tag[0].toUpperCase()) return false;
  for (const [, id] of s.matchAll(/#([\w-]+)/g)) if (el.id !== id) return false;
  for (const [, c] of s.matchAll(/\.([\w-]+)/g)) if (!el.classList.contains(c)) return false;
  for (const [, k, v] of s.matchAll(/\[([\w:-]+)(?:=["']?([^"'\]]*)["']?)?\]/g)) {
    if (!el.hasAttribute(k)) return false;
    if (v !== undefined && el.getAttribute(k) !== v) return false;
  }
  return nots.every(n => !simple(el, n));
}
function matches(el, sel, scope){
  return sel.split(',').some(part => {
    const chain = part.trim().split(/\s+/);
    if (!simple(el, chain[chain.length - 1])) return false;
    let n = el.parentNode;
    for (let k = chain.length - 2; k >= 0; k--) {
      while (n instanceof Element && n !== scope && !simple(n, chain[k])) n = n.parentNode;
      if (!(n instanceof Element) || n === scope) return false;
      n = n.parentNode;
    }
    return true;
  });
}

export class Document extends Node {
  constructor(){ super(null); this.ownerDocument = this; this.body = new Element(this, 'body'); this.appendChild(this.body); this.activeElement = this.body; }
  createElement(t){ return new Element(this, t); }
  getElementById(id){ return this.body.querySelector('#' + id); }
  querySelector(s){ return this.body.querySelector(s); }
  querySelectorAll(s){ return this.body.querySelectorAll(s); }
}

/// Render markup into a fresh document; returns { doc, root } where root is a
/// <div> holding the markup (attached to body, so ids resolve).
export function render(html){
  const doc = new Document(), root = doc.createElement('div');
  root.innerHTML = html;
  doc.body.appendChild(root);
  return { doc, root };
}

/// An event with preventDefault / stopPropagation, dispatched on `el`.
export function fire(el, type, props = {}){
  const e = { type, target: el, defaultPrevented: false, _stop: false, ...props,
              preventDefault(){ this.defaultPrevented = true; }, stopPropagation(){ this._stop = true; } };
  el.dispatchEvent(e);
  return e;
}

/// The payload every component test feeds through every text input. If any of
/// it survives as markup -- an <img>, an `onerror` attribute, a broken-out
/// attribute -- the component is an XSS hole.
export const XSS = `<img src=x onerror="alert(1)">"'&<script>x</script>`;
export function injected(html){
  const { root } = render(html);
  const bad = [];
  if (root.querySelector('img')) bad.push('img element');
  if (root.querySelector('script')) bad.push('script element');
  walk(root, n => { for (const k of n.attributes.keys()) if (/^on/i.test(k)) bad.push(`${k} attribute on ${n.tagName}`); });
  return bad;
}
