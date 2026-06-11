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
     const totalAll = data.total ? Math.round(data.total / 100) : 0;
     const quantity = data.quantity ?? 0;
     return {
       text: `🆕 #${escapeHtml(data.shortOrderId || '???')} · ${totalAll} ${escapeHtml(data.currency || 'ALL')} · ${quantity} поз. · ${escapeHtml(data.createdAtLocal || '')} · ⏳ timeout 7h`,
       reply_markup: {
         inline_keyboard: [
           [{ text: '✅ Підтвердити', callback_data: `order.confirm:${data.orderId}` }],
           [{ text: '❌ Відхилити', callback_data: `order.reject_choose:${data.orderId}` }]
         ]
       }
     };
   }

  if (event.type === 'order.substitution_needs_human') {
    return {
      text: `⚠️ #${escapeHtml(data.shortOrderId || '???')} — Produkti u zëvogëlua, klienti kërkoi kontaktnë. Nevojitet vendim.` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` : ''),
      reply_markup: {
        inline_keyboard: [
          [{ text: '🔗 Hap në PWA', url: `https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` }]
        ]
      }
    };
  }

  if (event.type === 'order.dwell_escalation') {
    return {
      text: `⏰ #${escapeHtml(data.shortOrderId || '???')} në statusi ${escapeHtml(data.createdAtLocal || 'PENDING')} ${data.ageMinutes || 0} min — jo i konfirmuar` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` : ''),
      reply_markup: {
        inline_keyboard: [
          [{ text: '🔗 Hap në PWA', url: `https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` }]
        ]
      }
    };
  }

  if (event.type === 'order.timeout_cancelled') {
    return {
      text: `🚫 #${escapeHtml(data.shortOrderId || '???')} anuluar automatikus (në konfirmim të shumti)` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` : '')
    };
  }

  if (event.type === 'cash.reconcile_discrepancy') {
    const discrepancy = data.total ?? 0; // total field reused for discrepancy amount
    const absDiscrepancy = Math.abs(discrepancy);
    const prefix = discrepancy > 0 ? '+' : '';
    return {
      text: `💱 Ndalimi i kasës: ${prefix}${absDiscrepancy} ALL. Behja e nevojshme.` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` : '')
    };
  }

  if (event.type === 'delivery.flag_raised') {
    return {
      text: `🚚 #${escapeHtml(data.shortOrderId || '???')} — Flag të lartëruar (GPS i largët)` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` : ''),
      reply_markup: {
        inline_keyboard: [
          [{ text: '🔗 Hap në PWA', url: `https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` }]
        ]
      }
    };
  }

  if (event.type === 'rating.low_received') {
    return {
      text: `⭐ #${escapeHtml(data.shortOrderId || '???')} — Vlerësim i Ulët: ${data.total ?? 0}/5` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` : ''),
      reply_markup: {
        inline_keyboard: [
          [{ text: '🔗 Hap në PWA', url: `https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` }]
        ]
      }
    };
  }

  if (event.type === 'ops.worker_liveness') {
    return {
      text: `🛑 Punëtori i procesit nuk përgjigjon >1 min. Kontrolloni sistemin.`
    };
  }

  if (event.type === 'ops.backup_failed') {
    return {
      text: `🛑 Kopia e sigurtë nuk u verificua. Kontrolloni sistemin.`
    };
  }

  if (event.type === 'ops.degradation_changed') {
    // Assuming we have a way to know if it's degradation or recovery
    // For now, we'll show a generic message; the worker could set a flag in data
    const isDegraded = data.ageMinutes ?? 0 > 0; // Using ageMinutes as a flag
    if (isDegraded) {
      return {
        text: `⚠️ Sistemi në režim degraduar: <details ngjinuar>.`
      };
    } else {
      return {
        text: `✅ Sistemi i riparuar nga režimi degraduar.`
      };
    }
  }

  if (event.type === 'courier.assigned') {
    return {
      text: `📦 Drejtim i ri #${escapeHtml(data.shortOrderId || '???')} → ${data.locationId ? 'Lokacioni' : '???'} (shiko detajet në app)` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/orders/${data.orderId}` : ''),
      // Optional: add a "Accepted" button for couriers
      reply_markup: {
        inline_keyboard: [
          [{ text: '👊 Marrë', callback_data: `courier.accept:${data.orderId}` }]
        ]
      }
    };
  }

  if (event.type === 'order.ready_for_pickup') {
    return {
      text: `🍽️ #${escapeHtml(data.shortOrderId || '???')} gati për mblidhje` +
        (data.orderId ? `\n🔗 https://app.dowiz.org/app/locations/${data.locationId}/orders/${data.orderId}` : '')
    };
  }

  if (event.type === 'shift.close_reminder') {
    return {
      text: `🌙 Nu lutem, mbyllni shiftin dhe balanconi kasën` +
        (data.locationId ? `\n🔗 https://app.dowiz.org/admin/locations/${data.locationId}/shifts` : '')
    };
  }

  return { text: 'Unknown event' };
}
