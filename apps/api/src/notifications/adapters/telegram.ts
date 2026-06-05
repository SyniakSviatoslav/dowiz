// @ts-nocheck
import type { NotificationProvider, NotificationTarget, NotificationEvent, NotificationData, NotifyResult } from '../provider.js';
import { renderTelegramMessage } from '../render.js';

export class TelegramAdapter implements NotificationProvider {
  readonly id = 'telegram';
  private token: string;
  private apiBase: string;

  constructor(token: string, apiBase = 'https://api.telegram.org/bot') {
    this.token = token;
    this.apiBase = apiBase;
  }

  async notify(target: NotificationTarget, event: NotificationEvent, data: NotificationData): Promise<NotifyResult> {
    if (!this.token) {
      return { delivered: false, reason: 'TELEGRAM_TOKEN_NOT_CONFIGURED' };
    }

    const { text, reply_markup } = renderTelegramMessage(event, data);

    try {
      const response = await fetch(`${this.apiBase}${this.token}/sendMessage`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          chat_id: target.address,
          text,
          parse_mode: 'HTML',
          reply_markup
        }),
        signal: AbortSignal.timeout(5000)
      });

      if (response.ok) {
        const body = await response.json();
        return { delivered: true, providerMessageId: body.result?.message_id?.toString() };
      }

      if (response.status === 401 || response.status === 403) {
        return { delivered: false, reason: `AUTH_OR_BLOCKED:${response.status}` }; // Will cause disablement
      }

      if (response.status === 429) {
        const retryAfter = response.headers.get('retry-after');
        return { delivered: false, reason: 'RATE_LIMIT', retryAfter: retryAfter ? parseInt(retryAfter, 10) * 1000 : 5000 };
      }

      return { delivered: false, reason: `HTTP_${response.status}` };
    } catch (err: any) {
      return { delivered: false, reason: err.message || 'NETWORK_ERROR' };
    }
  }

  // Used by polling worker
  async getUpdates(offset: number) {
    if (!this.token) return [];
    const response = await fetch(`${this.apiBase}${this.token}/getUpdates`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ offset, timeout: 5 }), // short timeout for internal loop
      signal: AbortSignal.timeout(10000)
    });
    if (!response.ok) throw new Error(`getUpdates HTTP ${response.status}`);
    const data = await response.json();
    if (!data.ok) throw new Error(data.description);
    return data.result;
  }
}
