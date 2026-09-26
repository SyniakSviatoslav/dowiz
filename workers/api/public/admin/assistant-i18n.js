// The agent panel's words (lane W-KITCHEN, 2026-09-26), merged into the
// console's table at import like `kitchen-i18n.js`. The four `as*` starter
// lines are SENT as typed, so each is written in words the hub's grammar reads
// (pinned by `voice/kitchen/tests.rs` the_starter_lines_mean_what_they_say).
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { T } from '/admin/i18n.js';
import { merge } from '/admin/kitchen-i18n.js';

export const WORDS = {
  sq: {
    asTitle: 'Asistenti', asHint: 'Shkruani çfarë doni: një pyetje, ose një veprim si «erdhi 4 kg salmon». Çdo ndryshim ju pyet para se të bëhet.',
    asPlaceholder: 'Shkruani këtu…', asSend: 'Dërgo', asConfirm: 'Po, bëje', asCancel: 'Jo',
    asDone: 'U bë.', asCancelled: 'U anulua.', asFailed: 'Nuk u bë.', asThinking: 'Po mendoj…',
    asOpened: 'U hap.', asNotUnderstood: 'Nuk u kuptua.', asHeard: 'dëgjova', asYou: 'Ju', asHub: 'Huba',
    asStatus: 'sa porosi presin', asShowKitchen: 'trego kuzhinën', asShowStock: 'trego magazinën',
    asLow: 'Cilët përbërës po mbarojnë?', asAgentKey: 'Çelësi i agjentit tim (MCP)',
  },
  en: {
    asTitle: 'Assistant', asHint: 'Type what you need: a question, or an action like "received 4 kg salmon". Every change asks you before it happens.',
    asPlaceholder: 'Type here…', asSend: 'Send', asConfirm: 'Yes, do it', asCancel: 'No',
    asDone: 'Done.', asCancelled: 'Cancelled.', asFailed: 'Not done.', asThinking: 'Thinking…',
    asOpened: 'Opened.', asNotUnderstood: 'Not understood.', asHeard: 'heard', asYou: 'You', asHub: 'Hub',
    asStatus: 'status', asShowKitchen: 'show me the kitchen', asShowStock: 'show me the stock',
    asLow: 'Which ingredients are running low?', asAgentKey: 'My agent key (MCP)',
  },
  uk: {
    asTitle: 'Асистент', asHint: 'Напишіть, що потрібно: питання або дію, як-от «прийшло 4 кг лосось». Кожну зміну спершу підтверджуєте ви.',
    asPlaceholder: 'Пишіть тут…', asSend: 'Надіслати', asConfirm: 'Так, зробити', asCancel: 'Ні',
    asDone: 'Зроблено.', asCancelled: 'Скасовано.', asFailed: 'Не зроблено.', asThinking: 'Думаю…',
    asOpened: 'Відкрито.', asNotUnderstood: 'Не зрозуміло.', asHeard: 'почуто', asYou: 'Ви', asHub: 'Хаб',
    asStatus: 'скільки чекає', asShowKitchen: 'покажи кухню', asShowStock: 'покажи склад',
    asLow: 'Яких інгредієнтів мало?', asAgentKey: 'Ключ мого агента (MCP)',
  },
};

merge(T, WORDS);
