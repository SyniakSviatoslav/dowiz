// The way in: the owner's Venue menu carries the money sheet and the payroll
// tab, a member of staff sees neither, a tap opens each, and every word either
// screen draws exists in uk, sq, en and ru.
// `node --test workers/api/public/admin/finance-menu.test.mjs`
import { page, tick } from './dom_test.harness.mjs';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const A = await import('./access.js');
const FW = (await import('./finance-i18n.js')).WORDS;
const PW = (await import('./payroll-i18n.js')).WORDS;
const LANGS = ['uk', 'sq', 'en', 'ru'];
const read = name => readFileSync(new URL(name, import.meta.url), 'utf8');

// The Venue rows as more.js declares them, read from its source (more.js itself
// needs the whole console to load): [group, [[key, icon, opener], ...]].
const MORE = read('more.js');
const GROUPS = [...MORE.split('const GROUPS = [')[1].split('\n];')[0].matchAll(/\['(\w+)', \[((?:.|\n)*?)\]\]/g)]
  .map(m => [m[1], [...m[2].matchAll(/\['(\w+)', '([\w-]+)', (open\w+)\]/g)].map(r => [r[1], r[2], r[3]])]);
const rowOf = key => GROUPS.flatMap(([, rows]) => rows).find(r => r[0] === key);

test('the owner finds Money and Payroll in the menu, and a member of staff finds neither', () => {
  assert.ok(GROUPS.flatMap(([, rows]) => rows).length >= 30, 'the menu was parsed, not an empty list');
  const owner = A.sectionsFor({}, GROUPS).flatMap(([, rows]) => rows.map(r => r[0]));
  assert.ok(owner.includes('finance'), 'the money sheet');
  assert.ok(owner.includes('payroll'), 'the payroll tab');
  const kitchen = A.sectionsFor({ staff: true, caps: new Set(['advance', 'catalog', 'stock']) }, GROUPS).flatMap(([, rows]) => rows.map(r => r[0]));
  assert.ok(kitchen.length > 0, 'staff still have rows of their own, so the list below is a real one');
  assert.deepEqual(kitchen.filter(k => k === 'finance' || k === 'payroll'), []);
  for (const key of ['finance', 'payroll']) assert.equal(GROUPS.find(([, rows]) => rows.some(r => r[0] === key))[0], 'analytics', `${key} sits with the numbers`);
});

test('each row has an icon the stylesheet draws, a tour anchor, and an opener that opens its own screen', () => {
  const icons = read('../lib/icons.css');
  assert.ok(MORE.includes("tour: 'more.tile.' + key"), 'every row is anchored as more.tile.<key>');
  for (const [key, file] of [['finance', 'finance.js'], ['payroll', 'payroll.js']]) {
    const [, ic, opener] = rowOf(key);
    assert.ok(new RegExp(`\\.ti-${ic}\\b`).test(icons), `${key}: icon ${ic}`);
    assert.ok(new RegExp(`async function ${opener}\\(\\)\\{ \\(await import\\('/admin/${file.replace('.', '\\.')}'\\)\\)\\.open\\(\\);`).test(MORE), `${opener} opens /admin/${file}`);
  }
});

test('opening each screen asks the hub for the venue on screen', async () => {
  const { f } = page({ 'GET /owner/finance': { month: '2026-11', today: '2026-11-08', currency: 'ALL', expenses: [], profit: { plus: { total: 0, rows: [] }, minus: { total: 0, rows: [] }, net: 0 }, history: [] },
    'GET /owner/payroll': { month: '2026-11', today: '2026-11-08', dates: [], rows: [], staff: [], totalMinutes: 0, totalMinor: 0 } });
  for (const key of ['finance', 'payroll']) {
    await (await import(`./${key}.js`)).open();
    await tick();
    assert.ok(f.calls.some(c => c[0] === 'GET' && c[1].startsWith(`/owner/${key}?location_id=V1`)), `${key} read: ${JSON.stringify(f.calls.map(c => c[1]))}`);
  }
});

test('every word the two screens and their menu rows draw exists in uk, sq, en and ru', () => {
  for (const [file, words, prefix, rows] of [['finance.js', FW, 'fn_', ['finance', 'financeSub']], ['payroll.js', PW, 'pay_', ['payroll', 'payrollSub']]]) {
    const src = read(file) + read(file.replace('.js', '-logic.js'));
    const used = new Set([...src.matchAll(new RegExp(`['"\`=](${prefix}[A-Za-z_]+)`, 'g'))].map(m => m[1]).filter(k => !k.endsWith('_')));
    for (const k of ['orders', 'till', 'purchases', 'taxes', 'other', 'payroll']) if (prefix === 'fn_') used.add('fn_src_' + k);
    assert.ok(used.size >= 20, `${file}: ${used.size} keys found, the scan reads the file`);
    for (const key of [...used, ...rows]) for (const l of LANGS) assert.ok(typeof words[l]?.[key] === 'string' && words[l][key].trim(), `${file}: ${key} in ${l}`);
  }
  assert.equal(FW.uk.fn_src_payroll, 'Зарплати');
  assert.equal(PW.uk.payroll, 'Зарплата');
});
