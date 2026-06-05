# Theme System

The Theme System is powered by `ThemeRenderer`, a pure function that deterministically generates a CSS file string and a `cssHash`.

## Features
- **Deterministic Output:** Same inputs always yield the same CSS Hash.
- **Cache Invalidations:** `location_themes` updates generate a new hash. The SSR client uses this hash as a URL parameter, guaranteeing an edge cache miss.
- **WCAG Warnings:** Checks `primary` against `bg_color` and warns if ratio < 4.5.
- **Font Whitelist:** Ensures only fonts with `latin-ext` subset (for Albanian `ë` and `ç`) are used. Allowed fonts: `Inter`, `Roboto`, `Source Sans 3`, `Lato`, `Open Sans`, `system-ui`.
