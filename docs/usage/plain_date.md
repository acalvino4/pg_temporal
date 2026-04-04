# PlainDate

`temporal.plaindate` is a calendar date with no time component and no timezone. It is the pg_temporal equivalent of the [TC39 Temporal `PlainDate`](https://tc39.es/proposal-temporal/#sec-temporal-plaindate).

Use it for dates that have no meaningful time component — a birth date, a holiday, a contract start date, or any value where attaching a time would add false precision.

## Quick start

```sql
-- Store a plain date
INSERT INTO holidays (d) VALUES
  ('2025-12-25'::temporal.plaindate);

-- Read it back
SELECT d FROM holidays;
-- 2025-12-25

-- Extract individual fields
SELECT plain_date_year(d), plain_date_month(d), plain_date_day(d)
FROM holidays;
-- 2025 | 12 | 25
```

## Text format

Input accepts an ISO 8601 date string, optionally with a calendar annotation:

```
2025-12-25
2025-12-25[u-ca=iso8601]    -- explicit ISO annotation (accepted, suppressed on output)
2025-12-25[u-ca=japanese]   -- non-ISO calendar preserved on output
```

Output produces an ISO 8601 date string. The `[u-ca=iso8601]` annotation is suppressed; non-ISO annotations are included.

## SQL functions

### Date components

| Function                      | Range | Description   |
| ----------------------------- | ----- | ------------- |
| `plain_date_year(pd) → int`   | any   | Calendar year |
| `plain_date_month(pd) → int`  | 1–12  | Month of year |
| `plain_date_day(pd) → int`    | 1–31  | Day of month  |

```sql
SELECT
  plain_date_year('2025-12-25'::temporal.plaindate),
  plain_date_month('2025-12-25'::temporal.plaindate),
  plain_date_day('2025-12-25'::temporal.plaindate);
-- 2025 | 12 | 25
```

### Calendar

#### `plain_date_calendar(pd plaindate) → text`

Returns the calendar identifier stored with the value.

```sql
SELECT plain_date_calendar('2025-12-25'::temporal.plaindate);
-- iso8601
```

## Comparison operators

All six comparison operators (`<`, `<=`, `=`, `<>`, `>=`, `>`) are supported and backed by a B-tree operator class, enabling `ORDER BY`, `GROUP BY`, `DISTINCT`, and B-tree indexes. Two `PlainDate` values are equal when all date fields and the calendar identifier match.

```sql
SELECT '2025-03-01'::temporal.plaindate
       < '2025-12-25'::temporal.plaindate;  -- true

-- ORDER BY sorts chronologically
SELECT * FROM holidays ORDER BY d;
```

### `plain_date_compare(a plaindate, b plaindate) → integer`

Returns -1, 0, or 1.

## Arithmetic

### `plain_date_add(pd plaindate, dur duration) → plaindate`

Adds a duration to a plain date. Day-of-month overflow is clamped (`Constrain`): e.g. Jan 31 + P1M → Feb 28/29.

```sql
SELECT plain_date_add(
  '2025-01-31'::temporal.plaindate,
  'P1M'::temporal.duration
)::text;  -- 2025-02-28
```

### `plain_date_subtract(pd plaindate, dur duration) → plaindate`

Subtracts a duration from a plain date with the same overflow behavior.

```sql
SELECT plain_date_subtract(
  '2025-03-01'::temporal.plaindate,
  'P1D'::temporal.duration
)::text;  -- 2025-02-28
```

### `plain_date_until(pd plaindate, other plaindate) → duration`

Returns the duration from `pd` to `other`. The default largest unit is days.

```sql
SELECT plain_date_until(
  '2025-01-01'::temporal.plaindate,
  '2025-12-31'::temporal.plaindate
)::text;  -- P364D
```

### `plain_date_since(pd plaindate, other plaindate) → duration`

Returns the duration elapsed from `other` to `pd`. The default largest unit is days.

```sql
SELECT plain_date_since(
  '2025-12-31'::temporal.plaindate,
  '2025-01-01'::temporal.plaindate
)::text;  -- P364D
```

## Constructors

### `make_plaindate(year int, month int, day int [, cal text]) → plaindate`

Constructs a `PlainDate` from individual field values. `cal` is optional and defaults to `'iso8601'`.

```sql
SELECT make_plaindate(2025, 12, 25)::text;
-- 2025-12-25

SELECT make_plaindate(2025, 12, 25, 'iso8601')::text;
-- 2025-12-25

-- Invalid dates are rejected at construction time
SELECT make_plaindate(2025, 2, 30);  -- error
```

## Multi-calendar support

All calendars supported by the Temporal specification are accepted via the `[u-ca=…]` annotation on input. Date fields are always stored internally as ISO 8601; accessor functions return calendar-specific values when a non-ISO calendar is used.

```sql
-- Japanese calendar annotation is preserved on output
SELECT '2025-03-01[u-ca=japanese]'::temporal.plaindate::text;
-- 2025-03-01[u-ca=japanese]

-- Year accessor returns the calendar-specific year
SELECT plain_date_year('2025-03-01[u-ca=persian]'::temporal.plaindate);
-- 1403  (Persian Solar Hijri year before Nowruz)
```

## Now functions

### `temporal_now_plaindate(tz text) → plaindate`

Returns the current `PlainDate` at transaction start time as observed in the given IANA timezone. The timezone is used only to determine the current date; it is **not** stored in the resulting value.

```sql
SELECT temporal_now_plaindate('America/New_York');
SELECT temporal_now_plaindate('Asia/Tokyo');
```
