// Magic-byte signatures for allowed image types
const IMAGE_SIGNATURES: Array<{ bytes: number[]; offset?: number; label: string }> = [
  { bytes: [0xff, 0xd8, 0xff],                label: 'JPEG' },
  { bytes: [0x89, 0x50, 0x4e, 0x47],          label: 'PNG'  },
  { bytes: [0x47, 0x49, 0x46, 0x38],          label: 'GIF'  },
  // WebP: RIFF at 0 + WEBP at offset 8
  { bytes: [0x52, 0x49, 0x46, 0x46],          label: 'RIFF_PREFIX' },
];
const WEBP_SUBTYPE = Buffer.from('WEBP');

export function assertImageMagicBytes(buf: Buffer): void {
  if (buf.length < 12) throw new Error('File too small to be a valid image');

  for (const sig of IMAGE_SIGNATURES) {
    const offset = sig.offset ?? 0;
    const match = sig.bytes.every((b, i) => buf[offset + i] === b);
    if (match) {
      // For RIFF files, verify WEBP sub-type at bytes 8-11
      if (sig.label === 'RIFF_PREFIX') {
        if (buf.slice(8, 12).equals(WEBP_SUBTYPE)) return; // WebP — OK
        throw new Error('RIFF file is not WebP');
      }
      return; // JPEG / PNG / GIF — OK
    }
  }

  throw new Error('Unsupported file type — only JPEG, PNG, WebP, and GIF are allowed');
}

export function assertTextFileMagicBytes(buf: Buffer): void {
  if (buf.length === 0) throw new Error('Empty file');
  // Reject obvious binary: check first 512 bytes for null bytes
  const probe = buf.slice(0, Math.min(512, buf.length));
  for (const byte of probe) {
    if (byte === 0x00) throw new Error('Binary content detected — only CSV or JSON files are allowed');
  }
}
