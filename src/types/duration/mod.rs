use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::str::FromStr;
use temporal_rs::{
    Duration as TemporalDuration,
    options::{DifferenceSettings, RelativeTo, RoundingOptions, ToStringRoundingOptions, Unit},
};

use crate::provider::TZ_PROVIDER;
use crate::types::plain_datetime::PlainDateTime;
use crate::types::zoned_datetime::ZonedDateTime;

// ---------------------------------------------------------------------------
// Storage type
//
// A Duration is a vector of calendar and time components with no implicit
// normalization. Every field is stored independently at full precision.
//
// The sign of all non-zero fields is uniform (Temporal validity rule).
// Fields are stored as signed values — matching the public temporal_rs
// accessor types exactly — so reconstruction is a direct pass-through.
//
//   years .. nanoseconds  – signed component values
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
pub struct Duration {
    years: i64,
    months: i64,
    weeks: i64,
    days: i64,
    hours: i64,
    minutes: i64,
    seconds: i64,
    milliseconds: i64,
    microseconds: i128,
    nanoseconds: i128,
}

// ---------------------------------------------------------------------------
// Text in / out
// ---------------------------------------------------------------------------

impl InOutFuncs for Duration {
    /// Parse an ISO 8601 duration string into a `Duration` datum.
    ///
    /// Example inputs:
    ///   `P1Y2M3DT4H5M6S`
    ///   `PT0.000000001S`
    ///   `-P1Y`
    fn input(input: &CStr) -> Self {
        let s = input.to_str().unwrap_or_else(|_| error!("duration input is not valid UTF-8"));

        let d = TemporalDuration::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid duration \"{s}\": {e}"));

        Self {
            years: d.years(),
            months: d.months(),
            weeks: d.weeks(),
            days: d.days(),
            hours: d.hours(),
            minutes: d.minutes(),
            seconds: d.seconds(),
            milliseconds: d.milliseconds(),
            microseconds: d.microseconds(),
            nanoseconds: d.nanoseconds(),
        }
    }

    /// Serialize a `Duration` datum back to an ISO 8601 duration string.
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let d = TemporalDuration::new(
            self.years,
            self.months,
            self.weeks,
            self.days,
            self.hours,
            self.minutes,
            self.seconds,
            self.milliseconds,
            self.microseconds,
            self.nanoseconds,
        )
        .unwrap_or_else(|e| error!("failed to reconstruct duration: {e}"));

        let s = d
            .as_temporal_string(ToStringRoundingOptions::default())
            .unwrap_or_else(|e| error!("failed to format duration: {e}"));

        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the years component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_years(d: Duration) -> i64 {
    d.years
}

/// Returns the months component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_months(d: Duration) -> i64 {
    d.months
}

/// Returns the weeks component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_weeks(d: Duration) -> i64 {
    d.weeks
}

/// Returns the days component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_days(d: Duration) -> i64 {
    d.days
}

/// Returns the hours component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_hours(d: Duration) -> i64 {
    d.hours
}

/// Returns the minutes component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_minutes(d: Duration) -> i64 {
    d.minutes
}

/// Returns the seconds component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_seconds(d: Duration) -> i64 {
    d.seconds
}

/// Returns the milliseconds component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_milliseconds(d: Duration) -> i64 {
    d.milliseconds
}

/// Returns the microseconds component as text (i128 has no native SQL type;
/// use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_microseconds(d: Duration) -> String {
    d.microseconds.to_string()
}

