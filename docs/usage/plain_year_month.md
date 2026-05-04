# PlainYearMonth

`temporal.plainyearmonth` is a calendar year and month with no day or time component. It is the pg_temporal equivalent of the [TC39 Temporal `PlainYearMonth`](https://tc39.es/proposal-temporal/#sec-temporal-plainyearmonth).

Use it for values that represent a whole month rather than a specific date — billing periods, reporting intervals, subscription months, or any situation where attaching a day would add false precision.

## Quick start

```sql
-- Store a year-month
INSERT INTO billing_periods (period) VALUES
  ('2025-03'::temporal.plainyearmonth);

-- Read it back
SELECT period FROM billing_periods;
-- 2025-03

-- Extract fields
SELECT
  plainyearmonth_year(period),
  plainyearmonth_month(period)
FROM billing_periods;
-- 2025 | 3
```

## Text format

Input accepts an ISO 8601 year-month string, optionally with a calendar annotation:

```
2025-03
2025-03[u-ca=iso8601]    -- explicit ISO annotation (accepted, suppressed on output)
2025-03[u-ca=persian]    -- non-ISO calendar preserved on output
```

Full date strings (e.g. `2025-03-15`) are also accepted; the day is discarded.

Output produces an ISO 8601 year-month string. The `[u-ca=iso8601]` annotation is suppressed; non-ISO annotations are included.

## SQL functions

### `plainyearmonth_year(pym plainyearmonth) → integer`

Returns the calendar year.

```sql
SELECT plainyearmonth_year('2025-03'::temporal.plainyearmonth);
-- 2025
```

### `plainyearmonth_month(pym plainyearmonth) → integer`

Returns the calendar month (1-indexed).

```sql
SELECT plainyearmonth_month('2025-03'::temporal.plainyearmonth);
-- 3
```

### `plainyearmonth_calendar(pym plainyearmonth) → text`

Returns the calendar identifier stored with the value.

```sql
SELECT plainyearmonth_calendar('2025-03'::temporal.plainyearmonth);
-- iso8601
```

## Comparison operators

All six comparison operators (`<`, `<=`, `=`, `<>`, `>=`, `>`) are supported and backed by a B-tree operator class, enabling `ORDER BY`, `GROUP BY`, `DISTINCT`, and B-tree indexes. Two `PlainYearMonth` values are equal when the ISO year, ISO month, and calendar identifier all match.

```sql
SELECT '2025-01'::temporal.plainyearmonth
       < '2025-06'::temporal.plainyearmonth;  -- true

-- ORDER BY sorts chronologically
SELECT * FROM billing_periods ORDER BY period;
```

### `plainyearmonth_cmp(a plainyearmonth, b plainyearmonth) → integer`

Returns -1, 0, or 1.

## Arithmetic

Per the Temporal spec, only year and month components may be added or subtracted from a `PlainYearMonth`. Supplying a duration that contains days, weeks, hours, or smaller units will raise an error.

### `plainyearmonth_add(pym plainyearmonth, dur duration) → plainyearmonth`

Adds a duration (years/months only) to a plain year-month.

```sql
SELECT plainyearmonth_add(
  '2025-11'::temporal.plainyearmonth,
  'P2M'::temporal.duration
)::text;  -- 2026-01

SELECT plainyearmonth_add(
  '2025-03'::temporal.plainyearmonth,
  'P1Y'::temporal.duration
)::text;  -- 2026-03
```

### `plainyearmonth_subtract(pym plainyearmonth, dur duration) → plainyearmonth`

Subtracts a duration (years/months only) from a plain year-month.

```sql
SELECT plainyearmonth_subtract(
  '2025-03'::temporal.plainyearmonth,
  'P3M'::temporal.duration
)::text;  -- 2024-12
```

### `plainyearmonth_until(pym plainyearmonth, other plainyearmonth) → duration`

Returns the duration from `pym` to `other`. The default largest unit is months.

```sql
SELECT plainyearmonth_until(
  '2025-01'::temporal.plainyearmonth,
  '2025-04'::temporal.plainyearmonth
)::text;  -- P3M
```

### `plainyearmonth_since(pym plainyearmonth, other plainyearmonth) → duration`

Returns the duration elapsed from `other` to `pym`. The default largest unit is months.

```sql
SELECT plainyearmonth_since(
  '2025-04'::temporal.plainyearmonth,
  '2025-01'::temporal.plainyearmonth
)::text;  -- P3M
```

## Constructors

### `make_plainyearmonth(year int, month int [, cal text]) → plainyearmonth`

Constructs a `PlainYearMonth` from year and month values. `cal` is optional and defaults to `'iso8601'`.

```sql
SELECT make_plainyearmonth(2025, 3)::text;
-- 2025-03

SELECT make_plainyearmonth(2025, 3, 'iso8601')::text;
-- 2025-03

-- Invalid months are rejected
SELECT make_plainyearmonth(2025, 13);  -- error
```

## Multi-calendar support

All calendars supported by the Temporal specification are accepted via the `[u-ca=…]` annotation on input. Fields are stored internally as ISO 8601; accessor functions return calendar-specific values when a non-ISO calendar is used.

```sql
SELECT plainyearmonth_year('2025-03[u-ca=persian]'::temporal.plainyearmonth);
-- 1403  (Persian Solar Hijri year before Nowruz)
```
