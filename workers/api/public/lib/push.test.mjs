// lib/push.js + lib/push-sw.js: the opt-in control's states and the service
// worker's notification. `node --test workers/api/public/lib/push.test.mjs`
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { viewOf, keyBytes, markup } from './push.js';
import { PUSH_WORDS, pushWords } from './push-words.js';

test('the four states: iOS-not-installed, unsupported, blocked, on/off', () => {
  assert.equal(viewOf({ isSupported: false, apple: true, permission: 'default', on: false }), 'ios');
  assert.equal(viewOf({ isSupported: false, apple: false, permission: 'default', on: false }), 'unsupported');
  assert.equal(viewOf({ isSupported: true, apple: false, permission: 'denied', on: true }), 'blocked');
  assert.equal(viewOf({ isSupported: true, apple: false, permission: 'granted', on: true }), 'on');
  assert.equal(viewOf({ isSupported: true, apple: false, permission: 'default', on: true }), 'off', 'on means nothing without the permission');
  assert.equal(viewOf({ isSupported: true, apple: true, permission: 'granted', on: false }), 'off', 'an INSTALLED iPhone app is supported');
});

test('a button only where a tap can work; the iOS note where it cannot', () => {
  assert.match(markup('off', 'uk', 'pushWhyCustomer', 'track.push'), /id="pushToggle"[^>]*data-tour="track.push"|data-tour="track.push"[^>]*id="pushToggle"/);
  assert.match(markup('off', 'uk', 'pushWhyCustomer', 't'), /Увімкнути сповіщення/);
  assert.match(markup('on', 'sq', 'pushWhyStaff', 't'), /Çaktivizo njoftimet/);
  for (const v of ['ios', 'unsupported', 'blocked']) assert.doesNotMatch(markup(v, 'en', 'pushWhyCourier', 't'), /<button/, v);
  assert.match(markup('ios', 'ru', 'pushWhyCourier', 't'), /iOS 16\.4\+/);
  assert.match(markup('ios', 'en', 'pushWhyCourier', 't'), /Add to Home Screen/);
});

test('every language has every word, and an unknown language reads English', () => {
  const keys = Object.keys(PUSH_WORDS.en);
  for (const l of ['sq', 'uk', 'ru']) {
    assert.deepEqual(Object.keys(PUSH_WORDS[l]).sort(), [...keys].sort(), l);
    for (const k of keys) assert.notEqual(PUSH_WORDS[l][k], PUSH_WORDS.en[k], `${l}.${k}`);
  }
  assert.equal(pushWords('de'), PUSH_WORDS.en);
});

test('the VAPID key decodes from base64url to the 65-byte point', () => {
  const k = keyBytes('BPzZDsH_vwvJJ0KZ0kPL-fxyDsw_vDpNu2TTGNsAnKDglChuA_Ywvz81UGVE3lw9ikZvYUNyYhj7v5PujRmU11s');
  assert.equal(k.length, 65);
  assert.equal(k[0], 4);
});

// The service worker's half: run the classic script in a fake `self`.
function sw(){
  const listeners = {};
  const self = { addEventListener: (n, f) => { listeners[n] = f; }, location: { origin: 'https://alpha.dowiz.org' } };
  new Function('self', readFileSync(new URL('./push-sw.js', import.meta.url), 'utf8'))(self);
  return { self, listeners };
}

test('a push becomes the notification the hub wrote, with its page to open', () => {
  const { self } = sw();
  const n = self.dowizPush.shape(JSON.stringify({ title: 'Order #1a2b3c4d', body: 'Ready', url: '/?order=1a2b', tag: '1a2b-ready' }), 'https://alpha.dowiz.org');
  assert.equal(n.title, 'Order #1a2b3c4d');
  assert.equal(n.options.body, 'Ready');
  assert.equal(n.options.data.url, '/?order=1a2b');
  assert.equal(n.options.tag, '1a2b-ready');
});

test('a payload can never send the phone to another site, and junk still shows something', () => {
  const { self } = sw();
  assert.equal(self.dowizPush.shape(JSON.stringify({ title: 'x', url: 'https://evil.example/phish' }), 'https://alpha.dowiz.org').options.data.url, '/');
  assert.equal(self.dowizPush.shape(JSON.stringify({ title: 'x', url: '/courier/' }), 'https://alpha.dowiz.org').options.data.url, '/courier/');
  const junk = self.dowizPush.shape('not json', 'https://alpha.dowiz.org');
  assert.equal(junk.title, 'dowiz');
  assert.equal(junk.options.body, 'not json');
});

test('the push and the click handlers are registered', () => {
  const { listeners } = sw();
  assert.equal(typeof listeners.push, 'function');
  assert.equal(typeof listeners.notificationclick, 'function');
});
