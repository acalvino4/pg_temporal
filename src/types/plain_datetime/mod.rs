use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ffi::CStr;
use temporal_rs::{
    PlainDateTime as TemporalPdt,
    options::{DifferenceSettings, DisplayCalendar, Overflow, ToStringRoundingOptions},
};

use crate::types::catalog::{lookup_calendar_by_oid, lookup_or_insert_calendar};
use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// A PlainDateTime is a calendar-local date and time with no timezone.
// It cannot represent an absolute instant without knowing the timezone.
//
//   year .. nanosecond  – ISO 8601 date/time field values
//   calendar_oid        – row id in temporal.calendar_catalog (for
//                         future multi-calendar support; ISO 8601 only
//                         in Phase 3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PostgresType)]
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
    calendar_oid: i32,
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
        let calendar_oid = lookup_or_insert_calendar(cal_id);

        Self {
            year: pdt.year(),
            month: pdt.month(),
            day: pdt.day(),
            hour: pdt.hour(),
            minute: pdt.minute(),
            second: pdt.second(),
            millisecond: pdt.millisecond(),
            microsecond: pdt.microsecond(),
            nanosecond: pdt.nanosecond(),
            calendar_oid,
        }
    }

    /// Serialize a `PlainDateTime` datum back to an IXDTF string.
    ///
    /// The calendar annotation is omitted for ISO 8601 (`DisplayCalendar::Auto`).
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        // calendar_oid is stored for future multi-calendar support.
        // In Phase 3 only ISO 8601 is supported, so try_new_iso is correct.
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
        .unwrap_or_else(|e| error!("failed to reconstruct plain_datetime: {e}"));

        let s = pdt
            .to_ixdtf_string(ToStringRoundingOptions::default(), DisplayCalendar::default())
            .unwrap_or_else(|e| error!("failed to format plain_datetime: {e}"));

        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the year component.
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_year(pdt: PlainDateTime) -> i32 {
    pdt.year
}

/// Returns the month component (1–12).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_month(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.month)
}

/// Returns the day-of-month component (1–31).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_day(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.day)
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
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_minute(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.minute)
}

/// Returns the second component (0–59).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_second(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.second)
}

/// Returns the millisecond component (0–999).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_millisecond(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.millisecond)
}

/// Returns the microsecond component (0–999).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_microsecond(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.microsecond)
}

/// Returns the nanosecond component (0–999).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_nanosecond(pdt: PlainDateTime) -> i32 {
    i32::from(pdt.nanosecond)
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_calendar(pdt: PlainDateTime) -> String {
    lookup_calendar_by_oid(pdt.calendar_oid)
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl PlainDateTime {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    /// Phase 3/4: ISO calendar only; `try_new_iso` is correct.
    // Clippy's wrong_self_convention wants `to_*` on Copy types to take self by value.
    pub(crate) fn to_temporal(self) -> TemporalPdt {
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
    }

    /// Build a `PlainDateTime` from a `temporal_rs` plain datetime.
    pub(crate) fn from_temporal(pdt: &TemporalPdt) -> Self {
        let cal_id = pdt.calendar().identifier();
        let calendar_oid = lookup_or_insert_calendar(cal_id);
        Self {
            year: pdt.year(),
            month: pdt.month(),
            day: pdt.day(),
            hour: pdt.hour(),
            minute: pdt.minute(),
            second: pdt.second(),
            millisecond: pdt.millisecond(),
            microsecond: pdt.microsecond(),
            nanosecond: pdt.nanosecond(),
            calendar_oid,
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison functions
// ---------------------------------------------------------------------------

/// Returns -1, 0, or 1 comparing two plain datetimes by ISO date/time fields
/// and, as a tiebreaker, by calendar OID.
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_compare(a: PlainDateTime, b: PlainDateTime) -> i32 {
    let a_key = (
        a.year,
        a.month,
        a.day,
        a.hour,
        a.minute,
        a.second,
        a.millisecond,
        a.microsecond,
        a.nanosecond,
        a.calendar_oid,
    );
    let b_key = (
        b.year,
        b.month,
        b.day,
        b.hour,
        b.minute,
        b.second,
        b.millisecond,
        b.microsecond,
        b.nanosecond,
        b.calendar_oid,
    );
    match a_key.cmp(&b_key) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_lt(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) < 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_le(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) <= 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_eq(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) == 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_ne(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) != 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_ge(a: PlainDateTime, b: PlainDateTime) -> bool {
    plain_datetime_compare(a, b) >= 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
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
