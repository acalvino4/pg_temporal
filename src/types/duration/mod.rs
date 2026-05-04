use pgrx::prelude::*;
use pgrx::Internal;
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
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_years(d: Duration) -> i64 {
    d.years
}

/// Returns the months component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_months(d: Duration) -> i64 {
    d.months
}

/// Returns the weeks component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_weeks(d: Duration) -> i64 {
    d.weeks
}

/// Returns the days component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_days(d: Duration) -> i64 {
    d.days
}

/// Returns the hours component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_hours(d: Duration) -> i64 {
    d.hours
}

/// Returns the minutes component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_minutes(d: Duration) -> i64 {
    d.minutes
}

/// Returns the seconds component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_seconds(d: Duration) -> i64 {
    d.seconds
}

/// Returns the milliseconds component (signed).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_milliseconds(d: Duration) -> i64 {
    d.milliseconds
}

/// Returns the microseconds component as text (i128 has no native SQL type;
/// use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_microseconds(d: Duration) -> String {
    let us = d.microseconds;
    us.to_string()
}

/// Returns the nanoseconds component as text (i128 has no native SQL type;
/// use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_negated(d: Duration) -> Duration {
    Duration::from_temporal(&d.to_temporal().negated())
}

/// Returns a copy with all components made non-negative.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_abs(d: Duration) -> Duration {
    Duration::from_temporal(&d.to_temporal().abs())
}

/// Returns -1, 0, or 1 indicating the overall sign of the duration.
///
/// A valid duration has uniform sign (all non-zero components share the same
/// sign), so the overall sign equals the sign of the first non-zero field.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
/// months, weeks, or days) — use `plaindatetime_add` or `zoneddatetime_add`
/// to add durations that include calendar components.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
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
/// months, weeks, or days) — use `plaindatetime_subtract` or
/// `zoneddatetime_subtract` to subtract durations that include calendar
/// components.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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
#[pg_extern(immutable, parallel_safe, strict)]
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

// ---------------------------------------------------------------------------
// Comparison
// ---------------------------------------------------------------------------

