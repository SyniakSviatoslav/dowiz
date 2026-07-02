// Localized title/body strings for web-push notifications (sq/en/uk), mirroring the
// event coverage of locales.ts (Telegram bodies). Web push shows a short title + body
// only — no rich formatting — so these are concise. The payload shape is unchanged;
// this module only supplies the human-facing text. `vars` carries the same data the
// Telegram path renders (shortOrderId, money already formatted, etc.).
import type { Locale } from './locales.js';
import type { NotificationEventType } from './provider.js';

export interface PushVars {
  shortOrderId?: string;
  ageMinutes?: string;
  discrepancyFmt?: string;
  rating?: string;
  courierName?: string;
  message?: string;
}

interface PushText {
  title: string;
  body: string;
}

type PushFn = (v: PushVars) => PushText;
type LocalePush = Partial<Record<NotificationEventType, PushFn>>;

const ord = (v: PushVars) => `#${v.shortOrderId || '???'}`;

const sq: LocalePush = {
  'order.created': (v) => ({ title: `Porosi e re ${ord(v)}`, body: 'Konfirmoni ose refuzoni.' }),
  'order.confirmed': (v) => ({ title: `Porosia e konfirmuar ${ord(v)}`, body: 'Porosia u konfirmua.' }),
  'order.rejected': (v) => ({ title: `Porosia e refuzuar ${ord(v)}`, body: 'Porosia u refuzua.' }),
  'order.delivered': (v) => ({ title: `Porosia e dorëzuar ${ord(v)}`, body: 'Porosia u dorëzua.' }),
  'order.substitution_needs_human': (v) => ({ title: `Zëvendësim i nevojshëm ${ord(v)}`, body: 'Nevojitet vendim.' }),
  'order.dwell_escalation': (v) => ({ title: `Në pritje ${ord(v)}`, body: `Pa konfirmim prej ${v.ageMinutes || '?'} min.` }),
  'order.timeout_cancelled': (v) => ({ title: `Anuluar automatikisht ${ord(v)}`, body: 'Nuk u konfirmua brenda afatit.' }),
  'order.dispatch_failed': (v) => ({ title: `Nuk u gjet korrier ${ord(v)}`, body: 'Caktoni një korrier manualisht ose anuloni.' }),
  'order.ready_for_pickup': (v) => ({ title: `Gati për marrje ${ord(v)}`, body: 'Porosia është gati.' }),
  'order.pending_aging': (v) => ({ title: `Në pritje ${ord(v)}`, body: `Pa konfirmim prej ${v.ageMinutes || '?'} min.` }),
  'cash.reconcile_discrepancy': (v) => ({ title: 'Diferencë në arkë', body: `${v.discrepancyFmt || '?'} — bëni rakordimin.` }),
  'delivery.flag_raised': (v) => ({ title: `Flamur i ngritur ${ord(v)}`, body: 'GPS nuk përputhet me vendndodhjen.' }),
  'rating.low_received': (v) => ({ title: `Vlerësim i ulët ${ord(v)}`, body: `${v.rating || '?'}/5` }),
  'courier.assigned': (v) => ({ title: `Korrieri në rrugë ${ord(v)}`, body: v.courierName || 'Korrieri u caktua.' }),
  'shift.started': (v) => ({ title: 'Ndërrimi filloi', body: v.courierName || '' }),
  'shift.closed': (v) => ({ title: 'Ndërrimi përfundoi', body: v.courierName || '' }),
  'shift.close_reminder': () => ({ title: 'Kujtesë: Mbyllni ndërrimin', body: 'Mos harroni të bëni rakordimin.' }),
  'ops.worker_liveness': () => ({ title: 'Procesi nuk përgjigjet', body: 'Kontrolloni sistemin.' }),
  'ops.backup_failed': () => ({ title: 'Kopja e sigurisë dështoi', body: 'Kontrolloni sistemin.' }),
  'ops.degradation_changed': () => ({ title: 'Statusi: Degraduar', body: 'Kontrolloni panelin.' }),
  'test': (v) => ({ title: 'Test nga Dowiz', body: v.message || 'Njoftimet funksionojnë.' }),
};

