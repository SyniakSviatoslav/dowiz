// UX-2 messenger deep-link builder. Turns a (kind, handle) pair into an app
// deep link. Telegram needs a username/link (a bare number doesn't deep-link),
// WhatsApp/Viber use the E.164 number. Returns null when there's nothing usable
// so callers can simply not render the button.
export type MessengerKind = 'telegram' | 'whatsapp' | 'viber';

export const MESSENGER_KINDS: MessengerKind[] = ['telegram', 'whatsapp', 'viber'];

export function messengerLabel(kind?: string | null): string {
  switch (kind) {
    case 'telegram': return 'Telegram';
    case 'whatsapp': return 'WhatsApp';
    case 'viber': return 'Viber';
    default: return '';
  }
}

export function messengerLink(kind?: string | null, handle?: string | null): string | null {
  if (!kind || !handle) return null;
  const h = String(handle).trim();
  if (!h) return null;
  switch (kind) {
    case 'telegram': {
      const u = h.replace(/^@/, '').replace(/^https?:\/\/(t\.me|telegram\.me)\//i, '').replace(/^\/+/, '');
      return u ? `https://t.me/${encodeURIComponent(u)}` : null;
    }
    case 'whatsapp': {
      const digits = h.replace(/[^\d]/g, '');
      return digits ? `https://wa.me/${digits}` : null;
    }
    case 'viber': {
      const digits = h.replace(/[^\d]/g, '');
      return digits ? `viber://chat?number=${encodeURIComponent('+' + digits)}` : null;
    }
    default:
      return null;
  }
}
