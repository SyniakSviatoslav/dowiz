/**
 * Lowercase + strip combining diacritics + collapse whitespace. Shared by the intent matcher and
 * the dietary denylist so "qumësht" ~ "qumesht", "Pa Gluten" ~ "pa gluten", and "  SHTO  dy " ~
 * "shto dy". Diacritic range is U+0300–U+036F (combining marks) via explicit escapes for portability.
 */
export function normalize(s: string): string {
  return s
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/\s+/g, ' ')
    .trim();
}
