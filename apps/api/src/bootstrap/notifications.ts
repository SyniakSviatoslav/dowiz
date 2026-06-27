import { TelegramAdapter } from '../notifications/adapters/telegram.js';
import { WebPushAdapter } from '../notifications/adapters/webpush.js';
import { NotificationDispatcher } from '../notifications/provider.js';
import { RetryPolicy } from '../notifications/retry.js';
import { NotificationWorker } from '../notifications/workers/index.js';
import type { MemoryService } from '../lib/memory.js';

// Notification provider wiring extracted from server.ts main(). Builds the
// dispatcher (telegram always; web-push only when VAPID is configured) and the
// NotificationWorker. Returns the handles main() still needs: the telegram
// adapter (for the disabled poller / type) and the worker (whose handlers are
// bound to the pg-boss queues). WhatsApp/Baileys was removed (P0-2) — telegram +
// push + email only.

/**
 * Normalize VAPID_SUBJECT into a valid web-push `mailto:` contact. Empty/unset
 * falls back to the default address; a bare address gets the mailto: scheme.
 * Pure — the one branchy bit of notification setup, unit-tested.
 */
export function normalizeVapidSubject(raw?: string): string {
  if (!raw) return 'mailto:admin@deliveryos.local';
  return raw.startsWith('mailto:') ? raw : `mailto:${raw}`;
}

export interface NotificationEnv {
  TELEGRAM_BOT_TOKEN?: string;
  VAPID_PUBLIC_KEY?: string;
  VAPID_PRIVATE_KEY?: string;
  VAPID_SUBJECT?: string;
}

export interface NotificationDeps {
  pool: any;
  queueBoss: any;
  memoryService: MemoryService;
}

export interface NotificationHandles {
  telegramAdapter: TelegramAdapter;
  notifyWorker: NotificationWorker;
}

export function buildNotifications(env: NotificationEnv, deps: NotificationDeps): NotificationHandles {
  const { pool, queueBoss, memoryService } = deps;

  const telegramAdapter = new TelegramAdapter(env.TELEGRAM_BOT_TOKEN || '');
  const notifyDispatcher = new NotificationDispatcher();
  notifyDispatcher.register('telegram', telegramAdapter);

  // Web-push channel only when VAPID keys are configured.
  if (env.VAPID_PUBLIC_KEY && env.VAPID_PRIVATE_KEY) {
    const webPushAdapter = new WebPushAdapter(
      env.VAPID_PUBLIC_KEY,
      env.VAPID_PRIVATE_KEY,
      normalizeVapidSubject(env.VAPID_SUBJECT),
    );
    notifyDispatcher.register('push', webPushAdapter);
  }

  const retryPolicy = new RetryPolicy();
  const notifyWorker = new NotificationWorker(pool, queueBoss, notifyDispatcher, retryPolicy, memoryService);

  return { telegramAdapter, notifyWorker };
}
