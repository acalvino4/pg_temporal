# pg_temporal

A PostgreSQL extension implementing [Temporal](https://tc39.es/proposal-temporal/)-compliant date/time types — nanosecond precision, IANA timezone semantics, full DST disambiguation, and calendar awareness.

## Why

PostgreSQL's built-in `timestamp` and `timestamptz` types have well-known shortcomings: no nanosecond precision, no explicit calendar support, ambiguous DST handling, and no standard for duration arithmetic. `pg_temporal` brings the Temporal API's rigorous date/time model directly into SQL.

## Types

| Type                      | Description                                                  |
| ------------------------- | ------------------------------------------------------------ |
| `temporal.zoneddatetime`  | Timezone-aware datetime (instant + IANA zone + calendar)     |
| `temporal.instant`        | Absolute UTC instant, no timezone                            |
| `temporal.plain_datetime` | Calendar-local datetime, no timezone                         |
| `temporal.duration`       | Full vector duration (years → nanoseconds), no normalization |

## Key properties

- **Nanosecond precision** throughout
- **Identity equality** for `zoned_datetime`: two values are equal only if instant, zone, and calendar all match
- **Explicit conversions only** — no implicit casts from native PG types
- **Cluster-wide configuration** via GUCs: default disambiguation strategy, timezone alias policy
- **Standards compatibility**
  - [TC39 Temporal](https://tc39.es/proposal-temporal/) — type semantics, identity equality, disambiguation rules
  - [RFC 9557 / IXDTF](https://www.rfc-editor.org/rfc/rfc9557) — text format for all I/O
  - [IANA TZDB](https://www.iana.org/time-zones) — timezone data, bundled at compile time
- **Bundled TZDB** — no runtime data files required

## Status

| Phase                                   | Status   |
| --------------------------------------- | -------- |
| Scaffold + environment                  | complete |
| Catalog tables + `zoned_datetime`       | complete |
| `instant`, `plain_datetime`, `duration` | planned  |
| Arithmetic + comparison operators       | planned  |

## Implementation

Built with [pgrx](https://github.com/pgcentralfoundation/pgrx) (Rust ↔ PostgreSQL FFI) and [temporal_rs](https://github.com/boa-dev/temporal) (Temporal spec implementation). Targets PostgreSQL 18+.

## Docs

- [Contributing / development guide](docs/contributing.md)
- [zoneddatetime](docs/usage/zoned_datetime.md)
