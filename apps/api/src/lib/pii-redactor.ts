export interface PiiRedaction {
  kind: 'email' | 'phone' | 'card' | 'iban' | 'url' | 'name';
  start: number;
  end: number;
  replacement: string;
}

export class PiiRedactor {
  // Simple regexes for PII detection
  private static readonly PATTERNS = [
    {
      // P6-3 (C1): ANCHORED person-name — a role/attribution trigger followed by a TitleCase name.
      // Anchored (not bare TitleCase) so menu item names ("Caesar Salad") are NOT redacted; catches
      // "Chef Maria Hoxha", "Owner: Jeton Berisha", "by Arben K." inline. The menu-region allowlist
      // (menu-region.ts) is the primary control; this is the converged-redactor secondary net.
      kind: 'name',
      // Case-insensitive TRIGGER (char-classes, not the /i flag — /i would let [A-Z] match lowercase
      // and break the TitleCase anchor), case-sensitive TitleCase NAME so menu items aren't redacted.
      regex: /\b(?:[Cc]hef|[Oo]wner|[Mm]anager|[Ff]ounder|[Dd]irector|[Pp]roprietor|[Hh]ost|[Hh]ostess|[Bb]y|[Mm]eet|[Ss]erved by|[Pp]repared by)\b[:.,]?\s+(?:[Oo]ur\s+)?[A-Z][a-z]+(?:\s+[A-Z][a-z.]+){0,2}/g
    },
    {
      kind: 'email',
      regex: /[\w.+-]+@[\w-]+\.[\w.-]+/gi
    },
    {
      kind: 'url',
      regex: /https?:\/\/[^\s]+\?[^\s]+/gi
    },
    {
      kind: 'iban',
      regex: /[A-Z]{2}\d{2}[A-Z0-9]{10,30}/gi
    },
    {
      kind: 'card',
      // 13-19 digits, possibly separated by spaces/dashes
      regex: /(?:\d[ -]*?){13,19}/g
    },
    {
      kind: 'phone',
      // Intl format: optional +, digit, then min 5 digits/spaces/dashes, digit.
      regex: /(?:\+|00)?(?:[0-9]{1,3})?[-\s()]*[0-9][-\s()0-9]{6,}[0-9]/g
    }
  ] as const;

  public redact(input: string): { text: string; redactions: PiiRedaction[] } {
    let currentText = input;
    const redactions: PiiRedaction[] = [];

    // Apply sequentially and replace with [REDACTED]
    for (const pattern of PiiRedactor.PATTERNS) {
      // Need a way to match without overlap, and while accounting for replaced offsets.
      // Easiest is just replace and record what we found.
      let match;
      // We must reset regex state if global
      const regex = new RegExp(pattern.regex.source, pattern.regex.flags);
      
      let textCopy = currentText;
      let offsetDiff = 0;

      while ((match = regex.exec(currentText)) !== null) {
        const str = match[0];

        // False positive filter for phone
        if (pattern.kind === 'phone' || pattern.kind === 'card') {
          const digitsOnly = str.replace(/\D/g, '');
          if (digitsOnly.length < 8) {
            continue; // Skip, too short to be a phone or card
          }
        }

        // Whitelist check
        if (this.isWhitelisted(str)) {
          continue;
        }

        const start = match.index;
        const end = start + str.length;
        const replacement = '[REDACTED]';

        redactions.push({
          kind: pattern.kind as any,
          start: start + offsetDiff,
          end: end + offsetDiff,
          replacement
        });

        // Mutate string for next iteration
        textCopy = textCopy.substring(0, start + offsetDiff) + replacement + textCopy.substring(start + offsetDiff + str.length);
        offsetDiff += replacement.length - str.length;
      }
      currentText = textCopy;
    }

    return { text: currentText, redactions };
  }

  private isWhitelisted(str: string): boolean {
    // "Rruga 12" style check, though regex for phone shouldn't match it if digits < 8
    return false;
  }
}
