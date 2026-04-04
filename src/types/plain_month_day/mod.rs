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
    Calendar, PlainMonthDay as TemporalPmd,
    options::{DisplayCalendar, Overflow},
};

// ---------------------------------------------------------------------------
// Storage type
//
// A PlainMonthDay is a calendar-local month and day with no year.
// Useful for recurring annual events (birthdays, holidays, anniversaries).
//
//   iso_year – reference ISO year used internally by temporal_rs
//              (defaults to 1972 — the first ISO 8601 leap year — for ISO
//              calendar so that Feb 29 is representable)
//   month    – ISO 8601 month (1–12)
//   day      – ISO 8601 day-of-month
//   cal_idx  – compact calendar index (see cal_index module)
//
// Layout: i32 + 3×u8 = 7 bytes.
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PostgresType)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct PlainMonthDay {
    pub(crate) iso_year: i32,
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

impl pgrx::datum::IntoDatum for PlainMonthDay {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for PlainMonthDay {
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

unsafe impl pgrx::callconv::BoxRet for PlainMonthDay {
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

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for PlainMonthDay
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

unsafe impl pgrx::datum::UnboxDatum for PlainMonthDay {
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

impl PgVarlenaInOutFuncs for PlainMonthDay {
    /// Parse an IXDTF plain month-day string into a `PlainMonthDay` datum.
    ///
    /// Example inputs:
    ///   `06-15`
    ///   `--06-15`
    ///   `1972-06-15[u-ca=iso8601]`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s =
            input.to_str().unwrap_or_else(|_| error!("plain_month_day input is not valid UTF-8"));

        let pmd = TemporalPmd::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid plain_month_day \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        *result = PlainMonthDay::from_temporal(&pmd);
        result
    }

    /// Serialize a `PlainMonthDay` datum back to an IXDTF string.
    ///
    /// The calendar annotation is omitted for ISO 8601 (`DisplayCalendar::Auto`).
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let this = *self;
        let pmd = this.to_temporal();
        let s = pmd.to_ixdtf_string(DisplayCalendar::default());
        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Constructor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Construct a `PlainMonthDay` from month and day values.
///
/// `cal` is optional and defaults to `'iso8601'`.
///
/// Example:
/// ```sql
/// SELECT make_plainmonthday(6, 15);
/// SELECT make_plainmonthday(2, 29);  -- Feb 29 (leap day)
/// ```
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn make_plainmonthday(
    month: i32,
    day: i32,
    cal: default!(&str, "'iso8601'"),
) -> PlainMonthDay {
    let month = u8::try_from(month)
        .unwrap_or_else(|_| error!("make_plainmonthday: invalid month {month}"));
    let day =
        u8::try_from(day).unwrap_or_else(|_| error!("make_plainmonthday: invalid day {day}"));
    let calendar = Calendar::try_from_utf8(cal.as_bytes())
        .unwrap_or_else(|e| error!("make_plainmonthday: invalid calendar \"{cal}\": {e}"));
    let cal_id = calendar.identifier();
    let cal_idx = crate::cal_index::index_of(cal_id)
        .unwrap_or_else(|| error!("make_plainmonthday: unsupported calendar \"{cal_id}\""));
    let pmd = TemporalPmd::new_with_overflow(month, day, calendar, Overflow::Reject, None)
        .unwrap_or_else(|e| error!("make_plainmonthday: {e}"));
    let iso_year = pmd.iso.year;
    PlainMonthDay { iso_year, month: pmd.iso.month, day: pmd.iso.day, cal_idx }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the ISO month (1–12). For most calendars this equals the calendar
/// month number; the ISO field is always accessible since `pmd.iso` is public.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_month(pmd: PlainMonthDay) -> i32 {
    i32::from(pmd.month)
}

/// Returns the ISO day-of-month.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_day(pmd: PlainMonthDay) -> i32 {
    i32::from(pmd.day)
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_calendar(pmd: PlainMonthDay) -> String {
    crate::cal_index::name_of(pmd.cal_idx)
        .unwrap_or_else(|| {
            error!("plain_month_day_calendar: unknown calendar index {}", pmd.cal_idx)
        })
        .to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl PlainMonthDay {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    pub(crate) fn to_temporal(self) -> TemporalPmd {
        let cal_id = crate::cal_index::name_of(self.cal_idx)
            .unwrap_or_else(|| error!("unknown calendar index {}", self.cal_idx));
        let cal = Calendar::try_from_utf8(cal_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{cal_id}\": {e}"));
        TemporalPmd::new_with_overflow(
            self.month,
            self.day,
            cal,
            Overflow::Reject,
            Some(self.iso_year),
        )
        .unwrap_or_else(|e| error!("failed to reconstruct plain_month_day: {e}"))
    }

    /// Build a `PlainMonthDay` from a `temporal_rs` plain month-day.
    ///
    /// `PlainMonthDay`'s `iso` field is publicly accessible, so we can
    /// read the raw ISO fields directly.
    pub(crate) fn from_temporal(pmd: &TemporalPmd) -> Self {
        let cal_id = pmd.calendar().identifier();
        let cal_idx = crate::cal_index::index_of(cal_id)
            .unwrap_or_else(|| error!("unsupported calendar: {cal_id}"));
        Self {
            iso_year: pmd.iso.year,
            month: pmd.iso.month,
            day: pmd.iso.day,
            cal_idx,
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison functions
//
// PlainMonthDay is ordered by ISO month/day (ignoring the reference year),
// with calendar index as a tiebreaker.
// ---------------------------------------------------------------------------

/// Returns -1, 0, or 1 comparing two plain month-days by ISO month/day fields
/// and, as a tiebreaker, by calendar index.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_compare(a: PlainMonthDay, b: PlainMonthDay) -> i32 {
    match (a.month, a.day)
        .cmp(&(b.month, b.day))
        .then_with(|| a.cal_idx.cmp(&b.cal_idx))
    {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_lt(a: PlainMonthDay, b: PlainMonthDay) -> bool {
    plain_month_day_compare(a, b) < 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_le(a: PlainMonthDay, b: PlainMonthDay) -> bool {
    plain_month_day_compare(a, b) <= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_eq(a: PlainMonthDay, b: PlainMonthDay) -> bool {
    plain_month_day_compare(a, b) == 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_ne(a: PlainMonthDay, b: PlainMonthDay) -> bool {
    plain_month_day_compare(a, b) != 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_ge(a: PlainMonthDay, b: PlainMonthDay) -> bool {
    plain_month_day_compare(a, b) >= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_month_day_gt(a: PlainMonthDay, b: PlainMonthDay) -> bool {
    plain_month_day_compare(a, b) > 0
}

extension_sql!(
    r"
    CREATE OPERATOR < (
        LEFTARG = PlainMonthDay, RIGHTARG = PlainMonthDay,
        FUNCTION = plain_month_day_lt,
        COMMUTATOR = >, NEGATOR = >=
    );
    CREATE OPERATOR <= (
        LEFTARG = PlainMonthDay, RIGHTARG = PlainMonthDay,
        FUNCTION = plain_month_day_le,
        COMMUTATOR = >=, NEGATOR = >
    );
    CREATE OPERATOR = (
        LEFTARG = PlainMonthDay, RIGHTARG = PlainMonthDay,
        FUNCTION = plain_month_day_eq,
        COMMUTATOR = =, NEGATOR = <>
    );
    CREATE OPERATOR <> (
        LEFTARG = PlainMonthDay, RIGHTARG = PlainMonthDay,
        FUNCTION = plain_month_day_ne,
        COMMUTATOR = <>, NEGATOR = =
    );
    CREATE OPERATOR >= (
        LEFTARG = PlainMonthDay, RIGHTARG = PlainMonthDay,
        FUNCTION = plain_month_day_ge,
        COMMUTATOR = <=, NEGATOR = <
    );
    CREATE OPERATOR > (
        LEFTARG = PlainMonthDay, RIGHTARG = PlainMonthDay,
        FUNCTION = plain_month_day_gt,
        COMMUTATOR = <, NEGATOR = <=
    );
    CREATE OPERATOR CLASS plain_month_day_btree_ops DEFAULT FOR TYPE PlainMonthDay USING btree AS
        OPERATOR 1  <,
        OPERATOR 2  <=,
        OPERATOR 3  =,
        OPERATOR 4  >=,
        OPERATOR 5  >,
        FUNCTION 1  plain_month_day_compare(PlainMonthDay, PlainMonthDay);
    ",
    name = "plain_month_day_comparison_operators",
    requires = [
        plain_month_day_lt,
        plain_month_day_le,
        plain_month_day_eq,
        plain_month_day_ne,
        plain_month_day_ge,
        plain_month_day_gt
    ],
);
