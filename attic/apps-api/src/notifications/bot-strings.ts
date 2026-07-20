// Localized strings for the inbound Telegram bot webhook (telegram-webhook.ts).
// Mirrors the server-side i18n pattern in locales.ts: a per-locale dictionary keyed
// by a short stable id, with a botT(locale, key, vars) helper and a
// locale → en → key fallback chain. Use {{var}} for dynamic interpolation.
//
// Tone matches the notification messages in locales.ts. Albanian = Standard Albanian,
// Ukrainian = standard Ukrainian.
import type { Locale } from './locales.js';

type LocaleStrings = Record<string, string>;

const sq: LocaleStrings = {
  // /start <token> connect flow
  'start.invalid_token': 'Ju lutemi jepni një token të vlefshëm.',
  'start.invalid_login_link': '⚠️ Lidhje hyrjeje e pavlefshme.',
  'start.login_expired': '⚠️ Kjo lidhje hyrjeje ka skaduar. Ju lutemi filloni përsëri nga shfletuesi juaj.',
  'start.logged_in': '✅ Hyrët! Kthehuni te shfletuesi — do të vazhdojë automatikisht.',
  'start.token_invalid_or_expired': 'Token i pavlefshëm ose i skaduar.',
  'start.connected': '✅ Telegram u lidh me sukses! Do të merrni njoftime.',

  // /stop
  'stop.disconnected': '🔌 Telegram u shkëput. Nuk do të merrni më njoftime.',
  'stop.not_connected': 'Telegram nuk ishte i lidhur.',

  // /open shift
  'open.not_linked': '⚠️ Telegram nuk është i lidhur me asnjë lokacion. Përdorni /start <token>.',
  'open.shift_opened': '🔓 Ndërrimi u hap. Filloni të pranoni porosi.',
  'open.shift_error_reason': '⚠️ Nuk mund të hapet ndërrimi: {{message}}',
  'open.shift_error': '⚠️ Gabim gjatë hapjes së ndërrimit',

  // /close shift
  'close.redirect': '🔒 Prisni, po ju ridrejtojmë te mbyllja e ndërrimit...\n🔗 https://app.dowiz.org/courier/shifts/close',

  // /store
  'store.not_linked': '⚠️ Telegram nuk është i lidhur me asnjë lokacion. Përdorni /start <token>.',
  'store.state_closed': '🔴 Pranimi ËSHTË MBYLLUR',
  'store.state_open': '🟢 Pranimi ËSHTË HAPUR',
  'store.btn_open': '🟢 Hap pranimin',
  'store.btn_close': '🔴 Mbyll pranimin',

  // store.* callbacks
  'store.open_failed': '⚠️ Dështoi. Provoni në aplikacion.',
  'store.open_noop': '🟢 Pranimi është tashmë i hapur',
  'store.open_ok': '🟢 Pranimi u hap',
  'store.not_found': '⚠️ Lokacioni nuk u gjet',
  'store.already_closed': '🔴 Pranimi është tashmë i mbyllur',
  'store.close_confirm_prompt': '⚠️ Të mbyllet pranimi për «{{name}}»? Klientët do të shohin «mbyllur».',
  'store.btn_confirm_close': '🔴 Po, mbyll',
  'store.invalid_request': '⚠️ Kërkesë e pavlefshme',
  'store.confirm_expired': '⚠️ Konfirmimi skadoi. Provoni përsëri ose në aplikacion.',
  'store.close_ok': '🔴 Pranimi u mbyll',

  // /settings + pref.* callbacks
  'settings.not_linked': '⚠️ Telegram nuk është i lidhur me asnjë lokacion. Përdorni /start <token>.',
  'settings.header': '«{{name}}» — njoftimet',
  'settings.transactional_always': '🔴 Transaksionale: gjithmonë të aktivizuara',
  'settings.operational_on': '🔔 Operacionale: aktivizuar',
  'settings.operational_off': '🔕 Operacionale: çaktivizuar',
  'settings.quality_on': '🔔 Cilësi/analitikë: aktivizuar',
  'settings.quality_off': '🔕 Cilësi/analitikë: çaktivizuar',
  'settings.btn_op_disable': '🔕 Çaktivizo operacionalet',
  'settings.btn_op_enable': '🔔 Aktivizo operacionalet',
  'settings.btn_q_disable': '🔕 Çaktivizo cilësinë',
  'settings.btn_q_enable': '🔔 Aktivizo cilësinë',
  'pref.invalid_request': '⚠️ Kërkesë e pavlefshme',
  'pref.update_failed': '⚠️ Përditësimi dështoi',
  'pref.cat_operational': 'Operacionale',
  'pref.cat_quality': 'Cilësi/analitikë',
  'pref.set_on': '🔔 {{label}}: aktivizuar',
  'pref.set_off': '🔕 {{label}}: çaktivizuar',

  // order.* callbacks (resultText)
  'order.confirmed': '✅ POROSIA E KONFIRMUAR',
  'order.rejected': '❌ POROSIA E REFUZUAR',
  'order.not_found': '❌ Porosia nuk u gjet',
  'order.already_confirmed': '✅ Tashmë e konfirmuar',
  'order.cannot_confirm_cancelled': '❌ Nuk mund të konfirmohet porosi e anuluar',
  'order.cannot_confirm_state': '⚠️ Nuk mund të konfirmohet porosia në gjendjen {{status}}',
  'order.error_confirming': '⚠️ Gabim gjatë konfirmimit të porosisë',
  'order.already_rejected': '❌ Tashmë e refuzuar',
  'order.cannot_reject_confirmed': '❌ Nuk mund të refuzohet porosi e konfirmuar',
  'order.cannot_reject_state': '⚠️ Nuk mund të refuzohet porosia në gjendjen {{status}}',
  'order.error_rejecting': '⚠️ Gabim gjatë refuzimit të porosisë',
  'order.followup': '{{result}}\n\nPorosia #{{shortId}}',

  // order.reject_choose
  'order.reject_select': 'Zgjidhni arsyen e refuzimit të porosisë #{{orderId}}:',
  'order.reason_changed_mind': 'Klienti ndërroi mendje',
  'order.reason_unavailable': 'Produkti i padisponueshëm',
  'order.reason_wrong_address': 'Adresë e gabuar',
  'order.reason_stop_list': 'Shto në stop-listë',

  // answerCallbackQuery short texts + generic
  'cb.invalid_request': 'Kërkesë e pavlefshme',
  'cb.invalid_action': 'Veprim i pavlefshëm',
  'cb.invalid_order': 'Porosi e pavlefshme',
  'cb.order_not_found': 'Porosia nuk u gjet',
  'cb.not_linked_location': 'Llogaria nuk është e lidhur me këtë lokacion',
  'cb.not_linked': 'Llogaria nuk është e lidhur',
  'cb.action_not_supported': 'Veprimi nuk mbështetet',
  'cb.reconnect': 'Rilidhni Telegram për të përdorur këtë veprim',
  'cb.unauthorized_member': 'Pa autorizim: nuk jeni anëtar i këtij lokacioni',
  'cb.unauthorized': 'Pa autorizim',
  'cb.unknown_action': 'Veprim i panjohur',
  'cb.processing_error': 'Gabim gjatë përpunimit',

  // generic message-handler error
  'msg.error': 'Ndodhi një gabim gjatë përpunimit të kërkesës suaj.',
};

