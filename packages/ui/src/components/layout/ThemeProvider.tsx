import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { applyBrandTheme, getPresetConfig, PRESETS, type BrandConfig, type BrandPreset } from '../../theme/index.js';

const ThemeContext = createContext<{
  preset: BrandPreset;
  config: BrandConfig;
  setPreset: (name: BrandPreset) => void;
  cyclePreset: () => void;
} | null>(null);

const PRESET_NAMES = Object.keys(PRESETS) as BrandPreset[];

export function ThemeProvider({
  children,
  initialPreset = 'food-dark',
  ssrConfig,
}: {
  children: ReactNode;
  initialPreset?: BrandPreset;
  ssrConfig?: BrandConfig;
}) {
  const [preset, setPresetState] = useState<BrandPreset>(initialPreset);

  const config = ssrConfig ?? getPresetConfig(preset) ?? PRESETS['food-dark'];

  useEffect(() => {
    if (!ssrConfig) {
      applyBrandTheme(config);
    }
  }, [config, ssrConfig]);

  function setPreset(name: BrandPreset) {
    const cfg = getPresetConfig(name);
    if (cfg) {
      applyBrandTheme(cfg);
      setPresetState(name);
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
