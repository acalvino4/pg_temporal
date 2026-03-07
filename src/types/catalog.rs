use pgrx::prelude::*;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Escape a string value for safe embedding in a SQL literal.
///
/// IANA timezone identifiers and calendar IDs are ASCII-only and never
/// contain single quotes, but we escape defensively.
fn escape_sql_literal(s: &str) -> String {
    s.replace('\'', "''")
}

// ---------------------------------------------------------------------------
// Timezone catalog
// ---------------------------------------------------------------------------

/// Return the existing `tz_oid` for `tz_id`, inserting a new row if needed.
pub fn lookup_or_insert_timezone(tz_id: &str) -> i32 {
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

/// Resolve a `tz_oid` back to its IANA identifier string.
pub fn lookup_timezone_by_oid(tz_oid: i32) -> String {
    Spi::get_one::<String>(&format!(
        "SELECT canonical_id FROM temporal.timezone_catalog WHERE tz_oid = {tz_oid}"
    ))
    .unwrap_or_else(|e| error!("timezone catalog lookup failed: {e}"))
    .unwrap_or_else(|| error!("unknown tz_oid {tz_oid}"))
}

// ---------------------------------------------------------------------------
// Calendar catalog
// ---------------------------------------------------------------------------

/// Return the existing `calendar_oid` for `cal_id`, inserting a new row if needed.
pub fn lookup_or_insert_calendar(cal_id: &str) -> i32 {
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

/// Resolve a `calendar_oid` back to its calendar identifier string.
pub fn lookup_calendar_by_oid(calendar_oid: i32) -> String {
    Spi::get_one::<String>(&format!(
        "SELECT calendar_id FROM temporal.calendar_catalog WHERE calendar_oid = {calendar_oid}"
    ))
    .unwrap_or_else(|e| error!("calendar catalog lookup failed: {e}"))
    .unwrap_or_else(|| error!("unknown calendar_oid {calendar_oid}"))
}
