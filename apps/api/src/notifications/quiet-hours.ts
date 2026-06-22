/**
 * Timezone-aware quiet-hours evaluation for the notification dispatcher (BR-6 / BR-14).
 *
 * Pure + deterministic given `now`: computes whether the current instant falls inside a
 * target's quiet window, interpreted in the location's timezone, and how long until the
 * window ends (for `held` deferred-deliver). Handles overnight wrap-around (from > to),
 * disabled windows (null / from === to), and missing/invalid timezone (falls back to a
 * default and flags it so the caller can audit 'quiet_tz_fallback').
 */

export interface QuietWindow {
  from: number; // local hour [0,23] inclusive — window start
  to: number;   // local hour [0,23] exclusive — window end
}

export interface QuietDecision {
  quiet: boolean;
  tzFallback: boolean; // timezone was missing/invalid → default used
  secondsUntilEnd: number; // seconds from `now` until the window's `to` hour (>=60 when quiet, else 0)
}

export const DEFAULT_TIMEZONE = 'Europe/Tirane';

function localHourMinute(now: Date, tz: string): { hour: number; minute: number } {
  // Throws RangeError for an invalid timezone — caller catches and falls back.
  const parts = new Intl.DateTimeFormat('en-GB', {
    timeZone: tz,
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).formatToParts(now);
  const hour = Number(parts.find((p) => p.type === 'hour')?.value);
  const minute = Number(parts.find((p) => p.type === 'minute')?.value);
  if (Number.isNaN(hour) || Number.isNaN(minute)) throw new Error('unparseable local time');
  return { hour: hour === 24 ? 0 : hour, minute }; // some ICU builds emit 24 for midnight
}

export function evaluateQuietHours(
  now: Date,
  timezone: string | null | undefined,
  window: QuietWindow | null | undefined,
): QuietDecision {
  if (!window) return { quiet: false, tzFallback: false, secondsUntilEnd: 0 };
  const { from, to } = window;
  if (from === to) return { quiet: false, tzFallback: false, secondsUntilEnd: 0 };

  let tz = timezone || DEFAULT_TIMEZONE;
  let tzFallback = !timezone;
  let local: { hour: number; minute: number };
  try {
    local = localHourMinute(now, tz);
  } catch {
    tz = DEFAULT_TIMEZONE;
    tzFallback = true;
    local = localHourMinute(now, tz);
  }

  const h = local.hour;
  const quiet = from < to ? h >= from && h < to : h >= from || h < to;
  if (!quiet) return { quiet: false, tzFallback, secondsUntilEnd: 0 };

  // Seconds until the next local occurrence of the `to` hour.
  let hoursUntil = (to - h + 24) % 24;
  if (hoursUntil === 0) hoursUntil = 24;
  const secondsUntilEnd = Math.max(60, hoursUntil * 3600 - local.minute * 60);
  return { quiet: true, tzFallback, secondsUntilEnd };
}
