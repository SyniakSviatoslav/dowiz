---
name: deliveryos-theme
description: >
  Use this skill when working with colors, CSS variables, brand tokens,
  theme presets, white-label switching, or any visual styling for DeliveryOS.
  Trigger phrases: "theme", "color", "CSS variable", "preset", "brand",
  "white-label", "Crimson Classic", "Ocean Fresh", "dark mode", "--brand-".
---

# DeliveryOS Theme System

## Goal
Provide the agent with all color tokens, font references, and preset definitions
so every file stays visually consistent across Crimson Classic, Ocean Fresh,
and Midnight Urban presets without any hardcoded values.

## Always read resources/tokens.css and resources/presets.json before writing any CSS.

## How themes work
1. All colors live in :root {} as CSS custom properties.
2. Switching theme = replacing :root values only. Zero DOM changes.
3. Tailwind uses these vars via tailwind.config extend.colors.
4. Dark mode: @media (prefers-color-scheme: dark) overrides :root.

## Theme switcher implementation
```html
<button id="theme-btn" onclick="cycleTheme()"
  style="position:fixed;top:16px;right:16px;z-index:9999;
         background:var(--brand-surface);border:1px solid var(--brand-border);
         border-radius:20px;padding:6px 14px;font-size:12px;cursor:pointer">
  🎨 Тема
</button>
<script>
const PRESETS = { /* loaded from resources/presets.json */ };
let current = 0;
const keys = Object.keys(PRESETS);
function cycleTheme() {
  current = (current + 1) % keys.length;
  const p = PRESETS[keys[current]];
  const r = document.documentElement.style;
  Object.entries(p).forEach(([k, v]) => r.setProperty(k, v));
  document.getElementById('theme-btn').textContent = '🎨 ' + keys[current];
}
</script>
```

## Constraints
- Never override --color-success, --color-warning, --color-danger with brand colors.
- Semantic colors are immutable across all presets.
- custom_css field (Business tier only) must be sanitized via DOMPurify before inject.
