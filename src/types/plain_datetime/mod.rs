// pgrx requires all custom PostgresType parameters in #[pg_extern] functions to be
// passed by value — references are not supported (`BorrowDatum`/`ArgAbi` are not
// implemented for user-defined types). The needless_pass_by_value lint correctly
// identifies that many of these functions don't need ownership, but they must
// take by value due to this pgrx constraint.
#![allow(clippy::needless_pass_by_value)]

use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ffi::CStr;
use temporal_rs::{
    Calendar, PlainDateTime as TemporalPdt,
    options::{DifferenceSettings, DisplayCalendar, Overflow, ToStringRoundingOptions},
};

use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// A PlainDateTime is a calendar-local date and time with no timezone.
// It cannot represent an absolute instant without knowing the timezone.
//
//   year .. nanosecond  – ISO 8601 date/time field values
//   calendar_id         – calendar identifier string (e.g. "iso8601")
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
pub struct PlainDateTime {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    millisecond: u16,
    microsecond: u16,
    nanosecond: u16,
    calendar_id: String,
}

// ---------------------------------------------------------------------------
// Text in / out
// ---------------------------------------------------------------------------

impl InOutFuncs for PlainDateTime {
    /// Parse an IXDTF plain datetime string into a `PlainDateTime` datum.
    ///
    /// Example inputs:
    ///   `2025-03-01T11:16:10`
    ///   `2025-03-01T11:16:10.000000001`
    ///   `2025-03-01T11:16:10[u-ca=iso8601]`
    fn input(input: &CStr) -> Self {
        let s =
            input.to_str().unwrap_or_else(|_| error!("plain_datetime input is not valid UTF-8"));

        let pdt = TemporalPdt::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid plain_datetime \"{s}\": {e}"));

        let cal_id = pdt.calendar().identifier();

        // Always store the ISO 8601 date fields regardless of calendar so that
        // output() can round-trip correctly via try_new_iso + with_calendar.
        // Calendar-specific year/month/day values are computed on demand by the
        // accessor functions via to_temporal().
        Self {
            year: pdt.iso_year(),
            month: pdt.iso_month(),
            day: pdt.iso_day(),
            hour: pdt.hour(),
            minute: pdt.minute(),
            second: pdt.second(),
            millisecond: pdt.millisecond(),
            microsecond: pdt.microsecond(),
            nanosecond: pdt.nanosecond(),
            calendar_id: cal_id.to_string(),
        }
    }

    /// Serialize a `PlainDateTime` datum back to an IXDTF string.
    ///
    /// The calendar annotation is omitted for ISO 8601 (`DisplayCalendar::Auto`).
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let cal = Calendar::try_from_utf8(self.calendar_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{}\": {e}", self.calendar_id));
        // Fields are always stored as ISO 8601. Use try_new_iso to reconstruct
        // the datetime, then attach the calendar for annotation only.
        let pdt = TemporalPdt::try_new_iso(
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            self.millisecond,
            self.microsecond,
            self.nanosecond,
        )
        .unwrap_or_else(|e| error!("failed to reconstruct plain_datetime: {e}"))
        .with_calendar(cal);

        let s = pdt
            .to_ixdtf_string(ToStringRoundingOptions::default(), DisplayCalendar::default())
            .unwrap_or_else(|e| error!("failed to format plain_datetime: {e}"));

        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Constructor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Construct a `PlainDateTime` from individual field values.
