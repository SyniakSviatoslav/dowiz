# Lessons: one YAML per lesson, read by the app, the recorder and the gate

Plan: `docs/design/BLUEPRINT-LEARNING-VIDEOS-WIKI-2026-09-24.md` §4–§9. Operator decisions of
2026-09-24 override it: **no voice** (the videos are silent, with sq/en/uk subtitle tracks and
burned-in step titles), recorded on the **real venue** `dubin-sushi.dowiz.org`, hosted **behind
staff/owner sign-in** through a Worker R2 binding (bucket `dowiz-learn`).

## Layout

```
docs/learn/lessons/<role>/<id>.yaml      THE source. role = owner | waiter | courier | guest
docs/learn/anchors-<app>.txt             one line per data-tour anchor: "<id> <file>:<line>"
tools/learn/build-lessons.mjs            YAML -> workers/api/public/learn/lessons.json (committed)
tools/learn/yaml-lite.mjs                the YAML subset the lessons use (no package needed)
workers/api/public/lib/learn.js          the in-app engine, on top of lib/guide.js
workers/api/public/lib/learn.css         the lesson list's sheet
tools/gates/learn.sh (+ .baseline, .prove.sh)   lessons and anchors cannot drift apart
```

## A lesson file

```yaml
id: C2                  # = the file name; <Letter><number>[a-z]
role: courier           # = the folder
module: offer
order: 2                # position in the app's list
covers: [waiting, pickList, offer]   # the app modules this lesson teaches (gate item 3)
title: { sq, en, uk }   # written as three indented lines, every language required
goal:  { sq, en, uk }
steps:
  - key: 1              # optional; C1 uses the first-run tour's keys (welcome, shift, ...)
    anchor: shift.open  # the data-tour id, or `none` for a card with no control (a welcome)
    pending: no         # yes = the markup does not carry this anchor yet
    writes: yes         # the learner's action changes real data
    action:             # what the recorder does
      do: click         # click | type | wait   (no anchor -> wait only; type -> value required)
      selector: '[data-tour="shift.open"]'   # must be exactly the anchor's selector
      value: "..."      # for type
    title:   { sq, en, uk }   # the tour card's heading = the burned-in step title
    caption: { sq, en, uk }   # the tour card's text = the subtitle cue
```

Plain scalars stay strings; `yes/no/true/false` and numbers are typed by the build, which refuses
anything it cannot read and names the file and field.

## Editing

1. Edit the YAML.
2. `node tools/learn/build-lessons.mjs` (writes `lessons.json`; `--check` refuses a stale one).
3. `sh tools/gates/learn.sh` — red when a lesson names an anchor the markup no longer has, when
   the markup has an anchor no lesson names, when an app module (console tabs and More tiles, the
   room's views, the courier's screens) is covered by no lesson, or when `lessons.json` is stale or
   invalid. `pending` anchors are printed, not counted; one that has appeared is printed as
   "resolved: drop pending".
4. `node --test workers/api/public/learn/ workers/api/public/lib/learn.test.mjs workers/api/public/courier/`.

A renamed control therefore fails the gate first, before a tour rings nothing or a recording clicks
a blank screen. Proof: `sh tools/gates/learn.prove.sh`.

## In the app

`createLearn({ role, lessons, lang, createGuide, toast, words, guideWords })` from `/lib/learn.js`:
`renderList()` + `bindList(root, onPick)` draw the list (title, goal, steps, done/paused/new, a
"changes real data" mark, "Watch the video" -> `/wiki/#/<lang>/<role>/<id>`); `open(id, step)` runs
the lesson as a guide tour keyed `dw_guide_<app>_<id>`; `deepLink(location.hash)` opens
`#learn=<id>` or `#learn=<id>/<step>` (1-based); progress is `dw_learn_<app>` in localStorage,
wrapped so blocked storage is an empty list. Every step is `soft`: a control on another screen is
explained in the middle of the screen instead of being skipped.

The courier app is wired (Learn button on the offline and waiting screens; `#learn=` on boot; C1 is
the existing first-run tour, same keys, same words — `courier/learn.test.mjs` holds them equal —
and `dw_guide_courier` is still honoured). The console, the room and the storefront get their entry
points from their own lanes.

## For the recorder

Drive the steps in order: `click` / `type value` / `wait` on `action.selector`, one capture per
language (set `dw_admin_lang` / `dw_room_lang` / `dw_c_lang` / `dw_lang`), and burn `title` in as
the step's heading and `caption` as its cue; ship the three captions as `.vtt` tracks. Steps with
`writes: yes` change the real venue when performed: stage that state (route interception, or an
order placed for the recording and closed after) rather than letting a recording mutate live orders.
Steps with `pending: yes` have no control to drive yet — skip them and say so.

## Counts (2026-09-24)

| role | lessons | steps | pending |
|---|---|---|---|
| owner (console) | 34 | 373 | 1 (`more.tile.learn`, waits on the console's Learn tile) |
| waiter (room) | 10 | 74 | 1 (`hud.learn`, waits on the room's Learn button) |
| courier | 6 | 34 | 0 |
| guest (storefront) | 6 | 63 | 0 |

Every lesson and every step carries all three languages (the build refuses one that does not).

The blueprint's "38 lessons" was an arithmetic slip: its own §4 tables list 26 owner + 10 waiter + 6
courier = 42. The owner track then grew to 34 to cover every anchor the console carries without a
lesson longer than ~14 steps: O4d, O5c, O13b, O14c, O16e, O16f, O16h, O16i were split out of the long
ones. The guest track (not in §4) is G1 browse, G2 basket, G3 where and when, G3b pay, G4 track,
G5 book. Every lesson file stays under 300 lines.
