// rank-once.mjs — print the deterministic ranking of ONE query as canonical JSON. eval.mjs spawns this
// in a fresh process to PROVE cross-process determinism (a new process must reproduce the in-process
// ranking byte-for-byte). Offline: the query + corpus chunks must already be in the committed cache.
import { collect } from './ingest.mjs';
import { createRetriever } from './lib/retriever.mjs';
import { createSemanticEmbedder } from './lib/embed-semantic.mjs';

const q = process.env.LK_Q || process.argv.slice(2).join(' ');
const files = collect();
const R = createRetriever(files.map((f) => ({ rel: f.rel, title: f.title, text: f.text })), createSemanticEmbedder());
const r = R.search(q, 5);
process.stdout.write(JSON.stringify({ ids: r.ids, top1: Number(r.top1.toFixed(6)) }));
