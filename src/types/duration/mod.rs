use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use temporal_rs::{Duration as TemporalDuration, options::ToStringRoundingOptions};

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
        if v > 0 { return 1; }
        if v < 0 { return -1; }
    }
    for v in [d.microseconds, d.nanoseconds] {
        if v > 0 { return 1; }
        if v < 0 { return -1; }
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

/// Add two durations component-wise.
///
/// Raises an error if either duration has calendar components (years, months,
/// weeks, or days) because the result depends on a reference date.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_add(a: Duration, b: Duration) -> Duration {
    let result = a
        .to_temporal()
        .add(&b.to_temporal())
        .unwrap_or_else(|e| error!("duration_add failed: {e}"));
    Duration::from_temporal(&result)
}

/// Subtract one duration from another component-wise.
///
/// Raises an error if either duration has calendar components (years, months,
/// weeks, or days) because the result depends on a reference date.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_subtract(a: Duration, b: Duration) -> Duration {
    let result = a
        .to_temporal()
        .subtract(&b.to_temporal())
        .unwrap_or_else(|e| error!("duration_subtract failed: {e}"));
    Duration::from_temporal(&result)
}
