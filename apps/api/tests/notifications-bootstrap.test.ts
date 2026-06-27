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
  assert.equal(normalizeVapidSubject('ops@dowiz.app'), 'mailto:ops@dowiz.app');
});

test('normalizeVapidSubject — existing mailto: is left untouched', () => {
  assert.equal(normalizeVapidSubject('mailto:ops@dowiz.app'), 'mailto:ops@dowiz.app');
});

const stubDeps = () => ({
  pool: {} as any,
  queueBoss: {} as any,
  memoryService: {} as any,
});

test('buildNotifications — telegram wired, no VAPID → push skipped, worker built', () => {
  const { telegramAdapter, notifyWorker } = buildNotifications({ TELEGRAM_BOT_TOKEN: 't' }, stubDeps());
  assert.ok(telegramAdapter, 'telegram adapter constructed');
  assert.ok(notifyWorker, 'notify worker constructed');
});

test('buildNotifications — with valid VAPID keys constructs without throwing (push registered)', () => {
  const { publicKey, privateKey } = webpush.generateVAPIDKeys();
  const handles = buildNotifications(
    { TELEGRAM_BOT_TOKEN: 't', VAPID_PUBLIC_KEY: publicKey, VAPID_PRIVATE_KEY: privateKey, VAPID_SUBJECT: 'ops@dowiz.app' },
    stubDeps(),
  );
  assert.ok(handles.telegramAdapter);
  assert.ok(handles.notifyWorker);
});
