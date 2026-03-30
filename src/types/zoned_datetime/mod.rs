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
    Calendar, TimeZone, ZonedDateTime as TemporalZdt,
    options::{
        DifferenceSettings, DisplayCalendar, DisplayOffset, DisplayTimeZone, OffsetDisambiguation,
        Overflow, ToStringRoundingOptions,
    },
};

use crate::gucs;
use crate::provider::TZ_PROVIDER;
use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// This struct is what PostgreSQL physically stores for each zoned_datetime
// value.  It is wrapped in a pgrx varlena by the #[derive(PostgresType)]
// machinery.
//
//   epoch_ns  – nanoseconds since Unix epoch (same as Temporal's
//               epochNanoseconds). i128 gives us the full ±292-year
//               range at nanosecond precision.
//   tz_idx    – compact index into the canonical IANA timezone list
//               (see tz_index module; generated at build time)
//   cal_idx   – compact calendar index (see cal_index module)
//
// Layout: 16 + 2 + 1 = 19 bytes.
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PostgresType)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct ZonedDateTime {
    pub(crate) epoch_ns: i128,
    pub(crate) tz_idx: u16,
    pub(crate) cal_idx: u8,
}

// ---------------------------------------------------------------------------
// Manual IntoDatum / FromDatum / BoxRet / ArgAbi / UnboxDatum
//
// The Serde/CBOR path is intentionally bypassed: pgrx's default
// PostgresType derive uses CBOR serialization, but all on-disk datums
// here are compact binary via PgVarlena.
// ---------------------------------------------------------------------------

impl pgrx::datum::IntoDatum for ZonedDateTime {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for ZonedDateTime {
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

unsafe impl pgrx::callconv::BoxRet for ZonedDateTime {
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

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for ZonedDateTime
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

unsafe impl pgrx::datum::UnboxDatum for ZonedDateTime {
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

impl PgVarlenaInOutFuncs for ZonedDateTime {
    /// Parse an IXDTF/RFC-9557 string into a `ZonedDateTime` datum.
    ///
    /// Example input: `2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=iso8601]`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s =
            input.to_str().unwrap_or_else(|_| error!("zoned_datetime input is not valid UTF-8"));

        let disambiguation = gucs::default_disambiguation();

