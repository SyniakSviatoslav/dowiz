export function formatALL(amountInCents: number): string {
  const all = Math.round(amountInCents / 100);
  return `${all} ALL`;
}

export function normalizePhone(phone: string): string {
  // Simple normalization: strip all non-numeric, prepend +355 if no country code
  let clean = phone.replace(/\D/g, '');
  if (clean.length === 9) { // Assuming 9 digit albanian numbers
    clean = `355${clean}`;
  }
  return `+${clean}`;
}

export function calcETA(createdAt: string | Date, elapsedSeconds: number): string {
  // Mock logic
  const created = new Date(createdAt);
  return '15-25 min';
}
