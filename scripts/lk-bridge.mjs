#!/usr/bin/env node
// lk-bridge.mjs — minimal DETERMINISTIC reference bridge for the kernel's
// living_knowledge adapter (scripts/automation NOT involved; this is the
// JSON-over-stdio contract the Rust SubprocessLivingKnowledge speaks).
//
// This is a PROTOCOL-CONFORMANT REFERENCE, not the production engine. It fuses
// a lexical (BM25-style) + title-label signal with a deterministic hash "semantic"
// stand-in so the kernel's RED->GREEN test runs fully OFFLINE (no ONNX, no network).
// The real living-knowledge engine (bge-small + vector cache, on branch
// recover/stash-1-2994e6c8) is plugged in at runtime by setting LK_BRIDGE_CMD to
// its own bridge — the kernel contract does not change.
//
// STDIN : {"files":[{"rel","title","text"}],"query":str,"k":int}
// STDOUT: {"results":[{"id":str,"score":f64}],"abs_confidence":f64}

import { readFileSync } from 'node:fs';

const STOP = new Set('the a an of to in and or for is are be with that this it as on at by from into no not any all before after your you our we they them these those what which who how when where than then so if'.split(' '));
const tok = (t) => (t || '').toLowerCase().split(/[^a-z0-9_]+/).filter((w) => w.length >= 2 && w.length <= 40 && !STOP.has(w));
const stem = (w) => (w.length > 4 && w.endsWith('ing')) ? w.slice(0, -3) : (w.length > 4 && w.endsWith('s')) ? w.slice(0, -1) : w;

// deterministic hash embed (stand-in for the ONNX bge-small vector)
function hashEmbed(text) {
  const v = new Array(64).fill(0);
  for (const w of tok(text)) {
    const s = stem(w);
    let h = 2166136261 >>> 0;
    for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619) >>> 0; }
    v[h % 64] += 1;
  }
  const norm = Math.sqrt(v.reduce((a, b) => a + b * b, 0)) || 1;
  return v.map((x) => x / norm);
}
const cos = (a, b) => { let s = 0; for (let i = 0; i < a.length; i++) s += a[i] * b[i]; return s; };

function bm25(docs, qToks) {
  const N = docs.length;
  const avgdl = docs.reduce((s, d) => s + d.toks.length, 0) / (N || 1);
  const df = new Map();
  for (const d of docs) for (const t of new Set(d.toks)) df.set(t, (df.get(t) || 0) + 1);
  const idf = (t) => Math.log((N - (df.get(t) || 0) + 0.5) / ((df.get(t) || 0) + 0.5) + 1);
  return docs.map((d) => {
    let score = 0;
    for (const t of new Set(qToks)) {
      const f = d.toks.filter((x) => x === t).length;
      if (f === 0) continue;
      score += idf(t) * (f * (1.5 + 1)) / (f + 1.5 * (1 - 0.75 + 0.75 * (d.toks.length / (avgdl || 1))));
    }
    return score;
  });
}

function main() {
  const raw = readFileSync(0, 'utf8');
  const req = JSON.parse(raw);
  const q = String(req.query || '');
  const k = Math.max(1, Number(req.k) || 3);
  const qToks = tok(q).map(stem);
  const qEmb = hashEmbed(q);

  const docs = (req.files || []).map((f) => {
    const summary = (f.title || '').trim();
    const text = `${f.title || ''} ${summary} ${f.text || ''}`;
    return { id: f.rel, toks: tok(text).map(stem), labelToks: new Set(tok(`${f.title} ${summary}`).map(stem)), emb: hashEmbed(text) };
  });

  const bm = bm25(docs, qToks);
  const scored = docs.map((d, i) => {
    const sem = cos(d.emb, qEmb);
    const title = [...d.labelToks].filter((t) => qToks.includes(t)).length / (qToks.length || 1);
    const fused = 0.45 * sem + 0.35 * bm[i] + 0.20 * title;
    return { id: d.id, score: fused };
  });

  const max = Math.max(...scored.map((s) => s.score), 1e-9);
  const results = scored
    .map((s) => ({ id: s.id, score: Number((s.score / max).toFixed(4)) }))
    .sort((a, b) => b.score - a.score || (a.id < b.id ? -1 : 1))
    .slice(0, k);

  const top = results[0] ? results[0].score : 0;
  process.stdout.write(JSON.stringify({ results, abs_confidence: Number(top.toFixed(4)) }));
}

main();
