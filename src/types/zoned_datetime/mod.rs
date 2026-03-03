use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use temporal_rs::{
    Calendar, TimeZone, ZonedDateTime as TemporalZdt,
    options::{
        DisplayCalendar, DisplayOffset, DisplayTimeZone, OffsetDisambiguation,
        ToStringRoundingOptions,
    },
};

use crate::gucs;

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
// Catalog helpers
// ---------------------------------------------------------------------------

/// Escape a string value for safe embedding in a SQL literal.
///
/// IANA timezone identifiers and calendar IDs are ASCII-only and never
/// contain single quotes, but we escape defensively.
fn escape_sql_literal(s: &str) -> String {
    s.replace('\'', "''")
}

/// Return the existing `tz_oid` for `tz_id`, inserting a new row if needed.
fn lookup_or_insert_timezone(tz_id: &str) -> i32 {
    let esc = escape_sql_literal(tz_id);

    // Upsert: insert new row OR return the existing one atomically.
    // The DO UPDATE SET touches no column so Postgres still returns the OID.
    Spi::get_one::<i32>(&format!(
        "INSERT INTO temporal.timezone_catalog (canonical_id)
         VALUES ('{esc}')
         ON CONFLICT (canonical_id) DO UPDATE
             SET canonical_id = EXCLUDED.canonical_id
         RETURNING tz_oid"
    ))
    .unwrap_or_else(|e| error!("timezone catalog insert failed: {e}"))
    .unwrap_or_else(|| error!("timezone catalog insert returned no row"))
}

/// Return the existing `calendar_oid` for `cal_id`, inserting a new row if needed.
fn lookup_or_insert_calendar(cal_id: &str) -> i32 {
    let esc = escape_sql_literal(cal_id);

    Spi::get_one::<i32>(&format!(
        "INSERT INTO temporal.calendar_catalog (calendar_id)
         VALUES ('{esc}')
         ON CONFLICT (calendar_id) DO UPDATE
             SET calendar_id = EXCLUDED.calendar_id
         RETURNING calendar_oid"
    ))
    .unwrap_or_else(|e| error!("calendar catalog insert failed: {e}"))
    .unwrap_or_else(|| error!("calendar catalog insert returned no row"))
}

/// Resolve a `tz_oid` back to its IANA identifier string.
fn lookup_timezone_by_oid(tz_oid: i32) -> String {
    Spi::get_one::<String>(&format!(
        "SELECT canonical_id FROM temporal.timezone_catalog WHERE tz_oid = {tz_oid}"
    ))
    .unwrap_or_else(|e| error!("timezone catalog lookup failed: {e}"))
    .unwrap_or_else(|| error!("unknown tz_oid {tz_oid}"))
}

/// Resolve a `calendar_oid` back to its calendar identifier string.
fn lookup_calendar_by_oid(calendar_oid: i32) -> String {
    Spi::get_one::<String>(&format!(
        "SELECT calendar_id FROM temporal.calendar_catalog WHERE calendar_oid = {calendar_oid}"
    ))
    .unwrap_or_else(|e| error!("calendar catalog lookup failed: {e}"))
    .unwrap_or_else(|| error!("unknown calendar_oid {calendar_oid}"))
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests;
