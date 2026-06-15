import type { NotificationProvider, NotificationTarget, NotificationEvent, NotificationData, NotifyResult } from '../provider.js';
import { renderWhatsAppMessage } from '../render.js';

/**
 * WhatsApp notification adapter built on @whiskeysockets/baileys.
 *
 * Baileys is a pure WebSocket implementation of the WhatsApp Web multi-device
 * protocol — no Chromium/Puppeteer, so it runs headless inside the API Docker
 * container. It maintains ONE long-lived socket per process, authenticated once
 * via a QR code (printed to logs on first run) and persisted to disk via
 * `useMultiFileAuthState`.
 *
 * `target.address` is the recipient phone number in international format without
 * the leading "+" (e.g. "355691234567"); we normalize to the Baileys JID
 * "<number>@s.whatsapp.net".
 *
 * Env:
 *   WHATSAPP_ENABLED      - "true" to activate the channel (default off)
 *   ***REDACTED***     - dir for persisted auth/session state
 *                           (default: ./.whatsapp-auth)
 */
export class WhatsAppAdapter implements NotificationProvider {
  readonly id = 'whatsapp';

  private authDir: string;
  private sock: any = null;
  private connecting: Promise<any> | null = null;
  private ready = false;

  constructor(authDir = process.env.***REDACTED*** || './.whatsapp-auth') {
    this.authDir = authDir;
  }

  /** Lazily establish (and cache) the Baileys socket. */
  private async getSocket(): Promise<any> {
    if (this.sock && this.ready) return this.sock;
    if (this.connecting) return this.connecting;

    this.connecting = (async () => {
      // Dynamic import: baileys is ESM-heavy and only needed when the channel runs.
      const baileys = await import('@whiskeysockets/baileys');
      const makeWASocket = (baileys as any).default ?? (baileys as any).makeWASocket;
      const { useMultiFileAuthState, DisconnectReason, fetchLatestBaileysVersion } = baileys as any;

      const { state, saveCreds } = await useMultiFileAuthState(this.authDir);
      const { version } = await fetchLatestBaileysVersion();

      const sock = makeWASocket({
        version,
        auth: state,
        printQRInTerminal: true, // first-run pairing: scan QR from API logs
        markOnlineOnConnect: false,
        syncFullHistory: false,
      });

      sock.ev.on('creds.update', saveCreds);

      sock.ev.on('connection.update', (update: any) => {
        const { connection, lastDisconnect, qr } = update;
        if (qr) {
          console.warn('[whatsapp] Pairing required — scan the QR code above with WhatsApp on the owner phone.');
        }
        if (connection === 'open') {
          this.ready = true;
          console.log('[whatsapp] connection open');
        } else if (connection === 'close') {
          this.ready = false;
          const statusCode = lastDisconnect?.error?.output?.statusCode;
          const loggedOut = statusCode === DisconnectReason?.loggedOut;
          console.warn(`[whatsapp] connection closed (status=${statusCode}, loggedOut=${loggedOut})`);
          // Drop the cached socket so the next notify() reconnects. If logged out,
          // the auth dir must be cleared and the QR re-scanned by an operator.
          this.sock = null;
          this.connecting = null;
        }
      });

      this.sock = sock;
      return sock;
    })();

    try {
      return await this.connecting;
    } finally {
      // Keep this.connecting set only while the open handshake is pending; the
      // connection.update handler resolves readiness asynchronously.
      this.connecting = null;
    }
  }

  private toJid(address: string): string {
    if (address.includes('@')) return address; // already a JID or group
    const digits = address.replace(/[^0-9]/g, '');
    return `${digits}@s.whatsapp.net`;
  }

  async notify(target: NotificationTarget, event: NotificationEvent, data: NotificationData): Promise<NotifyResult> {
    if (process.env.WHATSAPP_ENABLED !== 'true') {
      return { delivered: false, reason: 'WHATSAPP_NOT_CONFIGURED' };
    }

    const locale = ((target as any).locale || data.locale || 'sq') as any;
    const text = renderWhatsAppMessage(event, data, locale);
    const jid = this.toJid(target.address);

    let sock: any;
    try {
      sock = await this.getSocket();
    } catch (err: any) {
      return { delivered: false, reason: `CONNECT_FAILED:${err?.message || 'unknown'}` };
    }

    if (!this.ready) {
      // Socket exists but handshake not complete yet — let the worker retry.
      return { delivered: false, reason: 'NOT_READY', retryAfter: 3000 };
    }

    try {
      const res = await sock.sendMessage(jid, { text });
      return { delivered: true, providerMessageId: res?.key?.id };
    } catch (err: any) {
      const msg = err?.message || 'SEND_FAILED';
      // A 401/loggedOut style error means the session is dead → disable target.
      if (/logged.?out|401|unauthorized/i.test(msg)) {
        return { delivered: false, reason: `AUTH_OR_BLOCKED:${msg}` };
      }
      return { delivered: false, reason: msg };
    }
  }
}
