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
    Calendar, PlainDate as TemporalPd,
    options::{DifferenceSettings, DisplayCalendar, Overflow},
};

use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// A PlainDate is a calendar-local date with no time or timezone.
//
//   year     – ISO 8601 year
//   month    – ISO 8601 month (1–12)
//   day      – ISO 8601 day-of-month
//   cal_idx  – compact calendar index (see cal_index module)
//
// Layout (field order chosen for alignment): i32 + 3×u8 = 7 bytes.
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PostgresType)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct PlainDate {
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) day: u8,
    pub(crate) cal_idx: u8,
}

// ---------------------------------------------------------------------------
// Manual IntoDatum / FromDatum / BoxRet / ArgAbi / UnboxDatum
//
// The Serde/CBOR path is intentionally bypassed: pgrx's default
// PostgresType derive uses CBOR serialization, but all on-disk datums
// here are compact binary via PgVarlena.
// ---------------------------------------------------------------------------

impl pgrx::datum::IntoDatum for PlainDate {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for PlainDate {
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

unsafe impl pgrx::callconv::BoxRet for PlainDate {
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

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for PlainDate
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

unsafe impl pgrx::datum::UnboxDatum for PlainDate {
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

impl PgVarlenaInOutFuncs for PlainDate {
    /// Parse an IXDTF plain date string into a `PlainDate` datum.
    ///
    /// Example inputs:
    ///   `2025-03-01`
    ///   `2025-03-01[u-ca=iso8601]`
    ///   `2025-03-01[u-ca=persian]`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s =
            input.to_str().unwrap_or_else(|_| error!("plain_date input is not valid UTF-8"));

        let pd = TemporalPd::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid plain_date \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        *result = PlainDate::from_temporal(&pd);
        result
    }

