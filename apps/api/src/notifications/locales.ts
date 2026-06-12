export type Locale = 'sq' | 'en' | 'uk';

interface OrderItemData {
  name: string;
  price: number;
  quantity: number;
}

export interface MessageVars {
  shortOrderId?: string;
  totalFmt?: string;
  subtotalFmt?: string;
  deliveryFeeFmt?: string;
  discountFmt?: string;
  taxFmt?: string;
  cashPayWithFmt?: string;
  currency?: string;
  customerName?: string;
  customerPhone?: string;
  deliveryAddress?: string;
  deliveryInstructions?: string;
  orderTypeLabel?: string;
  items?: OrderItemData[];
  courierName?: string;
  shiftStartTime?: string;
  shiftDuration?: string;
  ageMinutes?: string;
  discrepancyFmt?: string;
  rating?: string;
  message?: string;
}

type TemplateFn = (v: MessageVars) => string;
type LocaleMessages = Record<string, TemplateFn>;

function fmtItems(v: MessageVars): string {
  if (!v.items || v.items.length === 0) return '';
  return '\n' + v.items.map(i => {
    const lineTotal = ((i.price * i.quantity) / 100).toFixed(2);
    return `• ${i.name} × ${i.quantity} — ${lineTotal} ${v.currency || 'ALL'}`;
  }).join('\n');
}

