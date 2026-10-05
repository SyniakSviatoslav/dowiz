// node --test workers/api/public/admin/kitchen-topics.test.mjs
// P16b: the "what the notes say" card from rows shaped exactly like the hub's
// (`services/analytics/kitchen/topics/tests.rs`), and its four languages.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { topicsCard, TOPICS } from './kitchen-topics.js';
import { LANGS } from '../lib/langs.js';

const rows = [
  { week: 20261005, dish: 'rice', name: 'Rice <b>bowl</b>', topic: 'cold', count: 2, examples: ['the rice bowl was cold', 'everything <i>cold</i>', 'third'] },
  { week: 20261012, dish: 'roll', name: 'Salmon roll', topic: 'tasty', count: 1, examples: ['very tasty'] },
  { week: 20261012, dish: 'roll', name: 'Salmon roll', topic: 'salty', count: 3, examples: [] },
  { week: 20261012, dish: 'roll', name: 'Salmon roll', topic: 'mood', count: 9, examples: [] },
];

test('topics: one row per week, dish and topic, the newest week first and problems before praise', () => {
  const html = topicsCard(rows);
  assert.ok(html.includes('data-tour="kitchen.topics"'));
  const order = [...html.matchAll(/data-t="ft_(\w+)"><\/td>/g)].map(m => m[1]);
  assert.deepEqual(order, ['salty', 'tasty', 'cold']);
  for (const v of ['2026-10-05', '2026-10-12', '>2<', '>3<', 'Salmon roll']) assert.ok(html.includes(v), v);
  assert.ok(!html.includes('mood'), 'a topic the hub does not fold is not drawn');
  assert.ok(!html.includes('third'), 'two examples at most');
});

test('topics: a name or a phrase cannot inject markup', () => {
  const html = topicsCard(rows);
  assert.ok(!html.includes('<b>') && !html.includes('<i>'));
  assert.ok(html.includes('&lt;b&gt;bowl'));
});

test('topics: nothing yet says so; a missing answer is nothing, not a crash', () => {
  for (const t of [[], undefined, null, {}]) assert.ok(topicsCard(t).includes('data-t="ft_none"'), JSON.stringify(t));
});

test('topics: every word of the card in all four languages', async () => {
  const src = readFileSync(new URL('./kitchen-topics-i18n.js', import.meta.url), 'utf8');
  const keys = l => new Set([...src.slice(src.indexOf(`  ${l}: {`)).split('\n  },')[0].matchAll(/(ft_\w+):/g)].map(m => m[1]));
  const want = ['ft_title', 'ft_hint', 'ft_none', 'ft_week', 'ft_topic', 'ft_notes', 'ft_example', ...TOPICS.map(t => `ft_${t}`)];
  for (const l of LANGS) for (const k of want) assert.ok(keys(l).has(k), `${l}.${k}`);
});
