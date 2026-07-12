import { test } from 'node:test';
import assert from 'node:assert/strict';
import webpush from 'web-push';
import { normalizeVapidSubject, buildNotifications } from '../src/bootstrap/notifications.js';

// Guardrail for the notification bootstrap. normalizeVapidSubject is the one
// branchy bit of setup (web-push rejects a non-mailto subject), so each arm is
// pinned. buildNotifications gets a construction smoke test: telegram always
// wired, web-push only when VAPID is present.

test('normalizeVapidSubject — unset/empty falls back to default mailto', () => {
  assert.equal(normalizeVapidSubject(undefined), 'mailto:admin@deliveryos.local');
  assert.equal(normalizeVapidSubject(''), 'mailto:admin@deliveryos.local');
});

test('normalizeVapidSubject — bare address gets the mailto: scheme', () => {
  assert.equal(normalizeVapidSubject('ops@example.com'), 'mailto:ops@example.com');
});

test('normalizeVapidSubject — existing mailto: is left untouched', () => {
  assert.equal(normalizeVapidSubject('mailto:ops@example.com'), 'mailto:ops@example.com');
});

const stubDeps = () => ({
  pool: {} as any,
  queueBoss: {} as any,
  memoryService: {} as any,
});

// The web-push branch is invisible from the returned handles (only telegramAdapter
// + notifyWorker are exposed), so reach into the worker's dispatcher to prove which
// channels were actually registered. Without this, a branch that silently skipped
// push registration would still pass on `ok(notifyWorker)` alone.
const registeredChannels = (handles: { notifyWorker: unknown }): Map<string, unknown> =>
  (handles.notifyWorker as any).dispatcher.adapters as Map<string, unknown>;

test('buildNotifications — telegram wired, no VAPID → push skipped, worker built', () => {
  const handles = buildNotifications({ TELEGRAM_BOT_TOKEN: 't' }, stubDeps());
  assert.ok(handles.telegramAdapter, 'telegram adapter constructed');
  assert.ok(handles.notifyWorker, 'notify worker constructed');
  const channels = registeredChannels(handles);
  assert.ok(channels.has('telegram'), 'telegram channel registered on dispatcher');
  assert.equal(channels.has('push'), false, 'push channel NOT registered without VAPID keys');
});

test('buildNotifications — no TELEGRAM_BOT_TOKEN → telegram still wired (empty-token adapter), no throw, push skipped', () => {
  // Source registers telegram unconditionally with `TELEGRAM_BOT_TOKEN || ''`, so
  // an absent token must NOT throw and must NOT drop the telegram channel; push
  // stays unregistered (no VAPID). Pins the actual no-token behaviour.
  const handles = buildNotifications({}, stubDeps());
  assert.ok(handles.telegramAdapter, 'telegram adapter constructed even without a token');
  assert.ok(handles.notifyWorker, 'notify worker constructed');
  const channels = registeredChannels(handles);
  assert.ok(channels.has('telegram'), 'telegram channel registered with empty-token adapter');
  assert.equal(channels.has('push'), false, 'push channel NOT registered without VAPID keys');
});

test('buildNotifications — with valid VAPID keys constructs without throwing (push registered)', () => {
  const { publicKey, privateKey } = webpush.generateVAPIDKeys();
  const handles = buildNotifications(
    { TELEGRAM_BOT_TOKEN: 't', VAPID_PUBLIC_KEY: publicKey, VAPID_PRIVATE_KEY: privateKey, VAPID_SUBJECT: 'ops@example.com' },
    stubDeps(),
  );
  assert.ok(handles.telegramAdapter);
  assert.ok(handles.notifyWorker);
  const channels = registeredChannels(handles);
  assert.ok(channels.has('push'), 'push channel registered when VAPID keys present');
  assert.ok(channels.has('telegram'), 'telegram channel still registered alongside push');
});
