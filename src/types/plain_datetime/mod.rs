// pgrx requires all custom PostgresType parameters in #[pg_extern] functions to be
// passed by value — references are not supported (`BorrowDatum`/`ArgAbi` are not
// implemented for user-defined types). The needless_pass_by_value lint correctly
// identifies that many of these functions don't need ownership, but they must
// take by value due to this pgrx constraint.
#![allow(clippy::needless_pass_by_value)]

use pgrx::prelude::*;
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
//   year .. second   – ISO 8601 date/time field values
//   subsecond_ns     – sub-second precision collapsed into one u32
//                     (ms*1_000_000 + µs*1_000 + ns); no info is lost
//   cal_idx          – compact calendar index (see cal_index module)
//
// Layout (field order chosen for alignment): i32 + u32 + 6×u8 = 14 bytes.
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PostgresType, PostgresEq, PostgresOrd)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct PlainDateTime {
    pub(crate) year: i32,
    pub(crate) subsecond_ns: u32,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) second: u8,
    pub(crate) cal_idx: u8,
}

impl PartialOrd for PlainDateTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlainDateTime {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.year, self.month, self.day, self.hour, self.minute, self.second, self.subsecond_ns)
            .cmp(&(other.year, other.month, other.day, other.hour, other.minute, other.second, other.subsecond_ns))
            .then_with(|| self.cal_idx.cmp(&other.cal_idx))
    }
}

// ---------------------------------------------------------------------------
// Manual IntoDatum / FromDatum / BoxRet / ArgAbi / UnboxDatum
//
// The Serde/CBOR path is intentionally bypassed: pgrx's default
// PostgresType derive uses CBOR serialization, but all on-disk datums
// here are compact binary via PgVarlena.
// ---------------------------------------------------------------------------

impl pgrx::datum::IntoDatum for PlainDateTime {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for PlainDateTime {
    unsafe fn from_polymorphic_datum(
        datum: pgrx::pg_sys::Datum,
        is_null: bool,
        _typoid: pgrx::pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            None
        } else {
            Some(*unsafe { PgVarlena::<Self>::from_datum(datum) })
        }
    }
}

unsafe impl pgrx::callconv::BoxRet for PlainDateTime {
    unsafe fn box_into<'fcx>(
        self,
        fcinfo: &mut pgrx::callconv::FcInfo<'fcx>,
    ) -> pgrx::datum::Datum<'fcx> {
        match pgrx::datum::IntoDatum::into_datum(self) {
            None => fcinfo.return_null(),
            Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
        }
    }
}

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for PlainDateTime
where
    Self: 'fcx,
{
    unsafe fn unbox_arg_unchecked(arg: pgrx::callconv::Arg<'_, 'fcx>) -> Self {
        let index = arg.index();
        unsafe {
            arg.unbox_arg_using_from_datum()
                .unwrap_or_else(|| panic!("argument {index} must not be null"))
        }
    }
}

unsafe impl pgrx::datum::UnboxDatum for PlainDateTime {
    type As<'dat> = Self
    where
        Self: 'dat;

