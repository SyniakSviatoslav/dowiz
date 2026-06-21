import type { NotificationData, NotificationEvent } from './provider.js';
import { getMessage, type MessageVars, type Locale } from './locales.js';
import { formatMoney } from '@deliveryos/shared-types';

function fmtPrice(v: number | undefined, currency: string | undefined): string {
  if (v == null) return '—';
  return formatMoney(v, (currency || 'ALL') as any);
}

function toVars(data: NotificationData): MessageVars {
  const orderTypeLabel =
    data.orderType === 'pickup' ? 'Pickup'
    : data.orderType === 'delivery' ? 'Delivery'
    : undefined;
  return {
    shortOrderId: data.shortOrderId,
    totalFmt: fmtPrice(data.total, data.currency),
    subtotalFmt: fmtPrice(data.subtotal, data.currency),
    deliveryFeeFmt: fmtPrice(data.deliveryFee, data.currency),
    discountFmt: fmtPrice(data.discountTotal, data.currency),
    taxFmt: fmtPrice(data.taxTotal, data.currency),
    cashPayWithFmt: fmtPrice(data.cashPayWith, data.currency),
    currency: data.currency,
    customerName: data.customerName,
    customerPhone: data.customerPhone,
    deliveryAddress: data.deliveryAddress,
    deliveryInstructions: data.deliveryInstructions,
    orderTypeLabel,
    items: data.items?.map(i => ({ name: i.name, price: i.price, quantity: i.quantity })),
    courierName: data.courierName,
    shiftStartTime: data.shiftStartTime,
    shiftDuration: data.shiftDuration,
    ageMinutes: data.ageMinutes != null ? String(data.ageMinutes) : undefined,
    discrepancyFmt: fmtPrice(data.discrepancy, data.currency),
    rating: data.rating != null ? String(data.rating) : undefined,
    message: data.message,
  };
}

// P0-2 (ADR-p0-privacy-hardening): renderWhatsAppMessage removed with the WhatsApp/
// Baileys channel. Telegram (below) + push + email remain.

export function renderTelegramMessage(event: NotificationEvent, data: NotificationData, locale: Locale = 'sq'): { text: string, reply_markup?: any } {
  const vars = toVars(data);
  const text = getMessage(locale, event.type, vars);

  // Generate buttons for events that support them
  const baseUrl = 'https://app.dowiz.org';
  const locationUrl = data.locationId ? `${baseUrl}/admin/locations/${data.locationId}/orders/${data.orderId}` : undefined;

  switch (event.type) {
    case 'order.created':
    case 'order.substitution_needs_human':
      return {
        text,
        reply_markup: {
          inline_keyboard: [[
            { text: locale === 'sq' ? '✅ Konfirmo' : locale === 'uk' ? '✅ Підтвердити' : '✅ Confirm', callback_data: `order.confirm:${data.orderId}` },
            { text: locale === 'sq' ? '❌ Refuzo' : locale === 'uk' ? '❌ Відхилити' : '❌ Reject', callback_data: `order.reject_choose:${data.orderId}` },
          ]]
        }
      };

    case 'order.confirmed':
    case 'order.rejected':
    case 'cash.reconcile_discrepancy':
    case 'order.delivered':
    case 'order.dwell_escalation':
    case 'order.ready_for_pickup':
    case 'delivery.flag_raised':
    case 'rating.low_received':
    case 'order.timeout_cancelled':
      return {
        text,
        reply_markup: locationUrl ? {
          inline_keyboard: [[{ text: '🔗 Open in app', url: locationUrl }]]
        } : undefined,
      };

    case 'order.pending_aging':
      return {
        text,
        reply_markup: data.orderId ? {
          inline_keyboard: [[
            { text: '✅ Confirm', callback_data: `order.confirm:${data.orderId}` },
            { text: '❌ Reject', callback_data: `order.reject_choose:${data.orderId}` },
          ]]
        } : undefined,
      };

    case 'courier.assigned':
      return {
        text,
        reply_markup: data.orderId && locationUrl ? {
          inline_keyboard: [[{ text: '👀 Track', url: locationUrl }]]
        } : undefined,
      };

    case 'shift.started':
    case 'shift.closed':
    case 'shift.close_reminder':
      return {
        text,
        reply_markup: data.locationId ? {
          inline_keyboard: [[{ text: '🔗 Close shift', url: `${baseUrl}/admin/locations/${data.locationId}/shifts` }]]
        } : undefined,
      };

    case 'ops.worker_liveness':
    case 'ops.backup_failed':
    case 'ops.degradation_changed':
    case 'test':
      return { text };

    default: {
      const _exhaustive: never = event.type;
      return _exhaustive;
    }
  }
}
