export function isEmbedMode(): boolean {
  if (typeof window === 'undefined') return false;
  const params = new URLSearchParams(window.location.search);
  return params.get('embed') === 'true';
}

export function posFixed(style: string): string {
  if (isEmbedMode()) {
    return style
      .replace(/position:\s*fixed/gi, 'position: sticky')
      .replace(/bottom:\s*\d+px/gi, '');
  }
  return style;
}