const sq: LocaleMessages = {
  'order.confirmed': (v) => [
    `✅ <b>POROSIA E KONFIRMUAR #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Produktet:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Totali: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'order.rejected': (v) => [
    `❌ <b>POROSIA E REFUZUA #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Produktet:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Totali: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'order.created': (v) => [
    `🆕 <b>POROSIA E RE #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.customerPhone ? `📞 ${v.customerPhone}` : '',
    v.deliveryAddress && v.orderTypeLabel === 'Dorëzim' ? `📍 ${v.deliveryAddress}` : '',
    v.deliveryInstructions ? `📝 ${v.deliveryInstructions}` : '',
    v.orderTypeLabel ? `📦 ${v.orderTypeLabel}` : '',
    '',
    v.items?.length ? `<b>📋 Produktet:</b>${fmtItems(v)}` : '',
    v.subtotalFmt ? `💰 Nëntotali: ${v.subtotalFmt}` : '',
    v.deliveryFeeFmt ? `🚚 Dorëzimi: ${v.deliveryFeeFmt}` : '',
    v.discountFmt ? `🏷️ Zbritja: -${v.discountFmt}` : '',
    v.taxFmt ? `🧾 TVSH: ${v.taxFmt}` : '',
    v.totalFmt ? `<b>🎯 Totali: ${v.totalFmt}</b>` : '',
    v.cashPayWithFmt ? `💵 Cash: ${v.cashPayWithFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.delivered': (v) => [
    `✅ <b>POROSIA E DORËZUAR #${v.shortOrderId || '???'}</b>`,
    v.courierName ? `👨‍🚀 Korrieri: ${v.courierName}` : '',
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Produktet:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Totali: ${v.totalFmt}</b>` : '',
    v.cashPayWithFmt ? `💵 Paguar: ${v.cashPayWithFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.substitution_needs_human': (v) => [
    `⚠️ <b>#${v.shortOrderId || '???'} — Zëvendësim i nevojshëm</b>`,
    '',
    `Produkti u zëvendësua. Klienti kërkoi kontakt. Nevojitet vendim.`,
    v.items?.length ? `<b>📋 Produktet:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `💰 Totali: ${v.totalFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.dwell_escalation': (v) => `⏰ <b>#${v.shortOrderId || '???'} — Në pritje ${v.ageMinutes || '?'} min</b>\n\nPorosia nuk u konfirmua. Merrni masa.`,
  'order.timeout_cancelled': (v) => `🚫 <b>#${v.shortOrderId || '???'} — Anuluar automatikisht</b>\n\nNuk u konfirmua brenda afatit.`,
  'order.ready_for_pickup': (v) => `🍽️ <b>#${v.shortOrderId || '???'} — Gati për marrje</b>`,
  'cash.reconcile_discrepancy': (v) => `💱 <b>Diferencë në arkë: ${v.discrepancyFmt || '?'} ALL</b>\n\nBëni rakordimin.`,
  'delivery.flag_raised': (v) => `🚚 <b>#${v.shortOrderId || '???'} — Flamur i ngritur</b>\n\nGPS nuk përputhet me vendndodhjen.`,
  'rating.low_received': (v) => `⭐ <b>#${v.shortOrderId || '???'} — Vlerësim i ulët: ${v.rating || '?'}/5</b>`,
  'courier.assigned': (v) => [
    `📦 <b>#${v.shortOrderId || '???'} — Korrieri në rrugë</b>`,
    v.courierName ? `👨‍🚀 ${v.courierName}` : '',
    '',
    v.items?.length ? `<b>📋 Produktet:</b>${fmtItems(v)}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    v.totalFmt ? `<b>💰 Totali: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'shift.started': (v) => `🟢 <b>Ndërrimi filloi</b>\n\n👨‍🚀 ${v.courierName || '?'}\n🕐 ${v.shiftStartTime || ''}`,
  'shift.closed': (v) => `🔴 <b>Ndërrimi përfundoi</b>\n\n👨‍🚀 ${v.courierName || '?'}\n🕐 Filloi: ${v.shiftStartTime || ''}\n⏱ Zgjati: ${v.shiftDuration || ''}`,
  'shift.close_reminder': (v) => `🌙 <b>Kujtesë: Mbyllni ndërrimin</b>\n\nMos harroni të bëni rakordimin.`,
  'ops.worker_liveness': (v) => `🛑 <b>Procesi nuk përgjigjet</b>\n\nKontrolloni sistemin.`,
  'ops.backup_failed': (v) => `🛑 <b>Kopja e sigurisë dështoi</b>\n\nKontrolloni sistemin.`,
  'ops.degradation_changed': (v) => `⚠️ <b>Statusi: Degraduar</b>\n\nKontrolloni panelin.`,
  'test': (v) => `🧪 <b>Test nga Dowiz</b>\n\n${v.message || 'Njoftimet funksionojnë.'}`,
  'order.pending_aging': (v) => `⏰ <b>#${v.shortOrderId || '???'} — Në pritje ${v.ageMinutes || '?'} min</b>\n\nNuk është konfirmuar ende.`,
};

const en: LocaleMessages = {
  'order.confirmed': (v) => [
    `✅ <b>ORDER CONFIRMED #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Items:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Total: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'order.rejected': (v) => [
    `❌ <b>ORDER REJECTED #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Items:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Total: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'order.created': (v) => [
    `🆕 <b>NEW ORDER #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.customerPhone ? `📞 ${v.customerPhone}` : '',
    v.deliveryAddress && v.orderTypeLabel === 'Delivery' ? `📍 ${v.deliveryAddress}` : '',
    v.deliveryInstructions ? `📝 ${v.deliveryInstructions}` : '',
    v.orderTypeLabel ? `📦 ${v.orderTypeLabel}` : '',
    '',
    v.items?.length ? `<b>📋 Items:</b>${fmtItems(v)}` : '',
    v.subtotalFmt ? `💰 Subtotal: ${v.subtotalFmt}` : '',
    v.deliveryFeeFmt ? `🚚 Delivery: ${v.deliveryFeeFmt}` : '',
    v.discountFmt ? `🏷️ Discount: -${v.discountFmt}` : '',
    v.taxFmt ? `🧾 VAT: ${v.taxFmt}` : '',
    v.totalFmt ? `<b>🎯 Total: ${v.totalFmt}</b>` : '',
    v.cashPayWithFmt ? `💵 Cash: ${v.cashPayWithFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.delivered': (v) => [
    `✅ <b>ORDER DELIVERED #${v.shortOrderId || '???'}</b>`,
    v.courierName ? `👨‍🚀 Courier: ${v.courierName}` : '',
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Items:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Total: ${v.totalFmt}</b>` : '',
    v.cashPayWithFmt ? `💵 Paid: ${v.cashPayWithFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.substitution_needs_human': (v) => [
    `⚠️ <b>#${v.shortOrderId || '???'} — Substitution needed</b>`,
    '',
    `Product was substituted. Customer requested contact. Decision required.`,
    v.items?.length ? `<b>📋 Items:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `💰 Total: ${v.totalFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.dwell_escalation': (v) => `⏰ <b>#${v.shortOrderId || '???'} — Pending ${v.ageMinutes || '?'} min</b>\n\nOrder not confirmed yet.`,
  'order.timeout_cancelled': (v) => `🚫 <b>#${v.shortOrderId || '???'} — Auto-cancelled</b>\n\nNot confirmed in time.`,
  'order.ready_for_pickup': (v) => `🍽️ <b>#${v.shortOrderId || '???'} — Ready for pickup</b>`,
  'cash.reconcile_discrepancy': (v) => `💱 <b>Cash discrepancy: ${v.discrepancyFmt || '?'} ALL</b>\n\nPlease reconcile.`,
  'delivery.flag_raised': (v) => `🚚 <b>#${v.shortOrderId || '???'} — Flag raised</b>\n\nCourier GPS does not match delivery location.`,
  'rating.low_received': (v) => `⭐ <b>#${v.shortOrderId || '???'} — Low rating: ${v.rating || '?'}/5</b>`,
  'courier.assigned': (v) => [
    `📦 <b>#${v.shortOrderId || '???'} — Courier en route</b>`,
    v.courierName ? `👨‍🚀 ${v.courierName}` : '',
    '',
    v.items?.length ? `<b>📋 Items:</b>${fmtItems(v)}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    v.totalFmt ? `<b>💰 Total: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'shift.started': (v) => `🟢 <b>Courier shift started</b>\n\n👨‍🚀 ${v.courierName || '?'}\n🕐 ${v.shiftStartTime || ''}`,
  'shift.closed': (v) => `🔴 <b>Courier shift ended</b>\n\n👨‍🚀 ${v.courierName || '?'}\n🕐 Started: ${v.shiftStartTime || ''}\n⏱ Duration: ${v.shiftDuration || ''}`,
  'shift.close_reminder': (v) => `🌙 <b>Reminder: Close your shift</b>\n\nDon't forget to reconcile the cash.`,
  'ops.worker_liveness': (v) => `🛑 <b>Worker unresponsive</b>\n\nCheck the system.`,
  'ops.backup_failed': (v) => `🛑 <b>Backup failed</b>\n\nCheck the system.`,
  'ops.degradation_changed': (v) => `⚠️ <b>Status: Degraded</b>\n\nCheck the admin panel.`,
  'test': (v) => `🧪 <b>Test from Dowiz</b>\n\n${v.message || 'Notifications are working.'}`,
  'order.pending_aging': (v) => `⏰ <b>#${v.shortOrderId || '???'} — Pending ${v.ageMinutes || '?'} min</b>\n\nNot confirmed yet.`,
};

const uk: LocaleMessages = {
  'order.confirmed': (v) => [
    `✅ <b>ЗАМОВЛЕННЯ ПІДТВЕРДЖЕНО #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Товари:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Всього: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'order.rejected': (v) => [
    `❌ <b>ЗАМОВЛЕННЯ ВІДХИЛЕНО #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Товари:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Всього: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'order.created': (v) => [
    `🆕 <b>НОВЕ ЗАМОВЛЕННЯ #${v.shortOrderId || '???'}</b>`,
    v.customerName ? `👤 ${v.customerName}` : '',
    v.customerPhone ? `📞 ${v.customerPhone}` : '',
    v.deliveryAddress && v.orderTypeLabel === 'Доставка' ? `📍 ${v.deliveryAddress}` : '',
    v.deliveryInstructions ? `📝 ${v.deliveryInstructions}` : '',
    v.orderTypeLabel ? `📦 ${v.orderTypeLabel}` : '',
    '',
    v.items?.length ? `<b>📋 Товари:</b>${fmtItems(v)}` : '',
    v.subtotalFmt ? `💰 Проміжний підсумок: ${v.subtotalFmt}` : '',
    v.deliveryFeeFmt ? `🚚 Доставка: ${v.deliveryFeeFmt}` : '',
    v.discountFmt ? `🏷️ Знижка: -${v.discountFmt}` : '',
    v.taxFmt ? `🧾 ПДВ: ${v.taxFmt}` : '',
    v.totalFmt ? `<b>🎯 Всього: ${v.totalFmt}</b>` : '',
    v.cashPayWithFmt ? `💵 Готівка: ${v.cashPayWithFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.delivered': (v) => [
    `✅ <b>ЗАМОВЛЕННЯ ДОСТАВЛЕНО #${v.shortOrderId || '???'}</b>`,
    v.courierName ? `👨‍🚀 Кур'єр: ${v.courierName}` : '',
    v.customerName ? `👤 ${v.customerName}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    '',
    v.items?.length ? `<b>📋 Товари:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `<b>💰 Всього: ${v.totalFmt}</b>` : '',
    v.cashPayWithFmt ? `💵 Оплачено: ${v.cashPayWithFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.substitution_needs_human': (v) => [
    `⚠️ <b>#${v.shortOrderId || '???'} — Потрібна заміна</b>`,
    '',
    `Товар замінено. Клієнт запросив контакт. Потрібне рішення.`,
    v.items?.length ? `<b>📋 Товари:</b>${fmtItems(v)}` : '',
    v.totalFmt ? `💰 Всього: ${v.totalFmt}` : '',
  ].filter(Boolean).join('\n'),
  'order.dwell_escalation': (v) => `⏰ <b>#${v.shortOrderId || '???'} — Очікує ${v.ageMinutes || '?'} хв</b>\n\nЗамовлення не підтверджено.`,
  'order.timeout_cancelled': (v) => `🚫 <b>#${v.shortOrderId || '???'} — Скасовано автоматично</b>\n\nНе підтверджено вчасно.`,
  'order.ready_for_pickup': (v) => `🍽️ <b>#${v.shortOrderId || '???'} — Готово до видачі</b>`,
  'cash.reconcile_discrepancy': (v) => `💱 <b>Розбіжність у касі: ${v.discrepancyFmt || '?'} ALL</b>\n\nЗвірте касу.`,
  'delivery.flag_raised': (v) => `🚚 <b>#${v.shortOrderId || '???'} — Прапорець піднято</b>\n\nGPS кур'єра не збігається.`,
  'rating.low_received': (v) => `⭐ <b>#${v.shortOrderId || '???'} — Низький рейтинг: ${v.rating || '?'}/5</b>`,
  'courier.assigned': (v) => [
    `📦 <b>#${v.shortOrderId || '???'} — Кур'єр в дорозі</b>`,
    v.courierName ? `👨‍🚀 ${v.courierName}` : '',
    '',
    v.items?.length ? `<b>📋 Товари:</b>${fmtItems(v)}` : '',
    v.deliveryAddress ? `📍 ${v.deliveryAddress}` : '',
    v.totalFmt ? `<b>💰 Всього: ${v.totalFmt}</b>` : '',
  ].filter(Boolean).join('\n'),
  'shift.started': (v) => `🟢 <b>Зміну кур'єра розпочато</b>\n\n👨‍🚀 ${v.courierName || '?'}\n🕐 ${v.shiftStartTime || ''}`,
  'shift.closed': (v) => `🔴 <b>Зміну кур'єра завершено</b>\n\n👨‍🚀 ${v.courierName || '?'}\n🕐 Початок: ${v.shiftStartTime || ''}\n⏱ Тривалість: ${v.shiftDuration || ''}`,
  'shift.close_reminder': (v) => `🌙 <b>Нагадування: Закрийте зміну</b>\n\nНе забудьте звірити касу.`,
  'ops.worker_liveness': (v) => `🛑 <b>Процес не відповідає</b>\n\nПеревірте систему.`,
  'ops.backup_failed': (v) => `🛑 <b>Резервне копіювання не вдалося</b>\n\nПеревірте систему.`,
  'ops.degradation_changed': (v) => `⚠️ <b>Статус: Деградація</b>\n\nДеталі в панелі.`,
  'test': (v) => `🧪 <b>Тест від Dowiz</b>\n\n${v.message || 'Сповіщення працюють.'}`,
  'order.pending_aging': (v) => `⏰ <b>#${v.shortOrderId || '???'} — Очікує ${v.ageMinutes || '?'} хв</b>\n\nЩе не підтверджено.`,
};

const messages: Record<Locale, LocaleMessages> = { sq, en, uk };

export function getMessage(locale: Locale, key: string, vars: MessageVars): string {
  const fn = messages[locale]?.[key];
  if (!fn) return messages['en'][key]?.(vars) || 'Unknown notification';
  return fn(vars);
}
