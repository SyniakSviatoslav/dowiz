import React, { createContext, useContext, useEffect } from 'react';

export interface ThemeConfig {
  primary: string;
  primaryHover: string;
  primaryLight: string;
  accent: string;
  bg: string;
  surface: string;
  surfaceRaised: string;
  text: string;
  textMuted: string;
  border: string;
}

interface ThemeContextType {
  theme: ThemeConfig | null;
}

const ThemeContext = createContext<ThemeContextType>({ theme: null });

export function ThemeProvider({ 
  children, 
  theme 
}: { 
  children: React.ReactNode; 
  theme?: ThemeConfig;
}) {
  useEffect(() => {
    if (typeof document === 'undefined') return;
    const root = document.documentElement;
    const vars = [
      '--brand-primary',
      '--brand-primary-hover',
      '--brand-primary-light',
      '--brand-accent',
      '--brand-bg',
      '--brand-surface',
      '--brand-surface-raised',
      '--brand-text',
      '--brand-text-muted',
      '--brand-border',
    ];
    const prev: Record<string, string> = {};
    vars.forEach(v => { prev[v] = root.style.getPropertyValue(v); });

    if (theme) {
      root.style.setProperty('--brand-primary', theme.primary);
      root.style.setProperty('--brand-primary-hover', theme.primaryHover);
      root.style.setProperty('--brand-primary-light', theme.primaryLight);
      root.style.setProperty('--brand-accent', theme.accent);
      root.style.setProperty('--brand-bg', theme.bg);
      root.style.setProperty('--brand-surface', theme.surface);
      root.style.setProperty('--brand-surface-raised', theme.surfaceRaised);
      root.style.setProperty('--brand-text', theme.text);
      root.style.setProperty('--brand-text-muted', theme.textMuted);
      root.style.setProperty('--brand-border', theme.border);
    } else {
      vars.forEach(v => root.style.removeProperty(v));
    }

    return () => {
      Object.entries(prev).forEach(([k, v]) => root.style.setProperty(k, v));
    };
  }, [theme]);

  return (
    <ThemeContext.Provider value={{ theme: theme || null }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useBrandTheme() {
  return useContext(ThemeContext);
}
