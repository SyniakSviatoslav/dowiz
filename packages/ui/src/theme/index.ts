export const PRESETS = {
  'food-dark': {
    primary: '#ea4f16', primaryHover: '#ffa12e', primaryLight: 'rgba(234,79,22,0.12)',
    accent: '#2a2a2a', bg: '#121212', surface: '#1e1e1e', surfaceRaised: '#2a2a2a',
    text: '#ffffff', textMuted: '#a8a8a8', border: '#2c2c2c',
    fontHeading: "'Inter', sans-serif", fontBody: "'Inter', sans-serif",
    radius: '12px', radiusSm: '8px', radiusBtn: '78px',
  },
  'crimson-classic': {
    primary: '#C1121F', primaryHover: '#9B0D17', primaryLight: '#FFF0F1',
    accent: '#F5F0E8', bg: '#FFFFFF', surface: '#F8F9FA', surfaceRaised: '#FFFFFF',
    text: '#1A1A1A', textMuted: '#4B5563', border: '#E5E7EB',
    fontHeading: "'DM Serif Display', serif", fontBody: "'DM Sans', sans-serif",
    radius: '12px', radiusSm: '6px', radiusBtn: '24px',
  },
  'ocean-fresh': {
    primary: '#0D9488', primaryHover: '#0F766E', primaryLight: '#F0FDFA',
    accent: '#CCFBF1', bg: '#FFFFFF', surface: '#F8FFFE', surfaceRaised: '#FFFFFF',
    text: '#134E4A', textMuted: '#4B5563', border: '#CCFBF1',
    fontHeading: "'Cormorant Garamond', serif", fontBody: "'DM Sans', sans-serif",
    radius: '16px', radiusSm: '8px', radiusBtn: '28px',
  },
  'midnight-urban': {
    primary: '#F97316', primaryHover: '#EA580C', primaryLight: 'rgba(249,115,22,0.12)',
    accent: '#1C1917', bg: '#0C0C0C', surface: '#1A1A1A', surfaceRaised: '#262626',
    text: '#FAFAFA', textMuted: '#A3A3A3', border: '#262626',
    fontHeading: "'DM Sans', sans-serif", fontBody: "'DM Sans', sans-serif",
    radius: '8px', radiusSm: '4px', radiusBtn: '8px',
  },
  'sage-garden': {
    primary: '#4D7C0F', primaryHover: '#3F6212', primaryLight: '#F7FEE7',
    accent: '#ECFCCB', bg: '#FAFAF5', surface: '#F5F5F0', surfaceRaised: '#FFFFFF',
    text: '#1A2E05', textMuted: '#4B5563', border: '#D9F99D',
    fontHeading: "'Playfair Display', serif", fontBody: "'Inter', sans-serif",
    radius: '12px', radiusSm: '6px', radiusBtn: '24px',
  },
  'royal-gold': {
    primary: '#B45309', primaryHover: '#92400E', primaryLight: 'rgba(180,83,9,0.12)',
    accent: '#1C1917', bg: '#0A0A0A', surface: '#1A1A1A', surfaceRaised: '#262626',
    text: '#FEF3C7', textMuted: '#A8A29E', border: '#292524',
    fontHeading: "'Cormorant Garamond', serif", fontBody: "'Inter', sans-serif",
    radius: '12px', radiusSm: '6px', radiusBtn: '78px',
  },
  'coral-breeze': {
    primary: '#DB2777', primaryHover: '#BE185D', primaryLight: '#FDF2F8',
    accent: '#FCE7F3', bg: '#FFFBFB', surface: '#FFF5F5', surfaceRaised: '#FFFFFF',
    text: '#1A1A2E', textMuted: '#4B5563', border: '#FBCFE8',
    fontHeading: "'Playfair Display', serif", fontBody: "'DM Sans', sans-serif",
    radius: '16px', radiusSm: '8px', radiusBtn: '32px',
  },
};

export type BrandPreset = keyof typeof PRESETS;
export type BrandConfig = typeof PRESETS['food-dark'];

export function applyBrandTheme(config: BrandConfig) {
  const root = document.documentElement;
  Object.entries(config).forEach(([key, value]) => {
    const cssKey = key.replace(/([A-Z])/g, '-$1').toLowerCase();
    root.style.setProperty(`--brand-${cssKey}`, value);
  });
}

export function getPresetConfig(presetName: string): BrandConfig | null {
  const preset = PRESETS[presetName as BrandPreset];
  if (!preset) return null;
  return { ...preset };
}

export function injectThemeSSR(headHtml: string, config: BrandConfig): string {
  const cssVars = Object.entries(config)
    .map(([k, v]) => {
      const cssKey = k.replace(/([A-Z])/g, '-$1').toLowerCase();
      return `  --brand-${cssKey}: ${v};`;
    })
    .join('\n');
  const styleTag = `<style id="brand-theme">:root {\n${cssVars}\n}</style>`;
  return headHtml.replace('</head>', `${styleTag}\n</head>`);
}
