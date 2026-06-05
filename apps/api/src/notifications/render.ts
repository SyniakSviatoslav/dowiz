// @ts-nocheck
import type { NotificationData, NotificationEvent } from './provider.js';

function escapeHtml(unsafe: string) {
  return unsafe
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

export function renderTelegramMessage(event: NotificationEvent, data: NotificationData): { text: string, reply_markup?: any } {
  if (event.type === 'test') {
    return {
      text: `🧪 <b>Test from Dowiz</b>\n\n${escapeHtml(data.message || 'If you see this, notifications are working.')}`
    };
  }

  if (event.type === 'order.created') {
    return {
      text: `🔔 <b>Porosi e re</b> · #${escapeHtml(data.shortOrderId || '???')}\n💰 ${data.total} ${escapeHtml(data.currency || 'ALL')}\n🕒 ${escapeHtml(data.createdAtLocal || '')}`,
      reply_markup: {
        inline_keyboard: [
          [{ text: 'Hap në panel (Open)', url: `https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` }]
        ]
      }
    };
  }

  if (event.type === 'order.pending_aging') {
    return {
      text: `⏰ <b>Porosi në pritje</b> · #${escapeHtml(data.shortOrderId || '???')}\n🕒 ${data.ageMinutes} min pa përgjigje`,
      reply_markup: {
        inline_keyboard: [
          [
            { text: '✅ Konfirmo', callback_data: `order.confirm:${data.orderId}` },
            { text: '❌ Anulo', callback_data: `order.cancel:${data.orderId}` }
          ]
        ]
      }
    };
  }

  return { text: 'Unknown event' };
}
