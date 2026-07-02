// Albania has no other realistic country here, so accept how people actually type
// their number — local "069...", "0 69 ...", "00355...", bare "69..." — and coerce
// to the E.164 (+355...) the backend requires, instead of silently rejecting it.
export function normalizeAlbanianPhone(raw: string): string {
  const compact = (raw || '').replace(/[\s()\-.]/g, '');
  if (!compact) return raw;
  if (compact.startsWith('+')) return compact;
  let digits = compact.replace(/\D/g, '');
  if (digits.startsWith('00')) return '+' + digits.slice(2);
  if (digits.startsWith('355')) return '+' + digits;
  if (digits.startsWith('0')) digits = digits.slice(1); // drop the national trunk 0
  return digits ? '+355' + digits : raw;
}
