// What a lesson's FILM is made from, and its hash.
//
// The films are recorded in ONE language (operator 2026-09-26: "training videos only 1 version
// with english subtitles"), so a film is stale only when something it SHOWS changed: the lesson's
// id and role, and per step its anchor, action, pending/writes flags and the English title and
// caption. A new Russian (or any other) string in the YAML changes nothing on screen and must not
// make 64 films due again. The same canonical form is computed by tools/gates/learn.sh item 5 (in
// Python) from workers/api/public/learn/lessons.json -- keep the two in step: explicit key order,
// compact JSON, non-ASCII kept as is.
import { createHash } from 'node:crypto';

export const FILM_LANG = 'en';

/// The part of a normalized lesson (build-lessons.mjs `normalize`, or a lessons.json entry)
/// that a film in `lang` shows.
export function filmSource(lesson, lang = FILM_LANG) {
  return {
    id: lesson.id, role: lesson.role, lang, title: lesson.title[lang],
    steps: lesson.steps.map(s => ({
      n: s.n, key: s.key, anchor: s.anchor, pending: s.pending, writes: s.writes,
      do: s.action.do, selector: s.action.selector, value: s.action.value ?? null,
      title: s.title[lang], caption: s.caption[lang],
    })),
  };
}

/// sha256 hex of the film's source.
export const filmSha = (lesson, lang = FILM_LANG) =>
  createHash('sha256').update(JSON.stringify(filmSource(lesson, lang))).digest('hex');