const en: LocaleStrings = {
  'start.invalid_token': 'Please provide a valid token.',
  'start.invalid_login_link': '⚠️ Invalid login link.',
  'start.login_expired': '⚠️ This login link has expired. Please start again from your browser.',
  'start.logged_in': '✅ Logged in! Return to your browser — it will continue automatically.',
  'start.token_invalid_or_expired': 'Invalid or expired token.',
  'start.connected': '✅ Telegram connected successfully! You will receive notifications.',

  'stop.disconnected': '🔌 Telegram disconnected. You will no longer receive notifications.',
  'stop.not_connected': 'Telegram was not connected.',

  'open.not_linked': '⚠️ Telegram is not linked to any location. Use /start <token>.',
  'open.shift_opened': '🔓 Shift opened. Start accepting orders.',
  'open.shift_error_reason': '⚠️ Cannot open shift: {{message}}',
  'open.shift_error': '⚠️ Error opening shift',

  'close.redirect': '🔒 Please wait, redirecting you to close the shift...\n🔗 https://app.dowiz.org/courier/shifts/close',

  'store.not_linked': '⚠️ Telegram is not linked to any location. Use /start <token>.',
  'store.state_closed': '🔴 Ordering is CLOSED',
  'store.state_open': '🟢 Ordering is OPEN',
  'store.btn_open': '🟢 Open ordering',
  'store.btn_close': '🔴 Close ordering',

  'store.open_failed': '⚠️ Failed. Try in the app.',
  'store.open_noop': '🟢 Ordering is already open',
  'store.open_ok': '🟢 Ordering opened',
  'store.not_found': '⚠️ Location not found',
  'store.already_closed': '🔴 Ordering is already closed',
  'store.close_confirm_prompt': '⚠️ Close ordering for «{{name}}»? Customers will see «closed».',
  'store.btn_confirm_close': '🔴 Yes, close',
  'store.invalid_request': '⚠️ Invalid request',
  'store.confirm_expired': '⚠️ Confirmation expired. Try again or in the app.',
  'store.close_ok': '🔴 Ordering closed',

  'settings.not_linked': '⚠️ Telegram is not linked to any location. Use /start <token>.',
  'settings.header': '«{{name}}» — notifications',
  'settings.transactional_always': '🔴 Transactional: always enabled',
  'settings.operational_on': '🔔 Operational: enabled',
  'settings.operational_off': '🔕 Operational: disabled',
  'settings.quality_on': '🔔 Quality/analytics: enabled',
  'settings.quality_off': '🔕 Quality/analytics: disabled',
  'settings.btn_op_disable': '🔕 Disable operational',
  'settings.btn_op_enable': '🔔 Enable operational',
  'settings.btn_q_disable': '🔕 Disable quality',
  'settings.btn_q_enable': '🔔 Enable quality',
  'pref.invalid_request': '⚠️ Invalid request',
  'pref.update_failed': '⚠️ Update failed',
  'pref.cat_operational': 'Operational',
  'pref.cat_quality': 'Quality/analytics',
  'pref.set_on': '🔔 {{label}}: enabled',
  'pref.set_off': '🔕 {{label}}: disabled',

  'order.confirmed': '✅ ORDER CONFIRMED',
  'order.rejected': '❌ ORDER REJECTED',
  'order.not_found': '❌ Order not found',
  'order.already_confirmed': '✅ Already confirmed',
  'order.cannot_confirm_cancelled': '❌ Cannot confirm cancelled order',
  'order.cannot_confirm_state': '⚠️ Cannot confirm order in state {{status}}',
  'order.error_confirming': '⚠️ Error confirming order',
  'order.already_rejected': '❌ Already rejected',
  'order.cannot_reject_confirmed': '❌ Cannot reject confirmed order',
  'order.cannot_reject_state': '⚠️ Cannot reject order in state {{status}}',
  'order.error_rejecting': '⚠️ Error rejecting order',
  'order.followup': '{{result}}\n\nOrder #{{shortId}}',

  'order.reject_select': 'Select reason for rejecting order #{{orderId}}:',
  'order.reason_changed_mind': 'Client changed mind',
  'order.reason_unavailable': 'Item unavailable',
  'order.reason_wrong_address': 'Wrong address',
  'order.reason_stop_list': 'Add to stop-list',

  'cb.invalid_request': 'Invalid request',
  'cb.invalid_action': 'Invalid action',
  'cb.invalid_order': 'Invalid order',
  'cb.order_not_found': 'Order not found',
  'cb.not_linked_location': 'Account not linked to this location',
  'cb.not_linked': 'Account not linked',
  'cb.action_not_supported': 'Action not supported',
  'cb.reconnect': 'Reconnect Telegram to use this action',
  'cb.unauthorized_member': 'Unauthorized: not a member of this location',
  'cb.unauthorized': 'Unauthorized',
  'cb.unknown_action': 'Unknown action',
  'cb.processing_error': 'Processing error',

  'msg.error': 'An error occurred while processing your request.',
};

