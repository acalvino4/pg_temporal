use pgrx::prelude::*;
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
// All fields are signed; the Temporal validity rule guarantees that all
// non-zero components share the same sign. Field types mirror those used
// by temporal_rs (i64 for years–milliseconds, i128 for µs/ns).
//
//   years .. milliseconds – i64
//   microseconds, nanoseconds – i128
//
// Layout: 8×8 + 2×16 = 96 bytes.
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PostgresType)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct Duration {
    pub(crate) years: i64,
    pub(crate) months: i64,
    pub(crate) weeks: i64,
    pub(crate) days: i64,
    pub(crate) hours: i64,
    pub(crate) minutes: i64,
    pub(crate) seconds: i64,
    pub(crate) milliseconds: i64,
    pub(crate) microseconds: i128,
    pub(crate) nanoseconds: i128,
}

// ---------------------------------------------------------------------------
// Manual IntoDatum / FromDatum / BoxRet / ArgAbi / UnboxDatum
//
// The Serde/CBOR path is intentionally bypassed: pgrx's default
// PostgresType derive uses CBOR serialization, but all on-disk datums
// here are compact binary via PgVarlena.
// ---------------------------------------------------------------------------

impl pgrx::datum::IntoDatum for Duration {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for Duration {
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

unsafe impl pgrx::callconv::BoxRet for Duration {
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

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for Duration
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

unsafe impl pgrx::datum::UnboxDatum for Duration {
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

impl PgVarlenaInOutFuncs for Duration {
    /// Parse an ISO 8601 duration string into a `Duration` datum.
    ///
    /// Example inputs:
    ///   `P1Y2M3DT4H5M6S`
    ///   `PT0.000000001S`
    ///   `-P1Y`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s = input.to_str().unwrap_or_else(|_| error!("duration input is not valid UTF-8"));

        let d = TemporalDuration::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid duration \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        *result = Duration::from_temporal(&d);
        result
    }

    /// Serialize a `Duration` datum back to an ISO 8601 duration string.
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        // Copy the packed struct to the stack to avoid unaligned references.
        let this = *self;
        let s = this
            .to_temporal()
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
    let us = d.microseconds;
    us.to_string()
}

/// Returns the nanoseconds component as text (i128 has no native SQL type;
/// use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_nanoseconds(d: Duration) -> String {
    let ns = d.nanoseconds;
    ns.to_string()
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
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn duration_sign(d: Duration) -> i32 {
    for v in [d.years, d.months, d.weeks, d.days, d.hours, d.minutes, d.seconds, d.milliseconds] {
        if v != 0 {
            return v.signum() as i32;
        }
    }
    for v in [d.microseconds, d.nanoseconds] {
        if v != 0 {
            return v.signum() as i32;
        }
    }
    0
}

/// Returns true if all components of the duration are zero.
/// Equivalent to Temporal's `Duration.blank`.
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
