// @ts-nocheck
import crypto from 'node:crypto';

export const ALLOWED_FONTS = ['Inter', 'Roboto', 'Source Sans 3', 'Lato', 'Open Sans', 'system-ui'] as const;
export type FontFamily = typeof ALLOWED_FONTS[number];

export interface ThemeInput {
  primary_color: string | null;
  secondary_color: string | null;
  font_family: FontFamily | null;
  bg_color?: string | null;
  text_color?: string | null;
  logo_url?: string | null;
}

export interface ThemeRendered {
  css: string;
  cssHash: string;
  version: number;
  warnings: string[];
}

function getLuminance(hex: string): number {
  const rgb = parseInt(hex.slice(1), 16);
  const r = (rgb >> 16) & 0xff;
  const g = (rgb >>  8) & 0xff;
  const b = (rgb >>  0) & 0xff;
  
  const a = [r, g, b].map(function (v) {
    v /= 255;
    return v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
  });
  return a[0] * 0.2126 + a[1] * 0.7152 + a[2] * 0.0722;
}

function getContrastRatio(hex1: string, hex2: string): number {
  const lum1 = getLuminance(hex1);
  const lum2 = getLuminance(hex2);
  const brightest = Math.max(lum1, lum2);
  const darkest = Math.min(lum1, lum2);
  return (brightest + 0.05) / (darkest + 0.05);
}

// Lighten/darken for variants
function adjustColor(hex: string, amount: number): string {
  const col = parseInt(hex.slice(1), 16);
  let r = (col >> 16) + amount;
  let g = (col >> 8 & 0x00FF) + amount;
  let b = (col & 0x0000FF) + amount;
  r = Math.max(Math.min(255, r), 0);
  g = Math.max(Math.min(255, g), 0);
  b = Math.max(Math.min(255, b), 0);
  return `#${(g | (b << 8) | (r << 16)).toString(16).padStart(6, '0')}`;
}

export function renderTheme(input: ThemeInput, currentVersion: number = 0): ThemeRendered {
  const warnings: string[] = [];

  const primary = input.primary_color || '#e63946';
  const secondary = input.secondary_color || '#457b9d';
  const bg = input.bg_color || '#ffffff';
  
  // Calculate best text color for contrast if not provided
  const bgLum = getLuminance(bg);
  const isDarkBg = bgLum < 0.5;
  const text = input.text_color || (isDarkBg ? '#ffffff' : '#212529');

  const primaryContrast = getContrastRatio(primary, bg);
  if (primaryContrast < 4.5) {
    warnings.push('LOW_CONTRAST_PRIMARY');
  }

  const font = input.font_family || 'system-ui';
  let fontFace = '';
  
  // Basic Google Fonts implementation with subsetting for ë/ç (latin-ext)
  if (font !== 'system-ui') {
    const fontSafe = font.replace(/ /g, '+');
    // Using a reliable CDN for testing; in prod might want self-hosted WOFF2
    fontFace = `
      @import url('https://fonts.googleapis.com/css2?family=${fontSafe}:wght@400;700&display=swap&subset=latin-ext');
    `;
  }

  const css = `
    ${fontFace}
    :root {
      --brand-primary: ${primary};
      --brand-primary-hover: ${adjustColor(primary, -20)};
      --brand-secondary: ${secondary};
      --brand-bg: ${bg};
      --brand-text: ${text};
      --brand-font: '${font}', system-ui, sans-serif;
      --brand-radius: 8px;
    }
    
    body {
      background-color: var(--brand-bg);
      color: var(--brand-text);
      font-family: var(--brand-font);
    }
    
    .btn-primary {
      background-color: var(--brand-primary);
      color: ${getLuminance(primary) > 0.5 ? '#000' : '#fff'};
      border: none;
      border-radius: var(--brand-radius);
      padding: 0.5rem 1rem;
      cursor: pointer;
    }
    .btn-primary:hover {
      background-color: var(--brand-primary-hover);
    }
    
    .text-primary {
      color: var(--brand-primary);
    }
    
    ${input.logo_url ? `
    .brand-logo {
      background-image: url('${input.logo_url}');
    }
    ` : ''}
  `.replace(/\s+/g, ' ').trim(); // Simple minification

  const cssHash = crypto.createHash('sha256').update(css).digest('hex').slice(0, 16);

  return {
    css,
    cssHash,
    version: currentVersion + 1,
    warnings
  };
}
