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
SELECT plain_datetime_year(dt), plain_datetime_month(dt), plain_datetime_day(dt)
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

Output omits the calendar annotation for `iso8601` (the only supported calendar in the current release).

## SQL functions

All numeric accessors return `integer`. Sub-second fields (`millisecond`, `microsecond`, `nanosecond`) are **independent components**, not cumulative — a value of `1500` microseconds is stored as `1` millisecond and `500` microseconds.

### Date components

| Function                          | Range | Description   |
| --------------------------------- | ----- | ------------- |
| `plain_datetime_year(pdt) → int`  | any   | Calendar year |
| `plain_datetime_month(pdt) → int` | 1–12  | Month of year |
| `plain_datetime_day(pdt) → int`   | 1–31  | Day of month  |

```sql
SELECT
  plain_datetime_year('2025-03-01T14:30:00'::temporal.plaindatetime),
  plain_datetime_month('2025-03-01T14:30:00'::temporal.plaindatetime),
  plain_datetime_day('2025-03-01T14:30:00'::temporal.plaindatetime);
-- 2025 | 3 | 1
```

### Time components

| Function                                | Range | Description           |
| --------------------------------------- | ----- | --------------------- |
| `plain_datetime_hour(pdt) → int`        | 0–23  | Hour of day           |
| `plain_datetime_minute(pdt) → int`      | 0–59  | Minute of hour        |
| `plain_datetime_second(pdt) → int`      | 0–59  | Second of minute      |
| `plain_datetime_millisecond(pdt) → int` | 0–999 | Millisecond component |
| `plain_datetime_microsecond(pdt) → int` | 0–999 | Microsecond component |
| `plain_datetime_nanosecond(pdt) → int`  | 0–999 | Nanosecond component  |

```sql
SELECT
  plain_datetime_hour('2025-03-01T14:30:01.002003004'::temporal.plaindatetime),
  plain_datetime_millisecond('2025-03-01T14:30:01.002003004'::temporal.plaindatetime),
  plain_datetime_microsecond('2025-03-01T14:30:01.002003004'::temporal.plaindatetime),
  plain_datetime_nanosecond('2025-03-01T14:30:01.002003004'::temporal.plaindatetime);
-- 14 | 2 | 3 | 4
```

### Calendar

### `plain_datetime_calendar(pdt plaindatetime) → text`

Returns the calendar identifier stored with the value.

```sql
SELECT plain_datetime_calendar('2025-03-01T14:30:00'::temporal.plaindatetime);
-- iso8601
```

## Planned

- Comparison operators (`<`, `<=`, `=`, `>=`, `>`)
- Arithmetic functions (`add`, `subtract`, `until`, `since`)
- Constructor functions
- Multi-calendar support (non-ISO calendars)
