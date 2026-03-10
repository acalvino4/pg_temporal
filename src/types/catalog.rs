use pgrx::prelude::*;

// ---------------------------------------------------------------------------
// Timezone catalog
// ---------------------------------------------------------------------------

/// Return the existing `tz_oid` for `tz_id`, inserting a new row if needed.
#[must_use]
pub fn lookup_or_insert_timezone(tz_id: &str) -> i32 {
    // Upsert: insert new row OR return the existing one atomically.
    // The DO UPDATE SET touches no column so Postgres still returns the OID.
    Spi::get_one_with_args::<i32>(
        "INSERT INTO temporal.timezone_catalog (canonical_id)
         VALUES ($1)
         ON CONFLICT (canonical_id) DO UPDATE
             SET canonical_id = EXCLUDED.canonical_id
         RETURNING tz_oid",
        &[tz_id.into()],
    )
    .unwrap_or_else(|e| error!("timezone catalog insert failed: {e}"))
    .unwrap_or_else(|| error!("timezone catalog insert returned no row"))
}

/// Resolve a `tz_oid` back to its IANA identifier string.
#[must_use]
pub fn lookup_timezone_by_oid(tz_oid: i32) -> String {
    Spi::get_one_with_args::<String>(
        "SELECT canonical_id FROM temporal.timezone_catalog WHERE tz_oid = $1",
        &[tz_oid.into()],
    )
    .unwrap_or_else(|e| error!("timezone catalog lookup failed: {e}"))
    .unwrap_or_else(|| error!("unknown tz_oid {tz_oid}"))
}

// ---------------------------------------------------------------------------
// Calendar catalog
// ---------------------------------------------------------------------------

/// Return the existing `calendar_oid` for `cal_id`, inserting a new row if needed.
#[must_use]
pub fn lookup_or_insert_calendar(cal_id: &str) -> i32 {
    Spi::get_one_with_args::<i32>(
        "INSERT INTO temporal.calendar_catalog (calendar_id)
         VALUES ($1)
         ON CONFLICT (calendar_id) DO UPDATE
             SET calendar_id = EXCLUDED.calendar_id
         RETURNING calendar_oid",
        &[cal_id.into()],
    )
    .unwrap_or_else(|e| error!("calendar catalog insert failed: {e}"))
    .unwrap_or_else(|| error!("calendar catalog insert returned no row"))
}

/// Resolve a `calendar_oid` back to its calendar identifier string.
#[must_use]
pub fn lookup_calendar_by_oid(calendar_oid: i32) -> String {
    Spi::get_one_with_args::<String>(
        "SELECT calendar_id FROM temporal.calendar_catalog WHERE calendar_oid = $1",
        &[calendar_oid.into()],
    )
    .unwrap_or_else(|e| error!("calendar catalog lookup failed: {e}"))
    .unwrap_or_else(|| error!("unknown calendar_oid {calendar_oid}"))
}