const uk: LocaleStrings = {
  'start.invalid_token': 'Будь ласка, надайте дійсний токен.',
  'start.invalid_login_link': '⚠️ Недійсне посилання для входу.',
  'start.login_expired': '⚠️ Це посилання для входу протерміновано. Будь ласка, почніть знову з браузера.',
  'start.logged_in': '✅ Ви увійшли! Поверніться до браузера — він продовжить автоматично.',
  'start.token_invalid_or_expired': 'Недійсний або протермінований токен.',
  'start.connected': '✅ Телеграм успішно підключено! Ви будете отримувати сповіщення.',

  'stop.disconnected': '🔌 Телеграм відключено. Ви не будете отримувати сповіщення.',
  'stop.not_connected': 'Телеграм не був підключений.',

  'open.not_linked': '⚠️ Телеграм не підключений до жодного закладу. Використайте /start <token>.',
  'open.shift_opened': '🔓 Зміна відкрита. Почніть приймати замовлення.',
  'open.shift_error_reason': '⚠️ Неможливо відкрити зміну: {{message}}',
  'open.shift_error': '⚠️ Помилка при відкритті зміни',

  'close.redirect': '🔒 Зачекайте, перенаправляємо до закриття зміни...\n🔗 https://app.dowiz.org/courier/shifts/close',

  'store.not_linked': '⚠️ Телеграм не підключений до жодного закладу. Використайте /start <token>.',
  'store.state_closed': '🔴 Приймання ЗАКРИТО',
  'store.state_open': '🟢 Приймання ВІДКРИТО',
  'store.btn_open': '🟢 Відкрити приймання',
  'store.btn_close': '🔴 Закрити приймання',

  'store.open_failed': '⚠️ Не вдалося. Спробуйте у застосунку.',
  'store.open_noop': '🟢 Приймання вже відкрите',
  'store.open_ok': '🟢 Приймання відкрито',
  'store.not_found': '⚠️ Заклад не знайдено',
  'store.already_closed': '🔴 Приймання вже закрите',
  'store.close_confirm_prompt': '⚠️ Закрити приймання для «{{name}}»? Клієнти бачитимуть «зачинено».',
  'store.btn_confirm_close': '🔴 Так, закрити',
  'store.invalid_request': '⚠️ Недійсний запит',
  'store.confirm_expired': '⚠️ Підтвердження протерміновано. Спробуйте ще раз або у застосунку.',
  'store.close_ok': '🔴 Приймання закрито',

  'settings.not_linked': '⚠️ Телеграм не підключений до жодного закладу. Використайте /start <token>.',
  'settings.header': '«{{name}}» — сповіщення',
  'settings.transactional_always': '🔴 Транзакційні: завжди увімкнено',
  'settings.operational_on': '🔔 Операційні: увімкнено',
  'settings.operational_off': '🔕 Операційні: вимкнено',
  'settings.quality_on': '🔔 Якість/аналітика: увімкнено',
  'settings.quality_off': '🔕 Якість/аналітика: вимкнено',
  'settings.btn_op_disable': '🔕 Вимкнути операційні',
  'settings.btn_op_enable': '🔔 Увімкнути операційні',
  'settings.btn_q_disable': '🔕 Вимкнути якість',
  'settings.btn_q_enable': '🔔 Увімкнути якість',
  'pref.invalid_request': '⚠️ Недійсний запит',
  'pref.update_failed': '⚠️ Не вдалося оновити',
  'pref.cat_operational': 'Операційні',
  'pref.cat_quality': 'Якість/аналітика',
  'pref.set_on': '🔔 {{label}}: увімкнено',
  'pref.set_off': '🔕 {{label}}: вимкнено',

  'order.confirmed': '✅ ЗАМОВЛЕННЯ ПІДТВЕРДЖЕНО',
  'order.rejected': '❌ ЗАМОВЛЕННЯ ВІДХИЛЕНО',
  'order.not_found': '❌ Замовлення не знайдено',
  'order.already_confirmed': '✅ Вже підтверджено',
  'order.cannot_confirm_cancelled': '❌ Неможливо підтвердити скасоване замовлення',
  'order.cannot_confirm_state': '⚠️ Неможливо підтвердити замовлення у стані {{status}}',
  'order.error_confirming': '⚠️ Помилка при підтвердженні замовлення',
  'order.already_rejected': '❌ Вже відхилено',
  'order.cannot_reject_confirmed': '❌ Неможливо відхилити підтверджене замовлення',
  'order.cannot_reject_state': '⚠️ Неможливо відхилити замовлення у стані {{status}}',
  'order.error_rejecting': '⚠️ Помилка при відхиленні замовлення',
  'order.followup': '{{result}}\n\nЗамовлення #{{shortId}}',

  'order.reject_select': 'Виберіть причину відхилення замовлення #{{orderId}}:',
  'order.reason_changed_mind': 'Клієнт передумав',
  'order.reason_unavailable': 'Товар недоступний',
  'order.reason_wrong_address': 'Неправильна адреса',
  'order.reason_stop_list': 'Додати до стоп-листа',

  'cb.invalid_request': 'Недійсний запит',
  'cb.invalid_action': 'Недійсна дія',
  'cb.invalid_order': 'Недійсне замовлення',
  'cb.order_not_found': 'Замовлення не знайдено',
  'cb.not_linked_location': 'Обліковий запис не пов’язаний із цим закладом',
  'cb.not_linked': 'Обліковий запис не пов’язаний',
  'cb.action_not_supported': 'Дія не підтримується',
  'cb.reconnect': 'Перепідключіть Телеграм, щоб використати цю дію',
  'cb.unauthorized_member': 'Немає доступу: ви не учасник цього закладу',
  'cb.unauthorized': 'Немає доступу',
  'cb.unknown_action': 'Невідома дія',
  'cb.processing_error': 'Помилка обробки',

  'msg.error': 'Виникла помилка при обробці вашого запиту.',
};

const strings: Record<Locale, LocaleStrings> = { sq, en, uk };

export function botT(locale: Locale, key: string, vars?: Record<string, string>): string {
  const raw = strings[locale]?.[key] ?? strings['en']?.[key] ?? key;
  if (!vars) return raw;
  return raw.replace(/\{\{(\w+)\}\}/g, (_m, name: string) => {
    const val = vars[name];
    return val === undefined ? `{{${name}}}` : val;
  });
}
