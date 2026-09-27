// The words of the kitchen's wider hub (lane W-KACCESS, 2026-09-27): the
// "I have a staff code" sign-in and the note on a screen staff only read.
// Merged into the console's table like the kitchen's own words; a language
// the console gains later falls back to English until its words
// are added here -- nothing below lists the languages.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { T } from '/admin/i18n.js';
import { merge } from '/admin/kitchen-i18n.js';

export const WORDS = {
  sq: {
    acc_changePw: 'Ndrysho fjalëkalimin', acc_setPw: 'Vendos një fjalëkalim të ri', acc_oldPw: 'Fjalëkalimi i tanishëm', acc_newPw: 'Fjalëkalimi i ri',
    acc_pwHint: 'Të paktën 8 shenja. Pajisjet e tjera ku keni hyrë dalin nga llogaria.',
    acc_setPwHint: 'Të paktën 8 shenja. Çdo pajisje ku ky person ka hyrë në këtë lokal del nga llogaria.',
    acc_short: 'Fjalëkalimi i ri duhet të ketë të paktën 8 shenja.', acc_pwChanged: 'Fjalëkalimi u ndryshua.',
    acc_claimOpen: 'Kam një kod stafi', acc_claimTitle: 'Hyni në lokalin tuaj',
    acc_claimHint: 'Shkruani kodin nga ftesa, email-in dhe një fjalëkalim. Nëse ky email ka tashmë llogari, përdorni fjalëkalimin e saj.',
    acc_code: 'Kodi i stafit', acc_claimGo: 'Hyr', acc_back: 'Kthehu te hyrja',
    acc_missing: 'Plotësoni të gjitha fushat.', acc_readOnly: 'Këtë e ndryshon vetëm pronari.',
  },
  en: {
    acc_changePw: 'Change password', acc_setPw: 'Set a new password', acc_oldPw: 'Current password', acc_newPw: 'New password',
    acc_pwHint: 'At least 8 characters. Your other signed-in devices are signed out.',
    acc_setPwHint: 'At least 8 characters. Every device this person is signed in on at this venue is signed out.',
    acc_short: 'The new password needs at least 8 characters.', acc_pwChanged: 'Password changed.',
    acc_claimOpen: 'I have a staff code', acc_claimTitle: 'Join your venue',
    acc_claimHint: 'Type the code from your invite, your email and a password. If this email already has an account, use its password.',
    acc_code: 'Staff code', acc_claimGo: 'Join and sign in', acc_back: 'Back to sign in',
    acc_missing: 'Fill in every field.', acc_readOnly: 'Only the owner changes this.',
  },
  uk: {
    acc_changePw: 'Змінити пароль', acc_setPw: 'Задати новий пароль', acc_oldPw: 'Поточний пароль', acc_newPw: 'Новий пароль',
    acc_pwHint: 'Щонайменше 8 символів. Інші пристрої, де ви увійшли, вийдуть з акаунта.',
    acc_setPwHint: 'Щонайменше 8 символів. Кожен пристрій, де ця людина увійшла в цьому закладі, вийде з акаунта.',
    acc_short: 'Новий пароль має містити щонайменше 8 символів.', acc_pwChanged: 'Пароль змінено.',
    acc_claimOpen: 'У мене є код працівника', acc_claimTitle: 'Приєднатися до закладу',
    acc_claimHint: 'Введіть код із запрошення, свою пошту і пароль. Якщо ця пошта вже має акаунт, введіть його пароль.',
    acc_code: 'Код працівника', acc_claimGo: 'Приєднатися й увійти', acc_back: 'Назад до входу',
    acc_missing: 'Заповніть усі поля.', acc_readOnly: 'Це змінює лише власник.',
  },
  ru: {
    acc_changePw: 'Сменить пароль', acc_setPw: 'Задать новый пароль', acc_oldPw: 'Текущий пароль', acc_newPw: 'Новый пароль',
    acc_pwHint: 'Не меньше 8 символов. На других устройствах, где вы вошли, будет выполнен выход.',
    acc_setPwHint: 'Не меньше 8 символов. На каждом устройстве, где этот человек вошёл в этом заведении, будет выполнен выход.',
    acc_short: 'Новый пароль должен содержать не меньше 8 символов.', acc_pwChanged: 'Пароль изменён.',
    acc_claimOpen: 'У меня есть код сотрудника', acc_claimTitle: 'Присоединиться к заведению',
    acc_claimHint: 'Введите код из приглашения, свою почту и пароль. Если у этой почты уже есть аккаунт, введите его пароль.',
    acc_code: 'Код сотрудника', acc_claimGo: 'Присоединиться и войти', acc_back: 'Назад ко входу',
    acc_missing: 'Заполните все поля.', acc_readOnly: 'Это меняет только владелец.',
  },
};

merge(T, WORDS);
