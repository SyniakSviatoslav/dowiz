//! When the venue is open.
//!
//! WHY THIS IS NOT A BOOLEAN. Today a venue's `status` is a flag the owner
//! flips, which means somebody has to remember at eleven at night and again at
//! eleven in the morning. The one they forget is the closing one, and the
//! consequence is orders arriving at a dark kitchen -- a customer waiting for
//! food nobody is making.
//!
//! So the flag becomes DERIVED: a weekly schedule, plus a manual pause that can
//! override it downwards. The owner can always close early; they cannot
//! accidentally stay open.
//!
//! INTEGER MINUTES SINCE MIDNIGHT, local to the venue. No floats, no date
//! library, and no timezone database -- one venue sits in one place, and its
//! offset is a single configured number. A venue that moves country has bigger
//! problems than this module.

use crate::minijson::int_field;

/// Minutes since local midnight, 0..1440.
pub type Minute = i64;

pub const DAY: Minute = 24 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub open: Minute,
    pub close: Minute,
}

impl Window {
    /// Does this window cover midnight?
    ///
    /// A kitchen open 18:00-02:00 is the normal case for the venue this is
    /// built for, and treating close < open as invalid would refuse half the
    /// restaurants in the country.
    pub fn wraps(&self) -> bool {
        self.close <= self.open
    }

    /// Is `m` inside this window, on the day the window belongs to?
    ///
    /// For a wrapping window this is only the part BEFORE midnight; the part
    /// after belongs to the next day and is handled by the schedule, which
    /// looks at yesterday too.
    fn covers_same_day(&self, m: Minute) -> bool {
        if self.wraps() {
            m >= self.open
        } else {
            m >= self.open && m < self.close
        }
    }

    /// The part of a wrapping window that falls on the following day.
    fn covers_spillover(&self, m: Minute) -> bool {
        self.wraps() && m < self.close
    }
}

/// Seven days, starting Monday. A day with no windows is a closing day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schedule {
    pub days: [Vec<Window>; 7],
}

impl Schedule {
    pub fn is_empty(&self) -> bool {
        self.days.iter().all(|d| d.is_empty())
    }

    /// Open at this instant?
    ///
    /// `weekday` is 0 for Monday. Yesterday is consulted as well, because a
    /// window that opened at 18:00 yesterday and closes at 02:00 is what makes
    /// "are we open at half past midnight" a question about yesterday.
    pub fn is_open_at(&self, weekday: usize, minute: Minute) -> bool {
        let today = weekday % 7;
        let yesterday = (today + 6) % 7;
        self.days[today].iter().any(|w| w.covers_same_day(minute))
            || self.days[yesterday].iter().any(|w| w.covers_spillover(minute))
    }

    /// When the venue next opens, as (weekday, minute), or `None` if it never
    /// does. Used to tell a customer WHEN rather than only that it is shut.
    pub fn next_open(&self, weekday: usize, minute: Minute) -> Option<(usize, Minute)> {
        if self.is_empty() {
            return None;
        }
        // Today's remaining windows first, then the next seven days.
        for ahead in 0..8 {
            let d = (weekday + ahead) % 7;
            let mut opens: Vec<Minute> = self.days[d].iter().map(|w| w.open).collect();
            opens.sort_unstable();
            for o in opens {
                if ahead > 0 || o > minute {
                    return Some((d, o));
                }
            }
        }
        None
    }
}

