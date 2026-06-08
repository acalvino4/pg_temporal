# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `Instant` type: a fixed point in time (nanosecond precision), analogous to `timestamptz` but without timezone assumptions
- `ZonedDateTime` type: a datetime with a named IANA timezone and calendar, supporting wall-clock arithmetic that respects DST transitions
- `PlainDateTime` type: a calendar date and clock time with no timezone
- `PlainDate` type: a calendar date with no time or timezone
- `PlainTime` type: a clock time with no date or timezone
- `PlainYearMonth` type: a year-month value (e.g. 2026-05)
- `PlainMonthDay` type: a month-day value (e.g. 05-04)
- `Duration` type: an ISO 8601 duration with distinct date and time components
- Full IANA timezone support: 598 identifiers with a compile-time binary-search index (`tz_index.rs`)
- Multi-calendar support: 17 calendars (`buddhist`, `chinese`, `coptic`, `dangi`, `ethioaa`, `ethiopic`, `gregory`, `hebrew`, `indian`, `islamic-civil`, `islamic-tbla`, `islamic-umalqura`, `iso8601`, `japanese`, `julian`, `persian`, `roc`) with compile-time index (`cal_index.rs`)
- Arithmetic operators (`+`, `-`) for all applicable type pairs
- Comparison operators (`<`, `<=`, `=`, `>=`, `>`, `<>`) for all types
- `now()` functions returning the current instant in all applicable types
- `duration_round()` and `duration_total()` per the Temporal specification
- `duration_add()` and `duration_subtract()` with optional `relativeTo` for calendar-aware arithmetic
- Explicit casts from native PostgreSQL `date`, `time`, `timestamp`, and `timestamptz` types
- Compact binary on-disk storage using `PgVarlena<T>` (zero heap allocation per value)
- Extension schema: all objects installed under the `temporal` schema
- Distribution pipeline: GitHub Actions CI (fmt, clippy, tests on pg16/17/18 × Linux/macOS/Windows) and release workflow (binary archives + PGXN source zip on tagged releases)

[Unreleased]: https://github.com/acalvino4/pg_temporal/compare/v0.0.1...HEAD
