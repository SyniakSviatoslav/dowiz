// @ts-nocheck
import webpush from 'web-push';
import type { NotificationProvider, NotificationTarget, NotificationEvent, NotificationData, NotifyResult } from '../provider.js';

export class WebPushAdapter implements NotificationProvider {
  readonly id = 'push';

  constructor(vapidPublicKey: string, vapidPrivateKey: string, vapidSubject: string) {
    webpush.setVapidDetails(vapidSubject, vapidPublicKey, vapidPrivateKey);
  }

  async notify(target: NotificationTarget, _event: NotificationEvent, data: NotificationData): Promise<NotifyResult> {
    let subscription: PushSubscriptionJSON;
    try {
      subscription = JSON.parse(target.address) as PushSubscriptionJSON;
    } catch {
      return { delivered: false, reason: 'invalid_push_subscription_json' };
    }

    if (!subscription.endpoint || !subscription.keys?.p256dh || !subscription.keys?.auth) {
      return { delivered: false, reason: 'invalid_push_subscription_keys' };
    }

    const payload = this.buildPayload(data);

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

  private buildPayload(data: NotificationData): string {
    const parts: string[] = [];
    const title = data.shortOrderId
      ? `Order #${data.shortOrderId}`
      : 'DeliveryOS';

    if (data.total != null && data.currency) {
      parts.push(`${data.total.toFixed(0)} ${data.currency}`);
    }
    if (data.message) parts.push(data.message);

    return JSON.stringify({
      title,
      body: parts.join(' · ') || 'New update',
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
