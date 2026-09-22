//! PURE. Where a venue is, and when it is open — checked before either is
//! written over the one the venue already had.

/// A coordinate pair, or the refusal in the owner's own words.
///
/// REFUSED, NOT CLAMPED. A latitude of 91 is a bug in whatever sent it, and
/// clamping it to 90 puts the venue at the North Pole with no error anybody
/// will ever see. `NaN` is refused by the same test, because a range check on
/// a float answers false for it.
///
/// BOTH OR NEITHER. A latitude without a longitude is not a place, and half a
/// coordinate written over a good one loses the venue's position.
pub fn coords_ok(lat: Option<f64>, lng: Option<f64>) -> Result<(), &'static str> {
    if let Some(v) = lat {
        if !(-90.0..=90.0).contains(&v) {
            return Err("a latitude is between -90 and 90");
        }
    }
    if let Some(v) = lng {
        if !(-180.0..=180.0).contains(&v) {
            return Err("a longitude is between -180 and 180");
        }
    }
    if lat.is_some() != lng.is_some() {
        return Err("a latitude without a longitude is not a place");
    }
    Ok(())
}

/// A week of opening windows, in minutes since local midnight.
///
/// A WINDOW THAT CLOSES BEFORE IT OPENS IS A LATE-NIGHT WINDOW, not an error:
/// `dowiz_hub::hours::Window` reads `close < open` as wrapping past midnight,
/// and a venue serving 22:00 to 02:00 says exactly that. Refusing it here
/// would close every late kitchen on the platform, so the rule is written down
/// and tested rather than left to whoever reads the validator next.
///
/// A ZERO-LENGTH WINDOW IS REFUSED, because `open == close` is the one pair
/// that means nothing at all: not a wrap, not a span, and it would read as
/// closed on a day the owner had just filled in.
pub fn week_ok(days: &[Vec<(i64, i64)>]) -> Result<(), &'static str> {
    if days.len() != 7 {
        return Err("a week has seven days");
    }
    for d in days {
        for (open, close) in d {
            // `open` stops one minute short of 1440: a window that starts at
            // midnight-tomorrow starts on the next day's row instead.
            if !(0..1440).contains(open) || !(0..=1440).contains(close) {
                return Err("a window is minutes from 0 to 1440");
            }
            if open == close {
                return Err("a window of zero length is not a window");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// REFUSED, NOT CLAMPED: a venue quietly moved to the pole is worse than a
    /// refusal somebody has to read.
    #[test]
    fn a_coordinate_out_of_range_is_refused() {
        assert_eq!(coords_ok(Some(91.0), Some(0.0)), Err("a latitude is between -90 and 90"));
        assert_eq!(coords_ok(Some(-90.1), Some(0.0)), Err("a latitude is between -90 and 90"));
        assert_eq!(coords_ok(Some(0.0), Some(180.1)), Err("a longitude is between -180 and 180"));
        assert!(coords_ok(Some(90.0), Some(180.0)).is_ok(), "the edges are on the map");
        assert!(coords_ok(Some(41.3), Some(19.8)).is_ok(), "Tirane");
    }

    /// A RANGE CHECK ON A FLOAT ANSWERS FALSE FOR `NaN`, which is the right
    /// answer and is the reason the check is written as a range rather than as
    /// two comparisons somebody might later invert.
    #[test]
    fn nan_is_not_a_coordinate() {
        assert!(coords_ok(Some(f64::NAN), Some(0.0)).is_err());
        assert!(coords_ok(Some(0.0), Some(f64::NAN)).is_err());
        assert!(coords_ok(Some(f64::INFINITY), Some(0.0)).is_err());
    }

    /// HALF A COORDINATE WRITTEN OVER A GOOD ONE LOSES THE VENUE'S POSITION.
    #[test]
    fn a_latitude_without_a_longitude_is_not_a_place() {
        assert_eq!(
            coords_ok(Some(41.3), None),
            Err("a latitude without a longitude is not a place")
        );
        assert_eq!(
            coords_ok(None, Some(19.8)),
            Err("a latitude without a longitude is not a place")
        );
        assert!(coords_ok(None, None).is_ok(), "an address-only update touches neither");
    }

    /// A WINDOW THAT CLOSES BEFORE IT OPENS IS A LATE-NIGHT WINDOW.
    /// `hours::Window` reads `close < open` as wrapping past midnight;
    /// refusing it here would close every late kitchen on the platform.
    #[test]
    fn a_window_that_wraps_past_midnight_is_accepted() {
        let mut week = vec![vec![]; 7];
        week[4] = vec![(1320, 120)]; // Friday 22:00 -> 02:00
        assert!(week_ok(&week).is_ok());
    }

    /// AND IT REALLY DOES WRAP: the rule this validator defers to is the
    /// hub's, so the test asks the hub rather than restating the belief.
    #[test]
    fn the_hub_agrees_that_such_a_window_covers_after_midnight() {
        let sched = dowiz_hub::hours::from_json(
            r#"[[],[],[],[],[{"open":1320,"close":120}],[],[]]"#,
        );
        assert!(sched.is_open_at(4, 1380), "Friday 23:00");
        assert!(sched.is_open_at(5, 60), "Saturday 01:00, on Friday's window");
        assert!(!sched.is_open_at(5, 180), "Saturday 03:00 is shut");
    }

    /// `open == close` is the one pair that means nothing at all: not a wrap,
    /// not a span, and it would read as closed on a day just filled in.
    #[test]
    fn a_zero_length_window_is_refused() {
        let mut week = vec![vec![]; 7];
        week[0] = vec![(600, 600)];
        assert_eq!(week_ok(&week), Err("a window of zero length is not a window"));
    }

    /// MINUTES, AND THE TWO ENDS ARE NOT THE SAME RANGE. A window may CLOSE at
    /// 1440 (midnight) but may not OPEN there — that window starts on the next
    /// day's row.
    #[test]
    fn the_minute_range_is_checked_at_both_ends() {
        let day = |w: Vec<(i64, i64)>| {
            let mut week = vec![vec![]; 7];
            week[0] = w;
            week_ok(&week)
        };
        assert!(day(vec![(0, 1440)]).is_ok(), "open all day");
        assert_eq!(day(vec![(1440, 60)]), Err("a window is minutes from 0 to 1440"));
        assert_eq!(day(vec![(-1, 60)]), Err("a window is minutes from 0 to 1440"));
        assert_eq!(day(vec![(60, 1441)]), Err("a window is minutes from 0 to 1440"));
    }

    /// A WEEK HAS SEVEN DAYS. A six-day week would silently shift every day
    /// after the gap onto the wrong weekday.
    #[test]
    fn a_week_is_exactly_seven_days() {
        assert_eq!(week_ok(&[]), Err("a week has seven days"));
        assert_eq!(week_ok(&vec![vec![]; 6]), Err("a week has seven days"));
        assert_eq!(week_ok(&vec![vec![]; 8]), Err("a week has seven days"));
        assert!(week_ok(&vec![vec![]; 7]).is_ok(), "a venue closed all week is a venue");
    }
}
