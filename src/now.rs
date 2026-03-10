#![allow(clippy::doc_markdown)]
//! `now()`-style functions backed by PostgreSQL's `GetCurrentTimestamp()`.
//!
//! PostgreSQL's `GetCurrentTimestamp()` returns the current transaction start
//! time (not wall-clock time), so repeated calls within the same transaction
//! yield the same value — matching Temporal's expected "frozen at BEGIN"
//! semantics for repeatable-read transactions.
//!
//! # Epoch conversion
//!
//! PostgreSQL timestamps are **microseconds since 2000-01-01** (the "PostgreSQL
//! epoch").  Temporal / Unix instants use **nanoseconds since 1970-01-01**.
//!
//! Offset: 30 years = 10 957 days = 946 684 800 seconds
//!       = 946 684 800 000 000 µs  →  multiply by 1 000 to get ns.

use pgrx::prelude::*;
use temporal_rs::{
    Calendar, TimeZone, UtcOffset, ZonedDateTime as TemporalZdt,
    host::{HostClock, HostHooks, HostTimeZone},
    now::Now,
};
use timezone_provider::epoch_nanoseconds::EpochNanoseconds;
use timezone_provider::provider::TimeZoneProvider;

use crate::provider::TZ_PROVIDER;
use crate::types::instant::Instant;
use crate::types::plain_datetime::PlainDateTime;
use crate::types::zoned_datetime::ZonedDateTime;

// ---------------------------------------------------------------------------
// PgClock — HostHooks backed by GetCurrentTimestamp()
// ---------------------------------------------------------------------------

/// A zero-size host-hooks implementation that reads the current timestamp
/// from PostgreSQL's transaction clock instead of the OS wall clock.
struct PgClock;

/// PostgreSQL epoch offset in microseconds: 2000-01-01 − 1970-01-01.
const PG_TO_UNIX_OFFSET_US: i64 = 946_684_800_000_000_i64;

impl HostClock for PgClock {
    fn get_host_epoch_nanoseconds(&self) -> temporal_rs::TemporalResult<EpochNanoseconds> {
        // SAFETY: GetCurrentTimestamp() is always safe to call inside a PG backend.
        let pg_us: i64 = unsafe { pg_sys::GetCurrentTimestamp() };
        // Saturating add avoids undefined behaviour for extreme timestamps.
        #[allow(clippy::similar_names)]
        let unix_us = pg_us.saturating_add(PG_TO_UNIX_OFFSET_US);
        let epoch_ns = i128::from(unix_us) * 1000_i128;
        Ok(EpochNanoseconds::from(epoch_ns))
    }
}

impl HostTimeZone for PgClock {
    fn get_host_time_zone(
        &self,
        _provider: &(impl TimeZoneProvider + ?Sized),
    ) -> temporal_rs::TemporalResult<TimeZone> {
        // UTC fallback — functions that need a specific timezone accept it as a parameter.
        Ok(TimeZone::from(UtcOffset::default()))
    }
}

impl HostHooks for PgClock {}

// ---------------------------------------------------------------------------
// SQL-callable now() functions
// ---------------------------------------------------------------------------

/// Returns the `TemporalZdt` for the current transaction timestamp in `tz`.
///
/// Shared by `temporal_now_zoneddatetime` and `temporal_now_plaindatetime`.
/// `fn_name` is included in error messages for clear attribution.
fn current_zdt(tz: &str, fn_name: &str) -> TemporalZdt {
    let epoch_ns = PgClock
        .get_host_epoch_nanoseconds()
        .unwrap_or_else(|e| error!("{fn_name}: clock error: {e}"));
    let time_zone = TimeZone::try_from_str_with_provider(tz, &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("{fn_name}: invalid timezone \"{tz}\": {e}"));
    TemporalZdt::try_new_with_provider(epoch_ns.as_i128(), time_zone, Calendar::default(), &*TZ_PROVIDER)
        .unwrap_or_else(|e| error!("{fn_name}: {e}"))
}

/// Returns the current `Instant` at transaction start time.
///
/// Backed by PostgreSQL's `GetCurrentTimestamp()`, which is frozen at the
/// start of the current transaction (repeatable-read semantics).
///
/// Example: `SELECT temporal_now_instant();`
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn temporal_now_instant() -> Instant {
    let inst = Now::new(PgClock).instant().unwrap_or_else(|e| error!("temporal_now_instant: {e}"));
    Instant::from_temporal(&inst)
}

/// Returns the current `ZonedDateTime` at transaction start time, expressed
/// in the given IANA timezone with an ISO 8601 calendar.
///
/// Example: `SELECT temporal_now_zoneddatetime('America/New_York');`
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn temporal_now_zoneddatetime(tz: &str) -> ZonedDateTime {
    ZonedDateTime::from_temporal(&current_zdt(tz, "temporal_now_zoneddatetime"))
}

/// Returns the current `PlainDateTime` at transaction start time as observed
/// in the given IANA timezone, with an ISO 8601 calendar.
///
/// The timezone is used only to project wall-clock fields; it is not stored
/// in the resulting `PlainDateTime`.
///
/// Example: `SELECT temporal_now_plaindatetime('Europe/Paris');`
#[must_use]
#[pg_extern(stable, parallel_safe)]
pub fn temporal_now_plaindatetime(tz: &str) -> PlainDateTime {
    PlainDateTime::from_temporal(&current_zdt(tz, "temporal_now_plaindatetime").to_plain_date_time())
}
