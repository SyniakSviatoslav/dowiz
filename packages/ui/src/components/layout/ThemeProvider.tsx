import { safeStorage } from '../../utils/safeStorage.js';
import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { applyBrandTheme, getPresetConfig, PRESETS, type BrandConfig, type BrandPreset } from '../../theme/index.js';

const ThemeContext = createContext<{
  preset: BrandPreset;
  config: BrandConfig;
  setPreset: (name: BrandPreset) => void;
  cyclePreset: () => void;
} | null>(null);

const PRESET_NAMES = Object.keys(PRESETS) as BrandPreset[];
const STORAGE_KEY = 'dowiz-preset';

function getInitialPreset(fallback: BrandPreset): BrandPreset {
  try {
    const saved = typeof window !== 'undefined' ? safeStorage.get(STORAGE_KEY) : null;
    if (saved && (saved in PRESETS)) return saved as BrandPreset;
  } catch {}
  if (typeof window !== 'undefined') {
    const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const detected = prefersDark ? 'food-dark' : 'crimson-classic';
    try { safeStorage.set(STORAGE_KEY, detected); } catch {}
    return detected;
  }
  return fallback;
}

export function ThemeProvider({
  children,
  initialPreset = 'food-dark',
  ssrConfig,
}: {
  children: ReactNode;
  initialPreset?: BrandPreset;
  ssrConfig?: BrandConfig;
}) {
  const [preset, setPresetState] = useState<BrandPreset>(() => getInitialPreset(initialPreset));

  const config = ssrConfig ?? getPresetConfig(preset) ?? PRESETS['food-dark'];

  useEffect(() => {
    if (!ssrConfig) {
      applyBrandTheme(config);
    }
  }, [config, ssrConfig]);

  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');

    const handler = (e: MediaQueryListEvent) => {
      const target = e.matches ? 'food-dark' : 'crimson-classic';
      const cfg = getPresetConfig(target);
      if (cfg) {
        applyBrandTheme(cfg);
        setPresetState(target);
        try { safeStorage.set(STORAGE_KEY, target); } catch {}
      }
    };

    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  function setPreset(name: BrandPreset) {
    const cfg = getPresetConfig(name);
    if (cfg) {
      applyBrandTheme(cfg);
      setPresetState(name);
      try { safeStorage.set(STORAGE_KEY, name); } catch {}
    }
  }

  function cyclePreset() {
    const current = PRESET_NAMES.indexOf(preset);
    const next = PRESET_NAMES[(current + 1) % PRESET_NAMES.length]!;
    setPreset(next);
  }

  return (
    <ThemeContext.Provider value={{ preset, config, setPreset, cyclePreset }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useBrandTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useBrandTheme must be used within ThemeProvider');
  return ctx;
}
