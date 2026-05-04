// pgrx requires all custom PostgresType parameters in #[pg_extern] functions to be
// passed by value — references are not supported (`BorrowDatum`/`ArgAbi` are not
// implemented for user-defined types). The needless_pass_by_value lint correctly
// identifies that many of these functions don't need ownership, but they must
// take by value due to this pgrx constraint.
#![allow(clippy::needless_pass_by_value)]

use pgrx::prelude::*;
use std::ffi::CStr;
use temporal_rs::{
    Calendar, PlainYearMonth as TemporalPym,
    options::{DifferenceSettings, DisplayCalendar, Overflow},
};

use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// A PlainYearMonth is a calendar-local year and month with no day or time.
// Useful for billing periods, reporting intervals, and recurring monthly events.
//
//   year     – ISO 8601 year (stored as the ISO year from the canonical
//              IXDTF representation; may differ from the calendar year)
//   month    – ISO 8601 month (1–12), similarly canonical ISO month
//   cal_idx  – compact calendar index (see cal_index module)
//
// Layout: i32 + 2×u8 = 6 bytes.
// Field declaration order must match intended sort priority — #[derive(Ord)] depends on it.
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, PostgresType, PostgresEq, PostgresOrd)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct PlainYearMonth {
    pub(crate) year: i32,
    pub(crate) month: u8,
    pub(crate) cal_idx: u8,
}

// ---------------------------------------------------------------------------
// Manual IntoDatum / FromDatum / BoxRet / ArgAbi / UnboxDatum
//
// The Serde/CBOR path is intentionally bypassed: pgrx's default
// PostgresType derive uses CBOR serialization, but all on-disk datums
// here are compact binary via PgVarlena.
// ---------------------------------------------------------------------------

impl pgrx::datum::IntoDatum for PlainYearMonth {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for PlainYearMonth {
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

unsafe impl pgrx::callconv::BoxRet for PlainYearMonth {
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

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for PlainYearMonth
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

unsafe impl pgrx::datum::UnboxDatum for PlainYearMonth {
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

impl PgVarlenaInOutFuncs for PlainYearMonth {
    /// Parse an IXDTF plain year-month string into a `PlainYearMonth` datum.
    ///
    /// Example inputs:
    ///   `2025-03`
    ///   `2025-03[u-ca=iso8601]`
    ///   `2025-03[u-ca=persian]`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s =
            input.to_str().unwrap_or_else(|_| error!("plain_year_month input is not valid UTF-8"));

        let pym = TemporalPym::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid plain_year_month \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        *result = PlainYearMonth::from_temporal(&pym);
        result
    }