/// Parse `[[{"open":660,"close":1380}], [], ...]` — seven arrays of windows,
/// in minutes since midnight.
///
/// Minutes rather than "11:00" strings because the comparison is arithmetic and
/// a string would have to be parsed at every check. The owner surface converts.
pub fn from_json(value: &str) -> Schedule {
    let mut sched = Schedule::default();
    let bytes = value.as_bytes();
    let (mut depth, mut day, mut start, mut in_str) = (0i32, 0usize, 0usize, false);
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' if in_str => i += 1,
            b'"' => in_str = !in_str,
            b'[' if !in_str => {
                depth += 1;
                if depth == 2 {
                    start = i;
                }
            }
            b']' if !in_str => {
                if depth == 2 && day < 7 {
                    for chunk in value[start..=i].split('{').skip(1) {
                        let (Some(o), Some(c)) = (int_field(chunk, "open"), int_field(chunk, "close"))
                        else {
                            continue;
                        };
                        // A window outside the day, or one of zero length, is
                        // not a window. Skipped rather than clamped: a clamped
                        // typo silently opens the venue at a time nobody chose.
                        if (0..DAY).contains(&o) && (0..=DAY).contains(&c) && o != c {
                            sched.days[day].push(Window { open: o, close: c });
                        }
                    }
                    day += 1;
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    sched
}

/// Weekday (0 = Monday) and minute-of-day, from epoch milliseconds and the
/// venue's offset.
///
/// 1 January 1970 was a Thursday, which is why the epoch day is shifted by 3 to
/// land Monday on zero.
pub fn local_now(now_ms: i64, tz_offset_minutes: i64) -> (usize, Minute) {
    let local = now_ms + tz_offset_minutes * 60_000;
    let day_ms = 24 * 60 * 60 * 1000;
    let days = local.div_euclid(day_ms);
    let minute = local.rem_euclid(day_ms) / 60_000;
    let weekday = (days + 3).rem_euclid(7) as usize;
    (weekday, minute)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(o: Minute, c: Minute) -> Window {
        Window { open: o, close: c }
    }
    const H: Minute = 60;

    fn weekdays_11_to_23() -> Schedule {
        let mut s = Schedule::default();
        for d in 0..5 {
            s.days[d].push(w(11 * H, 23 * H));
        }
        s
    }

    #[test]
    fn an_ordinary_day_opens_and_closes() {
        let s = weekdays_11_to_23();
        assert!(!s.is_open_at(0, 10 * H + 59));
        assert!(s.is_open_at(0, 11 * H), "open at exactly opening time");
        assert!(s.is_open_at(0, 22 * H + 59));
        assert!(!s.is_open_at(0, 23 * H), "shut at exactly closing time");
        // Saturday has no windows.
        assert!(!s.is_open_at(5, 12 * H));
    }

    /// The case that makes this more than a comparison: a kitchen open until
    /// two in the morning is open at half past midnight, and that is a question
    /// about YESTERDAY.
    #[test]
    fn a_window_past_midnight_stays_open_into_the_next_day() {
        let mut s = Schedule::default();
        s.days[4].push(w(18 * H, 2 * H)); // Friday 18:00 → Saturday 02:00
        assert!(s.is_open_at(4, 23 * H), "Friday night");
        assert!(s.is_open_at(5, 1 * H), "Saturday at one in the morning");
        assert!(!s.is_open_at(5, 3 * H), "and shut by three");
        // Saturday itself has no window of its own.
        assert!(!s.is_open_at(5, 20 * H));
        // Sunday must not inherit it.
        assert!(!s.is_open_at(6, 1 * H));
    }

    #[test]
    fn two_windows_in_one_day_leave_a_gap() {
        let mut s = Schedule::default();
        s.days[0].push(w(11 * H, 15 * H));
        s.days[0].push(w(18 * H, 23 * H));
        assert!(s.is_open_at(0, 12 * H));
        assert!(!s.is_open_at(0, 16 * H), "the afternoon gap");
        assert!(s.is_open_at(0, 19 * H));
    }

    /// An empty schedule means "no schedule", not "never open" -- a venue that
    /// has not set hours must keep working on its manual flag.
    #[test]
    fn an_empty_schedule_is_no_schedule() {
        let s = Schedule::default();
        assert!(s.is_empty());
        assert_eq!(s.next_open(0, 0), None);
    }

    #[test]
    fn next_open_says_when() {
        let s = weekdays_11_to_23();
        // Monday morning, before opening.
        assert_eq!(s.next_open(0, 9 * H), Some((0, 11 * H)));
        // Monday night, after closing → Tuesday.
        assert_eq!(s.next_open(0, 23 * H + 30), Some((1, 11 * H)));
        // Friday night → Monday, skipping the weekend.
        assert_eq!(s.next_open(4, 23 * H + 30), Some((0, 11 * H)));
        // Saturday → Monday.
        assert_eq!(s.next_open(5, 12 * H), Some((0, 11 * H)));
    }

    #[test]
    fn a_schedule_reads_back_out_of_json() {
        let json = r#"[[{"open":660,"close":1380}],[],[{"open":660,"close":900},{"open":1080,"close":1380}],[],[],[],[]]"#;
        let s = from_json(json);
        assert_eq!(s.days[0], vec![w(660, 1380)]);
        assert!(s.days[1].is_empty());
        assert_eq!(s.days[2].len(), 2);
        assert!(s.days[6].is_empty());
    }

    /// A typo must not silently open the venue at a time nobody chose.
    #[test]
    fn impossible_windows_are_skipped_not_clamped() {
        let s = from_json(r#"[[{"open":-60,"close":600},{"open":660,"close":660},{"open":2000,"close":2100},{"open":600,"close":900}]]"#);
        assert_eq!(s.days[0], vec![w(600, 900)], "only the real one survives");
        assert!(from_json("nonsense").is_empty());
        assert!(from_json("[]").is_empty());
        assert!(from_json(r#"[[{"open":600}]]"#).is_empty(), "half a window is not one");
    }

    /// Eight days of data must not write past the array.
    #[test]
    fn more_than_seven_days_is_ignored_rather_than_fatal() {
        let s = from_json(r#"[[],[],[],[],[],[],[],[{"open":0,"close":100}]]"#);
        assert!(s.is_empty());
    }

    /// 1 January 1970 was a Thursday; the shift is what lands Monday on zero.
    #[test]
    fn the_weekday_maths_is_right() {
        assert_eq!(local_now(0, 0).0, 3, "the epoch is a Thursday");
        let day = 24 * 60 * 60 * 1000i64;
        assert_eq!(local_now(4 * day, 0).0, 0, "four days later is a Monday");
        // Minutes of the day, and the offset moving them.
        assert_eq!(local_now(0, 0).1, 0);
        assert_eq!(local_now(90 * 60_000, 0).1, 90);
        assert_eq!(local_now(0, 120).1, 120, "UTC+2 makes the epoch 02:00 locally");
        // An offset that crosses midnight backwards must not go negative.
        let (d, m) = local_now(60 * 60_000, -120);
        assert_eq!(m, 23 * 60, "23:00 the previous day");
        assert_eq!(d, 2, "which is a Wednesday");
    }
}
