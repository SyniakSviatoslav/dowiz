import { readFile } from 'node:fs/promises';
import init, { FieldSim, knowledge_map } from './pkg/dowiz_wasm.js';

// node's fetch() does NOT support file:// URLs, so hand the bytes to init()
// instead of relying on the browser's relative-URL fetch path.
const bytes = await readFile(new URL('./pkg/dowiz_wasm_bg.wasm', import.meta.url));
await init({ module_or_path: bytes });

const assert = (c, m) => { if (!c) { console.error('ASSERT FAIL:', m); process.exit(1); } };

// 1) FieldSim live loop headless: 4 circles, 64x64
const sim = new FieldSim(new Float64Array([16,16,8, 48,16,8, 16,48,8, 48,48,8]), 64, 64);
for (let i = 0; i < 30; i++) sim.step();
const rgba = sim.frame();
assert(rgba.length === 64 * 64 * 4, 'frame dims');
assert(sim.width() === 64 && sim.height() === 64, 'dims');

// 2) knowledge_map returns a tag-grouped map string
const docs = JSON.stringify([
  { id: 'w1', title: 'Wave6',   tags: ['render', 'wasm'],      path: 'w6.md' },
  { id: 'w2', title: 'Wave7',   tags: ['render', 'knowledge'], path: 'w7.md' },
  { id: 'w3', title: 'SelfMod', tags: ['knowledge', 'autonomy'], path: 'sm.md' },
]);
const map = knowledge_map(docs);
assert(typeof map === 'string' && map.includes('##'), 'knowledge map groups by tag');

console.log('W12 SMOKE PASS: frame=' + rgba.length + 'b, map=' + map.length + 'ch');