/// Returns the nanoseconds component as text (i128 has no native SQL type;
/// use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_nanoseconds(d: Duration) -> String {
    d.nanoseconds.to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl Duration {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    // Clippy's wrong_self_convention wants `to_*` on Copy types to take self by value.
    pub(crate) fn to_temporal(self) -> TemporalDuration {
        TemporalDuration::new(
            self.years,
            self.months,
            self.weeks,
            self.days,
            self.hours,
            self.minutes,
            self.seconds,
            self.milliseconds,
            self.microseconds,
            self.nanoseconds,
        )
        .unwrap_or_else(|e| error!("failed to reconstruct duration: {e}"))
    }

    /// Build a `Duration` from a `temporal_rs` duration.
    // The accessor methods on TemporalDuration are const fn, but error! is not;
    // suppress the missing_const_for_fn lint rather than marking const.
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn from_temporal(d: &TemporalDuration) -> Self {
        Self {
            years: d.years(),
            months: d.months(),
            weeks: d.weeks(),
            days: d.days(),
            hours: d.hours(),
            minutes: d.minutes(),
            seconds: d.seconds(),
            milliseconds: d.milliseconds(),
            microseconds: d.microseconds(),
            nanoseconds: d.nanoseconds(),
        }
    }
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Returns a copy with the sign of every component flipped.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_negated(d: Duration) -> Duration {
    Duration::from_temporal(&d.to_temporal().negated())
}

/// Returns a copy with all components made non-negative.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_abs(d: Duration) -> Duration {
    Duration::from_temporal(&d.to_temporal().abs())
}

/// Returns -1, 0, or 1 indicating the overall sign of the duration.
///
/// A valid duration has uniform sign (all non-zero components share the same
/// sign), so the overall sign equals the sign of the first non-zero field.
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_sign(d: Duration) -> i32 {
    // i64 fields first (years … milliseconds), then i128 fields.
    for v in [d.years, d.months, d.weeks, d.days, d.hours, d.minutes, d.seconds, d.milliseconds] {
        if v > 0 {
            return 1;
        }
        if v < 0 {
            return -1;
        }
    }
    for v in [d.microseconds, d.nanoseconds] {
        if v > 0 {
            return 1;
        }
        if v < 0 {
            return -1;
        }
    }
    0
}

/// Returns true if all components of the duration are zero.
/// Equivalent to Temporal's `Duration.blank`.
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_is_zero(d: Duration) -> bool {
    duration_sign(d) == 0
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Returns `true` if the duration contains any calendar components (years,
/// months, weeks, or days). Calendar-component durations require a reference
/// date or timezone for arithmetic; time-only operations fail if this is true.
const fn has_calendar_components(d: Duration) -> bool {
    d.years != 0 || d.months != 0 || d.weeks != 0 || d.days != 0
}

/// Add two durations component-wise.
///
/// Only time-only durations (hours, minutes, seconds, milliseconds,
/// microseconds, nanoseconds) can be added without a reference date.
/// Raises an error if either argument contains calendar components (years,
/// months, weeks, or days) — use `plain_datetime_add` or `zoned_datetime_add`
/// to add durations that include calendar components.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_add(a: Duration, b: Duration) -> Duration {
    if has_calendar_components(a) || has_calendar_components(b) {
        error!(
            "duration_add: calendar components (years, months, weeks, days) require a \
             reference date; add this duration to a zoneddatetime or plaindatetime instead"
        );
    }
    let result = a
        .to_temporal()
        .add(&b.to_temporal())
        .unwrap_or_else(|e| error!("duration_add failed: {e}"));
    Duration::from_temporal(&result)
}

/// Subtract one duration from another component-wise.
///
/// Only time-only durations (hours, minutes, seconds, milliseconds,
/// microseconds, nanoseconds) can be subtracted without a reference date.
/// Raises an error if either argument contains calendar components (years,
/// months, weeks, or days) — use `plain_datetime_subtract` or
/// `zoned_datetime_subtract` to subtract durations that include calendar
/// components.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_subtract(a: Duration, b: Duration) -> Duration {
    if has_calendar_components(a) || has_calendar_components(b) {
        error!(
            "duration_subtract: calendar components (years, months, weeks, days) require a \
             reference date; subtract this duration from a zoneddatetime or plaindatetime instead"
        );
    }
    let result = a
        .to_temporal()
        .subtract(&b.to_temporal())
        .unwrap_or_else(|e| error!("duration_subtract failed: {e}"));
    Duration::from_temporal(&result)
}

// ---------------------------------------------------------------------------
// Rounding
// ---------------------------------------------------------------------------

