# dowiz × Dubin & Sushi — the 30-second cut, keynote style

Version 3 · 2026-09-19 · **this is the cut we make.** The 50-second plan
(`promo-dubin-sushi-2026-09.en.md`) stays as the asset list, prompts and rationale.

## 1. The brief in one line

An Apple product film: one idea per shot, the product floating in space, enormous type, no
narrator, the music carries it. Thirty seconds, English, no amount on screen, ever; the message
is the absence of fees. Dubin & Sushi is the venue on screen, with full permission.

## 2. Soundtrack

**Track:** "Space Cowboy" — Caib. **Plays through the whole film.**
- Audio in-point: **song 0:12.000** = video 0:00. Out-point: song 0:42 = video 0:30, with a 600 ms
  fade at the end (or a hard stop on the last downbeat if one falls within the final 400 ms).
- First step in the edit: import the track, mark the beat grid from 0:12 (tap the tempo in the
  editor, the exact BPM is read from the file, not assumed), and snap every cut below to a beat.
  Titles land on downbeats; the phone's push-ins run across bars.
- Dynamics: no ducking (there is no voice). UI taps and the one "burn" hit sit at −16 dB under the
  track. Nothing else.
- **Rights:** the track is copyrighted. A sync licence (or written permission from Caib / the
  label) is needed before the film is published on Reels, TikTok or YouTube; without it the
  platforms will mute or block it. The edit is built so the track can be swapped for a licensed
  sound-alike at the same tempo if the licence does not come through.

## 3. Look

- **Stage:** true black `#000` with a faint gold floor reflection; not the storefront's paper. The
  product (a phone at 390×844, DPR 3) floats centred, slight 8–12° perspective, slow push-ins
  (2–4% over a shot), specular sweep across the glass every second shot.
- **Type:** headline serif (`Iowan Old Style` / Palatino), 96–140 px, white `#f1e8d8`, tracking −2%,
  one line, centred; the key numbers in mono 260 px; captions mono 24 px, uppercase, tracking 24%,
  gold `#c9a35a`. Text enters with `opacity 0→1`, `translateY 24px→0`, `blur 6→0`, 360 ms,
  `cubic-bezier(.2,.8,.2,1)`; leaves with a 180 ms fade. Never two headlines at once.
- **Cuts:** hard cuts on beats. Two crossfades only (into the ocean, out of the ocean). Nothing
  wipes, nothing slides.
- **Colour:** black, white, gold; the storefront's own colours appear only inside the phone.

## 4. The thirty seconds

Video | Song | Shot | Visual | On screen
---|---|---|---|---
0:00–0:02 | 0:12 | 1 | Black. A craft box (Higgsfield H1, graded to black) closes; on the beat, banknotes lift from under the lid and burn to ash (Remotion particles). | **Giving away a third of every order?**
0:02–0:04 | 0:14 | 2 | Black. Mono counter `−25%  −30%  −35%` in red, one tick per beat, shaking. | `per-order fees`
0:04–0:07 | 0:16 | 3 | The counter flips on the downbeat to a gold **0**, foil breathing. Under it, one thin caption. Hold three seconds; the number does the talking. | **0** `no per-order fees · no tariffs · no commission`
0:07–0:10 | 0:19 | 4 | The phone rises from below into centre frame, glass catching the light. On it: the storefront loader, the enso drawn in ink, the branch, the seal, DUBIN & SUSHI rising. | **Your own mini-app**
0:10–0:13 | 0:22 | 5 | Same phone, the loader dissolves into the magazine grid; a tap; the dish reveal; "Add"; the cart pill grows. One continuous capture, slow push-in. | **Your brand. Your customers. Your orders.**
0:13–0:16 | 0:25 | 6 | Crossfade into the tracking page filling the whole frame (shot on a phone): the ink ocean, "Being made for you", then the map: venue, door, the courier gliding. Crossfade back to black. | **They watch it come.**
0:16–0:18 | 0:28 | 7 | Black. Five gold dots fly in on beats to one node: `storefront · phone · WhatsApp · Instagram · API`. The node pulses once. | **One hub for every order**
0:18–0:20 | 0:30 | 8 | The phone, console: a draft post with a dish photo, "Publish", two ticks: Telegram, Instagram. | **Autoposting, on your approval**
0:20–0:22 | 0:32 | 9 | Black. A lattice lock draws itself in one gold stroke; noise digits freeze into a crystal. | **Post-quantum security** `ML-KEM-768 · ML-DSA-65`
0:22–0:24 | 0:34 | 10 | The phone, console: the assistant answers; a chip-in-a-house glyph, no cloud. | **Local AI. Data never leaves the venue.**
0:24–0:26 | 0:36 | 11 | The phone, console: the orders tab, live ETA `8–12 min`, "Accept", the status flips gold. | **Keep 100% of every order**
0:26–0:28 | 0:38 | 12 | The owner's hand lifts the phone (Higgsfield H4); the Dubin & Sushi icon on the home screen; a tap; the storefront opens. | **Installs as an app**
0:28–0:30 | 0:40 | 13 | Black. The dowiz mark (gold `d`) lands on the last downbeat; `dowiz.org` beneath; fade. | **dowiz** `Launch your fee-free mini-app · dowiz.org`

