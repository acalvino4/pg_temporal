# pg_temporal

A PostgreSQL extension implementing [Temporal](https://tc39.es/proposal-temporal/)-compliant date/time types — nanosecond precision, IANA timezone semantics, full DST disambiguation, and calendar awareness.

> [!NOTE]
> This project was almost entirely vibe-coded - I have no prior experience writing database extensions or programming in Rust; expect bugs and non-idiomatic patterns. It is not intended to become a production-grade implmentation — people more familiar with Rust and databases should take that on. Take this as just a POC meant to raise awareness that robust datetime handling at the database layer is an essential, yet unsolved problem.

## Why

With JodaTime, NodaTime, Temporal, and temporal_rs out (or soon to be out) in the wild, application code in most popular languages can now easily follow sane and consistent standards. However, this robustness is **lost** as soon as we need our datetime data to persist in the database layer.

Sure, db's have timestamp types, and generally handle UTC offsets as well, but these solutions suffer from the same or similar shortcomings as most langauges' naive implementations: no nanosecond precision, no explicit calendar support, ambiguous DST handling, poor timezone support, and no standard for duration arithmetic. `pg_temporal` brings the Temporal API's rigorous date/time model directly into SQL.

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
| `instant`, `plain_datetime`, `duration` | complete |
| Arithmetic + comparison operators       | planned  |

## Implementation

Built with [pgrx](https://github.com/pgcentralfoundation/pgrx) (Rust ↔ PostgreSQL FFI) and [temporal_rs](https://github.com/boa-dev/temporal) (Temporal spec implementation). Targets PostgreSQL 18+.

## Docs

- [Contributing / development guide](docs/contributing.md)
- [ZonedDateTime](docs/usage/zoned_datetime.md)
- [Instant](docs/usage/instant.md)
- [PlainDateTime](docs/usage/plain_datetime.md)
- [Duration](docs/usage/duration.md)

## Thanks

- Temporal: this spec-driven ai-development would've been impossible without such a robust spec and the countless hours pours into it
- JodaTime/NodaTime - the forerunners of Temporal
- Rust: even with ai-assistance, I highly doubt I could've made an extension that doesn't immediately crash if not for such a robust compiler
- temporal_rs: saved me (that is, claude) from having to worry about any of the business logic. v0.2.0 happened to come out just days before I had this idea, and I suspect I may have hit many more blocks if not for it's timely release
