import React, { createContext, useContext, useEffect } from 'react';

export interface ThemeConfig {
  primary: string;
  primaryHover: string;
  /**
   * Brand primary nudged to AA on the surface — for primary-coloured TEXT.
   * Optional: derivePalette always supplies it, but hand-built ThemeConfig
   * literals may omit it (consumers fall back via the CSS var default).
   */
  primaryReadable?: string;
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
      '--brand-primary-readable',
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

    // Skip empty or self-referential values (e.g. 'var(--brand-bg)' when a tenant
    // hasn't set that color) — setting --brand-bg: var(--brand-bg) is an invalid
    // self-reference that wipes the stylesheet default (white flash). removeProperty
    // lets the tokens.css default stand instead.
    const applyVar = (prop: string, value: string | undefined) => {
      if (value && !value.startsWith('var(')) root.style.setProperty(prop, value);
      else root.style.removeProperty(prop);
    };
    if (theme) {
      applyVar('--brand-primary', theme.primary);
      applyVar('--brand-primary-hover', theme.primaryHover);
      applyVar('--brand-primary-readable', theme.primaryReadable);
      applyVar('--brand-primary-light', theme.primaryLight);
      applyVar('--brand-accent', theme.accent);
      applyVar('--brand-bg', theme.bg);
      applyVar('--brand-surface', theme.surface);
      applyVar('--brand-surface-raised', theme.surfaceRaised);
      applyVar('--brand-text', theme.text);
      applyVar('--brand-text-muted', theme.textMuted);
      applyVar('--brand-border', theme.border);
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
