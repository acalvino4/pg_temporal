# Quickstart

This guide walks through installing `pg_temporal` and using its core features with real SQL examples.

## Installation

### From source

```sh
# Build and install into your local PostgreSQL 18
cargo pgrx install --features pg18

# Or start an interactive psql session with the extension auto-loaded
cargo pgrx run pg18
```

### Enable the extension in your database

```sql
CREATE EXTENSION pg_temporal;
```

This creates the `temporal` schema containing all types and catalog tables.

All functions live in the `temporal` schema. Add it to your `search_path` so you don't have to schema-qualify every function call:

```sql
-- Current session only
SET search_path = temporal, "$user", public;

-- Persist for all future connections to this database
ALTER DATABASE your_db SET search_path = temporal, "$user", public;
```

> **Note:** `ALTER DATABASE` only takes effect for new connections. Run the `SET` command in your current session too.

---

## Types overview

| Type                     | Use when…                                             |
| ------------------------ | ----------------------------------------------------- |
| `temporal.zoneddatetime` | You need a full timestamp with timezone (most events) |
| `temporal.instant`       | You care only about the absolute moment, not the zone |
| `temporal.plaindatetime` | The value has no timezone (recurring alarms, agendas) |
| `temporal.duration`      | You need to represent or compute a span of time       |

---

## ZonedDateTime

The richest type: an exact instant **plus** an IANA timezone **plus** a calendar. Two values are equal only when all three match.

### Text format

```
2025-03-01T11:16:10+09:00[Asia/Tokyo]
                   │      └─ IANA timezone annotation (required)
                   └─ UTC offset (required)
```

### Store and retrieve

```sql
CREATE TABLE events (
  id   serial PRIMARY KEY,
  name text,
  ts   temporal.zoneddatetime
);

INSERT INTO events (name, ts) VALUES
  ('Launch',      '2025-06-15T09:00:00+00:00[UTC]'::temporal.zoneddatetime),
  ('Tokyo sync',  '2025-06-15T18:00:00+09:00[Asia/Tokyo]'::temporal.zoneddatetime),
  ('NY standup',  '2025-06-15T08:00:00-04:00[America/New_York]'::temporal.zoneddatetime);

SELECT name, ts FROM events ORDER BY ts;
```

### Inspect fields

```sql
SELECT
  zoned_datetime_timezone(ts)   AS tz,
  zoned_datetime_calendar(ts)   AS cal,
  zoned_datetime_epoch_ns(ts)::numeric AS epoch_ns
FROM events;
```

### Current time

```sql
SELECT temporal_now_zoneddatetime('America/Chicago');
SELECT temporal_now_zoneddatetime('Europe/London');
```

### Arithmetic

```sql
-- Add 1 hour
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
SELECT make_zoneddatetime('1740791770000000000', 'America/New_York', 'iso8601')::text;
```

---

## Instant

An absolute UTC moment with no timezone or calendar context.

### Text format

```
2025-03-01T02:16:10Z          -- UTC (Z suffix)
2025-03-01T11:16:10+09:00     -- offset accepted, normalized to UTC on storage
```

### Store and retrieve

```sql
CREATE TABLE logs (
  id  serial PRIMARY KEY,
  ts  temporal.instant
);

INSERT INTO logs (ts) VALUES
  ('2025-03-01T02:16:10Z'::temporal.instant),
  ('2025-03-01T11:16:10+09:00'::temporal.instant);  -- same instant as above

SELECT ts FROM logs;
-- 2025-03-01T02:16:10Z
-- 2025-03-01T02:16:10Z
```

### Current time

```sql
SELECT temporal_now_instant();
```

### Arithmetic

```sql
-- Add 6 hours
SELECT instant_add(
  '2025-03-01T00:00:00Z'::temporal.instant,
  'PT6H'::temporal.duration
)::text;
-- 2025-03-01T06:00:00Z

-- Duration between two instants (result uses seconds as largest unit)
SELECT instant_until(
  '2025-03-01T00:00:00Z'::temporal.instant,
  '2025-03-01T02:00:00Z'::temporal.instant
)::text;
-- PT7200S
```

### Construct from epoch nanoseconds

```sql
SELECT make_instant('1609459200000000000')::text;
-- 2021-01-01T00:00:00Z
```

---

## PlainDateTime

A wall-clock date and time with no timezone. Cannot be compared to `Instant` or `ZonedDateTime` without supplying a timezone separately.

### Text format

```
2025-03-01T14:30:00
2025-03-01T14:30:00.123456789   -- nanosecond precision
```

### Store and retrieve

```sql
CREATE TABLE meetings (
  id   serial PRIMARY KEY,
  name text,
  dt   temporal.plaindatetime
);

INSERT INTO meetings (name, dt) VALUES
  ('Weekly sync',   '2025-06-15T10:00:00'::temporal.plaindatetime),
  ('Lunch',         '2025-06-15T12:00:00'::temporal.plaindatetime);

SELECT name, dt FROM meetings ORDER BY dt;
```

### Extract fields

