# PlainTime

`temporal.plaintime` is a wall-clock time with nanosecond precision and **no date, timezone, or calendar**. It is the pg_temporal equivalent of the [TC39 Temporal `PlainTime`](https://tc39.es/proposal-temporal/#sec-temporal-plaintime).

Use it for times that are inherently dateless — a business opening time, a scheduled alarm, a recurring event time that applies regardless of what day it falls on.

## Quick start

```sql
-- Store a plain time
INSERT INTO schedule (open_time, close_time) VALUES
  ('09:00:00'::temporal.plaintime,
   '17:00:00'::temporal.plaintime);

-- Nanosecond precision is preserved
INSERT INTO log (ts) VALUES
  ('14:30:00.000000001'::temporal.plaintime);

-- Read it back
SELECT open_time FROM schedule;
-- 09:00:00
```

## Text format

Input accepts an ISO 8601 time string (with or without a leading `T`):

```
14:30:00
14:30:00.123              -- millisecond precision
14:30:00.000000001        -- nanosecond precision
T14:30:00                 -- leading T accepted
```

UTC offsets and timezone annotations are not meaningful for `PlainTime` and will cause a parse error.

Output produces an ISO 8601 time string:

```
14:30:00
14:30:00.123456789
```

## SQL functions

All numeric accessors return `integer`. Sub-second fields (`millisecond`, `microsecond`, `nanosecond`) are **independent components**, not cumulative — a value of `1500` microseconds is stored as `1` millisecond and `500` microseconds.

| Function                             | Range | Description           |
| ------------------------------------ | ----- | --------------------- |
| `plain_time_hour(pt) → int`          | 0–23  | Hour of day           |
| `plain_time_minute(pt) → int`        | 0–59  | Minute of hour        |
| `plain_time_second(pt) → int`        | 0–59  | Second of minute      |
| `plain_time_millisecond(pt) → int`   | 0–999 | Millisecond component |
| `plain_time_microsecond(pt) → int`   | 0–999 | Microsecond component |
| `plain_time_nanosecond(pt) → int`    | 0–999 | Nanosecond component  |

```sql
SELECT
  plain_time_hour('14:30:01.002003004'::temporal.plaintime),
  plain_time_millisecond('14:30:01.002003004'::temporal.plaintime),
  plain_time_microsecond('14:30:01.002003004'::temporal.plaintime),
  plain_time_nanosecond('14:30:01.002003004'::temporal.plaintime);
-- 14 | 2 | 3 | 4
```

## Comparison operators

All six comparison operators (`<`, `<=`, `=`, `<>`, `>=`, `>`) are supported and backed by a B-tree operator class, enabling `ORDER BY`, `GROUP BY`, `DISTINCT`, and B-tree indexes. Two `PlainTime` values are equal when all time fields match.

```sql
SELECT '09:00:00'::temporal.plaintime
       < '17:00:00'::temporal.plaintime;  -- true

-- ORDER BY sorts chronologically
SELECT * FROM schedule ORDER BY open_time;
```

### `plain_time_compare(a plaintime, b plaintime) → integer`

Returns -1, 0, or 1.

## Arithmetic

Time arithmetic **wraps around midnight** — adding 2 hours to `23:00:00` produces `01:00:00`. No date overflow is returned.

### `plain_time_add(pt plaintime, dur duration) → plaintime`

Adds a duration to a plain time.

```sql
SELECT plain_time_add(
  '23:00:00'::temporal.plaintime,
  'PT2H'::temporal.duration
)::text;  -- 01:00:00
```

### `plain_time_subtract(pt plaintime, dur duration) → plaintime`

Subtracts a duration from a plain time.

```sql
SELECT plain_time_subtract(
  '09:30:00'::temporal.plaintime,
  'PT30M'::temporal.duration
)::text;  -- 09:00:00
```

### `plain_time_until(pt plaintime, other plaintime) → duration`

Returns the duration from `pt` to `other`. The default largest unit is hours.

```sql
SELECT plain_time_until(
  '09:00:00'::temporal.plaintime,
  '17:00:00'::temporal.plaintime
)::text;  -- PT8H
```

### `plain_time_since(pt plaintime, other plaintime) → duration`

Returns the duration elapsed from `other` to `pt`. The default largest unit is hours.

```sql
SELECT plain_time_since(
  '17:00:00'::temporal.plaintime,
  '09:00:00'::temporal.plaintime
)::text;  -- PT8H
```

## Constructors

### `make_plaintime(hour int, minute int, second int [, millisecond int, microsecond int, nanosecond int]) → plaintime`

Constructs a `PlainTime` from individual field values. Sub-second fields are optional and default to `0`.

```sql
SELECT make_plaintime(14, 30, 0)::text;
-- 14:30:00

SELECT make_plaintime(14, 30, 0, 123, 456, 789)::text;
-- 14:30:00.123456789

-- Invalid times are rejected at construction time
SELECT make_plaintime(25, 0, 0);  -- error
```

## Now functions

### `temporal_now_plaintime(tz text) → plaintime`

Returns the current `PlainTime` at transaction start time as observed in the given IANA timezone. The timezone is used only to determine the current time; it is **not** stored in the resulting value.

```sql
SELECT temporal_now_plaintime('America/Chicago');
SELECT temporal_now_plaintime('Pacific/Auckland');
```
