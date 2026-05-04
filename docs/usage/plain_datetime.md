# PlainDateTime

`temporal.plaindatetime` is a calendar date and wall-clock time with nanosecond precision, but **no timezone**. It is the pg_temporal equivalent of the [TC39 Temporal `PlainDateTime`](https://tc39.es/proposal-temporal/#sec-temporal-plaindatetime).

A `PlainDateTime` cannot be converted to an absolute instant without supplying a timezone. Use it for inherently local datetimes — a meeting time printed on an agenda, a birth date and time recorded with no timezone context, a recurring alarm that fires at the same wall-clock time regardless of DST.

## Quick start

```sql
-- Store a plain datetime
INSERT INTO meetings (dt) VALUES
  ('2025-03-01T14:30:00'::temporal.plaindatetime);

-- Nanosecond precision is preserved
INSERT INTO log (dt) VALUES
  ('2025-03-01T00:00:00.000000001'::temporal.plaindatetime);

-- Read it back
SELECT dt FROM meetings;
-- 2025-03-01T14:30:00

-- Extract individual fields
SELECT plaindatetime_year(dt), plaindatetime_month(dt), plaindatetime_day(dt)
FROM meetings;
```

## Text format

Input accepts an ISO 8601 date/time string without a UTC offset or timezone annotation:

```
2025-03-01T14:30:00
2025-03-01T14:30:00.123              -- millisecond precision
2025-03-01T14:30:00.000000001        -- nanosecond precision
2025-03-01T14:30:00[u-ca=iso8601]    -- explicit calendar annotation (optional)
```

UTC offsets are not accepted — use `temporal.instant` or `temporal.zoneddatetime` for values that have a timezone.

Output omits the calendar annotation for `iso8601`. Non-ISO calendar annotations (e.g. `[u-ca=japanese]`) are preserved on output — see [Multi-calendar support](#multi-calendar-support) below.

## SQL functions

All numeric accessors return `integer`. Sub-second fields (`millisecond`, `microsecond`, `nanosecond`) are **independent components**, not cumulative — a value of `1500` microseconds is stored as `1` millisecond and `500` microseconds.

### Date components

| Function                          | Range | Description   |
| --------------------------------- | ----- | ------------- |
| `plaindatetime_year(pdt) → int`  | any   | Calendar year |
| `plaindatetime_month(pdt) → int` | 1–12  | Month of year |
| `plaindatetime_day(pdt) → int`   | 1–31  | Day of month  |

```sql
SELECT
  plaindatetime_year('2025-03-01T14:30:00'::temporal.plaindatetime),
  plaindatetime_month('2025-03-01T14:30:00'::temporal.plaindatetime),
  plaindatetime_day('2025-03-01T14:30:00'::temporal.plaindatetime);
-- 2025 | 3 | 1
```

### Time components

| Function                                | Range | Description           |
| --------------------------------------- | ----- | --------------------- |
| `plaindatetime_hour(pdt) → int`        | 0–23  | Hour of day           |
| `plaindatetime_minute(pdt) → int`      | 0–59  | Minute of hour        |
| `plaindatetime_second(pdt) → int`      | 0–59  | Second of minute      |
| `plaindatetime_millisecond(pdt) → int` | 0–999 | Millisecond component |
| `plaindatetime_microsecond(pdt) → int` | 0–999 | Microsecond component |
| `plaindatetime_nanosecond(pdt) → int`  | 0–999 | Nanosecond component  |

```sql
SELECT
  plaindatetime_hour('2025-03-01T14:30:01.002003004'::temporal.plaindatetime),
  plaindatetime_millisecond('2025-03-01T14:30:01.002003004'::temporal.plaindatetime),
  plaindatetime_microsecond('2025-03-01T14:30:01.002003004'::temporal.plaindatetime),
  plaindatetime_nanosecond('2025-03-01T14:30:01.002003004'::temporal.plaindatetime);
-- 14 | 2 | 3 | 4
```

### Calendar

### `plaindatetime_calendar(pdt plaindatetime) → text`

Returns the calendar identifier stored with the value.

```sql
SELECT plaindatetime_calendar('2025-03-01T14:30:00'::temporal.plaindatetime);
-- iso8601
```

## Comparison operators

All six comparison operators (`<`, `<=`, `=`, `<>`, `>=`, `>`) are supported and backed by a B-tree operator class, enabling `ORDER BY`, `GROUP BY`, `DISTINCT`, and B-tree indexes. Two `PlainDateTime` values are equal when all date/time fields and the calendar identifier match.

```sql
SELECT '2025-03-01T12:00:00'::temporal.plaindatetime
       < '2025-03-02T12:00:00'::temporal.plaindatetime;  -- true

-- ORDER BY sorts chronologically
SELECT * FROM meetings ORDER BY dt;
```

### `plaindatetime_cmp(a plaindatetime, b plaindatetime) → integer`

Returns -1, 0, or 1.

## Arithmetic

### `plaindatetime_add(pdt plaindatetime, dur duration) → plaindatetime`

Adds a duration to a plain datetime. Day-of-month overflow is clamped (`Constrain`): e.g. Jan 31 + P1M → Feb 28/29.

```sql
SELECT plaindatetime_add(
  '2025-03-01T12:00:00'::temporal.plaindatetime,
  'P1D'::temporal.duration
)::text;  -- 2025-03-02T12:00:00
```

### `plaindatetime_subtract(pdt plaindatetime, dur duration) → plaindatetime`

Subtracts a duration from a plain datetime with the same overflow behavior.

```sql
SELECT plaindatetime_subtract(
  '2025-03-02T12:00:00'::temporal.plaindatetime,
  'P1D'::temporal.duration
)::text;  -- 2025-03-01T12:00:00
```

### `plaindatetime_until(pdt plaindatetime, other plaindatetime) → duration`

Returns the duration from `pdt` to `other`. The default largest unit is days.

```sql
SELECT plaindatetime_until(
  '2025-03-01T00:00:00'::temporal.plaindatetime,
  '2025-03-02T00:00:00'::temporal.plaindatetime
)::text;  -- P1D
```

### `plaindatetime_since(pdt plaindatetime, other plaindatetime) → duration`

Returns the duration elapsed from `other` to `pdt`. The default largest unit is days.

```sql
SELECT plaindatetime_since(
  '2025-03-02T00:00:00'::temporal.plaindatetime,
  '2025-03-01T00:00:00'::temporal.plaindatetime
)::text;  -- P1D
```

## Constructors

### `make_plaindatetime(year int, month int, day int, hour int, minute int, second int [, millisecond int, microsecond int, nanosecond int, cal text]) → plaindatetime`

Constructs a `PlainDateTime` from individual field values. `millisecond`, `microsecond`, `nanosecond`, and `cal` are optional and default to `0`, `0`, `0`, and `'iso8601'` respectively.

```sql
SELECT make_plaindatetime(2025, 6, 15, 12, 30, 0)::text;
-- 2025-06-15T12:30:00

SELECT make_plaindatetime(2025, 6, 15, 12, 30, 0, 123, 456, 789)::text;
-- 2025-06-15T12:30:00.123456789

-- Invalid dates are rejected at construction time
SELECT make_plaindatetime(2025, 2, 30, 0, 0, 0);  -- error
```

## Multi-calendar support

All calendars supported by the Temporal specification are accepted via the `[u-ca=…]` annotation on input. The date/time fields are always stored internally as ISO 8601; accessor functions (`plaindatetime_year`, `plaindatetime_month`, `plaindatetime_day`) return calendar-specific values when a non-ISO calendar is used.

```sql
-- Japanese calendar annotation is preserved on output
SELECT '2025-03-01T11:16:10[u-ca=japanese]'::temporal.plaindatetime::text;
-- 2025-03-01T11:16:10[u-ca=japanese]

-- The calendar accessor returns the stored calendar name
SELECT plaindatetime_calendar('2025-03-01T00:00:00[u-ca=persian]'::temporal.plaindatetime);
-- persian

-- Year accessor returns the calendar-specific year
SELECT plaindatetime_year('2025-03-01T00:00:00[u-ca=persian]'::temporal.plaindatetime);
-- 1403  (Persian Solar Hijri year before Nowruz)
```

The ISO 8601 calendar annotation (`[u-ca=iso8601]`) is accepted on input but suppressed on output.

## Now functions

### `temporal_now_plaindatetime(tz text) → plaindatetime`

Returns the current `PlainDateTime` at transaction start time as observed in the given IANA timezone, with an ISO 8601 calendar. The timezone is used only to project wall-clock fields; it is **not** stored in the resulting value.

```sql
SELECT temporal_now_plaindatetime('America/New_York');
SELECT temporal_now_plaindatetime('Europe/Paris');
```

## Limitations / planned

- Cast from/to `timestamp` (explicit casts only) — not yet implemented