    /// Serialize a `PlainYearMonth` datum back to an IXDTF string.
    ///
    /// The calendar annotation is omitted for ISO 8601 (`DisplayCalendar::Auto`).
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let this = *self;
        let pym = this.to_temporal();
        let s = pym.to_ixdtf_string(DisplayCalendar::default());
        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Constructor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Construct a `PlainYearMonth` from year and month values.
///
/// `cal` is optional and defaults to `'iso8601'`.
///
/// Example:
/// ```sql
/// SELECT make_plainyearmonth(2025, 3);
/// SELECT make_plainyearmonth(2025, 3, 'persian');
/// ```
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn make_plainyearmonth(
    year: i32,
    month: i32,
    cal: default!(&str, "'iso8601'"),
) -> PlainYearMonth {
    let month = u8::try_from(month)
        .unwrap_or_else(|_| error!("make_plainyearmonth: invalid month {month}"));
    let calendar = Calendar::try_from_utf8(cal.as_bytes())
        .unwrap_or_else(|e| error!("make_plainyearmonth: invalid calendar \"{cal}\": {e}"));
    let cal_id = calendar.identifier();
    let cal_idx = crate::cal_index::index_of(cal_id)
        .unwrap_or_else(|| error!("make_plainyearmonth: unsupported calendar \"{cal_id}\""));
    TemporalPym::try_new(year, month, None, calendar)
        .unwrap_or_else(|e| error!("make_plainyearmonth: {e}"));
    PlainYearMonth { year, month, cal_idx }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the calendar year.
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_year_month_year(pym: PlainYearMonth) -> i32 {
    pym.to_temporal().year()
}

/// Returns the calendar month (1-indexed).
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn plain_year_month_month(pym: PlainYearMonth) -> i32 {
    i32::from(pym.to_temporal().month())
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_year_month_calendar(pym: PlainYearMonth) -> String {
    crate::cal_index::name_of(pym.cal_idx)
        .unwrap_or_else(|| {
            error!("plain_year_month_calendar: unknown calendar index {}", pym.cal_idx)
        })
        .to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl PlainYearMonth {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    ///
    /// Stored ISO year/month are passed directly to `try_new`, which
    /// stores them as the internal ISO date fields with the given calendar.
    pub(crate) fn to_temporal(self) -> TemporalPym {
        let cal_id = crate::cal_index::name_of(self.cal_idx)
            .unwrap_or_else(|| error!("unknown calendar index {}", self.cal_idx));
        let cal = Calendar::try_from_utf8(cal_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{cal_id}\": {e}"));
        TemporalPym::try_new(self.year, self.month, None, cal)
            .unwrap_or_else(|e| error!("failed to reconstruct plain_year_month: {e}"))
    }

    /// Build a `PlainYearMonth` from a `temporal_rs` plain year-month.
    ///
    /// Extracts the canonical ISO year/month by parsing the IXDTF string
    /// representation (which always leads with the ISO year-month), since
    /// the internal `iso_year()`/`iso_month()` fields are not pub outside
    /// the `temporal_rs` crate.
    pub(crate) fn from_temporal(pym: &TemporalPym) -> Self {
        let s = pym.to_ixdtf_string(DisplayCalendar::Never);
        // s is always "YYYY-MM" (ISO year-month, no calendar annotation)
        let (year_str, month_str) = s.rsplit_once('-').unwrap_or_else(|| {
            error!("plain_year_month from_temporal: unexpected ixdtf format \"{s}\"")
        });
        let year = year_str
            .trim_start_matches('+')
            .parse::<i32>()
            .unwrap_or_else(|_| {
                error!("plain_year_month from_temporal: invalid year \"{year_str}\"")
            });
        let month = month_str.parse::<u8>().unwrap_or_else(|_| {
            error!("plain_year_month from_temporal: invalid month \"{month_str}\"")
        });
        let cal_id = pym.calendar().identifier();
        let cal_idx = crate::cal_index::index_of(cal_id)
            .unwrap_or_else(|| error!("unsupported calendar: {cal_id}"));
        Self { year, month, cal_idx }
    }
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Add a duration to a plain year-month.
/// Only years and months components are accepted; days, weeks, hours, etc.
/// will cause an error per the Temporal spec.
/// Uses `Constrain` overflow.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_year_month_add(pym: PlainYearMonth, dur: Duration) -> PlainYearMonth {
    let result = pym
        .to_temporal()
        .add(&dur.to_temporal(), Overflow::Constrain)
        .unwrap_or_else(|e| error!("plain_year_month_add failed: {e}"));
    PlainYearMonth::from_temporal(&result)
}

/// Subtract a duration from a plain year-month.
/// Only years and months components are accepted.
/// Uses `Constrain` overflow.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_year_month_subtract(pym: PlainYearMonth, dur: Duration) -> PlainYearMonth {
    let result = pym
        .to_temporal()
        .subtract(&dur.to_temporal(), Overflow::Constrain)
        .unwrap_or_else(|e| error!("plain_year_month_subtract failed: {e}"));
    PlainYearMonth::from_temporal(&result)
}

/// Returns the duration elapsed from `other` to `pym` (default unit: months).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_year_month_since(pym: PlainYearMonth, other: PlainYearMonth) -> Duration {
    let d = pym
        .to_temporal()
        .since(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plain_year_month_since failed: {e}"));
    Duration::from_temporal(&d)
}

/// Returns the duration from `pym` to `other` (default unit: months).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn plain_year_month_until(pym: PlainYearMonth, other: PlainYearMonth) -> Duration {
    let d = pym
        .to_temporal()
        .until(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plain_year_month_until failed: {e}"));
    Duration::from_temporal(&d)
}