    unsafe fn unbox<'dat>(datum: pgrx::datum::Datum<'dat>) -> Self::As<'dat>
    where
        Self: 'dat,
    {
        unsafe {
            <Self as pgrx::datum::FromDatum>::from_datum(
                std::mem::transmute(datum),
                false,
            )
            .unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Text in / out
// ---------------------------------------------------------------------------

impl PgVarlenaInOutFuncs for PlainDateTime {
    /// Parse an IXDTF plain datetime string into a `PlainDateTime` datum.
    ///
    /// Example inputs:
    ///   `2025-03-01T11:16:10`
    ///   `2025-03-01T11:16:10.000000001`
    ///   `2025-03-01T11:16:10[u-ca=iso8601]`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s =
            input.to_str().unwrap_or_else(|_| error!("plain_datetime input is not valid UTF-8"));

        let pdt = TemporalPdt::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid plain_datetime \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        *result = PlainDateTime::from_temporal(&pdt);
        result
    }

    /// Serialize a `PlainDateTime` datum back to an IXDTF string.
    ///
    /// The calendar annotation is omitted for ISO 8601 (`DisplayCalendar::Auto`).
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let pdt = self.to_temporal();
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
    let cal_id = calendar.identifier();
    let cal_idx = crate::cal_index::index_of(cal_id)
        .unwrap_or_else(|| error!("make_plaindatetime: unsupported calendar \"{cal_id}\""));
    let subsecond_ns = (millisecond as u32) * 1_000_000
        + (microsecond as u32) * 1_000
        + nanosecond as u32;
    PlainDateTime {
        year,
        subsecond_ns,
        month,
        day,
        hour,
        minute,
        second,
        cal_idx,
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
    (pdt.subsecond_ns / 1_000_000) as i32
}

/// Returns the microsecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_microsecond(pdt: PlainDateTime) -> i32 {
    ((pdt.subsecond_ns % 1_000_000) / 1_000) as i32
}

/// Returns the nanosecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_nanosecond(pdt: PlainDateTime) -> i32 {
    (pdt.subsecond_ns % 1_000) as i32
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_datetime_calendar(pdt: PlainDateTime) -> String {
    crate::cal_index::name_of(pdt.cal_idx)
        .unwrap_or_else(|| error!("plain_datetime_calendar: unknown calendar index {}", pdt.cal_idx))
        .to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl PlainDateTime {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    /// Fields are always stored as ISO 8601; `try_new_iso` is correct.
    pub(crate) fn to_temporal(self) -> TemporalPdt {
        let cal_idx = self.cal_idx;
        let subsecond_ns = self.subsecond_ns;
        let cal_id = crate::cal_index::name_of(cal_idx)
            .unwrap_or_else(|| error!("unknown calendar index {cal_idx}"));
        let cal = Calendar::try_from_utf8(cal_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{cal_id}\": {e}"));
        let ms = (subsecond_ns / 1_000_000) as u16;
        let us = ((subsecond_ns % 1_000_000) / 1_000) as u16;
        let ns = (subsecond_ns % 1_000) as u16;
        // Fields are always stored as ISO 8601. Use try_new_iso then with_calendar
        // so the calendar is attached without reinterpreting the stored fields.
        TemporalPdt::try_new_iso(
            self.year, self.month, self.day,
            self.hour, self.minute, self.second,
            ms, us, ns,
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
        let cal_idx = crate::cal_index::index_of(cal_id)
            .unwrap_or_else(|| error!("unsupported calendar: {cal_id}"));
        let subsecond_ns = (pdt.millisecond() as u32) * 1_000_000
            + (pdt.microsecond() as u32) * 1_000
            + pdt.nanosecond() as u32;
        Self {
            year: pdt.iso_year(),
            subsecond_ns,
            month: pdt.iso_month(),
            day: pdt.iso_day(),
            hour: pdt.hour(),
            minute: pdt.minute(),
            second: pdt.second(),
            cal_idx,
        }
    }
}

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

// ---------------------------------------------------------------------------
// Explicit casts: timestamp ↔ PlainDateTime
// ---------------------------------------------------------------------------

/// Cast a `timestamp` (without time zone) to a `PlainDateTime`.
///
/// The ISO 8601 calendar is assigned. Sub-microsecond nanoseconds are always
/// zero since `timestamp` has only microsecond precision. Range and field
/// validity are delegated to `temporal_rs`.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn timestamp_to_plaindatetime(ts: Timestamp) -> PlainDateTime {
    // pgrx microseconds() returns the *entire* seconds field × 1_000_000
    // (e.g. for 10.5 s it returns 10_500_000). Take the remainder to get
    // only the sub-second part in microseconds.
    let subsecond_us = ts.microseconds() % 1_000_000;
    let ms = (subsecond_us / 1_000) as u16;
    let us = (subsecond_us % 1_000) as u16;
    let sec = ts.second() as u8; // truncates fractional part
    let pdt = TemporalPdt::try_new_iso(
        ts.year(), ts.month(), ts.day(),
        ts.hour(), ts.minute(), sec,
        ms, us, 0,
    )
    .unwrap_or_else(|e| error!("timestamp_to_plaindatetime: {e}"));
    PlainDateTime::from_temporal(&pdt)
}

/// Cast a `PlainDateTime` to a `timestamp` (without time zone).
///
/// Sub-microsecond precision (nanoseconds) is truncated, matching Temporal's
/// own `epochMicroseconds` truncation semantics. The nanosecond remainder is
/// dropped before the float conversion to prevent PG's `make_timestamp` from
/// rounding at the ns→µs boundary.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaindatetime_to_timestamp(pdt: PlainDateTime) -> Timestamp {
    // Truncate sub-µs: subsecond_ns = ms*1_000_000 + µs*1_000 + ns.
    // Integer division by 1_000 drops the nanoseconds portion exactly.
    let subsecond_us = pdt.subsecond_ns / 1_000; // in microseconds, 0..999_999
    let second_with_frac = pdt.second as f64 + subsecond_us as f64 / 1_000_000.0;
    Timestamp::new(pdt.year, pdt.month, pdt.day, pdt.hour, pdt.minute, second_with_frac)
        .unwrap_or_else(|e| error!("plaindatetime_to_timestamp: out of range: {e:?}"))
}

extension_sql!(
    r"
    CREATE CAST (timestamp AS PlainDateTime)
        WITH FUNCTION timestamp_to_plaindatetime(timestamp);
    CREATE CAST (PlainDateTime AS timestamp)
        WITH FUNCTION plaindatetime_to_timestamp(PlainDateTime);
    ",
    name = "plain_datetime_casts",
    requires = [timestamp_to_plaindatetime, plaindatetime_to_timestamp],
);
