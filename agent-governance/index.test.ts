// agent-governance/index.test.ts
//
// RED+GREEN node:test parity with bebop's Rust governance suite (agent_profile,
// gender, settings, drift, error_patterns). Runs via `npx tsx --test`.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as G from './index.ts';

test('gender: default masculine + parse ua/en', () => {
  assert.equal(G.DEFAULT_GENDER, 'masculine');
  assert.equal(G.parseGender('чоловічий'), 'masculine');
  assert.equal(G.parseGender('female'), 'feminine');
  assert.equal(G.parseGender('без роду'), 'neutral');
  assert.equal(G.parseGender('nope'), null);
  assert.match(G.genderRule('uk', 'masculine'), /чоловічим родом/);
});

test('profanity: default poderviansky + 3 levels', () => {
  assert.equal(G.DEFAULT_PROFANITY, 'poderviansky');
  assert.equal(G.parseProfanity('заборонена'), 'forbidden');
  assert.equal(G.parseProfanity('подервянський'), 'poderviansky');
  assert.match(G.profanityRule('en', 'poderviansky'), /Poderviansky/);
});

test('archetype: default corpo + collaborative themes', () => {
  assert.deepEqual(G.DEFAULT_ARCHETYPE, 'corpo');
  assert.equal(G.parseArchetype('рептилії'), 'reptiles');
  assert.match(G.archetypeRule('uk', 'reptiles'), /рептилії/);
  assert.match(G.archetypeRule('uk', 'corpo'), /ДЕФОЛТНИЙ антагоніст/);
});

test('archetype: witches + cbt + karma DISABLED by default (not hard-banned)', () => {
  // RED: these are opt-in, NOT a permanent ban.
  assert.equal(G.isHardBanned('witches'), false);
  assert.equal(G.isHardBanned('cbt'), false);
  assert.equal(G.isHardBanned('karma'), false);
  assert.match(G.archetypeRule('uk', 'witches'), /ВИМКНЕНО/);
  assert.match(G.archetypeRule('uk', 'cbt'), /скамом для бідних/);
});

test('archetype: VOODOO is HARD BANNED + never user-toggleable', () => {
  // GREEN+RED: permanent ban, author calls voodoo users "хуєсосами".
  assert.equal(G.isHardBanned('voodoo'), true);
  assert.match(G.archetypeRule('uk', 'voodoo'), /ПОВНА ЗАБОРОНА/);
  assert.match(G.archetypeRule('uk', 'voodoo'), /хуєсос/);
  // NOT present in the settings dictionary → cannot be enabled via config.
  assert.ok(G.settingEntries().every((e) => e.key !== 'voodoo'));
});

test('god relation: default serves God + configurable', () => {
  // GREEN: configurable, default = serves.
  assert.deepEqual(G.DEFAULT_GOD, 'serves');
  assert.equal(G.parseGodRelation('служить'), 'serves');
  assert.equal(G.parseGodRelation('шукає'), 'seeks');
  assert.match(G.godRelationRule('uk', 'serves'), /служить Богу/);
  // custom free-text allowed
  assert.equal(typeof G.parseGodRelation('мій шлях'), 'object');
});

test('settings dictionary: list + get + set + validation', () => {
  const entries = G.settingEntries();
  assert.ok(entries.length >= 8);
  // unknown key rejected
  assert.equal(G.setSetting('not_a_key', 'x').ok, false);
  // disallowed value rejected
  assert.equal(G.setSetting('gender', 'robot').ok, false);
  // valid set/get round-trips
  assert.equal(G.setSetting('gender', 'neutral').ok, true);
  assert.equal(G.getSetting('gender'), 'neutral');
  assert.equal(G.getSetting('profanity'), 'poderviansky');
});

test('drift: detects systems-thinking/architecture drift', () => {
  const d = G.detectDrift(G.defaultDriftPolicy(), 'Cargo.toml', 'add dependency serde');
  assert.ok(d.some((x) => x.practice === 'new-global-dep'));
  assert.match(G.renderDrift(d), /new-global-dep/);
  // clean input → no drift
  assert.equal(G.detectDrift(G.defaultDriftPolicy(), 'foo', 'bar').length, 0);
});

test('error-patterns: scan + learn accumulates + persists', () => {
  // GREEN: a Rust E-code is detected.
  const h1 = G.scanErrors('src/main.rs:3:1 error[E0599]: no method named `foo`', 'session');
  assert.ok(h1.some(([id]) => id === 'error[E'));
  // learn twice → count 2
  const store: G.ErrorPattern[] = [];
  const h2 = G.scanErrors('error[E0599]: again', 'debug');
  G.learnErrors(store, h1, 'session');
  G.learnErrors(store, h2, 'debug');
  assert.equal(store.length, 1);
  assert.equal(store[0].count, 2);
  assert.equal(store[0].last_scope, 'debug');
  // persistence round-trip
  const json = G.saveErrorStore(store);
  const loaded = G.loadErrorStore(json);
  assert.equal(loaded.length, 1);
  assert.equal(loaded[0].count, 2);
  // empty store honest
  assert.match(G.renderErrorSummary([]), /none learned/);
});

test('default agent profile: voiced in dowiz brand, includes all axes', () => {
  const uk = G.defaultAgentProfile('uk');
  assert.match(uk, /вільна душа/);
  assert.match(uk, /служить Богу/);
  assert.match(uk, /ДЕФОЛТНИЙ антагоніст/);
  assert.match(uk, /ВИМКНЕНО/);            // witches disabled
  assert.match(uk, /ПОВНА ЗАБОРОНА/);       // voodoo hard ban
  assert.match(uk, /Cosmo-Noir/);           // dowiz brand voice
  const en = G.defaultAgentProfile('en');
  assert.match(en, /free soul/);
});