/// Compare two durations, returning -1, 0, or 1.
///
/// Mirrors `Temporal.Duration.compare(one, two)`. Works for time-only
/// durations and durations whose only calendar component is `days` (treated
/// as fixed 24-hour days). Durations with years, months, or weeks require a
/// reference point — use `duration_compare_zoned` or `duration_compare_plain`.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_compare(a: Duration, b: Duration) -> i32 {
    let ord = a
        .to_temporal()
        .compare_with_provider(&b.to_temporal(), None, &*TZ_PROVIDER)
        .unwrap_or_else(|e| {
            error!(
                "duration_compare: durations with calendar components (years, months, weeks) \
                 require a reference point; use duration_compare_zoned or duration_compare_plain: {e}"
            )
        });
    match ord {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Compare two durations anchored to a `ZonedDateTime`, returning -1, 0, or 1.
///
/// Mirrors `Temporal.Duration.compare(one, two, { relativeTo: zonedDateTime })`.
/// Use this when either duration contains calendar components (years, months,
/// weeks, or days) and DST-aware day lengths are relevant.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_compare_zoned(a: Duration, b: Duration, relative_to: ZonedDateTime) -> i32 {
    let rel = RelativeTo::from(relative_to.to_temporal());
    let ord = a
        .to_temporal()
        .compare_with_provider(&b.to_temporal(), Some(rel), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_compare_zoned failed: {e}"));
    match ord {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Compare two durations anchored to a `PlainDateTime`, returning -1, 0, or 1.
///
/// Mirrors `Temporal.Duration.compare(one, two, { relativeTo: plainDateTime })`.
/// Use this when either duration contains calendar components (years, months,
/// or weeks) and timezone-aware day lengths are not needed.
#[allow(clippy::needless_pass_by_value)] // pgrx requires by-value for PostgresType params
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_compare_plain(a: Duration, b: Duration, relative_to: PlainDateTime) -> i32 {
    let plain_date = relative_to.to_temporal().to_plain_date();
    let rel = RelativeTo::from(plain_date);
    let ord = a
        .to_temporal()
        .compare_with_provider(&b.to_temporal(), Some(rel), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("duration_compare_plain failed: {e}"));
    match ord {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

// ---------------------------------------------------------------------------
// Explicit casts: interval ↔ Duration
// ---------------------------------------------------------------------------

/// Cast a PostgreSQL `interval` to a `Duration`.
///
/// `interval` stores three fields: `months`, `days`, and `microseconds`.
/// Months map directly; the sub-day time is expanded into hours, minutes,
/// seconds, and microseconds, preserving the sign of the value.
///
/// A PostgreSQL `interval` can have fields of mixed sign (e.g.,
/// `'1 month -2 days'::interval`), which is not a valid Temporal Duration.
/// Such values are rejected with an error.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn interval_to_duration(iv: Interval) -> Duration {
    let months = iv.months() as i64;
    let days = iv.days() as i64;
    let total_us = iv.micros(); // i64, can be negative
    // Split into hours / minutes / seconds / microseconds with the same sign.
    let (hours, rem_us) = (total_us / 3_600_000_000, total_us % 3_600_000_000);
    let (minutes, rem_us) = (rem_us / 60_000_000, rem_us % 60_000_000);
    let (seconds, rem_us) = (rem_us / 1_000_000, rem_us % 1_000_000);
    // Route through TemporalDuration::new() for sign-uniformity validation.
    let td = TemporalDuration::new(0, months, 0, days, hours, minutes, seconds, 0, rem_us as i128, 0)
        .unwrap_or_else(|e| error!("interval_to_duration: mixed-sign interval is not a valid Temporal Duration: {e}"));
    Duration::from_temporal(&td)
}

/// Cast a `Duration` to a PostgreSQL `interval`.
///
/// The Temporal vector is collapsed:
///   - `years × 12 + months` → `interval` months field
///   - `weeks × 7 + days`    → `interval` days field
///   - remaining time fields  → `interval` microseconds (nanoseconds truncated)
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn duration_to_interval(d: Duration) -> Interval {
    let months: i32 = d.years
        .checked_mul(12)
        .and_then(|y| y.checked_add(d.months))
        .and_then(|m| i32::try_from(m).ok())
        .unwrap_or_else(|| error!("duration_to_interval: months value out of range for interval"));
    let days: i32 = d.weeks
        .checked_mul(7)
        .and_then(|w| w.checked_add(d.days))
        .and_then(|d| i32::try_from(d).ok())
        .unwrap_or_else(|| error!("duration_to_interval: days value out of range for interval"));
    let micros: i64 = (|| -> Option<i64> {
        let h = d.hours.checked_mul(3_600_000_000)?;
        let m = d.minutes.checked_mul(60_000_000)?;
        let s = d.seconds.checked_mul(1_000_000)?;
        let ms = d.milliseconds.checked_mul(1_000)?;
        let us = i64::try_from(d.microseconds).ok()?;
        let ns = i64::try_from(d.nanoseconds / 1_000).ok()?;
        h.checked_add(m)?.checked_add(s)?.checked_add(ms)?.checked_add(us)?.checked_add(ns)
    })()
    .unwrap_or_else(|| error!("duration_to_interval: time value out of range for interval"));
    Interval::new(months, days, micros)
        .unwrap_or_else(|e| error!("duration out of range for interval: {e:?}"))
}

extension_sql!(
    r"
    CREATE CAST (interval AS Duration)
        WITH FUNCTION interval_to_duration(interval);
    CREATE CAST (Duration AS interval)
        WITH FUNCTION duration_to_interval(Duration);
    ",
    name = "duration_casts",
    requires = [interval_to_duration, duration_to_interval],
);

// ---------------------------------------------------------------------------
// Binary send / recv
// ---------------------------------------------------------------------------

/// Serialize a `Duration` to the binary wire format.
///
/// Wire format (96 bytes, all big-endian):
///   bytes 0–7: `years` (i64), bytes 8–15: `months` (i64),
///   bytes 16–23: `weeks` (i64), bytes 24–31: `days` (i64),
///   bytes 32–39: `hours` (i64), bytes 40–47: `minutes` (i64),
///   bytes 48–55: `seconds` (i64), bytes 56–63: `milliseconds` (i64),
///   bytes 64–79: `microseconds` (i128), bytes 80–95: `nanoseconds` (i128)
#[must_use]
#[pg_extern(immutable, strict)]
pub fn duration_send(val: Duration) -> Vec<u8> {
    // Copy packed struct fields to the stack to avoid unaligned references.
    let years        = val.years;
    let months       = val.months;
    let weeks        = val.weeks;
    let days         = val.days;
    let hours        = val.hours;
    let minutes      = val.minutes;
    let seconds      = val.seconds;
    let milliseconds = val.milliseconds;
    let microseconds = val.microseconds;
    let nanoseconds  = val.nanoseconds;
    let mut buf = Vec::with_capacity(96);
    buf.extend_from_slice(&years.to_be_bytes());
    buf.extend_from_slice(&months.to_be_bytes());
    buf.extend_from_slice(&weeks.to_be_bytes());
    buf.extend_from_slice(&days.to_be_bytes());
    buf.extend_from_slice(&hours.to_be_bytes());
    buf.extend_from_slice(&minutes.to_be_bytes());
    buf.extend_from_slice(&seconds.to_be_bytes());
    buf.extend_from_slice(&milliseconds.to_be_bytes());
    buf.extend_from_slice(&microseconds.to_be_bytes());
    buf.extend_from_slice(&nanoseconds.to_be_bytes());
    buf
}

/// Deserialize a `Duration` from the binary wire format.
///
/// Expects 96 bytes in the order described for `duration_send`.
#[must_use]
#[pg_extern(immutable, strict)]
pub fn duration_recv(internal: Internal) -> Duration {
    let buf = internal
        .unwrap()
        .unwrap_or_else(|| error!("duration_recv: null internal"))
        .cast_mut_ptr::<pgrx::pg_sys::StringInfoData>();
    let years        = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let months       = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let weeks        = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let days         = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let hours        = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let minutes      = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let seconds      = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let milliseconds = unsafe { pgrx::pg_sys::pq_getmsgint64(buf) };
    let mut us_bytes = [0u8; 16];
    let mut ns_bytes = [0u8; 16];
    unsafe {
        pgrx::pg_sys::pq_copymsgbytes(buf, us_bytes.as_mut_ptr() as *mut _, 16);
        pgrx::pg_sys::pq_copymsgbytes(buf, ns_bytes.as_mut_ptr() as *mut _, 16);
    }
    let microseconds = i128::from_be_bytes(us_bytes);
    let nanoseconds  = i128::from_be_bytes(ns_bytes);
    Duration { years, months, weeks, days, hours, minutes, seconds, milliseconds, microseconds, nanoseconds }
}

extension_sql!(
    r"ALTER TYPE Duration SET (SEND = duration_send, RECEIVE = duration_recv);",
    name = "duration_send_recv",
    requires = [duration_send, duration_recv],
);