///
/// `millisecond`, `microsecond`, `nanosecond`, and `cal` are optional; they
/// default to `0`, `0`, `0`, and `'iso8601'` respectively.
///
/// Example:
/// ```sql
/// SELECT make_plaindatetime(2025, 6, 15, 12, 30, 0);
/// SELECT make_plaindatetime(2025, 6, 15, 12, 30, 0, 0, 0, 0, 'iso8601');
/// ```
#[allow(clippy::too_many_arguments)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn make_plaindatetime(
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: i32,
    millisecond: default!(i32, 0),
    microsecond: default!(i32, 0),
    nanosecond: default!(i32, 0),
    cal: default!(&str, "'iso8601'"),
) -> PlainDateTime {
    let month = u8::try_from(month)
        .unwrap_or_else(|_| error!("make_plaindatetime: invalid month {month}"));
    let day =
        u8::try_from(day).unwrap_or_else(|_| error!("make_plaindatetime: invalid day {day}"));
    let hour =
        u8::try_from(hour).unwrap_or_else(|_| error!("make_plaindatetime: invalid hour {hour}"));
    let minute = u8::try_from(minute)
        .unwrap_or_else(|_| error!("make_plaindatetime: invalid minute {minute}"));
    let second = u8::try_from(second)
        .unwrap_or_else(|_| error!("make_plaindatetime: invalid second {second}"));
    let millisecond = u16::try_from(millisecond)
        .unwrap_or_else(|_| error!("make_plaindatetime: invalid millisecond {millisecond}"));
    let microsecond = u16::try_from(microsecond)
        .unwrap_or_else(|_| error!("make_plaindatetime: invalid microsecond {microsecond}"));
    let nanosecond = u16::try_from(nanosecond)
        .unwrap_or_else(|_| error!("make_plaindatetime: invalid nanosecond {nanosecond}"));
    // Validate that the combination of field values is a legal ISO 8601 date/time.
    TemporalPdt::try_new_iso(year, month, day, hour, minute, second, millisecond, microsecond, nanosecond)
        .unwrap_or_else(|e| error!("make_plaindatetime: {e}"));
    let calendar = Calendar::try_from_utf8(cal.as_bytes())
        .unwrap_or_else(|e| error!("make_plaindatetime: invalid calendar \"{cal}\": {e}"));
    PlainDateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond,
        microsecond,
        nanosecond,
        calendar_id: calendar.identifier().to_string(),
    }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the calendar year (e.g. Persian 1403 for ISO 2025-03-01 with u-ca=persian).
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_datetime_year(pdt: PlainDateTime) -> i32 {
    pdt.to_temporal().year()
}

/// Returns the calendar month (1-indexed within the calendar system).
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_datetime_month(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.to_temporal().month())
}

/// Returns the calendar day-of-month.
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_datetime_day(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.to_temporal().day())
}

/// Returns the hour component (0–23).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_hour(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.hour)
}

/// Returns the minute component (0–59).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_minute(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.minute)
}

/// Returns the second component (0–59).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_second(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.second)
}

/// Returns the millisecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_millisecond(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.millisecond)
}

/// Returns the microsecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_microsecond(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.microsecond)
}

