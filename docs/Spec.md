Postgres Temporal Extension — Spec

## 1. What

`pg_temporal` is a PostgreSQL extension that exposes Temporal-style date/time types in the `temporal` schema, backed by Rust (`pgrx`) and `temporal_rs`.

Current SQL types:

- `temporal.zoneddatetime` — timezone-aware datetime (`instant + IANA zone + calendar`)
- `temporal.instant` — absolute UTC instant
- `temporal.plaindatetime` — calendar-local datetime
- `temporal.plaindate` — calendar-local date
- `temporal.plaintime` — wall-clock time
- `temporal.plainyearmonth` — calendar-local year/month
- `temporal.plainmonthday` — calendar-local month/day
- `temporal.duration` — full vector duration (`years` through `nanoseconds`)

The SQL surface is functions-first: constructors, accessors, arithmetic, comparisons where Temporal semantics allow them, explicit casts from native PostgreSQL types, and `now()` helpers.

## 2. Why

PostgreSQL's native date/time types are useful, but they do not model the same semantics as Temporal:

- no first-class IANA timezone identity on values
- no calendar-aware local types
- no Temporal-style duration model
- no nanosecond precision
- no explicit disambiguation model for DST gaps/folds

`pg_temporal` exists to preserve Temporal semantics at the database layer so applications do not lose correctness when values cross the SQL boundary.

## 3. Guiding Principles

- Correctness before convenience.
- Temporal-compatible semantics where `temporal_rs` provides them.
- Explicit conversions only; avoid surprising implicit casts.
- Nanosecond precision throughout.
- Functions-first SQL API, with operators added only where the semantics are well-defined.
- Compact binary storage rather than text or self-describing encodings.

## 4. Current Implementation Model

### Implementation framework

- Rust + `pgrx` for PostgreSQL integration
- `temporal_rs` for Temporal semantics
- `timezone_provider` with compiled TZDB data

### Timezone and calendar identifiers

The implementation does not use SQL catalog tables for timezone or calendar lookup.

Instead, identifiers are stored as compact indices backed by compile-time generated arrays:

- `src/tz_index.rs` includes a generated append-only canonical IANA timezone list
- `src/cal_index.rs` includes a generated calendar identifier list
- write path: string identifier -> compact index via binary search
- read path: compact index -> string identifier via direct array lookup

Timezone rules are resolved through a single process-wide compiled TZDB provider. No runtime timezone catalog tables or runtime tzdata files are required.

### On-disk storage

All Temporal types use compact binary `PgVarlena<T>` storage rather than pgrx's default CBOR/serde path.

Representative layouts:

- `ZonedDateTime { epoch_ns: i128, tz_idx: u16, cal_idx: u8 }`
- `Instant { epoch_ns: i128 }`
- local calendar-bearing types store their calendar as `cal_idx`
- `Duration` stores the full Temporal vector without normalization

This keeps storage fixed-width where possible and preserves exact Temporal data instead of flattening it into PostgreSQL-native timestamp forms.

## 5. Semantics

### Equality and ordering

- `zoneddatetime` uses identity-style equality: instant, timezone, and calendar all matter
- ordering is implemented for types where a total order is meaningful
- `duration` deliberately does not define ordinary comparison operators; comparison requires dedicated functions with enough context

### Precision and formats

- nanosecond precision is preserved end-to-end
- text I/O uses RFC 9557 / IXDTF-style strings for Temporal-compatible round-tripping
- timezone semantics are based on bundled IANA TZDB data compiled into the extension

### Conversions

- casts from native PostgreSQL types are explicit
- cross-type conversion functions exist where the transformation is well-defined

## 6. Cluster / Session Configuration

Current GUCs:

- `pg_temporal.default_disambiguation` — controls how ambiguous local wall-clock times are resolved

## 7. Non-Goals / Constraints

- No SQL-backed timezone or calendar catalog tables.
- No implicit coercion intended to mimic PostgreSQL's native timestamp behavior.
- Currently targets PostgreSQL 18 via the `pg18` feature.

## 8. Standards Alignment

- [TC39 Temporal](https://tc39.es/proposal-temporal/) for value semantics and API behavior
- [RFC 9557 / IXDTF](https://www.rfc-editor.org/rfc/rfc9557) for textual representation
- [IANA TZDB](https://www.iana.org/time-zones) for timezone rules