/// Round a duration to the given `smallest_unit`.
///
/// Only time-only durations (no years/months/weeks/days) are accepted here.
/// For durations with calendar components use `duration_round_zoned` or
/// `duration_round_plain`, which anchor the rounding against a reference date.
///
/// `smallest_unit` is a Temporal unit string: `'hour'`, `'minute'`,
/// `'second'`, `'millisecond'`, `'microsecond'`, or `'nanosecond'`.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_round(d: Duration, smallest_unit: &str) -> Duration {
    let unit = Unit::from_str(smallest_unit)
        .unwrap_or_else(|_| error!("duration_round: invalid unit \"{smallest_unit}\""));
    let mut opts = RoundingOptions::default();
    opts.smallest_unit = Some(unit);
    let result = d
        .to_temporal()
        .round_with_provider(opts, None, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_round failed: {e}"));
    Duration::from_temporal(&result)
}

/// Round a duration to the given `smallest_unit` relative to a `ZonedDateTime`.
///
/// Use this for durations that contain calendar components (years, months,
/// weeks, or days), or when DST-aware day-length is relevant.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_round_zoned(
    d: Duration,
    smallest_unit: &str,
    relative_to: ZonedDateTime,
) -> Duration {
    let unit = Unit::from_str(smallest_unit)
        .unwrap_or_else(|_| error!("duration_round_zoned: invalid unit \"{smallest_unit}\""));
    let mut opts = RoundingOptions::default();
    opts.smallest_unit = Some(unit);
    let rel = RelativeTo::from(relative_to.to_temporal());
    let result = d
        .to_temporal()
        .round_with_provider(opts, Some(rel), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_round_zoned failed: {e}"));
    Duration::from_temporal(&result)
}

/// Round a duration to the given `smallest_unit` relative to a `PlainDateTime`.
///
/// Use this for durations that contain calendar components (years, months,
/// weeks, or days) when timezone-aware day-length is not needed.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_round_plain(
    d: Duration,
    smallest_unit: &str,
    relative_to: PlainDateTime,
) -> Duration {
    let unit = Unit::from_str(smallest_unit)
        .unwrap_or_else(|_| error!("duration_round_plain: invalid unit \"{smallest_unit}\""));
    let mut opts = RoundingOptions::default();
    opts.smallest_unit = Some(unit);
    let plain_date = relative_to.to_temporal().to_plain_date();
    let rel = RelativeTo::from(plain_date);
    let result = d
        .to_temporal()
        .round_with_provider(opts, Some(rel), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_round_plain failed: {e}"));
    Duration::from_temporal(&result)
}

// ---------------------------------------------------------------------------
// Total (fractional single-unit representation)
// ---------------------------------------------------------------------------

/// Return the total value of a time-only duration expressed in `unit` as a
/// floating-point number.
///
/// For durations with calendar components use `duration_total_zoned` or
/// `duration_total_plain` to supply a reference date for month/year lengths.
///
/// `unit` is a Temporal unit string: `'hour'`, `'minute'`, `'second'`, etc.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_total(d: Duration, unit: &str) -> f64 {
    let u =
        Unit::from_str(unit).unwrap_or_else(|_| error!("duration_total: invalid unit \"{unit}\""));
    d.to_temporal()
        .total_with_provider(u, None, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_total failed: {e}"))
        .as_inner()
}

/// Return the total value of a duration expressed in `unit`, anchored to a
/// `ZonedDateTime` for DST-aware day/month/year lengths.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_total_zoned(d: Duration, unit: &str, relative_to: ZonedDateTime) -> f64 {
    let u = Unit::from_str(unit)
        .unwrap_or_else(|_| error!("duration_total_zoned: invalid unit \"{unit}\""));
    let rel = RelativeTo::from(relative_to.to_temporal());
    d.to_temporal()
        .total_with_provider(u, Some(rel), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_total_zoned failed: {e}"))
        .as_inner()
}

/// Return the total value of a duration expressed in `unit`, anchored to a
/// `PlainDateTime` for calendar-aware month/year lengths.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_total_plain(d: Duration, unit: &str, relative_to: PlainDateTime) -> f64 {
    let u = Unit::from_str(unit)
        .unwrap_or_else(|_| error!("duration_total_plain: invalid unit \"{unit}\""));
    let plain_date = relative_to.to_temporal().to_plain_date();
    let rel = RelativeTo::from(plain_date);
    d.to_temporal()
        .total_with_provider(u, Some(rel), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_total_plain failed: {e}"))
        .as_inner()
}

// ---------------------------------------------------------------------------
// Relative arithmetic (duration + duration anchored to a reference datetime)
// ---------------------------------------------------------------------------

/// Add two durations anchored to a `ZonedDateTime`.
///
/// This is the correct operation when either duration contains calendar
/// components (years, months, weeks, or days): the durations are applied
/// to the reference datetime in turn, and the resulting elapsed duration
/// is returned.  DST transitions are respected.
///
/// The default `DifferenceSettings` produce a result in hours; use
/// `duration_round_zoned` afterwards to balance to larger units if required.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_add_zoned(a: Duration, b: Duration, relative_to: ZonedDateTime) -> Duration {
    let zdt_start = relative_to.to_temporal();
    let zdt_after_a = zdt_start
        .add_with_provider(&a.to_temporal(), None, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_add_zoned (add a) failed: {e}"));
    let zdt_after_ab = zdt_after_a
        .add_with_provider(&b.to_temporal(), None, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_add_zoned (add b) failed: {e}"));
    let result = zdt_start
        .until_with_provider(&zdt_after_ab, DifferenceSettings::default(), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_add_zoned (until) failed: {e}"));
    Duration::from_temporal(&result)
}

/// Subtract duration `b` from duration `a` anchored to a `ZonedDateTime`.
///
/// Equivalent to adding `a` then removing `b` relative to the reference
/// datetime.  DST transitions are respected.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_subtract_zoned(a: Duration, b: Duration, relative_to: ZonedDateTime) -> Duration {
    let zdt_start = relative_to.to_temporal();
    let zdt_after_a = zdt_start
        .add_with_provider(&a.to_temporal(), None, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_subtract_zoned (add a) failed: {e}"));
    let zdt_after_a_minus_b = zdt_after_a
        .subtract_with_provider(&b.to_temporal(), None, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_subtract_zoned (subtract b) failed: {e}"));
    let result = zdt_start
        .until_with_provider(&zdt_after_a_minus_b, DifferenceSettings::default(), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_subtract_zoned (until) failed: {e}"));
    Duration::from_temporal(&result)
}

/// Add two durations anchored to a `PlainDateTime`.
///
/// This is the correct operation when either duration contains calendar
/// components (years, months, weeks, or days) and timezone-aware day
/// lengths are not needed.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_add_plain(a: Duration, b: Duration, relative_to: PlainDateTime) -> Duration {
    let pdt_start = relative_to.to_temporal();
    let pdt_after_a = pdt_start
        .add(&a.to_temporal(), None)
        .unwrap_or_else(|e| error!("duration_add_plain (add a) failed: {e}"));
    let pdt_after_ab = pdt_after_a
        .add(&b.to_temporal(), None)
        .unwrap_or_else(|e| error!("duration_add_plain (add b) failed: {e}"));
    let result = pdt_start
        .until(&pdt_after_ab, DifferenceSettings::default())
        .unwrap_or_else(|e| error!("duration_add_plain (until) failed: {e}"));
    Duration::from_temporal(&result)
}

/// Subtract duration `b` from duration `a` anchored to a `PlainDateTime`.
///
/// Equivalent to adding `a` then removing `b` relative to the reference
/// datetime.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_subtract_plain(a: Duration, b: Duration, relative_to: PlainDateTime) -> Duration {
    let pdt_start = relative_to.to_temporal();
    let pdt_after_a = pdt_start
        .add(&a.to_temporal(), None)
        .unwrap_or_else(|e| error!("duration_subtract_plain (add a) failed: {e}"));
    let pdt_after_a_minus_b = pdt_after_a
        .subtract(&b.to_temporal(), None)
        .unwrap_or_else(|e| error!("duration_subtract_plain (subtract b) failed: {e}"));
    let result = pdt_start
        .until(&pdt_after_a_minus_b, DifferenceSettings::default())
        .unwrap_or_else(|e| error!("duration_subtract_plain (until) failed: {e}"));
    Duration::from_temporal(&result)
}
