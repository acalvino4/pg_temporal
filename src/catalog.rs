use pgrx::prelude::*;

// ---------------------------------------------------------------------------
// Catalog tables
//
// These tables are reserved for future alias resolution (e.g. mapping
// "US/Eastern" → "America/New_York", or "Gregory" → "iso8601").
//
// As of the current release, timezone and calendar identifiers are stored
// inline in each datum (no OID indirection), so these tables are not
// queried at runtime. They exist so that alias support can be layered on
// later without a schema migration.
//
// bootstrap = true ensures pgrx emits this SQL before all other generated
// DDL in the extension install script.
// ---------------------------------------------------------------------------

extension_sql!(
    r"
    CREATE TABLE temporal.timezone_catalog (
        tz_oid       SERIAL      PRIMARY KEY,
        canonical_id TEXT        NOT NULL UNIQUE,
        aliases      TEXT[]      NOT NULL DEFAULT '{}'
    );

    CREATE TABLE temporal.calendar_catalog (
        calendar_oid SERIAL      PRIMARY KEY,
        calendar_id  TEXT        NOT NULL UNIQUE
    );

    -- Seed the ISO 8601 calendar so it is always present before any type I/O runs.
    INSERT INTO temporal.calendar_catalog (calendar_id) VALUES ('iso8601');
    ",
    name = "create_catalogs",
    bootstrap
);
