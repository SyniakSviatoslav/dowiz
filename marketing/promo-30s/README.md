# dowiz × Dubin & Sushi — the 30-second cut

The Remotion composition behind `docs/marketing/promo-dubin-sushi-30s.en.md`. One composition,
`DowizPromo`, 1080×1920, 30 fps, 900 frames; `lang` picks the titles and burned-in subtitles
(`en`, `uk`, `sq`); `music` names a file under `public/promo/` (or `null` for a silent draft) and
`musicInSeconds` is the song's in-point (12 s for "Space Cowboy").

## Layout

- `src/DowizPromo.tsx` — the thirteen shots, the stage, the phone frame, every graphic.
- `fonts/` — EB Garamond, JetBrains Mono, DM Sans (OFL). Copy to `public/fonts/`; the
  composition loads them itself, so every machine renders the same glyphs.
- `capture/` — the Playwright scripts that record the six UI clips from the live hub. They read
  the owner credentials from `/root/.dowiz_owner` and run from the repo root.
- `public/promo/` (not committed) — the clips and the track.

## Clips and their in-points

| clip | shots | in-point | how it is made |
|---|---|---|---|
| `store-loader-grid-dish.webm` | 4, 5, 12 (stand-in) | 0.2 s, 5.2 s | `capture-surfaces.mjs` |
| `track-ocean-map.webm` | 6 | 5.6 s | `capture-surfaces.mjs`; headless has no WebGL, so no ocean and no map — shoot this on a phone |
| `admin-posts-approve.webm` | 8 | 7.0 s | `capture-posts.mjs`: the posts API is intercepted so a draft exists, then Publish |
| `admin-assistant.webm` | 10 | 8.5 s | `capture-surfaces.mjs`; the answer is staged in the DOM |
| `admin-orders.webm` | 11 | 4.9 s | `capture-surfaces.mjs` |
| `h1-craft-box.mp4`, `h4-hand-phone.mp4` | 1, 12 | — | Higgsfield; until they exist the shots use a drawn stand-in |

## Render

```
npm install
mkdir -p public/fonts public/promo && cp fonts/*.ttf public/fonts/
node capture/capture-surfaces.mjs $PWD/public/promo      # from the repo root
node capture/capture-posts.mjs $PWD/public/promo
npm run render:en
```

On this box Remotion cannot download its own Chrome; point it at Playwright's:
`--browser-executable=$HOME/.cache/ms-playwright/chromium-*/chrome-linux-arm64/chrome --gl=swiftshader --concurrency=2`.
`--scale=0.5` gives a five-minute proof render.

## Open

1. The soundtrack is not on disk. When it is: `public/promo/space-cowboy.wav`, set `music`, tap
   the beat grid from 0:12 and nudge `SHOTS` so every cut lands on a beat (shots 3, 7, 9, 13 on
   downbeats). The sync licence is needed before publishing.
2. Shot 6 is the tracking panel in the phone frame; the ocean and the map need real phone footage.
3. Shots 1 and 12 wait for the Higgsfield inserts (prompts in the 50-second plan, §5).
