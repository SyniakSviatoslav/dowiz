# In-app learning, lesson videos in three languages, and an internal wiki: what OpenMontage is, what this box can produce, and the plan

**Date:** 2026-09-24. **HEAD read:** `5abeb0ac` ("bebop-wasm: the decide fixtures' .log images were eaten by
*.log in .gitignore", 2026-09-24 17:50). **Tree state:** clean (`git status --short | wc -l` = 0, measured).
**Method.** Read-only on the tree; no code edit, no git write, no deploy. `measured` means a command was run on
this box today and its output is quoted; `derived` means arithmetic over measured inputs; `UNVERIFIED` means
neither. External claims cite the URL read on 2026-09-24. OpenMontage was cloned into the session scratchpad
(not the tree) and read there; the clone is disposable.

---

## 0. Verdict, in ten lines

1. **OpenMontage is a skill pack plus a Python tool registry for coding agents, not a renderer.** AGPL-3.0,
   Python 3.10+, FFmpeg, Node 18+, `make setup`; every API key optional (§1). It *can* be installed here.
2. **The three parts of it this job needs are the three parts that do not fit this box:** its screen capture is
   `x11grab`/macOS/Windows or the Cap desktop app (there is no X server here and no Playwright capture tool
   despite the pipeline text naming one); its Remotion composer has no way to be pointed at Playwright's Chrome
   (the promo memory: Remotion cannot fetch its own Chrome on this box); its only offline TTS, Piper, has **no
   Albanian voice**. Measured, §1.3.
3. **So the honest answer is: the videos will not be "produced with OpenMontage".** They will be produced by
   Playwright captures + FFmpeg + neural TTS, which is what the promo already does in this tree. Two ideas are
   worth taking from OpenMontage (§1.4); nothing is worth depending on.
4. **Voices exist for all three languages, and were synthesised from this box today:** `sq-AL-AnilaNeural`,
   `sq-AL-IlirNeural`, `uk-UA-PolinaNeural`, `uk-UA-OstapNeural`, `en-GB-SoniaNeural` (Microsoft neural, via
   `edge-tts`; ~4.6 s wall per sentence; word-timed VTT emitted). For release renders the same voices come
   through an Azure Speech key (F0: 0.5 M characters/month free). Offline: Ukrainian yes (Piper, CC0 voice),
   Albanian only under a non-commercial licence. §2.
5. **A tour engine already exists and is good:** `workers/api/public/lib/guide.js` (368 lines) — declarative
   help table, ordered tour, "?" hints, per-device progress, reduced-motion, CSP-safe, MutationObserver
   remount. It is wired into the **courier app only** (5 steps); the console and the room use it nowhere, and
   `data-tour` appears **0** times in the tree. The plan extends it; it does not write a second one. §3, §5.
6. **The module map has 16 owner modules, 10 waiter modules, 7 courier modules** (from `TABS`, `GROUPS`, the
   room's `S.view` switch and the courier's render functions), and **38 lessons**. §4.
7. **Hosting: there is no R2 binding in `wrangler.toml` (measured) and the CSP has no `media-src`.** The
   landing already ships 13 MB of MP4 as static assets; 38 lessons × 3 languages will be ~0.8 GB and do not
   belong in git. An R2 bucket on `learn.dowiz.org` plus one CSP line, or the Worker streaming from R2 behind
   the apps' Bearer token if the wiki is to be gated. §6.5, §8.
8. **The QA venue (`qa-durres.dowiz.org`) does not exist yet** (`curl` → `000`, measured; `tools/platform/`
   holds only `attach-host.sh`). Every lesson that *writes* depends on roadmap row F7. §5.4.
9. **A full re-render is an overnight job on this box, an incremental one is minutes:** measured 31.7 s to
   burn subtitles into and re-encode a 30 s 1080×1920 clip (≈1× real time), so ~3 h of lesson video is ~3.5 h
   of encode plus ~4 h of capture. Remotion is kept for the ten-second intro/outro only. §6.6.
10. **Wave L is 12 rows** (§9); the order is anchors → engine → practice venue → content → captures → wiki →
    gate, with the operator's design refresh in front of all of it because the tours anchor to the DOM.

---

## 1. OpenMontage, measured

### 1.1 What it is

- **Repository:** `https://github.com/calesthio/OpenMontage` (the README's own Discussions link points there;
  `github.com/Open-Montage/OpenMontage` and `OpenMontage-app/OpenMontage`, both returned by search, answer
  404 / ask for credentials — measured). Cloned at `08e2151f` (2026-09-05). Website `openmontage.video`.
- **Licence:** `LICENSE` is the GNU Affero GPL v3 (measured, first line of the file). AGPL bites only if a
  *modified* copy is run as a network service; running it locally to make files carries no obligation. It
  would matter if its Backlot board or tools were ever served from the platform — they will not be.
- **What it contains:** `pipeline_defs/` (13 YAML pipelines: `animated-explainer`, `animation`,
  `avatar-spokesperson`, `character-animation`, `cinematic`, `clip-factory`, `documentary-montage`,
  `framework-smoke`, `hybrid`, `localization-dub`, `podcast-repurpose`, `screen-demo`, `talking-head`),
  `tools/` (Python classes with a `provider` and an `estimate_cost`, discovered by `tools/tool_registry.py`),
  `skills/` (the agent-side "director" prompts per pipeline stage), `remotion-composer/` (Remotion
  `^4.0.484`), `backlot/` (a FastAPI storyboard board). The **brain is the coding agent** (Claude Code,
  Cursor, Codex…): `grep -rn ANTHROPIC_API_KEY tools lib scripts` finds **0** hits (measured); `config.yaml`'s
  `llm: provider: anthropic` block is not read by any tool.
- **Prerequisites (README "Quick Start"):** Python 3.10+, FFmpeg, Node.js 18+, an AI coding assistant.
  `make setup` = `pip install -r requirements.txt` (13 top-level packages incl. `pydantic`, `fastapi`,
  `google-genai`, `openai`) + `npm install` in `remotion-composer` + `pip install piper-tts` (skipped on
  failure) + an `npx hyperframes` cache warm.

### 1.2 What it needs — keys, and what is free

From `docs/PROVIDERS.md` (env var names verbatim). **Every key is optional**; each unlocks tools.

| Class | Env var(s) | Free? | Relevant here? |
|---|---|---|---|
| Stock media | `PEXELS_API_KEY`, `PIXABAY_API_KEY`, `UNSPLASH_ACCESS_KEY` | free keys | no — the footage is our own screens |
| Google (TTS, Imagen, Lyria, Veo) | `GOOGLE_API_KEY` / `GOOGLE_TTS_API_KEY` / `GEMINI_API_KEY` | 1 M TTS chars/month free (PROVIDERS.md:14) | TTS only; **no Albanian or Ukrainian in Chirp 3 HD** (§2) |
| ElevenLabs | `ELEVENLABS_API_KEY` | 10 K chars/month free | sq + uk exist in Eleven v3 (§2); paid past the trial |
| Azure Speech | `AZURE_SPEECH_KEY` (+ region) | 0.5 M neural chars/month free | **yes — the one provider with native sq and uk voices** |
| OpenAI, xAI, Fish, Doubao, DashScope, Kling, MiniMax | `OPENAI_API_KEY`, `XAI_API_KEY`, `FISH_AUDIO_API_KEY`, `DOUBAO_SPEECH_API_KEY`, `DASHSCOPE_API_KEY`, `KLING_API_KEY`, `MINIMAX_API_KEY` | trials | no |
| Video generation gateways | `FAL_KEY`, `ATLASCLOUD_API_KEY`, `ARK_API_KEY`, `HEYGEN_API_KEY`, `RUNWAY_API_KEY`, `SUNO_API_KEY`, `TENCENT_TOKENHUB_API_KEY` | paid | no — nothing is generated; the screens are real |
| Local, GPU | `VIDEO_GEN_LOCAL_ENABLED`, `VIDEO_GEN_LOCAL_MODEL`, ComfyUI at `localhost:8188`; `requirements-gpu.txt` = torch | free + GPU | **no GPU on this box** |
| Offline TTS | Piper (`tools/audio/piper_tts.py`, provider `piper`, cost 0) | free | uk yes, **sq no** (§2) |
| Transcription | `tools/analysis/transcriber.py`: faster-whisper / WhisperX, CPU or CUDA | free | not needed — we write the script, the TTS returns the timings |

### 1.3 Can it run on this box — component by component (measured)

| Component | This box | Fits? |
|---|---|---|
| Python | 3.14.4; `pip 25.1.1`; `python3 -m venv` creates a venv **without pip** (ensurepip absent) — `pip install --target` works | yes, with `--target` or `--break-system-packages` |
| FFmpeg | 8.0.1 with `--enable-libass --enable-libfreetype --enable-libfontconfig --enable-libharfbuzz`; filters `subtitles`, `ass`, `drawtext`; encoders `libx264`, `libvpx-vp9`, `libaom-av1`, `libwebp_anim`, `gif` | yes — this is the part that does all the work |
| Node | v22.22.1; Playwright with `chromium-1243` (arm64) installed; Remotion `^4.0.441` in `marketing/promo-30s` | yes |
| `screen-demo` pipeline, mode `real_capture` | `tools/capture/screen_recorder.py` records with `x11grab`+`pulse` on Linux, `gdigrab` on Windows, `avfoundation` on macOS; `cap_recorder.py` picks up files from the Cap desktop app. No X server on an Android/proot box; no Cap. The pipeline description mentions "playwright-recording" but `ls tools/capture` = `cap_recorder.py`, `screen_capture_selector.py`, `screen_recorder.py` — **no Playwright tool exists** | **no** |
| `screen-demo` mode `synthetic_terminal` | a Remotion terminal animation for CLI demos | irrelevant (our surfaces are not terminals) |
| Remotion compose (`tools/video/video_compose.py`, `npx remotion render`) | `grep browserExecutable\|browser-executable\|chromiumOptions tools/video remotion-composer` = **0** hits; Remotion cannot download its Chrome here (`dowiz-promo-30s-remotion-2026-09-19`), so every OpenMontage Remotion render would fail until the tool is patched to pass `--browser-executable` | no, without a patch |
| HyperFrames compose | `npx hyperframes` — a Chromium render as well; the same Chrome problem; WebGL renders nothing here (`webgl-renders-nothing-on-the-box`) | no |
| TTS | Piper: wheels exist for this arch and Python (`piper_tts-1.8.0-cp39-abi3-…aarch64.whl`, `onnxruntime-1.30.0-cp314-…aarch64.whl`, measured with `pip download`) — but no `sq` voice in `rhasspy/piper-voices`; cloud TTS through keys | uk offline, sq cloud only |
| Subtitles (`tools/subtitle/subtitle_gen.py`) | pure Python: takes `segments` with word timings, renders SRT/VTT/JSON, optional per-word highlight style | yes — and it is ~300 lines we can equal with 40 |
| Self-review (`meta/reviewer`: ffprobe validation, frame sampling, audio level analysis, subtitle checks) | a checklist the agent runs | yes — worth copying as our gate (§8.4) |

### 1.4 Verdict on OpenMontage, and what to take from it

**Do not adopt it as a dependency.** On this box it would contribute: a subtitle writer we can replace in an
afternoon, a Piper wrapper that has no Albanian, a Remotion path that cannot start Chrome, and a screen
recorder that cannot see a screen. Its real value — the director skills that turn "make me an explainer" into
a plan — is worth nothing for 38 lessons whose script *is* the product's own copy in three languages, written
by people who know the product. The cost of adopting it is a 13-package Python environment, an AGPL boundary
to keep in mind, and a second Remotion install at a different version than the promo's.

**Take two things.** (1) Its shape for a lesson's timing: narration as *segments with word-level times*
that drive both the burned-in subtitles and the `.vtt` — `edge-tts --write-subtitles` already emits this.
(2) Its self-review list as a gate on every rendered file: `ffprobe` (duration, streams, resolution), one
sampled frame per chapter tiled into a contact sheet (the promo memory's rule: never trust a render's exit
code), integrated loudness within a band, and every subtitle cue inside the video's duration.

If the operator wants the words "produced with OpenMontage" to be true in some sense: the coding agent that
produces these lessons can *read* OpenMontage's `skills/pipelines/screen-demo/*` as style guidance. That is the
only part of it that transfers.

---

## 2. Voices for sq / en / uk, measured

`edge-tts` was installed into the scratchpad (`pip install --target`) and `--list-voices` run from this box:

```
sq-AL-AnilaNeural   Female  General  Friendly, Positive
sq-AL-IlirNeural    Male    General  Friendly, Positive
uk-UA-OstapNeural   Male    General  Friendly, Positive
uk-UA-PolinaNeural  Female  General  Friendly, Positive
en-GB-SoniaNeural   Female  General  Friendly, Positive     (en-US-JennyNeural etc. also listed)
```

One sentence per language was then synthesised (measured):

| Voice | Text | Output | Wall |
|---|---|---|---|
| `sq-AL-AnilaNeural` | «Shtypni «Merr» dhe ekrani ju çon hap pas hapi deri te «U dorëzua».» | 28 368 B mp3, 4.73 s audio, VTT `00:00:00,100 --> 00:00:04,725` | 4.6 s |
| `uk-UA-PolinaNeural` | «Натисніть «Взяти», і екран проведе вас крок за кроком до «Доставлено».» | 29 808 B, 4.97 s | 4.7 s |
| `en-GB-SoniaNeural` | "Tap Take, and the screen walks you step by step to Delivered." | 24 624 B, 4.10 s | 4.4 s |

| Provider | sq | uk | en | Runs here | Licence / cost | Note |
|---|---|---|---|---|---|---|
| **Microsoft neural via `edge-tts`** | Anila, Ilir | Polina, Ostap | many | **yes (measured)** | free; the Edge read-aloud endpoint, not a contracted API — the bare voices URL answers **401** without the client's signed header (measured), i.e. Microsoft gates it and may change it | drafts and iteration |
| **Microsoft neural via Azure Speech key** | same voices | same | same | yes (needs `AZURE_SPEECH_KEY` from the operator) | F0 tier: 0.5 M characters/month free ([pricing](https://azure.microsoft.com/en-us/pricing/details/cognitive-services/speech-services/)); the voices are confirmed as Azure's by [json2video's Azure voice list](https://json2video.com/ai-voices/azure/voices/sq-al-anilaneural/) and Microsoft's [11-new-languages post](https://techcommunity.microsoft.com/blog/azure-ai-foundry-blog/11-new-languages-and-variants-and-more-voices-are-added-to-azure%E2%80%99s-neural-text-t/3541770) | **release renders** |
| Piper (MIT engine) | **none** | `lada`, `mykyta`, `oleksa`, `tetiana`, `ukrainian_tts` ([voices tree](https://huggingface.co/rhasspy/piper-voices/tree/main/uk/uk_UA)); `ukrainian_tts/medium` MODEL_CARD: dataset OHF voice-datasets, **CC0**, "trained from scratch" | yes | yes, CPU, wheels measured | free | offline fallback for uk only |
| Meta MMS-TTS `sqi` | VITS 36.3 M params, CPU via `transformers` ([model card](https://huggingface.co/facebook/mms-tts-sqi)) | — | — | probably (torch on aarch64, UNVERIFIED) | **CC-BY-NC 4.0** — non-commercial | the only offline Albanian; not usable for a commercial platform's material without the operator accepting the licence risk |
| ElevenLabs Eleven v3 | yes | yes | yes | key | 10 K chars/month free, then paid ([languages](https://elevenlabs.io/docs/help-center/other/what-languages-do-you-support)) | best quality; cost scales with re-renders |
| Google Cloud TTS Chirp 3 HD | **no** | **no** | yes | key | — ([supported languages](https://docs.cloud.google.com/text-to-speech/docs/chirp3-hd)) | standard uk voices: UNVERIFIED (the list page truncates under the fetcher) |
| Kokoro-82M | no | no | yes | — | Apache-2.0 | — |

**Volume, derived.** 38 lessons × ~200 words (~1 200 characters) × 3 languages ≈ 140 K characters per full
render; ten full renders a year stay inside Azure F0.

**Recommendation.** One provider for all three languages, so the three cuts of a lesson sound like one
product: Anila (sq), Polina (uk), Sonia (en) — female, "friendly, positive" in all three, the same pacing.
Ilir / Ostap / Ryan for the owner-console lessons if a second voice is wanted to mark the role. `edge-tts`
while a lesson is being written (free, 5 s per sentence), the Azure key for the render that ships. The
pipeline takes a **human WAV per lesson per language** in place of the TTS (§6.3), so a native reader can
replace any voice later without touching anything else.

---

## 3. What the tree already has (the ground the plan stands on)

| Thing | Where | What it gives the plan |
|---|---|---|
| **A tour engine** | `workers/api/public/lib/guide.js` (368), `lib/guide.css` (129) | `createGuide({ key, help, tour, mount, toast, words })`: one declarative table `{ at, title, body, hint, place }`, a tour as an ordered list of keys, "?" hints beside controls, `localStorage dw_guide_<key> = { state: open|paused|done, step }`, a step whose element is hidden is skipped in the direction of travel, `prefers-reduced-motion` respected (`calm()`, and `guide.css:125`), CSS injected as a same-origin `<link>` (CSP-safe), MutationObserver remount after every re-render, arrow keys / Escape / focus trap, positioning from `--space-*` tokens |
| Its one user | `public/courier/app.js:1200-1219` | `HELP()` with 6 rows, `TOUR = ['welcome','shift','sheet','mic','help']`, words from `t('gStep'…)`; help button mounted only when the courier is standing still |
| Zero users elsewhere | `grep -rn guide.js public/admin public/room` = 0; `grep -rn data-tour public` = **0** (measured) | the console and the room have no tour, no hints, no anchors |
| **i18n** | `admin/i18n.js` (394 lines, `T.sq/en/uk`, `t(k)`), `room/i18n.js` (161, imports the console's status words), `courier/i18n.js` (128, `t(k, vars)`), `store/i18n.js` | one table per app, three languages, `tools/gates/vocabulary.sh` holds status words to the kernel's twelve; language keys `dw_admin_lang`, `dw_room_lang`, `dw_c_lang`, `dw_lang` |
| Module structure | `admin/app.js:15` `TABS` (orders, menu, stock, couriers, more); `admin/more.js` `GROUPS` (inbox; roomGroup: bookings, floorPlan; marketing: promos, posts, campaigns, social; analytics: analytics, customers, staff, exceptions; settings: integrations, ebills, printer, tableQr, preview, venue, hours, deliveryTerms, payments, notifications, channels, mcp, cloud, branding, features, assistant, apiKeys, activation, health); `room/app.js:175-187` views `room, sitting, round, add, pay, transfer, moveSit, open, till, floor, login`; courier `render*` functions | the module map in §4 is read off these, not invented |
| **Browser harness** | `e2e/walk/_lib.mjs`: `browser()` refuses to launch above 26 processes, flags `--single-process --renderer-process-limit=1 --disable-gpu`, `watch()` collects page errors, HTTP ≥ 400 and **CSP violations** per screen, `creds` from `/root/.dowiz_owner`; walks `a-room-open … h-room-floor-plan` | the capture scripts inherit the process cap and the CSP listener for free |
| **Capture precedent** | `marketing/promo-30s/capture/capture-surfaces.mjs`: Playwright `recordVideo` at 540×1170 @2, `addInitScript` sets the language keys, route interception stages state | the lesson captures are this script generalised |
| **Video hosting precedent** | `public/platform/video/`: `promo-{en,sq,uk}-1080x1920.mp4` = 4.39 / 4.44 / 4.50 MB (1 168 kbps, H.264, 30 fps, measured) + posters, served as static assets; `platform/index.html:119` a `<video preload="metadata" playsinline muted loop>` with `data-src` per language | static assets serve video; requests to them are free and unlimited ([billing](https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/)); 25 MiB per file, 20 000 files on the free plan ([limits](https://developers.cloudflare.com/workers/platform/limits/)) |
| **Storage bindings** | `wrangler.toml`: `[assets]`, `[[kv_namespaces]] MEDIA` (25 MiB per value, [KV limits](https://developers.cloudflare.com/kv/platform/limits/)), `[[durable_objects.bindings]] HUB`, `[[send_email]]`; **no `r2_buckets`** (measured: `grep -n r2_buckets wrangler.toml` = 0); per-venue "cloud" copies go to a venue-supplied S3 bucket | a platform R2 bucket is new infrastructure |
| **CSP** | `public/_headers`: `default-src 'self'; script-src 'self' https://js.stripe.com; style-src 'self'; img-src 'self' data: blob: …; media-src` **absent** → media falls to `default-src 'self'` | video from any other origin needs one `media-src` line; the memory rule applies: audit after any header change |
| **The QA venue** | `curl -sI https://qa-durres.dowiz.org/` → `000` (measured); `ls tools/platform` = `attach-host.sh` only | F7 is not landed; the sandbox in §5.4 depends on it |
| **Roadmap** | `docs/design/ROADMAP-2026-09-22.md` is live (`ROADMAP.md` says "SUPERSEDED FOR STATUS"); waves A, B, C, F used; §Wave F defines **S ≤ a lane-day, M a few, L a week or blocked on a third party** and a `Goal / Files / CHECK / Depends on / Size` table | Wave L follows that shape, §9 |
| Design refresh | `grep -in "design refresh\|redesign" ROADMAP-2026-09-22.md BLUEPRINT-LAUNCH-GAPS-2026-09-24.md` = 0 rows (measured) | the operator's precondition has no row; §9 names it L0 so the dependency is written down |

---

## 4. The module map and the lesson list

Read off the code (function names per file, measured with `grep -oE "^(export )?(async )?function \w+"`).
Each lesson names its **anchors** (`data-tour` ids, §5.1), a **goal** the learner can be checked against, and
whether it **writes** (then it runs only on the practice venue, §5.4). Durations are estimates for the video
cut; the in-app tour is the same steps without narration.

### 4.1 Owner console (`/admin/`) — 16 modules, 22 lessons

| # | Module (file) | Functions in the code | Lessons | Goal (what the learner can do afterwards) | Writes | ~s |
|---|---|---|---|---|---|---|
| O1 | Orders (`orders.js`: `render`, `row`, `doAction` accept/confirm/reject/ready/assign/cancel, `foodBack`, `markSeen`, `openAssign`, `openOrder`, `exportCsv`, `loadJobs`) | **O1a Read the board**: the live statuses (PENDING → CONFIRMED → PREPARING → READY → IN_DELIVERY), the ring, the detail sheet | say what each column means and which order needs a hand | no | 75 |
| | | **O1b Move an order**: accept, ready, assign a courier, the reject with a reason | move a fresh order to READY and hand it to a courier | yes | 90 |
| | | **O1c Food back and CSV export** (`foodBack`, `exportCsv`) | record a return; export a day | yes | 60 |
| O2 | Platform orders (`aggregator.js`: `openAggregator`, `aggAddsUp`) | **O2 Enter a Wolt/Glovo order by hand** so stock and numbers stay whole | enter an outside order that "adds up" | yes | 60 |
| O3 | Refunds (`refund.js`: `openRefund`, `moneyBack`) | **O3 Money back**: what can be refunded today (pickup half; `dowiz-order-fsm-has-no-exit`) and what is recorded | refund a pickup order; explain why a delivery cannot yet be | yes | 60 |
| O4 | Menu (`menu.js`: categories, `openDish`, `openNewDish`, `openImport` dry-run/apply, photos, `sizeCm`, `drawRecipe`, `tasteMarkup`) | **O4a Categories and dishes**: add, edit, hide from sale, photo | put a new dish on sale with a photo | yes | 90 |
| | | **O4b Import the menu from CSV**: dry run first, then apply, retire what is missing | import a CSV and read its dry run | yes | 75 |
| | | **O4c A dish's recipe and taste profile**: lines from the supplies, derived kcal/weight/cost, the five taste axes | attach a recipe and read the derived numbers | yes | 90 |
| O5 | Stock (`stock.js`: `render`, `drawList`, `openSupply`, `openMove` received/wasted/stocktake; `bulk.js`: `openBulk`, supplies and recipes CSV) | **O5a The three movements**: came in, thrown away, counted; stranded reserves | book a delivery, a waste, a count | yes | 90 |
| | | **O5b Supplies and recipes in bulk** (F1) | import both CSVs, dry run then apply | yes | 75 |
| O6 | Couriers (`couriers.js`: `openInvite`, `openCourier`, deactivate; F5 delivery of the invite) | **O6 Hire and manage couriers**: invite code, on shift / in flight / cash held, deactivate | invite a courier and read their card | yes | 75 |
| O7 | Staff (`staff.js`: `openInvite`, `openStaff`; roles waiter / counter-manager / kitchen) | **O7 Staff and their rights** | invite a waiter; explain the three roles | yes | 60 |
| O8 | Bookings (`bookings.js`: `load`, `row`, `act`, `newBooking`) | **O8 Reservations**: the list, confirm / decline, a booking by phone | take a booking by phone and confirm one from the web | yes | 75 |
| O9 | Floor plan (`floorplan.js`: `draw`, `edit`, `save`) | **O9 Draw the floor** so the waiter's floor and the table QR codes have somewhere to be | draw four tables and save | yes | 75 |
| O10 | Table QR (`tableqr.js`: `print`, `download`) | **O10 Table codes**: print, download, what a guest sees | print the sheet | no | 45 |
| O11 | eBills till import (`ebills.js`: settings, code ↔ dish mapping, floor from the till) and fiscal sending (`fiscal.js`: arm / disarm with the typed phrase) | **O11a Read the till**: credentials, mapping every till code to a dish, the pending/unmatched lists | connect and map ten codes | yes | 90 |
| | | **O11b Fiscal sending**: what arming means legally, the phrase, the stages | explain the consequence before arming; arm on the practice venue only | yes | 60 |
| O12 | Exceptions (`exceptions.js`: `open`, `saveNumbers`, `legs`) | **O12 Voids, comps, refunds** as a report | read a week of exceptions | no | 45 |
| O13 | Customers (`customers.js`: `cardLine`, `openCard`, reveal with a written reason, reveal log, links) | **O13 Customers without spying**: masked cards, the one-time reveal, the log | reveal one contact with a reason and find it in the log | yes | 75 |
| O14 | Campaigns, promos, posts, social (`campaigns.js`: `open/edit/detail/preview`; `more.js`: `openPromos`, `openPosts`, `openSocial`) | **O14a Promo codes**: percent / fixed, window, max uses, status words | create a scheduled code | yes | 60 |
| | | **O14b Campaigns and post drafts**: consent first, preview, approve | preview a campaign; approve a post draft | yes | 75 |
| O15 | Stamp card (`stamps.js`: `mount`) | **O15 The stamp card**: N stamps, the reward, where the guest sees it | switch it on with 8 stamps | yes | 45 |
| O16 | Settings (`more.js`: `openVenue`, `openHours`, `openDelivery`, `openPayments`, `openNotifications`, `openChannels`, `openPrinter`, `openBranding`, `openFeatures`, `openIntegrations`, `openCloud`, `openMcp`, `openKeys`, `openActivation`, `openHealth`, `openAssistant`, `openPreview`, `openInbox`) | **O16a First day**: venue, hours, delivery terms, payments, the kitchen bell (Telegram / WhatsApp), the printer, activation checks | pass every activation check on the practice venue | yes | 120 |
| | | **O16b Branding, features, preview** | change the accent and see the storefront | yes | 60 |
| | | **O16c Integrations, nightly copy, agents, API keys, health** | run "check all" and read the health page | yes (probe writes) | 90 |
| | | **O16d Inbox and the assistant** | answer a WhatsApp message; ask the assistant one question | yes | 60 |

### 4.2 Waiter / till (`/room/`) — 10 modules, 10 lessons

Capabilities gate what is shown: `take_orders`, `take_payment`, `void`, `open_till` (`room/logic.js`,
measured). A lesson whose anchor is behind a capability the learner lacks is skipped by the engine's own rule.

| # | Module (file) | Functions | Lesson | Goal | Writes | ~s |
|---|---|---|---|---|---|---|
| W1 | Sign in / claim (`app.js`: `renderLogin`, `onLogin`; claim code) | | **W1 Your first sign-in**: the invite code, the password, the language and theme buttons, the sync and outbox tags | sign in and read the header | no | 45 |
| W2 | The room (`renderRoom`, `loadRoom`, `hud`) | | **W2 The room**: open tables, rounds, what is due; live / offline age | find the table with the oldest unpaid round | no | 45 |
| W3 | Open a table (`open.js`: `renderOpen`, `bindOpen`, `tableOk`) | | **W3 Open a table** | open table 4 for two | yes | 45 |
| W4 | Floor (`floor.js`: `renderFloor`, `loadFloor`, `bindFloor`; states free / booked / ordering / waiting / paying / dirty; clear) | | **W4 The floor**: the legend, tap a table, mark it cleared | seat a booked table and clear a paid one | yes | 60 |
| W5 | A round (`sheet.js`: `renderRound`, `amend`, `reasonPanel`, `sendAsk`; `menu.js`: `renderAdd`, search, unavailable) | | **W5 Take a round**: add items, quantities, send; remove or comp with a reason; "changed meanwhile — reloaded" | send a round of three and comp one item with a reason | yes | 90 |
| W6 | Guest rounds (`guest.js`: `renderGuestBar`, `answerGuest`) | | **W6 Orders from the table's QR**: confirm or reject | confirm one, reject one | yes | 45 |
| W7 | Pay (`pay.js`: `renderPay`, `bindPay`, amount / method / currency / rate, keep the change, tips) | | **W7 Take payment**: full, split, another currency at the board rate, the tip | close a table in two payments with a tip | yes | 90 |
| W8 | Transfer and move (`transfer.js`: `renderTransfer`, `renderMoveSitting`, `canTransfer`) | | **W8 Move a round or a sitting** | move a round to another table | yes | 45 |
| W9 | Till (`till.js`: `renderTillScreen`, open / close, float, counted vs expected, over / short, pay in / out) | | **W9 Open and close the till**: the float, cash in / out with a reason, the count | open with a float, close with a count | yes | 90 |
| W10 | Z report and tips (`till-view.js`: `renderTill`, `renderTips`, tips by person) | | **W10 The Z report and who gets which tip** | read yesterday's Z and the tip record | no | 60 |

### 4.3 Courier (`/courier/`) — 7 modules, 6 lessons

The courier already has a 5-step first-run tour (`welcome, shift, sheet, mic, help`). The lessons below keep
it as C1 and add the rest.

| # | Module (functions) | Lesson | Goal | Writes | ~s |
|---|---|---|---|---|---|
| C1 | Sign in / claim (`renderLogin`, `renderClaim`), shift (`setShift`, `setShiftTag`) | **C1 First shift** (= the existing tour, re-anchored) | claim the invite, open the shift | yes | 45 |
| C2 | Offers and claiming (`renderOffer`, `timeLeft`, `offerLapsed`) | **C2 Take an order**: the offer, the timer, "lapsed but still free" | take a ready order | yes | 45 |
| C3 | The run (`renderActive`, `pickedUp`, `inMaps`, `call`, `etaText`, the map, GPS tag) | **C3 The run**: picked up, in maps, call, the ETA the guest sees | complete a pickup and reach the door | yes | 60 |
| C4 | Delivery and cash (`deliver`, `renderCash`, shortfall, `renderRefused`) | **C4 At the door**: swipe delivered, count the cash, a shortfall, refused at the door | deliver with cash; record a refusal | yes | 75 |
| C5 | Offline (`drawOutbox`, `tapped`, queued / queueFull / queuedChanged) | **C5 No signal**: what is saved, what is sent later, what is refused | explain the outbox tag | no | 45 |
| C6 | Voice, ask, earnings (`startVoice`, `handleVoice`, `bindAsk`, `openEarnings`, `openHistory`) | **C6 Hands free, and your numbers**: "e mora", "u dorëzua"; the question box; today / 7 / 30 days | ask one question; read the earnings | no | 60 |

**Totals, derived:** 38 lessons, ~43 minutes of video per language, ~2 h 10 min for three; 38 tours.

---

## 5. The in-app learning engine

### 5.1 Anchors: `data-tour` in the templates

Every anchor is an attribute in the template string that renders the control, named `<module>.<thing>`:
`data-tour="orders.row"`, `data-tour="pay.method"`, `data-tour="till.close"`. The guide's `help` table rows
point at them with `at: '[data-tour="orders.row"]'`. Rules:

- **One anchor, three readers.** The tour highlights it, the capture script clicks it, the gate (§8.4) counts
  it. A renamed control breaks all three loudly, in the gate before it breaks in a lesson.
- **Anchors are stable across the design refresh** because they are semantic, not visual: the operator's
  refresh (L0) may move and restyle a control and the attribute travels with it. This is why L0 is first and L1
  is not blocked by it: an anchor added today on the old markup is the same anchor on the new one, provided
  the refresh keeps the attributes — the gate says whether it did.
- `id`s are not used as anchors: the room re-renders whole views by `innerHTML`, so the one thing that
  survives a re-render is what the template writes, and the engine already remounts on every DOM change.

### 5.2 `lib/learn.js` on top of `lib/guide.js` (not a second engine)

The brief names `public/lib/tour.js`; the tree already has the tour half of it in `guide.js`. The proposal is
one new module, `public/lib/learn.js`, that owns *lessons* and uses `createGuide` for the drawing:

```
createLearn({ app: 'owner'|'room'|'courier', lessons, guide, t, host })
  lessons: [{ id:'O1b', module:'orders', steps:[{ at:'[data-tour=orders.row]', key:'L_O1b_1', before, writes }], practice:true }]
  .open(id, step)        // runs guide.start over that lesson's steps (a lesson = a tour with its own key)
  .progress()            // { [id]: { state:'done'|'paused'|'new', step, at } } from localStorage dw_learn_<app>
  .mountEntry(...)       // the "Learn" surface: list of lessons per module, done ticks, "watch" links
  .deepLink()            // /admin/#learn=O1b  → opens that lesson (the wiki links here)
```

- **Progress per device** stays where `guide.js` already keeps it (`localStorage`, one key per lesson:
  `dw_guide_owner_O1b`) plus one index key `dw_learn_owner`; wrapped in the same try/catch the tree uses
  (`safeGet`/`safeSet` in `store/storage.js`). An owner-visible mirror (which waiter finished what) is a later
  row (L11): `POST /api/staff/learn` appending `{lesson, state}` to the venue's people table; not needed to
  ship.
- **i18n keys per step**, in the app's own `T` tables (`admin/i18n.js`, `room/i18n.js`, `courier/i18n.js`),
  named `L_<lesson>_<n>T` / `L_<lesson>_<n>` (the courier already uses `hShiftT`/`hShift`), so the vocabulary
  gate and the "one table, three languages" rule cover lesson copy like everything else. The **same strings
  are the narration script** of the video (§6.2): the wiki's text, the app's tour card and the voice read the
  same sentence, so none can drift.
- **Reduced motion:** inherited — `guide.js` uses `calm()` for scroll behaviour, `guide.css:125` drops the
  ring's transition; the lesson list adds nothing animated.
- **CSP:** nothing inline; `learn.js` is a same-origin module imported by each app's `app.js`; its stylesheet
  is `guide.css` plus a small `learn.css` linked the same way; embedded loops are same-origin `<video>` or an
  `<img>` of a WebP (§7); the room's and the courier's `sw.js` precache lists get the two files added.
- **Where the "Learn" entry lives:** console — a tile group `learn` in `more.js`'s `GROUPS` (one tile per
  role the owner can watch: owner / waiter / courier, since the owner trains staff) plus the existing "?" help
  mount in the header; room — a button in the room header beside `langBtn` (the room has no More screen);
  courier — the existing `mount` (`into:'#app', when: standing still`) grows a lesson list. Each entry also
  shows "Watch in the wiki" (§8) and, for lessons that write, "Practice" (§5.4).

### 5.3 Why extend and not replace

`guide.js` already does the four hard things (positioning off tokens, skipping hidden targets, remounting
after re-render, keyboard and focus), and it does them in the courier app today with the CSP on. A second
module would be a second list of the same thing, which the codebase's own rule forbids (`guide.js:1-12`).
What it lacks is only what a *lesson* adds over a *tour*: many of them, per module, with a list and progress.

### 5.4 Practice mode: lessons never touch real orders

The apps decide the venue from the **host** (`courier-login-venue-from-host`; `room/logic.js slugOfHost`), so
practice is not a flag inside the app; it is the same app on another host: `https://qa-durres.dowiz.org/{admin,room,courier}/`
(F7). The rules `learn.js` enforces:

1. A step marked `writes: true` runs only when `location.host` starts with `qa-`; elsewhere the card shows
   the step, the highlight and a "Practice on the training venue" button that deep-links to the same lesson on
   the QA host. **No lesson ever mutates a real venue.**
2. Read-only tours (O1a, O10, O12, W1, W2, W10, C5, C6) run on any host.
3. The QA venue's Telegram chat is the operator's (F7's CHECK), so a practised order rings nobody's kitchen;
   practice orders are placed with the `QA-LEARN` note prefix and closed by the same `ensureClosed` pattern
   the walk harness uses, on a nightly reseed (`tools/platform/qa-venue.sh`, idempotent, F7).
4. Practice credentials: the QA venue's owner / waiter / courier live in `/root/.dowiz_owner` beside the
   existing `QA_COURIER_*` entries; the lesson list on a real venue offers "open the training venue" and the
   staff member signs in there with the credential the owner gives them (the invite flow O6/O7 produces it).

---

## 6. The video pipeline

### 6.1 Layout

```
docs/learn/lessons/{owner,waiter,courier}/<id>.yaml   the source: module, anchors, steps, narration keys, practice
tools/learn/capture.mjs                               Playwright on the QA host → <id>.<lang>.webm + marks.json
tools/learn/voice.py                                  narration → <id>.<lang>.<n>.mp3 + .vtt (edge-tts / Azure / a human WAV)
tools/learn/assemble.sh                               ffmpeg: pad, concat, mix, burn, chapters, poster, loops
tools/learn/check.sh                                  the self-review (ffprobe, contact sheet, loudness, cue bounds)
tools/learn/publish.sh                                upload to R2 + write public/learn/manifest.json
tools/learn/all.sh                                    the one command; incremental by content hash
workers/api/public/learn/manifest.json                committed: lesson → files, sha256, duration, chapters, built_from
(R2) learn/<id>/<lang>/{video.mp4,subs.vtt,poster.jpg,step-<n>.mp4,step-<n>.webp}
```

Narration text is **not** in the YAML: it references the `L_<id>_<n>` keys, and `voice.py` reads them from the
three `T` tables (a 20-line Node script exports them as JSON), so the app, the wiki and the voice read one
string.

### 6.2 Capture (`tools/learn/capture.mjs <role> <id> <lang>`)

- Imports `browser()`, `watch()`, `csp()`, `creds` from `e2e/walk/_lib.mjs` (process cap 26, CSP listener,
  the same flags). `HOST` defaults to `https://qa-durres.dowiz.org`; the script **refuses any host not
  matching `/^qa-/`**, the same refusal F20's soak has.
- Context: viewport **360×640, deviceScaleFactor 2** → 720×1280 frames, `isMobile, hasTouch`, `colorScheme`
  from the lesson (light for the console, dark for the courier — the products' own defaults),
  `recordVideo { size: 720×1280 }`, `addInitScript` sets `dw_admin_lang` / `dw_room_lang` / `dw_c_lang` to
  `<lang>` (the UI text is in the language of the cut, which is why there are three captures, not one).
- Steps come from the YAML's anchors: `await p.click('[data-tour="pay.method"]')`, with each step's `before`
  (the same function the tour uses, exported from the app for the harness — or a route interception where the
  state must exist, as `capture-posts.mjs` does). After each step the script waits for the DOM to settle
  (`networkidle` or a named selector) and writes `marks.json`: `{ step, tVideoMs }` from
  `Date.now() - contextStartMs`. The promo memory's trap applies: in-points are what the sampled frames say,
  so `check.sh` tiles one frame per mark and the author looks at it once per lesson.
- A **cursor and tap ring** are drawn in-page (an `addInitScript` overlay div moved on `pointer` events),
  since a headless capture has no visible pointer. WebGL surfaces are absent from every lesson frame by
  construction (the console and the room draw none; the courier's map and sea would be blank —
  `webgl-renders-nothing-on-the-box`), so C3's map is captured with the map container replaced by the
  drawn map card the promo already has, and the wiki page says "map drawn".
- Output: `webm` (VP8, Playwright's native), converted once to a mezzanine `mp4` (libx264 crf 18).

### 6.3 Narration (`tools/learn/voice.py <id> <lang>`)

- One call per step: `edge-tts --voice sq-AL-AnilaNeural --text "$L_O1b_3" --write-media step-3.mp3 --write-subtitles step-3.vtt`
  (measured today; ~5 s each). With `AZURE_SPEECH_KEY` set the same voice names go through the Azure REST
  endpoint and return the word boundaries as events → the same VTT shape.
- A **human recording** replaces a step by dropping `step-3.wav` beside the YAML; `voice.py` then aligns the
  cue to the file's duration (whole-cue, no word timing) — good enough for a burned line per step.
- Loudness-normalised to **−16 LUFS** (`loudnorm=I=-16:TP=-1.5:LRA=11`), mono AAC 96 kbps in the final.

### 6.4 Assembly (`tools/learn/assemble.sh <id> <lang>`), all FFmpeg

1. **Timing rule:** step *n*'s footage runs from `marks[n]` to `marks[n+1]`; its narration starts at
   `marks[n]`. If the narration is longer than the footage, the last frame is **held** (`tpad=stop_mode=clone`)
   until the narration ends plus 400 ms; if shorter, the footage plays out. Deterministic, no hand editing.
2. Concat the padded segments; mix the narration track; **burn** the cues with
   `subtitles=<id>.<lang>.vtt:force_style='FontName=Manrope,FontSize=22,Outline=1,MarginV=48'` (Manrope is
   in `marketing/promo-30s/fonts/`, OFL; measured today that the `subtitles` filter renders «ë ç ї є ґ»
   correctly with a fallback face). The **un-burned `.vtt` ships beside** the file so the wiki player can lay
   another language's cues over the same cut (a Ukrainian waiter watching the Albanian UI is the real case).
3. **Chapters:** one per step, written both as MP4 metadata (`-f ffmetadata` `[CHAPTER]` blocks, `-map_metadata 1`)
   and as `chapters.json` (`{ n, title, startMs }`) for the player's step list.
4. A ten-second **intro/outro** per language (title, role, module, the dowiz mark) rendered **once** with
   Remotion from the promo's world (bone / ink / hot; `marketing/promo-30s` already renders on this box with
   `--browser-executable … --gl=swiftshader --concurrency=2`) and concatenated as a file — Remotion never
   renders lesson bodies (§6.6).
5. **Poster** = the frame at `marks[1]`, 720×1280 JPEG q=80 (~45 KB, like the landing's).
6. Outputs per lesson per language: `video.mp4` (H.264 High, 24 fps, crf 23, `+faststart`, ~1.0–1.3 Mbps →
   **~6–8 MB per 60–90 s**, derived from the landing film's measured 1 168 kbps at 1080×1920 scaled to
   720×1280), `subs.vtt`, `chapters.json`, `poster.jpg`, plus the step loops of §7.

### 6.5 Hosting

| Option | Cost | Requests | CSP | Verdict |
|---|---|---|---|---|
| **Static assets** (`public/learn/**`, like the landing film) | free; 25 MiB/file | free, unlimited | none needed (`'self'`) | fine for the **manifest, posters and WebP loops** (~40 MB); wrong for ~0.8 GB of MP4 in git |
| **R2 bucket `dowiz-learn` + custom domain `learn.dowiz.org`** | free tier 10 GB-month, 1 M class A, 10 M class B, **egress free** ([R2 pricing](https://developers.cloudflare.com/r2/pricing/)) | zero Worker requests (the memory rule: request counts bind the bill) | one line: `media-src 'self' https://learn.dowiz.org` in `_headers`, then the four-grep audit | **recommended for a public wiki** |
| **R2 behind the Worker** (`GET /api/learn/media/:key`, Bearer, Range passthrough) | same storage | one Worker request per media fetch (Range → several); 100 000/day free | `'self'` | required if the wiki is **gated** (§8.2); fine at tens of staff |
| KV `MEDIA` | 25 MiB/value, 1 000 writes/day free | edge cached | `'self'` | possible but a video is not a photo; writes/day cap makes a full re-render a two-day upload; no |

Both R2 routes need an **R2 write permission on a token this box holds** — none of the three tokens in
`dowiz-waitlist-mail-needs-email-routing` is known to carry it (UNVERIFIED; the operator checks the token's
permissions, as memory says to do before trusting a deploy). `publish.sh` uses `wrangler r2 object put` with
that token, or `rclone` against the S3 endpoint.

### 6.6 Time, measured and derived

- **Encode:** 30 s of 1080×1920 with a burned VTT, `libx264 -preset veryfast -crf 23`: **31.7 s** wall on this
  box (8 cores, measured today). At 720×1280 the same work is ~2.5× fewer pixels → ~0.4× real time. A 90 s
  lesson ≈ 40 s; 38 × 3 ≈ **1.3 h of encoding** for a full render (derived).
- **Capture:** each lesson's steps run at UI speed with settle waits; ~2 min per capture, 114 captures ≈
  **4 h**, one browser at a time under the process cap (derived). Captures are the long pole; they are also
  the part that is skipped when nothing in a lesson's inputs changed.
- **Narration:** 38 × ~6 steps × 3 ≈ 700 calls × 5 s ≈ **1 h** with edge-tts; seconds with Azure batch.
- **Remotion** is 5 min per 30 s at half scale here (promo memory): the two 10 s stingers per language are
  rendered once and cached.
- **The one command:** `tools/learn/all.sh [--lang sq,en,uk] [--only O1b,W7]` computes, per lesson and
  language, a hash of (the YAML, the three narration strings' text, the anchors' selectors, the app's JS files
  the lesson touches, the voice name) and re-runs only the stages whose inputs changed; the manifest records
  the hash as `built_from`, so the gate (§8.4) can say **which videos are stale** after a UI change without
  rendering anything. A full cold run is an overnight job (~6.5 h derived); a typical UI change re-renders
  the three cuts of two or three lessons in under fifteen minutes.

---

## 7. Interactive GIFs: short loops per step

Measured today on a 6 s slice of the landing film at 540 px wide:

| Format | Settings | Size | Encode |
|---|---|---|---|
| **MP4 loop** (`<video autoplay muted loop playsinline>`) | libx264, 24 fps, crf 26, faststart, no audio | **290 KB** | 3.4 s |
| WebP animated (`<img>`) | `libwebp_anim`, 12 fps, q 60 | 1.08 MB | 4.7 s |
| GIF | 10 fps, 360 px, 128-colour palette, Bayer dither | 2.30 MB | 3.9 s |

So: **the "GIF" is an MP4 loop** cut from the mezzanine between `marks[n]` and `marks[n+1]` (capped at 8 s),
540×960, ~50 KB/s; a WebP of the same loop at 12 fps is the fallback for contexts that cannot autoplay video
(the wiki's `<video>` carries `poster` and the WebP as `<picture>` fallback). A real GIF is 8× the MP4 for a
worse picture and is not produced. Per lesson: ~6 loops × ~300 KB ≈ 2 MB × 3 languages; the loops **are**
language-specific (the UI text is), ~230 MB in all — R2 with the videos, or static assets if only the `sq`
loops are shipped in-app.

In the app, `learn.js` shows the step's loop inside the tour card only on request ("show me") to keep the
card light; the wiki shows every loop inline under its step.

---

## 8. The wiki

### 8.1 Where it lives

`workers/api/public/wiki/` — a static shell (`index.html`, `wiki.js`, `wiki.css`, no framework, the same
tokens as the apps), served by the one Worker on **every venue host** (`<slug>.dowiz.org/wiki/`) and on the
platform host. It must be on the venue host, not only `dowiz.org`, for two reasons: `localStorage` is
per origin, so only there can it read the app's language and (if gated) the app's token; and the "open this
lesson in the app" and "practice" links are same-host. `docs/` rendered was considered and rejected: the
lessons' text is the apps' `T` tables, not Markdown in `docs/`, and a rendered `docs/` would be the second
copy the rule forbids.

### 8.2 Public or gated — the decision the operator has to make

- **Public, `X-Robots-Tag: noindex`** (recommended for the first release): the shell, `manifest.json` and the
  media on `learn.dowiz.org` are readable by anyone with the link. What they show is the QA venue with seeded
  data, and screens the promo already shows publicly. Cost: zero Worker requests; the PWA can precache the
  manifest and the posters. What is lost: nothing secret, because no lesson shows a real secret (the eBills
  password field is drawn masked; the fiscal phrase is the venue's own choice and is typed on the practice
  venue).
- **Gated:** the shell stays static and empty; `wiki.js` reads the app's JWT (`dw_rt` for the console,
  `dw_c_jwt` for the courier, the room's own key — `grep`, measured) and calls `GET /api/learn/manifest` and
  `GET /api/learn/media/:key` with `Authorization: Bearer` (the `auth::bearer` / `authenticate` path,
  `workers/api/src/auth.rs:205, 464`); the Worker streams from an **R2 binding** with `Range` passed through.
  Any principal of the venue (owner, staff, courier) may read; role filters the list. Static assets cannot be
  gated without `run_worker_first`, which is why the media does not go under `public/`. Cost: one Worker
  request per fetch; at tens of users that is noise against 100 000/day.

Either way the shell is the same; the difference is `publish.sh`'s target and one Worker route (L7/L8).

### 8.3 Structure

```
/wiki/#/<lang>/<role>                       owner | waiter | courier  (a fourth, guest, later: the storefront)
/wiki/#/<lang>/<role>/<module>              the module page: what it is for, its lessons, its anchors
/wiki/#/<lang>/<role>/<module>/<lesson>     the lesson page
```

A lesson page: title and goal (the `L_<id>_0T/0` strings); the **video** with `<track kind="subtitles">` for
all three `.vtt` (default = page language; the burned-in cues are the page language's, so a viewer of another
language gets both, which is the bilingual-staff case); the **chapter list** from `chapters.json` (click =
seek, and the step's loop plays inline under its text); **"Open this tour in the app"** (`/admin/#learn=O1b`);
**"Practice"** (the QA host deep link) for lessons that write; **"Verified against build"** = the manifest's
`built_from` short hash and the date, and a yellow line when the gate says the render is stale.

**Search:** client-side, no library (CSP `script-src 'self'`; no CDN): `wiki.js` builds an index over
titles, goals and step texts in the current language from the manifest (~40 KB), tokenises with
`Intl.Segmenter` where present and a whitespace split otherwise, and ranks by term hits; Albanian and Ukrainian
need no stemming for a corpus this small. The language switch is the apps' own (`dw_admin_lang` etc., read
and written by the same `safeGet`/`safeSet`).

**Where a videos' words come from:** the same `T` tables. The wiki has no copy of its own except the module
introductions, which live in the manifest as `M_<module>` keys — also in the `T` tables, so the vocabulary
gate sees them.

### 8.4 Staying in sync with the code: `tools/gates/learn.sh` (+ `learn.baseline`)

Runs in CI beside the ten existing gates; the shape is the tree's (`tools/gates/*.sh` + `.baseline`). Red if:

1. an anchor in the tree (`grep -rhoE 'data-tour="[^"]+"' public/{admin,room,courier}`) is referenced by no
   lesson step;
2. a lesson step references an anchor not in the tree (a renamed control), or its `before` names an export
   that no longer exists;
3. a module in `TABS`, `GROUPS`, the room's view list or the courier's screen list has no lesson;
4. a lesson's `L_<id>_<n>` keys are missing in any of the three `T` tables, or the step counts differ per
   language;
5. a lesson has no wiki page — which, since pages are generated from the manifest, means: the manifest lacks
   the lesson or any of its three cuts.

**Warn, not red** (a UI change is allowed to land before its overnight render): the manifest's `built_from`
differs from the current input hash for N lessons — printed as `stale: O1b W7 (sq,en,uk)`, which is also the
list `all.sh` would re-render. The render check (`tools/learn/check.sh`) is the second half: every published
file passes `ffprobe` (streams, duration = chapters' end ± 0.5 s), integrated loudness within −18…−14 LUFS,
every cue inside the duration, and the per-chapter contact sheet exists; a file that fails is not published.

---

## 9. Wave L — proposed rows (not written to the roadmap)

Sizes as Wave F defines them: **S** ≤ a lane-day, **M** a few, **L** a week or blocked on a third party.

| | Goal | Files it owns | CHECK | Depends on | Size |
|---|---|---|---|---|---|
| **L0** | The operator's **design refresh** of the three apps lands first, keeping every `data-tour` attribute where one exists (no row for it exists today — this row is the placeholder that makes the dependency visible) | the apps' templates and CSS (owned by that lane) | the design gate green; `learn.sh` item 2 = 0 after the refresh | operator | — |
| **L1** | **Anchors and the lesson registry**: `data-tour` on every control the 38 lessons name; `docs/learn/lessons/**.yaml`; `L_*`/`M_*` keys in the three `T` tables (sq/en/uk written, not machine-translated) | `public/admin/*.js`, `public/room/*.js`, `public/courier/app.js` (attributes only), `admin/i18n.js`, `room/i18n.js`, `courier/i18n.js`, `docs/learn/lessons/**`, `tools/gates/learn.sh` (items 1–4) | `learn.sh` items 1–4 = 0; `vocabulary.sh` unchanged; every existing e2e walk green (attributes change no behaviour) | L0 for the final positions; can start on today's markup | **M** |
| **L2** | **`lib/learn.js`** on `guide.js`: lessons, progress, the Learn entry in the three apps, deep links, practice refusal off-host, reduced-motion, CSP-clean | `public/lib/learn.js`, `lib/learn.css`, `lib/guide.js` (small: expose `show(i)` for a lesson key), `admin/more.js` (tile group), `room/app.js` (header button), `courier/app.js` (list under the existing mount), `room/sw.js`, `courier/sw.js` | on a scratch build: `/admin/#learn=O1a` opens O1a at step 0 with the ring on `[data-tour=orders.row]`; a `writes` step on `sushi-durres` shows the Practice button and calls no API (network log = 0 writes); `dw_learn_owner` records `done`; the walk harness's CSP listener fires nothing; `prefers-reduced-motion` = no transition | L1 | **M** |
| **L3** | **Practice venue** wired: `qa-venue.sh` seeds the QA venue with practice staff/courier credentials, `QA-LEARN` notes, nightly reseed; the credential file gains `QA_WAITER_*` | `tools/platform/qa-venue.sh` (F7's), `/root/.dowiz_owner` (operator) | W3 practised on `qa-durres.dowiz.org/room/` opens a table; the next morning `rebuild.stranded == []` and no `QA-LEARN` order is live | **F7** | **S** |
| **L4a** | **Owner lessons** (22): scripts in three languages, anchors, `before` functions, tour order; reviewed by a native sq and uk reader | `docs/learn/lessons/owner/*.yaml`, `admin/i18n.js`, `admin/*.js` (anchors) | every O-lesson opens as a tour on the practice venue end to end; `learn.sh` green | L1, L2, L3 | **L** |
| **L4b** | **Waiter lessons** (10) | `docs/learn/lessons/waiter/*.yaml`, `room/i18n.js`, `room/*.js` | as L4a for W1–W10, incl. capability-gated steps skipped for a plain waiter | L1, L2, L3 | **M** |
| **L4c** | **Courier lessons** (6), re-anchoring the existing 5-step tour as C1 | `docs/learn/lessons/courier/*.yaml`, `courier/i18n.js`, `courier/app.js` | as L4a for C1–C6; the existing first-run behaviour unchanged (`dw_guide_courier` still honoured) | L1, L2 | **S** |
| **L5** | **Capture + assemble + check**: `tools/learn/{capture.mjs,voice.py,assemble.sh,check.sh,all.sh}`, the two Remotion stingers, edge-tts as the default voice | `tools/learn/**`, `marketing/promo-30s/src/LearnStinger.tsx`, `docs/learn/README.md` | `all.sh --only W7 --lang sq` on this box yields `video.mp4` (720×1280, 24 fps, ≤ 1.4 Mbps), `subs.vtt` with ≥ 1 cue per step, `chapters.json` with N = steps, six step loops ≤ 400 KB each, a contact sheet; `check.sh` passes; `capture.mjs` against `sushi-durres` **refuses** (exit ≠ 0, nothing written); a second run with no change re-renders nothing (hash hit) | L3, L4b (one lesson is enough to build against) | **M** |
| **L6** | **Voices for release**: `AZURE_SPEECH_KEY` from the operator in the token file, `voice.py`'s Azure path, the human-WAV override | `tools/learn/voice.py`, `/root/.dowiz_tokens` (operator, 0600) | the same sentence through Azure and edge-tts differ only in bytes, not words; a dropped `step-3.wav` replaces the cue and the render | operator | **S** |
| **L7** | **Hosting**: R2 bucket `dowiz-learn`, custom domain `learn.dowiz.org` (public) **or** the `[[r2_buckets]]` binding + `GET /api/learn/{manifest,media/:key}` with Bearer and Range (gated); `media-src` in `_headers` only for the public route; `publish.sh` | `workers/api/wrangler.toml`, `public/_headers`, `workers/api/src/learn.rs` (gated only), `tools/learn/publish.sh` | public: `curl -sI https://learn.dowiz.org/O1a/sq/video.mp4` → 200 with `accept-ranges: bytes`, and the four CSP greps unchanged except the one intended line; gated: 401 without a token, 206 on a Range with one; either: `publish.sh` twice uploads nothing the second time | the R2 token permission (operator); L5 | **S** |
| **L8** | **The wiki shell**: `/wiki/`, manifest-driven pages per role/module/lesson, three-track player, chapters, loops, search, deep links, "verified against" line | `public/wiki/{index.html,wiki.js,wiki.css}`, `public/learn/manifest.json` | on `sushi-durres.dowiz.org/wiki/#/uk/waiter/pay/W7`: the video plays with `uk` cues selected, the sq and en tracks selectable, seven chapters seek, "Open in app" lands on `/room/#learn=W7`; the design gate green on the new surface; no CSP violation in the walk harness; search for «bakshish» finds W7 in sq | L5, L7 | **M** |
| **L9** | **The gate**: `learn.sh` item 5 and the stale-render warning; `check.sh` in CI on the manifest | `tools/gates/learn.sh`, `learn.baseline`, `.github/workflows/*` (or the tree's CI entry) | renaming one `data-tour` in a scratch build turns the gate red naming the lesson; changing one narration string turns it yellow naming the three cuts | L1, L5, L8 | **S** |
| **L10** | **The full render and publish** of 38 × 3 on this box, overnight, from `all.sh`; the manifest committed | `public/learn/manifest.json` | 114 cuts pass `check.sh`; the wiki lists 38 lessons in each language with no yellow line; the contact sheets reviewed once by a human | L4a–c, L5, L6, L7 | **S** (compute) |
| **L11** | *(optional)* **Progress the owner can see**: `POST /api/staff/learn`, a column in the Staff pane | `workers/api/src/services/staff/learn.rs`, `admin/staff.js`, `lib/learn.js` | a waiter finishing W7 on their phone shows as done on the owner's Staff card within one poll; nothing is written for a guest | L2, L4b | **M** |

**Order.** L0 (operator) ‖ L1 → L2 → L4c (smallest, proves the engine on the app that already has a tour)
→ L3 (needs F7) → L4b → L5 (built against W7) → L6 + L7 (operator inputs, in parallel) → L4a (the long
one, can overlap L5–L7) → L8 → L9 → L10. L11 when the owner asks for it.

---

## 10. What the operator provides or decides

1. **The design refresh (L0)** — or the word that L1 may proceed on today's markup and re-anchor after.
2. **F7 landed** (the QA venue); every writing lesson and every capture waits on it.
3. **`AZURE_SPEECH_KEY`** (F0 tier is free) for the release renders — or accept edge-tts for release too,
   knowing it is an uncontracted endpoint.
4. **An R2 write permission** on a token this box holds, and the choice **public + noindex** vs **gated**
   (§8.2); the public route also needs `learn.dowiz.org` attached to the bucket in the dashboard.
5. **A native Albanian and a native Ukrainian reader** for the 38 scripts before L10 — the strings are the
   product's copy; a machine translation shipped as training material would teach the wrong words.
6. Whether a **guest** track (the storefront: ordering, tracking, booking, the stamp card) is a fifth wave
   row; it is not in the brief and not in §4.

---

## 11. Not verified

- Whether the Azure REST endpoint returns word boundaries for `sq-AL` (edge-tts did today; the key path is
  UNVERIFIED until L6).
- Google Cloud TTS standard `uk-UA` voices (the list page truncates under the fetcher); irrelevant to the
  recommendation.
- MMS-TTS `sqi` on this box (torch on aarch64/py3.14) — not needed unless the operator wants offline Albanian
  and accepts CC-BY-NC.
- Which of the three Cloudflare tokens, if any, carries R2 write; and the account's R2 status (never
  created — `wrangler.toml` has no binding).
- The per-version *total* static-asset size limit (the docs state per-file and file-count limits only); the
  plan does not rely on it because the MP4s do not go under `public/`.
- OpenMontage's tool registry envelope with zero keys (`registry.support_envelope()`) was not executed: it
  needs its 13-package environment installed, and §1.3 is decided by files that were read, not by the
  envelope.

---

## Sources

- OpenMontage: [github.com/calesthio/OpenMontage](https://github.com/calesthio/OpenMontage) (cloned, `08e2151f`), `README.md`,
  `docs/PROVIDERS.md`, `pipeline_defs/screen-demo.yaml`, `tools/capture/screen_recorder.py`, `tools/audio/piper_tts.py`,
  `tools/subtitle/subtitle_gen.py`, `tools/analysis/transcriber.py`, `tools/video/video_compose.py`, `Makefile`, `LICENSE`;
  search context: [visionstory.ai](https://www.visionstory.ai/open-source/openmontage),
  [scriptbyai.com](https://www.scriptbyai.com/open-ai-video-production-agent/),
  [developersdigest.tech](https://www.developersdigest.tech/blog/openmontage-agentic-video-production).
- Voices: `edge-tts` (`pip install --target`, `--list-voices`, three syntheses — measured);
  [Azure Speech pricing](https://azure.microsoft.com/en-us/pricing/details/cognitive-services/speech-services/);
  [Azure 11 new languages](https://techcommunity.microsoft.com/blog/azure-ai-foundry-blog/11-new-languages-and-variants-and-more-voices-are-added-to-azure%E2%80%99s-neural-text-t/3541770);
  [json2video Azure voice list (sq-AL-AnilaNeural)](https://json2video.com/ai-voices/azure/voices/sq-al-anilaneural/);
  [Piper VOICES.md](https://raw.githubusercontent.com/rhasspy/piper/master/VOICES.md);
  [piper-voices uk_UA](https://huggingface.co/rhasspy/piper-voices/tree/main/uk/uk_UA) and the
  [ukrainian_tts MODEL_CARD](https://huggingface.co/rhasspy/piper-voices/raw/main/uk/uk_UA/ukrainian_tts/medium/MODEL_CARD);
  [facebook/mms-tts-sqi](https://huggingface.co/facebook/mms-tts-sqi);
  [ElevenLabs languages](https://elevenlabs.io/docs/help-center/other/what-languages-do-you-support);
  [Google Chirp 3 HD languages](https://docs.cloud.google.com/text-to-speech/docs/chirp3-hd);
  [Kokoro / local TTS survey](https://localaimaster.com/blog/best-local-tts-models).
- Cloudflare: [static assets billing](https://developers.cloudflare.com/workers/static-assets/billing-and-limitations/),
  [platform limits](https://developers.cloudflare.com/workers/platform/limits/),
  [KV limits](https://developers.cloudflare.com/kv/platform/limits/), [R2 pricing](https://developers.cloudflare.com/r2/pricing/).
- Tree: `workers/api/public/lib/guide.js`, `lib/guide.css`, `courier/app.js:1200-1219`, `admin/app.js:15`,
  `admin/more.js` (`GROUPS`), `room/app.js:175-187`, `room/logic.js`, the three `i18n.js`, `public/_headers`,
  `workers/api/wrangler.toml`, `workers/api/src/lib.rs:176-244`, `workers/api/src/auth.rs`,
  `e2e/walk/_lib.mjs`, `marketing/promo-30s/README.md` and `capture/capture-surfaces.mjs`,
  `public/platform/video/`, `docs/design/ROADMAP-2026-09-22.md` §Wave F.
- Memory notes applied: `webgl-renders-nothing-on-the-box`, `dowiz-promo-30s-remotion-2026-09-19`,
  `dowiz-csp-blocked-every-stylesheet`, `courier-login-venue-from-host`, `dowiz-order-fsm-has-no-exit`,
  `dowiz-hub-unit-costs`, `one-agent-one-worker`, `status-code-is-not-working`.