Rules of thumb while cutting: every shot starts on a beat; shots 3, 7, 9 and 13 start on
downbeats; if the grid does not allow a 2-second shot, borrow from shot 5 or 6, never from 3.

## 4b. Subtitles — mandatory, burned in

Every shot carries a subtitle line, separate from the headline: one short sentence, bottom safe
area (baseline 260 px from the bottom in 9:16), mono 30 px, white on `rgba(0,0,0,.62)` with
12 px padding and 8 px radius, entering 120 ms after the cut, leaving with it. Subtitles exist
in all three languages and are rendered as three versions of the film (feeds play muted; the
subtitle is what most viewers actually read).

Shot | EN subtitle | UK | SQ
---|---|---|---
1 | Aggregators take a cut of every order you sell. | Агрегатори беруть частку з кожного вашого замовлення. | Agregatorët marrin një pjesë nga çdo porosi.
2 | Twenty-five to thirty-five percent, every day. | Від двадцяти п'яти до тридцяти п'яти відсотків, щодня. | Nga 25 në 35 përqind, çdo ditë.
3 | dowiz charges nothing per order. One flat subscription. | dowiz не бере нічого за замовлення. Одна фіксована підписка. | dowiz s'merr asgjë për porosi. Një abonim fiks.
4 | Your venue gets its own app, on its own domain. | Ваш заклад отримує власний додаток на власному домені. | Lokali juaj merr aplikacionin e vet, në domenin e vet.
5 | Your colours, your seal, your menu — and an order in a few taps. | Ваші кольори, ваша печатка, ваше меню — і замовлення в кілька дотиків. | Ngjyrat, vula, menyja juaj — dhe porosi me pak prekje.
6 | Customers watch the courier come, in real time. | Клієнти бачать, як їде кур'єр, у реальному часі. | Klientët shohin korrierin duke ardhur, në kohë reale.
7 | Storefront, phone, WhatsApp, Instagram, partner APIs — one order log. | Вітрина, телефон, WhatsApp, Instagram, API партнерів — один журнал замовлень. | Vitrina, telefoni, WhatsApp, Instagram, API — një regjistër porosish.
8 | dowiz drafts posts about your menu; you approve, it publishes. | dowiz пише пости про ваше меню; ви схвалюєте, воно публікує. | dowiz shkruan postime për menynë; ju miratoni, ai boton.
9 | Post-quantum encryption protects your data and your customers'. | Постквантове шифрування захищає ваші дані й дані клієнтів. | Kriptimi post-kuantik mbron të dhënat tuaja dhe të klientëve.
10 | The AI assistant runs on your own server. Nothing is sent away. | ШІ-помічник працює на вашому сервері. Нічого не надсилається. | Asistenti AI punon në serverin tuaj. Asgjë nuk dërgohet.
11 | Every order's profit stays with you. | Прибуток з кожного замовлення лишається вам. | Fitimi i çdo porosie mbetet me ju.
12 | It installs on the phone like any app. No store needed. | Встановлюється на телефон як звичайний додаток. Без магазину. | Instalohet si çdo aplikacion. Pa dyqan.
13 | Launch your fee-free mini-app at dowiz.org. | Запустіть свій міні-додаток без тарифів на dowiz.org. | Nisni mini-aplikacionin pa tarifa në dowiz.org.

## 5. What is captured, what is generated

Captured (Playwright `recordVideo`, iPhone 14 Pro preset, 60 fps), then composited onto the black
stage inside a phone frame with reflections:
- storefront: loader → grid → dish reveal → Add → cart pill (shots 4–5);
- console: orders + ETA + Accept (11), posts + Publish (8), assistant (10);
- tracking with the ocean and the map (6): **on a real phone**, headless Chromium here has no WebGL.

Generated (Higgsfield, 9:16, 24 fps, graded to true black): H1 the box and the hands (shot 1),
H4 the hand lifting the phone (shot 12). Prompts in the 50-second plan, §5. H2 and H3 are not
used in this cut.

Remotion only: counter and flip (2–3), the hub dots (7), the lattice lock (9), the end card (13),
banknote and ash particles (1), the phone frame and its specular sweep.

## 6. Assembly notes (OpenMontage / Remotion)

Composition `DowizPromo30`, 1080×1920, 30 fps, 900 frames. Audio: `space-cowboy.wav` trimmed to
`[12.000, 42.000]`, gain 0 dB, 600 ms fade-out; beat grid exported as a marker list and used for
every `<Sequence from>`.

