// The invite-send words (W-INVITE). Merged into the console's dictionary by
// couriers.js for every language the console has; a language with no row here
// reads English (never a blank control).
//
// ASCII QUOTES ONLY, and no apostrophes inside the words: a typographic quote
// once took down the whole console.

export const WORDS = {
  sq: {
    invs_send: 'Dërgo lidhjen', invs_self: 'E ndaj vetë', invs_wa: 'Me WhatsApp në këtë numër', invs_tg: 'Me Telegram',
    invs_chat: 'ID e bisedës në Telegram', invs_chatHint: 'Korrieri shtyp Start te boti juaj dhe ju dërgon numrin që tregon.',
    invs_noRail: 'Ky lokal nuk ka WhatsApp ose Telegram të lidhur, ndaj lidhja tregohet që ta ndani vetë. Lidhini te Lokali > Njoftimet.',
    invs_queued: 'në radhë për dërgim', invs_retrying: 'po provohet sërish', invs_parked: 'pret: kanali nuk është i lidhur',
    invs_sent: 'iu dorëzua', invs_failed: 'nuk u dërgua', invs_unknown: 'gjendja nuk lexohet', invs_error: 'nuk u fut në radhë', invs_noLink: 'lokali nuk ka adresë',
  },
  en: {
    invs_send: 'Send the link', invs_self: 'I will share it myself', invs_wa: 'By WhatsApp to this number', invs_tg: 'By Telegram',
    invs_chat: 'Telegram chat id', invs_chatHint: 'The courier presses Start on your bot and sends you the number it shows.',
    invs_noRail: 'This venue has no WhatsApp or Telegram connected, so the link is shown for you to share. Connect one under Venue > Notifications.',
    invs_queued: 'queued to send', invs_retrying: 'retrying', invs_parked: 'waiting: the channel is not connected',
    invs_sent: 'handed to', invs_failed: 'not delivered', invs_unknown: 'state unreadable', invs_error: 'not queued', invs_noLink: 'the venue has no address',
  },
  uk: {
    invs_send: 'Надіслати посилання', invs_self: 'Поділюся сам', invs_wa: 'У WhatsApp на цей номер', invs_tg: 'У Telegram',
    invs_chat: 'ID чату в Telegram', invs_chatHint: 'Курʼєр натискає Start у вашому боті й надсилає вам число, яке той показує.',
    invs_noRail: 'У закладу не підключено WhatsApp чи Telegram, тож посилання показано, щоб ви поділилися ним самі. Підключіть у Заклад > Сповіщення.',
    invs_queued: 'у черзі на відправку', invs_retrying: 'повторна спроба', invs_parked: 'чекає: канал не підключено',
    invs_sent: 'передано в', invs_failed: 'не доставлено', invs_unknown: 'стан не читається', invs_error: 'не поставлено в чергу', invs_noLink: 'у закладу немає адреси',
  },
  ru: {
    invs_send: 'Отправить ссылку', invs_self: 'Поделюсь сам', invs_wa: 'В WhatsApp на этот номер', invs_tg: 'В Telegram',
    invs_chat: 'ID чата в Telegram', invs_chatHint: 'Курьер нажимает Start в вашем боте и присылает вам число, которое тот показывает.',
    invs_noRail: 'У заведения не подключены WhatsApp или Telegram, поэтому ссылка показана, чтобы вы поделились ею сами. Подключите в Заведение > Уведомления.',
    invs_queued: 'в очереди на отправку', invs_retrying: 'повторная попытка', invs_parked: 'ждёт: канал не подключён',
    invs_sent: 'передано в', invs_failed: 'не доставлено', invs_unknown: 'состояние не читается', invs_error: 'не поставлено в очередь', invs_noLink: 'у заведения нет адреса',
  },
};
