import { safeStorage } from './safeStorage.js';

// Sunlight Mode — a high-contrast outdoor theme. Research (NN/g, WebAIM, industrial UX):
// in direct sun, light beats dark, push text to AAA (~7:1), flatten blur/gradients/shadows.
// State lives in localStorage; the actual styling is driven by html[data-sunlight="on"]
// (see tokens.css). Defaults ON when the user's OS requests more contrast.
const KEY = 'dowiz-sunlight';

export function isSunlightOn(): boolean {
  const v = safeStorage.get(KEY);
  if (v === 'on') return true;
  if (v === 'off') return false;
  try {
    return typeof window !== 'undefined' && window.matchMedia('(prefers-contrast: more)').matches;
  } catch {
    return false;
  }
}

export function applySunlight(on: boolean): void {
  if (typeof document === 'undefined') return;
  if (on) document.documentElement.setAttribute('data-sunlight', 'on');
  else document.documentElement.removeAttribute('data-sunlight');
}

export function setSunlight(on: boolean): void {
  safeStorage.set(KEY, on ? 'on' : 'off');
  applySunlight(on);
}
