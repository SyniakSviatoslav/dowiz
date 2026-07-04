// Telegram Mini App (TMA) — pure config builders for the bot-side "menu button" that opens
// the existing /s/:slug storefront as a Mini App inside Telegram (2026-07-04
// customer-distribution-channels research §3). No network calls live here — the caller
// (telegram-webhook.ts) performs the actual Bot API request via its existing callTelegramApi
// helper, so this module stays trivially unit-testable (node --test, no fetch mocking).
//
// Scope: bot-side menu-button wiring ONLY, gated by TMA_ENABLED (packages/config EnvSchema).
// Checkout is unchanged — the Mini App WebView just loads the normal /s/:slug web checkout.
// No Telegram Payments, no new money surface, no initData auth (that's the Phase-2 item;
// see docs/design/channel-hub/TMA-VALIDATION.md).
//
// Council-gated (docs/design/tma-menu-button-wiring/, resolution.md APPROVED 2026-07-04):
// - chatId is REQUIRED (throws if empty) — Telegram's setChatMenuButton treats chat_id as
//   optional and, if omitted, sets the bot-GLOBAL default button for every user of the
//   single shared bot token. Never build a request without a chatId (B-SEC finding).
// - Recovery note: the caller's connect token is single-use, so a transient API failure is
//   NOT retried by "the next /start" with the same link — only a fresh reconnect (new
//   token) reaches this code again. Best-effort/non-blocking either way (breaker finding).

export interface MiniAppButtonOpts {
  /** e.g. the validated packages/config APP_BASE_URL — trailing slash is stripped. */
  appBaseUrl: string;
  /** The vendor's location slug (locations.slug), used verbatim in /s/:slug. */
  slug: string;
  /** Menu-button label; Telegram truncates long labels client-side. Default is first-person
   *  and honest about what it opens (Counsel: label must describe true purpose). */
  text?: string;
}

const DEFAULT_BUTTON_TEXT = 'My Storefront';

/**
 * The storefront URL the Mini App button opens. `ch=telegram-tma` stamps channel
 * attribution (matches the ?ch= convention noted in the distribution-channels research) —
 * it does NOT change checkout behavior, which stays the single normal web checkout. The
 * param is inert today: this build wires no analytics pipeline to read it.
 */
export function buildMiniAppUrl(opts: MiniAppButtonOpts): string {
  if (!opts.slug) throw new Error('buildMiniAppUrl: slug is required');
  const base = opts.appBaseUrl.replace(/\/+$/, '');
  const slug = encodeURIComponent(opts.slug);
  return `${base}/s/${slug}?ch=telegram-tma`;
}

/**
 * Telegram Bot API `setChatMenuButton` request body for a single chat's web_app button
 * (https://core.telegram.org/bots/api#setchatmenubutton). Pure builder — no I/O. Requires
 * chatId (see file header — omitting it would set a bot-global default button).
 */
export function buildSetChatMenuButtonRequest(
  chatId: string,
  opts: MiniAppButtonOpts,
): { chat_id: string; menu_button: { type: 'web_app'; text: string; web_app: { url: string } } } {
  if (!chatId) throw new Error('buildSetChatMenuButtonRequest: chatId is required (never build a global-button request)');
  return {
    chat_id: chatId,
    menu_button: {
      type: 'web_app',
      text: opts.text || DEFAULT_BUTTON_TEXT,
      web_app: { url: buildMiniAppUrl(opts) },
    },
  };
}
