# pg_temporal — Agent Instructions

## Project Summary

`pg_temporal` is a PostgreSQL extension that brings [TC39 Temporal](https://tc39.es/proposal-temporal/)-compliant date/time types into SQL. It is built with [pgrx](https://github.com/pgcentralfoundation/pgrx) (Rust ↔ PostgreSQL FFI) and [temporal_rs](https://github.com/boa-dev/temporal) (Temporal spec implementation), and supports PostgreSQL 16, 17, and 18.

Key properties: nanosecond precision, IANA timezone semantics, full DST disambiguation, calendar awareness, RFC compliance.

### Types (all under the `temporal` schema)

| Type | Description |
|---|---|
| `zoneddatetime` | Timezone-aware datetime (instant + IANA zone + calendar) |
| `instant` | Absolute UTC instant, no timezone |
| `plaindatetime` | Calendar-local datetime, no timezone |
| `plaindate` | Calendar-local date, no time or timezone |
| `plaintime` | Wall-clock time, no date, timezone, or calendar |
| `plainyearmonth` | Calendar-local year and month, no day |
| `plainmonthday` | Calendar-local month and day, no year |
| `duration` | Full vector duration (years → nanoseconds), no normalization |

## Key Commands

| Command | Description |
|---|---|
| `cargo check --features pg17` | Build/type-check (works in sandboxed terminals; substitute `pg16`/`pg18` as needed) |
| `cargo fmt` | Format code (uses `rustfmt.toml` settings) |
| `cargo pgrx test <pg_major> > /tmp/pg_temporal_test_output.txt 2>&1` | Run tests for a single Postgres version (e.g. `pg17`) |
| `cargo pgrx test <pg_major> <test_name> > /tmp/pg_temporal_test_output.txt 2>&1` | Run a single test |
| `cargo pgrx test all --no-default-features > /tmp/pg_temporal_test_matrix.txt 2>&1` | Run full test matrix across all Postgres versions |

