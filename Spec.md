Postgres Temporal Extension — Spec

1. What

A Postgres extension implementing full Temporal-compliant date/time types, aligned with Temporal API, NodaTime/JodaTime semantics, and IANA tzdb rules.

Core features:

Types:

pg_temporal.zoned_datetime — timezone-aware datetime

pg_temporal.instant — absolute UTC instant

pg_temporal.plain_datetime — calendar-local datetime

pg_temporal.duration — full vector durations (calendar + exact)

Functions: Constructors, arithmetic, conversions, disambiguation handling

Cluster-wide configuration: TZDB version, aliasing behavior, default disambiguation

SQL-first integration: Explicit conversions; indexable, filterable, and sortable

Implementation framework: Rust + pgrx for Postgres extension development; temporal_rs crate for core Temporal logic

2. Why

Spec compliance: Fully matches Temporal, nanosecond precision, calendar semantics, identity equality

Deterministic scheduling: DST changes, ambiguous times, global timezones handled automatically

Unified Temporal model in Postgres: Replaces inconsistent timestamp/timestamptz usage for timezone-aware applications

Future-proof: Supports multiple calendars, Temporal-style durations, and cluster-wide configuration

3. Guiding Principles

Correctness first: Full Temporal semantics, DST, and disambiguation handled natively

Identity equality: ZonedDateTime equality includes local datetime, zone, and calendar

Explicit conversions only: No implicit casting from native types

Cluster-wide determinism: All configuration uniform across cluster

Temporal-compliant durations: Preserve full vector structure for exact and calendar durations

Functions-first SQL interface: Operators added later if justified

Strict parsing and serialization: RFC-compliant literals and custom binary layouts

Performance secondary to correctness: OID-based zone storage, nanosecond precision

4. High-Level Technical / Implementation Decisions
   Decision Choice / Implementation
   Implementation Framework Rust, using pgrx for Postgres integration; temporal_rs crate for Temporal logic
   Equality Identity equality for ZonedDateTime
   Ordering Primary: instant; tie-breaker: zone OID / calendar OID lexicographically
   Duration Full vector storage (years → nanoseconds), no normalization
   Disambiguation Default "compatible"; configurable cluster-wide
   Calendar OID, ISO-only initially, extensible later
   Alias Policy Cluster-wide default (IANA/JodaTime), configurable via GUC
   TZDB Versioning Bundled with temporal_rs crate; latest version always used; cluster-wide
   Precision Nanoseconds
   Casts Explicit only
   Binary Layout Custom per type, varlena Datum
   SQL Surface Functions first; operators optional
   Namespace Default schema recommended (pg_temporal)
   Cluster Config GUCs control TZDB, aliasing, disambiguation, uniform across cluster
   Storage Structs

ZonedDateTime

struct ZonedDateTimeDatum {
instant_ns: i128, // nanoseconds since epoch
tz_oid: i32, // OID from timezone catalog
calendar_oid: i32, // OID from calendar catalog
}

Duration

struct DurationDatum {
years: i32,
months: i32,
weeks: i32,
days: i32,
hours: i32,
minutes: i32,
seconds: i32,
nanoseconds: i64,
}

Catalogs

pg_temporal.timezone_catalog → canonical tzdb IDs, OIDs, aliases

pg_temporal.calendar_catalog → calendar OIDs, names
