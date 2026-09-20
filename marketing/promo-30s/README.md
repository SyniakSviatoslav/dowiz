# dowiz × Dubin & Sushi — the 30-second cut

The Remotion composition behind `docs/marketing/promo-dubin-sushi-30s.en.md`. One composition,
`DowizPromo`, 1080×1920, 30 fps, 900 frames; `lang` picks the titles and burned-in subtitles
(`en`, `uk`, `sq`); `music` names a file under `public/promo/` (or `null` for a silent draft) and
`musicInSeconds` is the song's in-point (12 s for "Space Cowboy").

## Layout

- `src/DowizPromo.tsx` — the thirteen shots, the stage, the phone frame, every graphic.
- `fonts/` — Unbounded, Manrope, JetBrains Mono as woff2 (the landing's faces) and EB Garamond
  for the drawn tracking sheet (all OFL). Copy to `public/fonts/`; the composition loads them
  itself, so every machine renders the same glyphs.
- `capture/` — the Playwright scripts that record the six UI clips from the live hub. They read
  the owner credentials from `/root/.dowiz_owner` and run from the repo root.
- `public/promo/` (not committed) — the clips and the track.

## Clips and their in-points

| clip | shots | in-point | how it is made |
|---|---|---|---|
| `store-loader-grid-dish.webm` | 4, 5, 12 | 0.2 s, 5.2 s, 0 s | `capture-surfaces.mjs` |
| `admin-posts-approve.webm` | 8 | 7.0 s | `capture-posts.mjs`: the posts API is intercepted so a draft exists, then Publish |
| `admin-assistant.webm` | 10 | 8.5 s | `capture-surfaces.mjs`; the answer is staged in the DOM |
| `admin-orders.webm` | 11 | 4.9 s | `capture-surfaces.mjs`; the detail sheet opens at 6.0 s of the clip |

Drawn in the composition, no footage: shot 1 (the craft box, the notes lifting and burning),
shot 6 (the tracking sheet with the ink ocean and the map, in the storefront's palette), shot 12
(a home screen, the tap, the storefront opening). Shot 6 is drawn because software WebGL on the
render box rasterises nothing: the storefront's ocean shader and maplibre's layers come out blank
under both `--use-gl=angle` (and the GPU process dies as soon as the storefront runs) and
`--use-gl=swiftshader` (contexts live, draws return 0,0,0,0). Measured 2026-09-20 with
`e2e/kit-regression/_webgl_probe*.mjs`. The `track-ocean-map` clip the capture script still
records shows the sheet with a white map; it is not used.

## The world

The cut wears the landing's world (`workers/api/public/platform/landing.css`): bone paper
`#f2f1ec`, ink `#0b0b0c`, one hot accent `#ff4d1c`; Unbounded 800 for the headlines, Manrope
for the subtitles, JetBrains Mono for captions and numerals. No gold, no gradients, no
textures on the stage. The one place gold survives is inside the phone on shot 6: the tracking
sheet is drawn in the venue's own palette (its seal gold, cream ink, EB Garamond), because
that is what the product looks like, not what the promo looks like.

Motion: everything sits inside `Camera` (the whole picture kicks on the three hits at 0.9, 4
and 28 s) and each shot inside `Shot` (a slow push-in with a little drift, alternating
direction). Headlines reveal word by word with a blur-in. The phone turns in as it rises.
`Impact` is the flash-and-shockwave on a beat: the burn, the zero, the lock, the mark. There is
no grain and no bokeh: `assets/grain.png` is left over from an earlier pass and is not loaded.

The cut is delivered in English. The `lang` prop still renders `uk` and `sq` (titles and
subtitles), but those are not part of the deliverable.

## Soundtrack

`music/pulse.py` synthesises the track from nothing with numpy: 30 s at exactly 120 BPM, A minor,
a ticking intro, the drop on the gold zero at 4 s, plucks under the phone, claps from the hub
shot, the final hit on the mark at 28 s, a 600 ms fade. The plan's UI taps and the burn hit are
baked in at −16 dB. It writes `dowiz-pulse-120.wav` and `dowiz-pulse-120-beat-grid.json`
(beat = 0.5 s, bar = 2 s, downbeats at 0, 4, 16, 20, 28 s, which is why every shot boundary in
`SHOTS` already sits on a beat). Loudness −15.4 LUFS integrated, peak −2 dBFS. It is ours, so
nothing needs licensing; a licensed track can replace it via the `music` prop.

```
python3 music/pulse.py public/promo/dowiz-pulse-120.wav
```

## Render

```
npm install
mkdir -p public/fonts public/promo && cp fonts/*.ttf public/fonts/
python3 music/pulse.py public/promo/dowiz-pulse-120.wav
node capture/capture-surfaces.mjs $PWD/public/promo      # from the repo root
node capture/capture-posts.mjs $PWD/public/promo
npm run render:en            # the deliverable; render:uk and render:sq exist
```

On this box Remotion cannot download its own Chrome; point it at Playwright's:
`--browser-executable=$HOME/.cache/ms-playwright/chromium-*/chrome-linux-arm64/chrome --gl=swiftshader --concurrency=2`.
`--scale=0.5` gives a ten-minute proof render; full scale takes about twenty minutes.

Other frames from the same 9:16 master, on the bone stage (`scripts/reframe.sh`):
16:9 pillarboxes the phone-tall frame on bone, centred on 1920×1080; 1:1 does the same on 1080×1080.

## Open

1. Higgsfield video generation needs a paid plan (`Requires basic plan or higher`); shots 1 and
   12 are drawn instead, and stay drawn unless real footage is wanted.
2. The three masters are silent-safe: feeds play muted, the subtitles carry the message.
