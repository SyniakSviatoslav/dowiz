import { formatMoney, ensureCurrency } from '@deliveryos/shared-types';
import webpush from 'web-push';
import type { NotificationProvider, NotificationTarget, NotificationEvent, NotificationData, NotifyResult } from '../provider.js';
import type { Locale } from '../locales.js';
import { getPushText } from '../push-strings.js';

export class WebPushAdapter implements NotificationProvider {
  readonly id = 'push';

  constructor(vapidPublicKey: string, vapidPrivateKey: string, vapidSubject: string) {
    webpush.setVapidDetails(vapidSubject, vapidPublicKey, vapidPrivateKey);
  }

  async notify(target: NotificationTarget, event: NotificationEvent, data: NotificationData): Promise<NotifyResult> {
    let subscription: PushSubscriptionJSON;
    try {
      subscription = JSON.parse(target.address) as PushSubscriptionJSON;
    } catch (err: any) {
      console.warn('[webpush] invalid push subscription JSON:', err?.message);
      return { delivered: false, reason: 'invalid_push_subscription_json' };
    }

    if (!subscription.endpoint || !subscription.keys?.p256dh || !subscription.keys?.auth) {
      return { delivered: false, reason: 'invalid_push_subscription_keys' };
    }

    // Thread the recipient locale (same source the Telegram path uses).
    const locale = ((target.locale || data.locale || 'sq') as Locale);
    const payload = this.buildPayload(event, data, locale);

    try {
      const result = await webpush.sendNotification(
        subscription as webpush.PushSubscription,
        payload,
        { TTL: 86400, urgency: 'high' },
      );

      return { delivered: true, providerMessageId: result.headers?.['location'] || undefined };
    } catch (err: any) {
      if (err.statusCode === 410 || err.statusCode === 404) {
        return { delivered: false, reason: 'subscription_gone' };
      }
      if (err.statusCode === 401 || err.statusCode === 403) {
        return { delivered: false, reason: 'auth_failed' };
      }
      if (err.statusCode === 429) {
        const retryAfter = err.headers?.['retry-after']
          ? parseInt(err.headers['retry-after'], 10) * 1000
          : 60000;
        return { delivered: false, reason: 'rate_limited', retryAfter };
      }
      return { delivered: false, reason: err.message || 'push_failed' };
    }
  }

  private buildPayload(event: NotificationEvent, data: NotificationData, locale: Locale): string {
    const { title, body } = getPushText(locale, event.type, {
      shortOrderId: data.shortOrderId,
      ageMinutes: data.ageMinutes != null ? String(data.ageMinutes) : undefined,
      discrepancyFmt: data.discrepancy != null && data.currency
        ? formatMoney(data.discrepancy, ensureCurrency(data.currency)) : undefined,
      rating: data.rating != null ? String(data.rating) : undefined,
      courierName: data.courierName,
      message: data.message,
    });

    const parts: string[] = [body];
    if (data.total != null && data.currency) {
      parts.push(formatMoney(data.total, ensureCurrency(data.currency)));
    }

    return JSON.stringify({
      title,
      body: parts.filter(Boolean).join(' · ') || 'New update',
      tag: data.orderId ? `order-${data.orderId}` : 'deliveryos',
      data: {
        orderId: data.orderId,
        locationId: data.locationId,
        url: data.orderId
          ? `/s/${data.locationId}/order/${data.orderId}`
          : undefined,
      },
    });
  }
}
