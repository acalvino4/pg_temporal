# pg_temporal

A PostgreSQL extension implementing **[Temporal](https://tc39.es/proposal-temporal/)-compliant** date/time types, including
  - nanosecond precision
  - IANA timezone semantics
  - full DST disambiguation
  - calendar awareness
  - RFC compliance.

> [!NOTE]
> I have no prior experience writing database extensions or programming in Rust, and will gladly accept feedback about bugs or non-idiomatic rust patterns from people more familiar with the language and database development. My hope is for this to raise awareness that robust datetime handling at the database layer is essential, yet an unsolved problem. Whether this project grows to fill that gap, or merely inspires others to do so, it will have achieved its purpose.

## Why

With JodaTime, NodaTime, Temporal, and temporal_rs out (or soon to be out) in the wild, application code in most popular languages can now easily follow sane and consistent standards. However, this robustness is **lost** as soon as we need our datetime data to persist in the database layer.

Sure, db's have timestamp types, and generally handle UTC offsets as well, but these solutions suffer from the same or similar shortcomings as most languages' naive implementations: no nanosecond precision, no explicit calendar support, ambiguous DST handling, poor timezone support, and no standard for duration arithmetic. `pg_temporal` brings the Temporal API's rigorous date/time model directly into SQL.

## Types

(These are exact parallels to Temporal's types)

| Type                           | Description                                                  |
| ------------------------------ | ------------------------------------------------------------ |
| `temporal.zoneddatetime`       | Timezone-aware datetime (instant + IANA zone + calendar)     |
| `temporal.instant`             | Absolute UTC instant, no timezone                            |
| `temporal.plain_datetime`      | Calendar-local datetime, no timezone                         |
| `temporal.plain_date`          | Calendar-local date, no time or timezone                     |
| `temporal.plain_time`          | Wall-clock time, no date, timezone, or calendar              |
| `temporal.plain_year_month`    | Calendar-local year and month, no day                        |
| `temporal.plain_month_day`     | Calendar-local month and day, no year                        |
| `temporal.duration`            | Full vector duration (years → nanoseconds), no normalization |

## Docs

- [Quickstart](docs/quickstart.md)
- [Contributing / development guide](docs/contributing.md)
- [ZonedDateTime](docs/usage/zoned_datetime.md)
- [Instant](docs/usage/instant.md)
- [PlainDateTime](docs/usage/plain_datetime.md)
- [PlainDate](docs/usage/plain_date.md)
- [PlainTime](docs/usage/plain_time.md)
- [PlainYearMonth](docs/usage/plain_year_month.md)
- [PlainMonthDay](docs/usage/plain_month_day.md)
- [Duration](docs/usage/duration.md)

## Implementation

Built with [pgrx](https://github.com/pgcentralfoundation/pgrx) (Rust ↔ PostgreSQL FFI) and [temporal_rs](https://github.com/boa-dev/temporal) (Temporal spec rust implementation). Targets PostgreSQL 18+ at the moment.

## Thanks

- Temporal: this spec-driven ai-development would've been impossible without such a robust spec and the countless hours pours into it
- JodaTime/NodaTime - the forerunners of Temporal
- Rust: even with ai-assistance, I highly doubt I could've made an extension that doesn't immediately crash if not for such a robust compiler
- temporal_rs: saved me (that is, claude) from having to worry about any of the business logic. v0.2.0 happened to come out just days before I had this idea, and I suspect I may have hit many more blocks if not for it's timely release. My understanding is that this project is at the core of all browser and js engine temporal implementations, which means that using it at the db layer should keep behavior aligned across all layers of a js web app!
