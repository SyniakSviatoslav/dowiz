export function maskName(name: string | null | undefined): string {
  if (!name) return '***';
  return name.charAt(0) + '***';
}

export function maskPhone(phone: string | null | undefined): string {
  if (!phone) return '***';
  const cleaned = phone.replace(/\D/g, '');
  if (cleaned.length < 4) return '+*** *** ****';
  return '+*** *** ' + cleaned.substring(cleaned.length - 4);
}
