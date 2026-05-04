# Quickstart

> [!WARNING]
> - Has not been tested on postgres <18 or windows yet.
> - This is not a guide to the Temporal spec — see the [Temporal documentation](https://tc39.es/proposal-temporal/docs/) for full type definitions if you are unfamiliar.

## Prerequisites

- **PostgreSQL 18** — On macOS via Homebrew: `brew install postgresql`. You may also need to add to path with `brew link postgresql`
- **Rust 1.93.1** — via [rustup](https://rustup.rs/). `rust-toolchain.toml` ensures correct version gets installed.
- **cargo-run-bin** — `cargo install cargo-run-bin`
- Install project dependencies - `cargo bin --install`

## Installation

```sh
# Build the extension from source and install into your local PostgreSQL 18
cargo pgrx install --features pg18

# Start an interactive psql session with the extension auto-loaded
cargo pgrx run pg18

# Enable the extension in your database
CREATE EXTENSION pg_temporal;

# All functions live in the `temporal` schema. Add it to `search_path` so you don't have to schema-qualify every function call
# Updates for current and future sessions
SET search_path = temporal, "$user", public;
ALTER DATABASE pg_temporal SET search_path = temporal, "$user", public;
```

## Basic storage and retrieval

```sql
CREATE TABLE events (
  id          serial PRIMARY KEY,
  name        text,
  -- full timestamp with timezone and calendar
  scheduled   temporal.zoneddatetime,
  -- absolute UTC moment (no zone or calendar)
  logged_at   temporal.instant,
  -- wall-clock time with no timezone (e.g. recurring alarm)
  local_time  temporal.plaindatetime
);

INSERT INTO events (name, scheduled, logged_at, local_time) VALUES
  (
    'Tokyo launch',
    '2025-06-15T09:00:00+09:00[Asia/Tokyo]'::temporal.zoneddatetime,
    '2025-06-15T00:00:00Z'::temporal.instant,
    '2025-06-15T09:00:00'::temporal.plaindatetime
  ),
  (
    'NY standup',
    '2025-06-15T08:00:00-04:00[America/New_York]'::temporal.zoneddatetime,
    '2025-06-15T12:00:00Z'::temporal.instant,
    '2025-06-15T08:00:00'::temporal.plaindatetime
  ),
  (
    'Persian calendar meeting',
    '2025-06-15T14:00:00+03:30[Asia/Tehran][u-ca=persian]'::temporal.zoneddatetime,
    '2025-06-15T10:30:00Z'::temporal.instant,
    '2025-06-15T14:00:00[u-ca=persian]'::temporal.plaindatetime
  ),
  (
    'Hebrew calendar event',
    '2025-06-15T18:00:00+03:00[Asia/Jerusalem][u-ca=hebrew]'::temporal.zoneddatetime,
    '2025-06-15T15:00:00Z'::temporal.instant,
    '2025-06-15T18:00:00[u-ca=hebrew]'::temporal.plaindatetime
  );

-- ZonedDateTimes sort chronologically by actual instant they occured
SELECT name, scheduled, logged_at, local_time FROM events ORDER BY scheduled;

-- Extract timezone and calendar from each time.
SELECT
  name,
  zoned_datetime_timezone(scheduled)            AS tz,
  zoned_datetime_calendar(scheduled)            AS cal,
  zoned_datetime_epoch_ns(scheduled)::numeric   AS epoch_ns
FROM events;

-- Get current time in different forms:
SELECT temporal_now_instant();                        -- absolute UTC moment
SELECT temporal_now_zoneddatetime('America/Chicago'); -- with zone
SELECT temporal_now_zoneddatetime('Asia/Tokyo');      -- different zone, same instant
SELECT temporal_now_plaindatetime('Europe/London');   -- wall-clock, zone not stored
```

### Arithmetic

```sql
-- Instant: add 6 hours
SELECT instant_add(
  '2025-03-01T00:00:00Z'::temporal.instant,
  'PT6H'::temporal.duration
)::text;
-- 2025-03-01T06:00:00Z

-- Instant: duration between two moments (result uses seconds as largest unit)
SELECT instant_until(
  '2025-03-01T00:00:00Z'::temporal.instant,
  '2025-03-01T02:00:00Z'::temporal.instant
)::text;
-- PT7200S

-- ZonedDateTime: add 1 hour
SELECT zoned_datetime_add(
  '2025-03-09T01:30:00-05:00[America/New_York]'::temporal.zoneddatetime,
  'PT1H'::temporal.duration
)::text;
-- 2025-03-09T03:30:00-04:00[America/New_York]  ← DST transition respected

-- Subtract 1 day
SELECT zoned_datetime_subtract(
  '2025-06-15T09:00:00+00:00[UTC]'::temporal.zoneddatetime,
  'P1D'::temporal.duration
)::text;

-- Duration between two timestamps
SELECT zoned_datetime_until(
  '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime,
  '2025-03-03T12:00:00+00:00[UTC]'::temporal.zoneddatetime
)::text;
-- PT60H

-- PlainDateTime: add 1 month in the Persian calendar (month boundaries are calendar-aware)
SELECT plain_datetime_add(
  '1403-12-01T00:00:00[u-ca=persian]'::temporal.plaindatetime,
  'P1M'::temporal.duration
)::text;
-- 1404-01-01T00:00:00[u-ca=persian]
```

### Comparison and identity equality

```sql
-- Same instant, different zones → NOT equal (identity equality)
SELECT '2025-03-01T02:16:10+00:00[UTC]'::temporal.zoneddatetime
     = '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime;
-- false

-- ORDER BY sorts chronologically (by epoch nanoseconds)
SELECT name, ts FROM events ORDER BY ts;
```

### Construct from epoch nanoseconds

```sql
-- Instant
SELECT make_instant('1609459200000000000')::text;
-- 2021-01-01T00:00:00Z

-- ZonedDateTime
SELECT make_zoneddatetime('1740791770000000000', 'America/New_York', 'iso8601')::text;
```

## Duration

A vector of ISO 8601 date/time components stored independently — `PT90S` and `PT1M30S` are distinct values.

```sql
CREATE TABLE schedules (
  id       serial PRIMARY KEY,
  label    text,
  interval temporal.duration
);

INSERT INTO schedules (label, interval) VALUES
  ('Daily reminder',   'P1D'::temporal.duration),
  ('Quarterly review', 'P3M'::temporal.duration);

-- Extract components
SELECT duration_hours('PT2H30M15S'::temporal.duration),    -- 2
       duration_minutes('PT2H30M15S'::temporal.duration),  -- 30
       duration_seconds('PT2H30M15S'::temporal.duration);  -- 15

-- Total in a given unit
SELECT duration_total('PT2H30M'::temporal.duration, 'minute');  -- 150.0

-- How many days in the next calendar month? (leap-year aware)
SELECT duration_total_zoned(
  'P1M'::temporal.duration, 'day',
  '2024-02-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
);  -- 29.0

-- Add two durations
SELECT duration_add('PT1H'::temporal.duration, 'PT45M'::temporal.duration)::text;  -- PT1H45M

-- Utility
SELECT duration_negated('PT1H30M'::temporal.duration)::text;  -- -PT1H30M
SELECT duration_abs('-P1Y6M'::temporal.duration)::text;       -- P1Y6M
SELECT duration_round('PT1H30M'::temporal.duration, 'hour')::text;  -- PT2H

-- Round with a calendar anchor (required when duration has date components)
SELECT duration_round_plain(
  'P1Y6M'::temporal.duration, 'year',
  '2025-01-01T00:00:00'::temporal.plaindatetime
)::text;  -- P2Y
```

### DST disambiguation

Controls how wall-clock times that fall in a DST gap or fold are resolved when parsing `ZonedDateTime`.

```sql
-- Show current setting
SHOW pg_temporal.default_disambiguation;

-- Options: compatible (default), earlier, later, reject
SET pg_temporal.default_disambiguation = 'reject';

-- With 'reject', ambiguous input raises an error
SELECT '2025-03-09T02:30:00-05:00[America/New_York]'::temporal.zoneddatetime;
-- ERROR: ambiguous datetime in DST gap
```

## Next steps

- [ZonedDateTime reference](usage/zoned_datetime.md)
- [Instant reference](usage/instant.md)
- [PlainDateTime reference](usage/plain_datetime.md)
- [Duration reference](usage/duration.md)
- [Contributing / development guide](contributing.md)
