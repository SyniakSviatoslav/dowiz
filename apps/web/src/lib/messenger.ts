// "Communication" channel model (ADR-0016). A (kind, handle) pair → a courier deep link.
// v1 kinds: Phone (call/SMS, first-class — keeps a real phone for throttle/OTP/dedup), WhatsApp, Viber,
// Telegram (username), Signal (phone), SimpleX (TEXT-ONLY — never a clickable link; courier copies it).
// Google Meet / MS Teams were CUT (category error for food delivery — Counsel). messengerLink returns null
// when there's nothing safely linkable (incl. SimpleX) so callers simply don't render an "open" button.
export type MessengerKind = 'phone' | 'whatsapp' | 'viber' | 'telegram' | 'signal' | 'simplex';

export const MESSENGER_KINDS: MessengerKind[] = ['phone', 'whatsapp', 'viber', 'telegram', 'signal', 'simplex'];

// Kinds whose handle IS a phone number → feed the order's `phone` (Fastify per-phone throttle + OTP + dedup).
const PHONE_KINDS = new Set<string>(['phone', 'whatsapp', 'viber', 'signal']);
export function messengerIsPhone(kind?: string | null): boolean {
  return !!kind && PHONE_KINDS.has(kind);
}

/** Input affordance per kind: a phone field, a @username field, or free text (SimpleX link). */
export function messengerInputType(kind?: string | null): 'phone' | 'username' | 'text' {
  if (messengerIsPhone(kind)) return 'phone';
  if (kind === 'telegram') return 'username';
  return 'text';
}

export function messengerLabel(kind?: string | null): string {
  switch (kind) {
    case 'phone': return 'Phone';
    case 'whatsapp': return 'WhatsApp';
    case 'viber': return 'Viber';
    case 'telegram': return 'Telegram';
    case 'signal': return 'Signal';
    case 'simplex': return 'SimpleX';
    default: return '';
  }
}

/** Tabler/simple-icons class or vendored svg id per kind (rendered by the selector). */
export function messengerIcon(kind?: string | null): string {
  switch (kind) {
    case 'phone': return 'ti ti-phone';
    case 'whatsapp': return 'ti ti-brand-whatsapp';
    case 'viber': return 'ti ti-brand-viber';
    case 'telegram': return 'ti ti-brand-telegram';
    case 'signal': return 'ti ti-brand-signal';
    case 'simplex': return 'ti ti-lock'; // SimpleX has no Tabler glyph; a lock conveys private messaging
    default: return 'ti ti-message';
  }
}

export function messengerLink(kind?: string | null, handle?: string | null): string | null {
  if (!kind || !handle) return null;
  const h = String(handle).trim();
  if (!h) return null;
  switch (kind) {
    case 'phone': {
      const digits = h.replace(/[^\d+]/g, '');
      return digits ? `tel:${digits}` : null;
    }
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
    case 'signal': {
      const digits = h.replace(/[^\d]/g, '');
      return digits ? `https://signal.me/#p/+${digits}` : null;
    }
    case 'simplex':
      // TEXT-ONLY by decision — never produce a clickable link (self-hosted invite hosts can't be
      // allowlisted; courier copies the text). Callers render the handle as copyable text.
      return null;
    default:
      return null;
  }
}
