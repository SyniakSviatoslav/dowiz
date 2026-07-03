import React, { createContext, useContext, useEffect } from 'react';
import { LazyMotion, domAnimation } from 'framer-motion';

export interface ThemeConfig {
  primary: string;
  primaryHover: string;
  /**
   * Brand primary nudged to AA on the surface — for primary-coloured TEXT.
   * Optional: derivePalette always supplies it, but hand-built ThemeConfig
   * literals may omit it (consumers fall back via the CSS var default).
   */
  primaryReadable?: string;
  /**
   * Brand primary as a CTA FILL — darkened/lightened until `onPrimary` text clears AA on it,
   * with `onPrimary` the matching text colour. Fixes the illegible-CTA bug where a pale brand
   * primary shipped white button text on a pale fill (sub-AA). Optional for hand-built literals
   * (CSS-var defaults in tokens.css stand when omitted).
   */
  primaryStrong?: string;
  onPrimary?: string;
  primaryLight: string;
  accent: string;
  bg: string;
  surface: string;
  surfaceRaised: string;
  text: string;
  textMuted: string;
  border: string;
  /**
   * Resolved CSS font stacks (NOT allowlist ids) applied to --brand-font-heading / --brand-font-body.
   * Optional: derivePalette always supplies them (cuisine default at minimum); hand-built literals may
   * omit them and the tokens.css defaults stand.
   */
  fontHeading?: string;
  fontBody?: string;
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
      '--brand-primary-strong',
      '--color-on-primary',
      '--brand-primary-light',
      '--brand-accent',
      '--brand-bg',
      '--brand-surface',
      '--brand-surface-raised',
      '--brand-text',
      '--brand-text-muted',
      '--brand-border',
      '--brand-font-heading',
      '--brand-font-body',
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
      applyVar('--brand-primary-strong', theme.primaryStrong);
      applyVar('--color-on-primary', theme.onPrimary);
      applyVar('--brand-primary-light', theme.primaryLight);
      applyVar('--brand-accent', theme.accent);
      applyVar('--brand-bg', theme.bg);
      applyVar('--brand-surface', theme.surface);
      applyVar('--brand-surface-raised', theme.surfaceRaised);
      applyVar('--brand-text', theme.text);
      applyVar('--brand-text-muted', theme.textMuted);
      applyVar('--brand-border', theme.border);
      applyVar('--brand-font-heading', theme.fontHeading);
      applyVar('--brand-font-body', theme.fontBody);
    } else {
      vars.forEach(v => root.style.removeProperty(v));
    }

    return () => {
      Object.entries(prev).forEach(([k, v]) => root.style.setProperty(k, v));
    };
  }, [theme]);

  return (
    <ThemeContext.Provider value={{ theme: theme || null }}>
      {/* Phase 2.1 (LazyMotion + `m`): ONE shared feature provider for every `m.*` component
          in packages/ui (~46KB motion → ~4.6KB `m` + lazy domAnimation). ThemeProvider is the
          root seam both apps already mount (apps/web main.tsx wraps the whole router), so no
          app-side integration is needed. NO `strict` flag: apps/web still renders full
          `motion.*` components (their own migration is a separate lane) and strict would
          throw on those. Nested ThemeProviders (ClientLayout) nest LazyMotion with the same
          static `domAnimation` feature object — supported and idempotent. */}
      <LazyMotion features={domAnimation}>
        {children}
      </LazyMotion>
    </ThemeContext.Provider>
  );
}

export function useBrandTheme() {
  return useContext(ThemeContext);
}