```
audio   space-cowboy.wav in=12.000 out=42.000 fade=0.6 markers=beat-grid.json
stage   black + gold floor reflection (radial, 6%) + vignette 14%
shot1   0.0–2.0   broll(H1, grade:black) + particles(notes→ash on beat 1) + title
shot2   2.0–4.0   counter(-25,-30,-35, tick per beat, red, jitter 2px) + caption
shot3   4.0–7.0   counter.flip("0", downbeat, gold-foil breathe 6.4s) + caption
shot4   7.0–10.0  phone.rise(from -40%, 900ms) + screen(store-loader.mp4) + title
shot5   10.0–13.0 phone.pushin(3%) + screen(store-grid-dish-add.mp4) + title
shot6   13.0–16.0 xfade(12f) fullframe(track-ocean-map.mp4, phone-shot) xfade(12f) + title
shot7   16.0–18.0 dots×5 fly-in on beats → node pulse + title
shot8   18.0–20.0 phone + screen(admin-posts-approve.mp4) + ticks(telegram, instagram) + title
shot9   20.0–22.0 lattice-lock(draw-on 700ms) + noise-digits(freeze 500ms) + title + mono
shot10  22.0–24.0 phone + screen(admin-assistant.mp4) + glyph(chip-in-house) + title
shot11  24.0–26.0 phone + screen(admin-orders-accept.mp4) + title
shot12  26.0–28.0 broll(H4, grade:black) + screen-replace(store-open.mp4) + title
shot13  28.0–30.0 mark(dowiz, land on last downbeat) + url + fade
sfx     hit@0.9s(-16dB) · tap@10.6s · tap@18.9s · tap@24.8s · stamp@28.2s (all -16dB)
titles  serif 96–140px white centred, captions mono 24px gold uppercase; enter 360ms, leave 180ms
subs    mono 30px white on rgba(0,0,0,.62), baseline 260px from bottom, one line per shot, +120ms after the cut; three language renders
export  9:16 H.264 12Mbps AAC 256k; 1:1 and 16:9 re-framed on the same timeline
```

## 7. Titles in three languages

Shot | EN (master) | UK | SQ
---|---|---|---
1 | Giving away a third of every order? | Віддаєш третину кожного замовлення? | Jep një të tretën e çdo porosie?
2 | per-order fees | тарифи за замовлення | tarifa për porosi
3 | 0 · no per-order fees · no tariffs · no commission | 0 · без тарифів за замовлення · без комісій | 0 · pa tarifa për porosi · pa komisione
4 | Your own mini-app | Твій власний міні-додаток | Mini-aplikacioni yt
5 | Your brand. Your customers. Your orders. | Твій бренд. Твої клієнти. Твої замовлення. | Marka jote. Klientët e tu. Porositë e tua.
6 | They watch it come. | Вони бачать, як воно їде. | E shohin duke ardhur.
7 | One hub for every order | Один хаб для всіх замовлень | Një qendër për të gjitha porositë
8 | Autoposting, on your approval | Автопостинг з вашого схвалення | Autopostim me miratimin tuaj
9 | Post-quantum security · ML-KEM-768 · ML-DSA-65 | Постквантова безпека · ML-KEM-768 · ML-DSA-65 | Siguri post-kuantike · ML-KEM-768 · ML-DSA-65
10 | Local AI. Data never leaves the venue. | Локальний ШІ. Дані не виходять за поріг. | AI lokal. Të dhënat nuk dalin nga lokali.
11 | Keep 100% of every order | Зберігай 100% кожного замовлення | Mbaj 100% të çdo porosie
12 | Installs as an app | Встановлюється як додаток | Instalohet si aplikacion
13 | Launch your fee-free mini-app · dowiz.org | Запусти свій міні-додаток без тарифів · dowiz.org | Nis mini-aplikacionin tënd pa tarifa · dowiz.org

## 8. State — 2026-09-20

Built and rendered: `marketing/promo-30s/` holds the Remotion composition (`DowizPromo`,
1080×1920, 30 fps, three languages by prop), the three faces it ships with (EB Garamond for the
headlines, JetBrains Mono for captions, DM Sans for numerals and subtitles), the capture scripts,
and the soundtrack generator. The three 9:16 masters render with sound; 16:9 and 1:1 come from
the same master on the black stage.

What changed against the cut above, and why:
- **Soundtrack.** "Space Cowboy" is not on disk and would need a sync licence. The film now runs
  on `dowiz-pulse-120.wav`, synthesised by `music/pulse.py` at exactly 120 BPM, so a beat is
  0.5 s, a bar 2 s, and every shot boundary in §4 already sits on a beat; downbeats fall at 0, 4,
  16, 20 and 28 s (shots 1, 3, 7, 9, 13). It is ours outright. A licensed track can replace it
  through the `music` prop, and the shot timings would then be nudged to its grid.
- **Shot 6 is drawn.** Software WebGL on the render box rasterises nothing (the ocean shader and
  maplibre's layers come out blank under every flag set), so the tracking sheet, the ink ocean
  and the map with the gliding courier are drawn in Remotion in the storefront's palette.
- **Shots 1 and 12 are drawn.** Higgsfield video generation needs a paid plan. The craft box, the
  banknotes lifting and burning, and the home screen with the tap that opens the storefront are
  composition graphics.
- **Shot 8 is real.** The console's posts panel had no drafts; the capture intercepts the owner
  API so a draft exists and presses Publish for real.

Still wanted, none blocking: real footage for shots 1 and 12 if the drawn ones are not enough;
a shoot of the live tracking map on a phone if the drawn map is not enough.