/// Returns the nanosecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_nanosecond(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.nanosecond)
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_calendar(pdt: PlainDateTime) -> String {
    pdt.calendar_id
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl PlainDateTime {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    /// ISO calendar only in the current implementation; `try_new_iso` is correct.
    pub(crate) fn to_temporal(&self) -> TemporalPdt {
        let cal = Calendar::try_from_utf8(self.calendar_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{}\": {e}", self.calendar_id));
        // Fields are always stored as ISO 8601. Use try_new_iso then with_calendar
        // so the calendar is attached without reinterpreting the stored fields.
        TemporalPdt::try_new_iso(
            self.year,
            self.month,
            self.day,
            self.hour,
            self.minute,
            self.second,
            self.millisecond,
            self.microsecond,
            self.nanosecond,
        )
        .unwrap_or_else(|e| error!("failed to reconstruct plain_datetime: {e}"))
        .with_calendar(cal)
    }

    /// Build a `PlainDateTime` from a `temporal_rs` plain datetime.
    ///
    /// Always stores ISO 8601 fields (`iso_year` / `iso_month` / `iso_day`) regardless
    /// of the attached calendar, matching the invariant expected by `to_temporal()`
    /// and `output()` which both reconstruct via `try_new_iso`.
    pub(crate) fn from_temporal(pdt: &TemporalPdt) -> Self {
        let cal_id = pdt.calendar().identifier();
        Self {
            year: pdt.iso_year(),
            month: pdt.iso_month(),
            day: pdt.iso_day(),
            hour: pdt.hour(),
            minute: pdt.minute(),
            second: pdt.second(),
            millisecond: pdt.millisecond(),
            microsecond: pdt.microsecond(),
            nanosecond: pdt.nanosecond(),
            calendar_id: cal_id.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison functions
// ---------------------------------------------------------------------------

/// Returns -1, 0, or 1 comparing two plain datetimes by ISO date/time fields
/// and, as a tiebreaker, by calendar identifier.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_compare(a: PlainDateTime, b: PlainDateTime) -> i32 {
    match (
        a.year,
        a.month,
        a.day,
        a.hour,
        a.minute,
        a.second,
        a.millisecond,
        a.microsecond,
        a.nanosecond,
    )
        .cmp(&(
            b.year,
            b.month,
            b.day,
            b.hour,
            b.minute,
            b.second,
            b.millisecond,
            b.microsecond,
            b.nanosecond,
        ))
        .then_with(|| a.calendar_id.cmp(&b.calendar_id))
    {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_lt(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) < 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_le(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) <= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_eq(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) == 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_ne(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) != 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_ge(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) >= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_gt(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) > 0
}

extension_sql!(
    r"
    CREATE OPERATOR < (
        LEFTARG = PlainDateTime, RIGHTARG = PlainDateTime,
        FUNCTION = plain_datetime_lt,
        COMMUTATOR = >, NEGATOR = >=
    );
    CREATE OPERATOR <= (
        LEFTARG = PlainDateTime, RIGHTARG = PlainDateTime,
        FUNCTION = plain_datetime_le,
        COMMUTATOR = >=, NEGATOR = >
    );
    CREATE OPERATOR = (
        LEFTARG = PlainDateTime, RIGHTARG = PlainDateTime,
        FUNCTION = plain_datetime_eq,
        COMMUTATOR = =, NEGATOR = <>
    );
    CREATE OPERATOR <> (
        LEFTARG = PlainDateTime, RIGHTARG = PlainDateTime,
        FUNCTION = plain_datetime_ne,
        COMMUTATOR = <>, NEGATOR = =
    );
    CREATE OPERATOR >= (
        LEFTARG = PlainDateTime, RIGHTARG = PlainDateTime,
        FUNCTION = plain_datetime_ge,
        COMMUTATOR = <=, NEGATOR = <
    );
    CREATE OPERATOR > (
        LEFTARG = PlainDateTime, RIGHTARG = PlainDateTime,
        FUNCTION = plain_datetime_gt,
        COMMUTATOR = <, NEGATOR = <=
    );
    CREATE OPERATOR CLASS plain_datetime_btree_ops DEFAULT FOR TYPE PlainDateTime USING btree AS
        OPERATOR 1  <,
        OPERATOR 2  <=,
        OPERATOR 3  =,
        OPERATOR 4  >=,
        OPERATOR 5  >,
        FUNCTION 1  plain_datetime_compare(PlainDateTime, PlainDateTime);
    ",
    name = "plain_datetime_comparison_operators",
    requires = [
        plain_datetime_lt,
        plain_datetime_le,
        plain_datetime_eq,
        plain_datetime_ne,
        plain_datetime_ge,
        plain_datetime_gt
    ],
);

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Add a duration to a plain datetime.
/// Uses `Constrain` overflow: day-of-month is clamped to the last valid day
/// (e.g., Jan 31 + P1M → Feb 28/29).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_add(pdt: PlainDateTime, dur: Duration) -> PlainDateTime {
    let result = pdt
        .to_temporal()
        .add(&dur.to_temporal(), Some(Overflow::Constrain))
        .unwrap_or_else(|e| error!("plain_datetime_add failed: {e}"));
    PlainDateTime::from_temporal(&result)
}

/// Subtract a duration from a plain datetime.
/// Uses `Constrain` overflow: day-of-month is clamped to the last valid day.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_subtract(pdt: PlainDateTime, dur: Duration) -> PlainDateTime {
    let result = pdt
        .to_temporal()
        .subtract(&dur.to_temporal(), Some(Overflow::Constrain))
        .unwrap_or_else(|e| error!("plain_datetime_subtract failed: {e}"));
    PlainDateTime::from_temporal(&result)
}

/// Returns the duration elapsed from `other` to `pdt` (default unit: days).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_since(pdt: PlainDateTime, other: PlainDateTime) -> Duration {
    let d = pdt
        .to_temporal()
        .since(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plain_datetime_since failed: {e}"));
    Duration::from_temporal(&d)
}

/// Returns the duration from `pdt` to `other` (default unit: days).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_until(pdt: PlainDateTime, other: PlainDateTime) -> Duration {
    let d = pdt
        .to_temporal()
        .until(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plain_datetime_until failed: {e}"));
    Duration::from_temporal(&d)
}