const en: LocalePush = {
  'order.created': (v) => ({ title: `New order ${ord(v)}`, body: 'Confirm or reject.' }),
  'order.confirmed': (v) => ({ title: `Order confirmed ${ord(v)}`, body: 'The order was confirmed.' }),
  'order.rejected': (v) => ({ title: `Order rejected ${ord(v)}`, body: 'The order was rejected.' }),
  'order.delivered': (v) => ({ title: `Order delivered ${ord(v)}`, body: 'The order was delivered.' }),
  'order.substitution_needs_human': (v) => ({ title: `Substitution needed ${ord(v)}`, body: 'Decision required.' }),
  'order.dwell_escalation': (v) => ({ title: `Pending ${ord(v)}`, body: `Not confirmed for ${v.ageMinutes || '?'} min.` }),
  'order.timeout_cancelled': (v) => ({ title: `Auto-cancelled ${ord(v)}`, body: 'Not confirmed in time.' }),
  'order.dispatch_failed': (v) => ({ title: `No courier found ${ord(v)}`, body: 'Assign a courier manually or cancel.' }),
  'order.ready_for_pickup': (v) => ({ title: `Ready for pickup ${ord(v)}`, body: 'The order is ready.' }),
  'order.pending_aging': (v) => ({ title: `Pending ${ord(v)}`, body: `Not confirmed for ${v.ageMinutes || '?'} min.` }),
  'cash.reconcile_discrepancy': (v) => ({ title: 'Cash discrepancy', body: `${v.discrepancyFmt || '?'} — please reconcile.` }),
  'delivery.flag_raised': (v) => ({ title: `Flag raised ${ord(v)}`, body: 'Courier GPS does not match delivery location.' }),
  'rating.low_received': (v) => ({ title: `Low rating ${ord(v)}`, body: `${v.rating || '?'}/5` }),
  'courier.assigned': (v) => ({ title: `Courier en route ${ord(v)}`, body: v.courierName || 'Courier assigned.' }),
  'shift.started': (v) => ({ title: 'Courier shift started', body: v.courierName || '' }),
  'shift.closed': (v) => ({ title: 'Courier shift ended', body: v.courierName || '' }),
  'shift.close_reminder': () => ({ title: 'Reminder: Close your shift', body: "Don't forget to reconcile the cash." }),
  'ops.worker_liveness': () => ({ title: 'Worker unresponsive', body: 'Check the system.' }),
  'ops.backup_failed': () => ({ title: 'Backup failed', body: 'Check the system.' }),
  'ops.degradation_changed': () => ({ title: 'Status: Degraded', body: 'Check the admin panel.' }),
  'test': (v) => ({ title: 'Test from Dowiz', body: v.message || 'Notifications are working.' }),
};

const uk: LocalePush = {
  'order.created': (v) => ({ title: `Нове замовлення ${ord(v)}`, body: 'Підтвердьте або відхиліть.' }),
  'order.confirmed': (v) => ({ title: `Замовлення підтверджено ${ord(v)}`, body: 'Замовлення підтверджено.' }),
  'order.rejected': (v) => ({ title: `Замовлення відхилено ${ord(v)}`, body: 'Замовлення відхилено.' }),
  'order.delivered': (v) => ({ title: `Замовлення доставлено ${ord(v)}`, body: 'Замовлення доставлено.' }),
  'order.substitution_needs_human': (v) => ({ title: `Потрібна заміна ${ord(v)}`, body: 'Потрібне рішення.' }),
  'order.dwell_escalation': (v) => ({ title: `Очікує ${ord(v)}`, body: `Не підтверджено ${v.ageMinutes || '?'} хв.` }),
  'order.timeout_cancelled': (v) => ({ title: `Скасовано автоматично ${ord(v)}`, body: 'Не підтверджено вчасно.' }),
  'order.dispatch_failed': (v) => ({ title: `Кур'єра не знайдено ${ord(v)}`, body: "Призначте кур'єра вручну або скасуйте." }),
  'order.ready_for_pickup': (v) => ({ title: `Готово до видачі ${ord(v)}`, body: 'Замовлення готове.' }),
  'order.pending_aging': (v) => ({ title: `Очікує ${ord(v)}`, body: `Не підтверджено ${v.ageMinutes || '?'} хв.` }),
  'cash.reconcile_discrepancy': (v) => ({ title: 'Розбіжність у касі', body: `${v.discrepancyFmt || '?'} — звірте касу.` }),
  'delivery.flag_raised': (v) => ({ title: `Прапорець піднято ${ord(v)}`, body: "GPS кур'єра не збігається." }),
  'rating.low_received': (v) => ({ title: `Низький рейтинг ${ord(v)}`, body: `${v.rating || '?'}/5` }),
  'courier.assigned': (v) => ({ title: `Кур'єр в дорозі ${ord(v)}`, body: v.courierName || "Кур'єра призначено." }),
  'shift.started': (v) => ({ title: "Зміну кур'єра розпочато", body: v.courierName || '' }),
  'shift.closed': (v) => ({ title: "Зміну кур'єра завершено", body: v.courierName || '' }),
  'shift.close_reminder': () => ({ title: 'Нагадування: Закрийте зміну', body: 'Не забудьте звірити касу.' }),
  'ops.worker_liveness': () => ({ title: 'Процес не відповідає', body: 'Перевірте систему.' }),
  'ops.backup_failed': () => ({ title: 'Резервне копіювання не вдалося', body: 'Перевірте систему.' }),
  'ops.degradation_changed': () => ({ title: 'Статус: Деградація', body: 'Деталі в панелі.' }),
  'test': (v) => ({ title: 'Тест від Dowiz', body: v.message || 'Сповіщення працюють.' }),
};

const pushMessages: Record<Locale, LocalePush> = { sq, en, uk };

const FALLBACK: PushText = { title: 'DeliveryOS', body: 'New update' };

export function getPushText(locale: Locale, type: NotificationEventType, vars: PushVars): PushText {
  const fn = pushMessages[locale]?.[type] ?? en[type];
  return fn ? fn(vars) : FALLBACK;
}