        let zdt =
            TemporalZdt::from_utf8(s.as_bytes(), disambiguation, OffsetDisambiguation::Reject)
                .unwrap_or_else(|e| error!("invalid zoned_datetime \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        *result = ZonedDateTime::from_temporal(&zdt);
        result
    }

    /// Serialize a `ZonedDateTime` datum back to an IXDTF string.
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        // Copy the packed struct to the stack to avoid unaligned references.
        let this = *self;
        let tz_idx = this.tz_idx;
        let cal_idx = this.cal_idx;
        let epoch_ns = this.epoch_ns;
        let tz_id = crate::tz_index::name_of(tz_idx)
            .unwrap_or_else(|| error!("unknown timezone index {tz_idx}"));
        let tz = TimeZone::try_from_str_with_provider(tz_id, &*TZ_PROVIDER)
            .unwrap_or_else(|e| error!("failed to load timezone \"{tz_id}\": {e}"));
        let cal_id = crate::cal_index::name_of(cal_idx)
            .unwrap_or_else(|| error!("unknown calendar index {cal_idx}"));
        let cal = Calendar::try_from_utf8(cal_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{cal_id}\": {e}"));
        let zdt = TemporalZdt::try_new_with_provider(epoch_ns, tz, cal, &*TZ_PROVIDER)
            .unwrap_or_else(|e| error!("failed to reconstruct zoned_datetime: {e}"));
        let s = zdt
            .to_ixdtf_string_with_provider(
                DisplayOffset::default(),
                DisplayTimeZone::default(),
                DisplayCalendar::default(),
                ToStringRoundingOptions::default(),
                &*TZ_PROVIDER,
            )
            .unwrap_or_else(|e| error!("failed to format zoned_datetime: {e}"));
        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Constructor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Construct a `ZonedDateTime` from a nanosecond epoch, an IANA timezone
/// identifier, and a calendar identifier.
///
/// `epoch_ns` is supplied as `text` because i128 has no native SQL type.
///
/// Example:
/// ```sql
/// SELECT make_zoneddatetime('1609459200000000000', 'America/New_York', 'iso8601');
/// ```
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn make_zoneddatetime(epoch_ns: &str, tz: &str, cal: &str) -> ZonedDateTime {
    let ns: i128 = epoch_ns.trim().parse().unwrap_or_else(|_| {
        error!("make_zoneddatetime: invalid epoch_ns \"{epoch_ns}\": expected an integer")
    });
    let timezone = TimeZone::try_from_str_with_provider(tz, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("make_zoneddatetime: invalid timezone \"{tz}\": {e}"));
    let tz_id = timezone
        .identifier()
        .unwrap_or_else(|e| error!("make_zoneddatetime: failed to get timezone identifier: {e}"));
    let tz_idx = crate::tz_index::index_of(&tz_id)
        .unwrap_or_else(|| error!("make_zoneddatetime: unsupported timezone \"{tz_id}\""));
    let calendar = Calendar::try_from_utf8(cal.as_bytes())
        .unwrap_or_else(|e| error!("make_zoneddatetime: invalid calendar \"{cal}\": {e}"));
    let cal_id = calendar.identifier();
    let cal_idx = crate::cal_index::index_of(cal_id)
        .unwrap_or_else(|| error!("make_zoneddatetime: unsupported calendar \"{cal_id}\""));
    ZonedDateTime { epoch_ns: ns, tz_idx, cal_idx }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the timezone name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_timezone(zdt: ZonedDateTime) -> String {
    let tz_idx = zdt.tz_idx;
    crate::tz_index::name_of(tz_idx)
        .unwrap_or_else(|| error!("zoned_datetime_timezone: unknown timezone index {tz_idx}"))
        .to_string()
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_calendar(zdt: ZonedDateTime) -> String {
    let cal_idx = zdt.cal_idx;
    crate::cal_index::name_of(cal_idx)
        .unwrap_or_else(|| error!("zoned_datetime_calendar: unknown calendar index {cal_idx}"))
        .to_string()
}

/// Returns the UTC epoch in nanoseconds as a text value (i128 has no native
/// SQL type; use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_epoch_ns(zdt: ZonedDateTime) -> String {
    let ns = zdt.epoch_ns;
    ns.to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl ZonedDateTime {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    pub(crate) fn to_temporal(&self) -> TemporalZdt {
        // Copy the packed struct to the stack to avoid unaligned references.
        let this = *self;
        let tz_idx = this.tz_idx;
        let cal_idx = this.cal_idx;
        let epoch_ns = this.epoch_ns;
        let tz_id = crate::tz_index::name_of(tz_idx)
            .unwrap_or_else(|| error!("unknown timezone index {tz_idx}"));
        // Use our TZ_PROVIDER so the ResolvedId inside the TimeZone comes from the
        // same provider we pass to add_with_provider / subtract_with_provider etc.
        // Using the internal temporal_rs provider here would cause a ResolvedId
        // mismatch and a "Time zone identifier does not exist" error at runtime.
        let tz = TimeZone::try_from_str_with_provider(tz_id, &*TZ_PROVIDER)
            .unwrap_or_else(|e| error!("failed to load timezone \"{tz_id}\": {e}"));
        let cal_id = crate::cal_index::name_of(cal_idx)
            .unwrap_or_else(|| error!("unknown calendar index {cal_idx}"));
        let cal = Calendar::try_from_utf8(cal_id.as_bytes())
            .unwrap_or_else(|e| error!("failed to load calendar \"{cal_id}\": {e}"));
        TemporalZdt::try_new_with_provider(epoch_ns, tz, cal, &*TZ_PROVIDER)
            .unwrap_or_else(|e| error!("failed to reconstruct zoned_datetime: {e}"))
    }

    /// Build a `ZonedDateTime` from a `temporal_rs` zoned datetime.
    pub(crate) fn from_temporal(zdt: &TemporalZdt) -> Self {
        let epoch_ns = zdt.epoch_nanoseconds().as_i128();
        let tz_id = zdt
            .time_zone()
            .identifier()
            .unwrap_or_else(|e| error!("failed to get timezone identifier: {e}"));
        let tz_idx = crate::tz_index::index_of(&tz_id)
            .unwrap_or_else(|| error!("unknown timezone: {tz_id}"));
        let cal_id = zdt.calendar().identifier();
        let cal_idx = crate::cal_index::index_of(cal_id)
            .unwrap_or_else(|| error!("unsupported calendar: {cal_id}"));
        Self { epoch_ns, tz_idx, cal_idx }
    }
}

// ---------------------------------------------------------------------------
// Comparison functions
// ---------------------------------------------------------------------------

/// Returns -1, 0, or 1 comparing two zoned datetimes.
/// Primary key: epoch nanoseconds; tiebreakers: timezone index, calendar index.
/// Two values are equal only when instant, timezone, and calendar all match
/// (Temporal identity equality).
///
/// Note: `Temporal.ZonedDateTime.compare()` returns 0 for same-instant different-zone
/// values, but PostgreSQL btree requires `compare = 0 ↔ equals`, so identity semantics
/// are used throughout. Same-instant different-zone ordering is unspecified by the
/// Temporal spec; index ordering is a valid stable choice within that ambiguity.
#[allow(clippy::doc_markdown)] // "PostgreSQL" is a proper noun, not a code identifier
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_compare(a: ZonedDateTime, b: ZonedDateTime) -> i32 {
    let (a_epoch_ns, a_tz_idx, a_cal_idx) = (a.epoch_ns, a.tz_idx, a.cal_idx);
    let (b_epoch_ns, b_tz_idx, b_cal_idx) = (b.epoch_ns, b.tz_idx, b.cal_idx);
    match a_epoch_ns
        .cmp(&b_epoch_ns)
        .then_with(|| a_tz_idx.cmp(&b_tz_idx))
        .then_with(|| a_cal_idx.cmp(&b_cal_idx))
    {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_lt(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) < 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_le(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) <= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_eq(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) == 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_ne(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) != 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_ge(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) >= 0
}

#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_gt(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) > 0
}

extension_sql!(
    r"
    CREATE OPERATOR < (
        LEFTARG = ZonedDateTime, RIGHTARG = ZonedDateTime,
        FUNCTION = zoned_datetime_lt,
        COMMUTATOR = >, NEGATOR = >=
    );
    CREATE OPERATOR <= (
        LEFTARG = ZonedDateTime, RIGHTARG = ZonedDateTime,
        FUNCTION = zoned_datetime_le,
        COMMUTATOR = >=, NEGATOR = >
    );
    CREATE OPERATOR = (
        LEFTARG = ZonedDateTime, RIGHTARG = ZonedDateTime,
        FUNCTION = zoned_datetime_eq,
        COMMUTATOR = =, NEGATOR = <>
    );
    CREATE OPERATOR <> (
        LEFTARG = ZonedDateTime, RIGHTARG = ZonedDateTime,
        FUNCTION = zoned_datetime_ne,
        COMMUTATOR = <>, NEGATOR = =
    );
    CREATE OPERATOR >= (
        LEFTARG = ZonedDateTime, RIGHTARG = ZonedDateTime,
        FUNCTION = zoned_datetime_ge,
        COMMUTATOR = <=, NEGATOR = <
    );
    CREATE OPERATOR > (
        LEFTARG = ZonedDateTime, RIGHTARG = ZonedDateTime,
        FUNCTION = zoned_datetime_gt,
        COMMUTATOR = <, NEGATOR = <=
    );
    CREATE OPERATOR CLASS zoned_datetime_btree_ops DEFAULT FOR TYPE ZonedDateTime USING btree AS
        OPERATOR 1  <,
        OPERATOR 2  <=,
        OPERATOR 3  =,
        OPERATOR 4  >=,
        OPERATOR 5  >,
        FUNCTION 1  zoned_datetime_compare(ZonedDateTime, ZonedDateTime);
    ",
    name = "zoned_datetime_comparison_operators",
    requires = [
        zoned_datetime_lt,
        zoned_datetime_le,
        zoned_datetime_eq,
        zoned_datetime_ne,
        zoned_datetime_ge,
        zoned_datetime_gt
    ],
);

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Add a duration to a zoned datetime.
/// Uses `Constrain` overflow and the compiled IANA TZDB for DST-aware
/// wall-clock arithmetic.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_add(zdt: ZonedDateTime, dur: Duration) -> ZonedDateTime {
    let result = zdt
        .to_temporal()
        .add_with_provider(&dur.to_temporal(), Some(Overflow::Constrain), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("zoned_datetime_add failed: {e}"));
    ZonedDateTime::from_temporal(&result)
}

/// Subtract a duration from a zoned datetime.
/// Uses `Constrain` overflow and the compiled IANA TZDB for DST-aware
/// wall-clock arithmetic.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_subtract(zdt: ZonedDateTime, dur: Duration) -> ZonedDateTime {
    let result = zdt
        .to_temporal()
        .subtract_with_provider(&dur.to_temporal(), Some(Overflow::Constrain), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("zoned_datetime_subtract failed: {e}"));
    ZonedDateTime::from_temporal(&result)
}

/// Returns the duration elapsed from `other` to `zdt` (default unit: hours).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_since(zdt: ZonedDateTime, other: ZonedDateTime) -> Duration {
    let d = zdt
        .to_temporal()
        .since_with_provider(&other.to_temporal(), DifferenceSettings::default(), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("zoned_datetime_since failed: {e}"));
    Duration::from_temporal(&d)
}

/// Returns the duration from `zdt` to `other` (default unit: hours).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_until(zdt: ZonedDateTime, other: ZonedDateTime) -> Duration {
    let d = zdt
        .to_temporal()
        .until_with_provider(&other.to_temporal(), DifferenceSettings::default(), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("zoned_datetime_until failed: {e}"));
    Duration::from_temporal(&d)
}