```sql
SELECT
  plain_datetime_year(dt),
  plain_datetime_month(dt),
  plain_datetime_day(dt),
  plain_datetime_hour(dt),
  plain_datetime_minute(dt)
FROM meetings;
```

### Current time (projected into a timezone)

```sql
-- Timezone is used only to compute wall-clock fields; it is NOT stored.
SELECT temporal_now_plaindatetime('Asia/Tokyo');
```

### Arithmetic

```sql
-- Add 1 month (Jan 31 → Feb 28, overflow clamped)
SELECT plain_datetime_add(
  '2025-01-31T12:00:00'::temporal.plaindatetime,
  'P1M'::temporal.duration
)::text;
-- 2025-02-28T12:00:00

-- Days between two datetimes
SELECT plain_datetime_until(
  '2025-03-01T00:00:00'::temporal.plaindatetime,
  '2025-03-08T00:00:00'::temporal.plaindatetime
)::text;
-- P7D
```

### Construct from components

```sql
SELECT make_plaindatetime(2025, 6, 15, 14, 30, 0)::text;
-- 2025-06-15T14:30:00

-- With sub-second precision
SELECT make_plaindatetime(2025, 6, 15, 14, 30, 0, 123, 456, 789)::text;
-- 2025-06-15T14:30:00.123456789
```

---

## Duration

A vector of date and time components. Components are stored independently — `PT90S` and `PT1M30S` are distinct values.

### Text format (ISO 8601)

```
P1Y2M3DT4H5M6S    -- 1 year, 2 months, 3 days, 4 hours, 5 minutes, 6 seconds
PT2H30M           -- 2 hours, 30 minutes
PT0.000000001S    -- 1 nanosecond
-P1Y              -- negative 1 year
```

### Store and retrieve

```sql
CREATE TABLE schedules (
  id       serial PRIMARY KEY,
  label    text,
  interval temporal.duration
);

INSERT INTO schedules (label, interval) VALUES
  ('Daily reminder',   'P1D'::temporal.duration),
  ('Quarterly review', 'P3M'::temporal.duration),
  ('Short delay',      'PT30M'::temporal.duration);

SELECT label, interval FROM schedules;
```

### Extract components

```sql
SELECT
  duration_hours('PT2H30M15S'::temporal.duration),
  duration_minutes('PT2H30M15S'::temporal.duration),
  duration_seconds('PT2H30M15S'::temporal.duration);
-- 2 | 30 | 15

SELECT duration_total('PT2H30M'::temporal.duration, 'minute');
-- 150.0
```

### Utility functions

```sql
SELECT duration_negated('PT1H30M'::temporal.duration)::text;    -- -PT1H30M
SELECT duration_abs('-P1Y6M'::temporal.duration)::text;         -- P1Y6M
SELECT duration_sign('-PT1S'::temporal.duration);               -- -1
SELECT duration_is_zero('PT0S'::temporal.duration);             -- true
```

### Add and subtract time-only durations

```sql
SELECT duration_add(
  'PT1H'::temporal.duration,
  'PT45M'::temporal.duration
)::text;  -- PT1H45M
```

### Round a duration

```sql
-- Round to nearest hour
SELECT duration_round('PT1H29M'::temporal.duration, 'hour')::text;  -- PT1H
SELECT duration_round('PT1H30M'::temporal.duration, 'hour')::text;  -- PT2H

-- Round with a calendar anchor (needed when duration has date components)
SELECT duration_round_plain(
  'P1Y6M'::temporal.duration, 'year',
  '2025-01-01T00:00:00'::temporal.plaindatetime
)::text;  -- P2Y
```

### Get total in a given unit

```sql
-- How many days in the next calendar month from a given date?
SELECT duration_total_zoned(
  'P1M'::temporal.duration, 'day',
  '2024-02-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
);  -- 29.0  (2024 is a leap year)
```

---

## Configuration

Both GUCs apply cluster-wide but are overridable per session.

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

### Timezone alias policy

```sql
-- Options: iana (default), jodatime
-- Requires superuser; controls alias resolution at insert time (planned)
ALTER SYSTEM SET pg_temporal.alias_policy = 'iana';
```

---

## Multi-calendar support

All Temporal-spec calendars are accepted via the `[u-ca=…]` annotation. The ISO 8601 calendar is the default and its annotation is suppressed on output.

```sql
-- Japanese calendar annotation round-trips
SELECT '2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=japanese]'::temporal.zoneddatetime::text;

-- Calendar-aware year accessor
SELECT plain_datetime_year(
  '2025-03-01T00:00:00[u-ca=persian]'::temporal.plaindatetime
);
-- 1403  (Persian Solar Hijri year)

-- Same instant, different calendars → not equal
SELECT '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
     = '2025-03-01T00:00:00+00:00[UTC][u-ca=japanese]'::temporal.zoneddatetime;
-- false
```

---

## Next steps

- [ZonedDateTime reference](usage/zoned_datetime.md)
- [Instant reference](usage/instant.md)
- [PlainDateTime reference](usage/plain_datetime.md)
- [Duration reference](usage/duration.md)
- [Contributing / development guide](contributing.md)
