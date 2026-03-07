use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
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
use crate::types::catalog::{
    lookup_calendar_by_oid, lookup_or_insert_calendar, lookup_or_insert_timezone,
    lookup_timezone_by_oid,
};
use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// This struct is what PostgreSQL physically stores for each zoned_datetime
// value.  It is wrapped in a pgrx varlena by the #[derive(PostgresType)]
// machinery.
//
//   instant_ns    – nanoseconds since Unix epoch (same as Temporal's
//                   epochNanoseconds). i128 gives us the full ±292-year
//                   range at nanosecond precision.
//   tz_oid        – row id in temporal.timezone_catalog
//   calendar_oid  – row id in temporal.calendar_catalog
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
pub struct ZonedDateTime {
    instant_ns: i128,
    tz_oid: i32,
    calendar_oid: i32,
}

// ---------------------------------------------------------------------------
// Text in / out
// ---------------------------------------------------------------------------

impl InOutFuncs for ZonedDateTime {
    /// Parse an IXDTF/RFC-9557 string into a `ZonedDateTime` datum.
    ///
    /// Example input: `2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=iso8601]`
    #[allow(clippy::similar_names)] // tz_id (string) and tz_oid (integer) are semantically distinct
    fn input(input: &CStr) -> Self {
        let s =
            input.to_str().unwrap_or_else(|_| error!("zoned_datetime input is not valid UTF-8"));

        let disambiguation = gucs::default_disambiguation();

        let zdt =
            TemporalZdt::from_utf8(s.as_bytes(), disambiguation, OffsetDisambiguation::Reject)
                .unwrap_or_else(|e| error!("invalid zoned_datetime \"{s}\": {e}"));

        let instant_ns = zdt.epoch_nanoseconds().as_i128();

        let tz_id = zdt
            .time_zone()
            .identifier()
            .unwrap_or_else(|e| error!("failed to get timezone identifier: {e}"));

        let cal_id = zdt.calendar().identifier();

        let tz_oid = lookup_or_insert_timezone(&tz_id);
        let calendar_oid = lookup_or_insert_calendar(cal_id);

        Self { instant_ns, tz_oid, calendar_oid }
    }

    /// Serialize a `ZonedDateTime` datum back to an IXDTF string.
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let tz_id = lookup_timezone_by_oid(self.tz_oid);
        // calendar_oid is stored but not yet used for formatting; ISO is the only
        // supported calendar in Phase 2. Extend here when multi-calendar lands.

        let tz = TimeZone::try_from_str(&tz_id)
            .unwrap_or_else(|e| error!("failed to load timezone \"{tz_id}\": {e}"));

        let cal = Calendar::default(); // iso8601

        let zdt = TemporalZdt::try_new(self.instant_ns, tz, cal)
            .unwrap_or_else(|e| error!("failed to reconstruct zoned_datetime: {e}"));

        let s = zdt
            .to_ixdtf_string(
                DisplayOffset::default(),
                DisplayTimeZone::default(),
                DisplayCalendar::default(),
                ToStringRoundingOptions::default(),
            )
            .unwrap_or_else(|e| error!("failed to format zoned_datetime: {e}"));

        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the timezone name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_timezone(zdt: ZonedDateTime) -> String {
    lookup_timezone_by_oid(zdt.tz_oid)
}

/// Returns the calendar name stored with this value.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_calendar(zdt: ZonedDateTime) -> String {
    lookup_calendar_by_oid(zdt.calendar_oid)
}

/// Returns the UTC epoch in nanoseconds as a text value (i128 has no native
/// SQL type; use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_epoch_ns(zdt: ZonedDateTime) -> String {
    zdt.instant_ns.to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl ZonedDateTime {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    // Clippy's wrong_self_convention wants `to_*` on Copy types to take self by value.
    pub(crate) fn to_temporal(self) -> TemporalZdt {
        let tz_id = lookup_timezone_by_oid(self.tz_oid);
        // Use our TZ_PROVIDER so the ResolvedId inside the TimeZone comes from the
        // same provider we pass to add_with_provider / subtract_with_provider etc.
        // Using the internal temporal_rs provider here would cause a ResolvedId
        // mismatch and a "Time zone identifier does not exist" error at runtime.
        let tz = TimeZone::try_from_str_with_provider(&tz_id, &*TZ_PROVIDER)
            .unwrap_or_else(|e| error!("failed to load timezone \"{tz_id}\": {e}"));
        let cal = Calendar::default();
        TemporalZdt::try_new(self.instant_ns, tz, cal)
            .unwrap_or_else(|e| error!("failed to reconstruct zoned_datetime: {e}"))
    }

    /// Build a `ZonedDateTime` from a `temporal_rs` zoned datetime.
    #[allow(clippy::similar_names)] // tz_id (string) and tz_oid (integer) are semantically distinct
    pub(crate) fn from_temporal(zdt: &TemporalZdt) -> Self {
        let instant_ns = zdt.epoch_nanoseconds().as_i128();
        let tz_id = zdt
            .time_zone()
            .identifier()
            .unwrap_or_else(|e| error!("failed to get timezone identifier: {e}"));
        let cal_id = zdt.calendar().identifier();
        let tz_oid = lookup_or_insert_timezone(&tz_id);
        let calendar_oid = lookup_or_insert_calendar(cal_id);
        Self { instant_ns, tz_oid, calendar_oid }
    }
}

// ---------------------------------------------------------------------------
// Comparison functions
// ---------------------------------------------------------------------------

/// Returns -1, 0, or 1 comparing two zoned datetimes.
/// Primary key: epoch nanoseconds; tiebreakers: timezone OID, calendar OID.
/// Two values are equal only when instant, timezone, and calendar all match
/// (Temporal identity equality).
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_compare(a: ZonedDateTime, b: ZonedDateTime) -> i32 {
    let a_key = (a.instant_ns, a.tz_oid, a.calendar_oid);
    let b_key = (b.instant_ns, b.tz_oid, b.calendar_oid);
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
pub fn zoned_datetime_lt(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) < 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_le(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) <= 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_eq(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) == 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_ne(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) != 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn zoned_datetime_ge(a: ZonedDateTime, b: ZonedDateTime) -> bool {
    zoned_datetime_compare(a, b) >= 0
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
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
