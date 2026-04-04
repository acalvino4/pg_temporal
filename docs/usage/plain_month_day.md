# PlainMonthDay

`temporal.plainmonthday` is a calendar month and day with no year or time component. It is the pg_temporal equivalent of the [TC39 Temporal `PlainMonthDay`](https://tc39.es/proposal-temporal/#sec-temporal-plainmonthday).

Use it for recurring annual events that happen on the same date each year — birthdays, holidays, anniversaries, or any schedule that repeats without regard to year.

Because `PlainMonthDay` carries no year, February 29 is a valid value; it represents the leap day without implying a specific year.

## Quick start

```sql
-- Store a recurring event date
INSERT INTO holidays (name, date) VALUES
  ('Christmas',  '--12-25'::temporal.plainmonthday),
  ('Leap Day',   '--02-29'::temporal.plainmonthday);

-- Read it back
SELECT name, date FROM holidays;
-- Christmas  | --12-25
-- Leap Day   | --02-29

-- Extract fields
SELECT plain_month_day_month(date), plain_month_day_day(date) FROM holidays;
-- 12 | 25
-- 2  | 29
```

## Text format

Input accepts two equivalent notations:

```
--12-25       -- RFC 3339 / ISO 8601 month-day (double-dash prefix)
12-25         -- Short form (double-dash may be omitted on input)
12-25[u-ca=iso8601]   -- explicit calendar annotation
```

Output always uses the `--MM-DD` (double-dash) form. The `[u-ca=iso8601]` annotation is suppressed; non-ISO annotations are included.

## SQL functions

### `plain_month_day_month(pmd plainmonthday) → integer`

Returns the calendar month (1-indexed).

```sql
SELECT plain_month_day_month('--12-25'::temporal.plainmonthday);
-- 12
```

### `plain_month_day_day(pmd plainmonthday) → integer`

Returns the day of the month.

```sql
SELECT plain_month_day_day('--12-25'::temporal.plainmonthday);
-- 25
```

### `plain_month_day_calendar(pmd plainmonthday) → text`

Returns the calendar identifier stored with the value.

```sql
SELECT plain_month_day_calendar('--12-25'::temporal.plainmonthday);
-- iso8601
```

## Comparison operators

All six comparison operators (`<`, `<=`, `=`, `<>`, `>=`, `>`) are supported and backed by a B-tree operator class, enabling `ORDER BY`, `GROUP BY`, `DISTINCT`, and B-tree indexes. Two `PlainMonthDay` values are equal when the ISO reference year, ISO month, ISO day, and calendar identifier all match. In practice, ISO calendar values compare by month then day.

```sql
SELECT '--03-15'::temporal.plainmonthday
       < '--06-01'::temporal.plainmonthday;  -- true

-- ORDER BY sorts by month, then day
SELECT * FROM holidays ORDER BY date;
```

### `plain_month_day_compare(a plainmonthday, b plainmonthday) → integer`

Returns -1, 0, or 1.

## Constructors

### `make_plainmonthday(month int, day int [, cal text]) → plainmonthday`

Constructs a `PlainMonthDay` from month and day values. `cal` is optional and defaults to `'iso8601'`. Invalid combinations (e.g. month 2 day 30) are rejected.

```sql
SELECT make_plainmonthday(12, 25)::text;
-- --12-25

SELECT make_plainmonthday(2, 29)::text;   -- valid: leap day
-- --02-29

SELECT make_plainmonthday(2, 30);         -- error: invalid date
```

## Note: no arithmetic

Per the Temporal specification, `PlainMonthDay` does not support addition, subtraction, `since`, or `until`. These operations require a year, which `PlainMonthDay` intentionally omits. Use `temporal.plaindate` or `temporal.plaindatetime` when you need date arithmetic.

## Multi-calendar support

All calendars supported by the Temporal specification are accepted via the `[u-ca=…]` annotation on input. Fields are stored internally as ISO 8601; accessor functions return calendar-specific values when a non-ISO calendar is used.

```sql
SELECT plain_month_day_month('--03-21[u-ca=persian]'::temporal.plainmonthday);
-- 1  (Farvardin 1, Nowruz — first month of the Persian calendar)
```
