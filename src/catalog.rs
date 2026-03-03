use pgrx::prelude::*;

// ---------------------------------------------------------------------------
// Catalog tables
//
// These tables are created before any type/function SQL so that the
// zoneddatetime in/out functions can perform OID lookups at runtime.
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

    -- Seed the ISO 8601 calendar; it is always OID 1.
    INSERT INTO temporal.calendar_catalog (calendar_id) VALUES ('iso8601');
    ",
    name = "create_catalogs",
    bootstrap
);
