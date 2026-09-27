// A miniature dowiz tree for the collector tests: five surfaces, three service workers, the
// poll constants, four dictionaries, a lib.rs route table, tests and gates. Every file is
// written from this one table so a test can say exactly what it expects to count.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const dict = extra => `export const T = { sq: { a: 'a', qtyWords: { one: 'nje' } }, en: { a: 'a', b: { c: 'c' }${extra} }, uk: { a: 'a', b: { c: 'c' } }, ru: { a: 'a', b: { c: 'c' } } };\n`;

export const FILES = {
  'workers/api/public/store/index.html': '<link rel="stylesheet" href="/store/s.css"><!-- <script src="/gone.js"></script> -->\n'
    + '<link rel="icon" href="/fav.png"><link rel="modulepreload" href="https://cdn.example/x.js"><script type="module" src="app.js"></script>',
  'workers/api/public/store/s.css': 'body{}',
  'workers/api/public/store/app.js': "import { t } from './i18n.js';\nimport '/lib/ui.js'; // import './commented.js'\n/* import './block.js' */\nconst lazy = () => import('./later.js');\nexport { t };",
  'workers/api/public/store/i18n.js': "import '/store/storage.js';\n" + dict(''),
  'workers/api/public/store/storage.js': 'localStorage.getItem("x"); export const S = 1;',
  'workers/api/public/store/later.js': 'export const L = 1;',
  'workers/api/public/store/track.js': 'const POLL_MS = 12_000;\nconst POLL_SLOW_MS = 60_000;',
  'workers/api/public/lib/ui.js': "export * from './missing.js';",
  'workers/api/public/admin/index.html': '<script type="module" src="/admin/app.js"></script>',
  'workers/api/public/admin/app.js': "import { POLL_MS } from './core.js';\nconst tab = () => import('./more.js');",
  'workers/api/public/admin/core.js': 'export const POLL_MS = 15_000;\nexport const POLL_IDLE_MS = 60_000;',
  'workers/api/public/admin/more.js': 'export const M = "' + 'm'.repeat(100) + '";',
  'workers/api/public/admin/i18n.js': dict(", d: 'd'"),
  'workers/api/public/room/index.html': '<script src="/room/app.js"></script>',
  'workers/api/public/room/app.js': 'const POLL_MS = 5000;',
  'workers/api/public/room/i18n.js': 'export const nothing = 1;',
  'workers/api/public/room/sw.js': "const SHELL = ['/room/', '/room/gone.html'];",
  'workers/api/public/courier/index.html': '<script src="/courier/app.js"></script>',
  'workers/api/public/courier/app.js': 'const wait = S.onShift ? 12_000 : 60_000;',
  'workers/api/public/courier/i18n.js': 'throw new Error("boom at import");',
  'workers/api/public/courier/sw.js': "const SHELL = ['/courier/', '/courier/app.js', '/gone.js'];",
  'workers/api/public/platform/index.html': '<p>landing</p>',
  'workers/api/public/sw.js': "const SHELL_FILES = ['/', '/store/app.js', '/admin/'];\n// const SHELL_X = ['/nope.js'];",
  'workers/api/public/fav.png': 'png',
  'workers/api/public/_headers': '/*\n  x: y',
  'workers/api/public/orphan/dead.bin': 'x'.repeat(50),
  'workers/api/public/named/used.txt': 'u',
  'workers/api/src/lib.rs': 'fn r() { x.get_async("/api/a", a).post_async("/api/b/:id", b).get_async("/notapi", c); } // "/named/used.txt"',
  'kernel/src/a.rs': '#[test]\nfn a() {}\n#[test]\nfn b() {}',
  'crates/dowiz-core/src/b.rs': '#[test] fn c() {}',
  'crates/dowiz-core/target/x.rs': '#[test] fn skipped() {}',
  'tools/x.test.mjs': "test('a', () => {});\n  it('b', () => {});\n// test('c')",
  'spikes/y/z.test.mjs': "test('s', () => {});",
  'e2e/tests/one.mjs': '',
  'e2e/tests/readme.md': '',
  'tools/gates/a.baseline': 'a=1',
  'tools/gates/a.sh': 'echo',
};

export function tree(extra = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'evals-fx-'));
  for (const [rel, body] of Object.entries({ ...FILES, ...extra })) {
    if (body === null) continue;
    const p = path.join(root, rel);
    fs.mkdirSync(path.dirname(p), { recursive: true });
    fs.writeFileSync(p, body);
  }
  return root;
}