    /// Serialize a `PlainDate` datum back to an IXDTF string.
    ///
    /// The calendar annotation is omitted for ISO 8601 (`DisplayCalendar::Auto`).
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let this = *self;
        let pd = this.to_temporal();
        let s = pd.to_ixdtf_string(DisplayCalendar::default());
        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Constructor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Construct a `PlainDate` from individual field values.
///
/// `cal` is optional and defaults to `'iso8601'`.
///
/// Example:
/// ```sql
/// SELECT make_plaindate(2025, 6, 15);
/// SELECT make_plaindate(2025, 6, 15, 'persian');
/// ```
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn make_plaindate(
    year: i32,
    month: i32,
    day: i32,
    cal: default!(&str, "'iso8601'"),
) -> PlainDate {
    let month = u8::try_from(month)
        .unwrap_or_else(|_| error!("make_plaindate: invalid month {month}"));
    let day =
        u8::try_from(day).unwrap_or_else(|_| error!("make_plaindate: invalid day {day}"));
    let calendar = Calendar::try_from_utf8(cal.as_bytes())
        .unwrap_or_else(|e| error!("make_plaindate: invalid calendar \"{cal}\": {e}"));
    let cal_id = calendar.identifier();
    let cal_idx = crate::cal_index::index_of(cal_id)
        .unwrap_or_else(|| error!("make_plaindate: unsupported calendar \"{cal_id}\""));
    TemporalPd::try_new(year, month, day, calendar)
        .unwrap_or_else(|e| error!("make_plaindate: {e}"));
    PlainDate { year, month, day, cal_idx }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the calendar year (e.g. Persian 1403 for ISO 2025-03-01 with u-ca=persian).
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_date_year(pd: PlainDate) -> i32 {
    pd.to_temporal().year()
}

/// Returns the calendar month (1-indexed within the calendar system).
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_date_month(pd: PlainDate) -> i32 {
    i32::from(pd.to_temporal().month())
}

/// Returns the calendar day-of-month.
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_date_day(pd: PlainDate) -> i32 {
    i32::from(pd.to_temporal().day())
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_calendar(pd: PlainDate) -> String {
    crate::cal_index::name_of(pd.cal_idx)
        .unwrap_or_else(|| error!("plain_date_calendar: unknown calendar index {}", pd.cal_idx))
        .to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl PlainDate {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    pub(crate) fn to_temporal(self) -> TemporalPd {
        let cal_id = crate::cal_index::name_of(self.cal_idx)
            .unwrap_or_else(|| error!("unknown calendar index {}", self.cal_idx));
        let cal = Calendar::try_from_utf8(cal_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{cal_id}\": {e}"));
        TemporalPd::try_new(self.year, self.month, self.day, cal)
            .unwrap_or_else(|e| error!("failed to reconstruct plain_date: {e}"))
    }

    /// Build a `PlainDate` from a `temporal_rs` plain date.
    ///
    /// Always stores ISO 8601 fields. Since `PlainDate`'s `iso` field is not
    /// publicly accessible, we convert via `to_plain_date_time` (which has
    /// public `iso_year/month/day` accessors) to extract the raw ISO fields.
    pub(crate) fn from_temporal(pd: &TemporalPd) -> Self {
        let pdt = pd
            .to_plain_date_time(None)
            .unwrap_or_else(|e| error!("failed to convert PlainDate for storage: {e}"));
        let cal_id = pd.calendar().identifier();
        let cal_idx = crate::cal_index::index_of(cal_id)
            .unwrap_or_else(|| error!("unsupported calendar: {cal_id}"));
        Self {
            year: pdt.iso_year(),
            month: pdt.iso_month(),
            day: pdt.iso_day(),
            cal_idx,
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison functions
// ---------------------------------------------------------------------------

/// Returns -1, 0, or 1 comparing two plain dates by ISO date fields
/// and, as a tiebreaker, by calendar index.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_compare(a: PlainDate, b: PlainDate) -> i32 {
    match (a.year, a.month, a.day)
        .cmp(&(b.year, b.month, b.day))
        .then_with(|| a.cal_idx.cmp(&b.cal_idx))
    {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_lt(a: PlainDate, b: PlainDate) -> bool {
    plain_date_compare(a, b) < 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_le(a: PlainDate, b: PlainDate) -> bool {
    plain_date_compare(a, b) <= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_eq(a: PlainDate, b: PlainDate) -> bool {
    plain_date_compare(a, b) == 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_ne(a: PlainDate, b: PlainDate) -> bool {
    plain_date_compare(a, b) != 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_ge(a: PlainDate, b: PlainDate) -> bool {
    plain_date_compare(a, b) >= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_gt(a: PlainDate, b: PlainDate) -> bool {
    plain_date_compare(a, b) > 0
}

extension_sql!(
    r"
    CREATE OPERATOR < (
        LEFTARG = PlainDate, RIGHTARG = PlainDate,
        FUNCTION = plain_date_lt,
        COMMUTATOR = >, NEGATOR = >=
    );
    CREATE OPERATOR <= (
        LEFTARG = PlainDate, RIGHTARG = PlainDate,
        FUNCTION = plain_date_le,
        COMMUTATOR = >=, NEGATOR = >
    );
    CREATE OPERATOR = (
        LEFTARG = PlainDate, RIGHTARG = PlainDate,
        FUNCTION = plain_date_eq,
        COMMUTATOR = =, NEGATOR = <>
    );
    CREATE OPERATOR <> (
        LEFTARG = PlainDate, RIGHTARG = PlainDate,
        FUNCTION = plain_date_ne,
        COMMUTATOR = <>, NEGATOR = =
    );
    CREATE OPERATOR >= (
        LEFTARG = PlainDate, RIGHTARG = PlainDate,
        FUNCTION = plain_date_ge,
        COMMUTATOR = <=, NEGATOR = <
    );
    CREATE OPERATOR > (
        LEFTARG = PlainDate, RIGHTARG = PlainDate,
        FUNCTION = plain_date_gt,
        COMMUTATOR = <, NEGATOR = <=
    );
    CREATE OPERATOR CLASS plain_date_btree_ops DEFAULT FOR TYPE PlainDate USING btree AS
        OPERATOR 1  <,
        OPERATOR 2  <=,
        OPERATOR 3  =,
        OPERATOR 4  >=,
        OPERATOR 5  >,
        FUNCTION 1  plain_date_compare(PlainDate, PlainDate);
    ",
    name = "plain_date_comparison_operators",
    requires = [
        plain_date_lt,
        plain_date_le,
        plain_date_eq,
        plain_date_ne,
        plain_date_ge,
        plain_date_gt
    ],
);

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Add a duration to a plain date.
/// Uses `Constrain` overflow: day-of-month is clamped to the last valid day.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_add(pd: PlainDate, dur: Duration) -> PlainDate {
    let result = pd
        .to_temporal()
        .add(&dur.to_temporal(), Some(Overflow::Constrain))
        .unwrap_or_else(|e| error!("plain_date_add failed: {e}"));
    PlainDate::from_temporal(&result)
}

/// Subtract a duration from a plain date.
/// Uses `Constrain` overflow: day-of-month is clamped to the last valid day.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_subtract(pd: PlainDate, dur: Duration) -> PlainDate {
    let result = pd
        .to_temporal()
        .subtract(&dur.to_temporal(), Some(Overflow::Constrain))
        .unwrap_or_else(|e| error!("plain_date_subtract failed: {e}"));
    PlainDate::from_temporal(&result)
}

/// Returns the duration elapsed from `other` to `pd` (default unit: days).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_since(pd: PlainDate, other: PlainDate) -> Duration {
    let d = pd
        .to_temporal()
        .since(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plain_date_since failed: {e}"));
    Duration::from_temporal(&d)
}

/// Returns the duration from `pd` to `other` (default unit: days).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_date_until(pd: PlainDate, other: PlainDate) -> Duration {
    let d = pd
        .to_temporal()
        .until(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plain_date_until failed: {e}"));
    Duration::from_temporal(&d)
}

// ---------------------------------------------------------------------------
// Explicit casts: date ↔ PlainDate
// ---------------------------------------------------------------------------

/// Cast a `date` to a `PlainDate`.
///
/// The ISO 8601 calendar is assigned. Range and field validity are delegated
/// to `temporal_rs`.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn date_to_plaindate(d: Date) -> PlainDate {
    let pd = TemporalPd::try_new_iso(d.year(), d.month(), d.day())
        .unwrap_or_else(|e| error!("date_to_plaindate: {e}"));
    PlainDate::from_temporal(&pd)
}

/// Cast a `PlainDate` to a `date`.
///
/// The calendar annotation is dropped; ISO 8601 fields are used.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaindate_to_date(pd: PlainDate) -> Date {
    Date::new(pd.year, pd.month, pd.day)
        .unwrap_or_else(|e| error!("plaindate_to_date: out of range: {e:?}"))
}

extension_sql!(
    r"
    CREATE CAST (date AS PlainDate)
        WITH FUNCTION date_to_plaindate(date);
    CREATE CAST (PlainDate AS date)
        WITH FUNCTION plaindate_to_date(PlainDate);
    ",
    name = "plain_date_casts",
    requires = [date_to_plaindate, plaindate_to_date],
);
