# Duration

`temporal.duration` is a vector of date and time components representing an elapsed span. It is the pg_temporal equivalent of the [TC39 Temporal `Duration`](https://tc39.es/proposal-temporal/#sec-temporal-duration).

A `Duration` stores each component independently with no implicit normalization. `PT90S` and `PT1M30S` are distinct values. Use it as the result of datetime arithmetic, or to express precise time spans for scheduling and interval calculations.

## Quick start

```sql
-- Store a duration
INSERT INTO delays (d) VALUES ('PT2H30M'::temporal.duration);

-- ISO 8601 duration string round-trips exactly
SELECT d FROM delays;
-- PT2H30M

-- Mixed calendar and time components
SELECT 'P1Y2M3DT4H5M6S'::temporal.duration;
-- P1Y2M3DT4H5M6S

-- Negative duration
SELECT '-P1Y'::temporal.duration;
-- -P1Y

-- Extract a field
SELECT duration_hours('PT2H30M'::temporal.duration);
-- 2
```

## Text format

Input and output use the [ISO 8601 duration format](https://en.wikipedia.org/wiki/ISO_8601#Durations):

```
P[n]Y[n]M[n]DT[n]H[n]M[n]S
│                 │
│                 └─ time designator (required before any time component)
└─ period designator (always required)
```

Examples:

```
P1Y2M3DT4H5M6S          -- 1 year, 2 months, 3 days, 4 hours, 5 minutes, 6 seconds
PT0.000000001S           -- 1 nanosecond (fractional seconds carry sub-second precision)
-P1Y                     -- negative 1 year
P3W                      -- 3 weeks
```

All non-zero fields in a valid `Duration` must have the same sign — you cannot mix positive and negative components.

## SQL functions

### Calendar components

Calendar components are returned as `bigint` (signed 64-bit integer).

| Function                      | Description      |
| ----------------------------- | ---------------- |
| `duration_years(d) → bigint`  | Years component  |
| `duration_months(d) → bigint` | Months component |
| `duration_weeks(d) → bigint`  | Weeks component  |
| `duration_days(d) → bigint`   | Days component   |

```sql
SELECT
  duration_years('P1Y2M3W4D'::temporal.duration),
  duration_months('P1Y2M3W4D'::temporal.duration),
  duration_weeks('P1Y2M3W4D'::temporal.duration),
  duration_days('P1Y2M3W4D'::temporal.duration);
-- 1 | 2 | 3 | 4
```

### Time components

| Function                            | Type     | Description            |
| ----------------------------------- | -------- | ---------------------- |
| `duration_hours(d) → bigint`        | `bigint` | Hours component        |
| `duration_minutes(d) → bigint`      | `bigint` | Minutes component      |
| `duration_seconds(d) → bigint`      | `bigint` | Seconds component      |
| `duration_milliseconds(d) → bigint` | `bigint` | Milliseconds component |
| `duration_microseconds(d) → text`   | `text`   | Microseconds component |
| `duration_nanoseconds(d) → text`    | `text`   | Nanoseconds component  |

`microseconds` and `nanoseconds` return `text` because there is no native 128-bit integer SQL type. Cast to `numeric` for arithmetic.

```sql
SELECT
  duration_hours('PT1H2M3.004005006S'::temporal.duration),
  duration_minutes('PT1H2M3.004005006S'::temporal.duration),
  duration_seconds('PT1H2M3.004005006S'::temporal.duration),
  duration_milliseconds('PT1H2M3.004005006S'::temporal.duration),
  duration_microseconds('PT1H2M3.004005006S'::temporal.duration)::numeric,
  duration_nanoseconds('PT1H2M3.004005006S'::temporal.duration)::numeric;
-- 1 | 2 | 3 | 4 | 5 | 6
```

## Sign convention

All non-zero fields in a `Duration` must share the same sign. Negative durations are expressed with a leading `-` on the `P` designator, not by negating individual fields. A duration with mixed-sign components is rejected on input.

```sql
SELECT '-P1Y2M'::temporal.duration;   -- ok: -1 years, -2 months
SELECT 'P1Y-2M'::temporal.duration;   -- error: mixed signs
```

## Utility functions

### `duration_negated(d duration) → duration`

Returns a copy with the sign of every component flipped.

```sql
SELECT duration_negated('PT1H30M'::temporal.duration)::text;  -- -PT1H30M
```

### `duration_abs(d duration) → duration`

Returns a copy with all components made non-negative.

```sql
SELECT duration_abs('-PT2H'::temporal.duration)::text;  -- PT2H
```

### `duration_sign(d duration) → integer`

Returns -1, 0, or 1 indicating the overall sign of the duration.

```sql
SELECT duration_sign('PT1H'::temporal.duration);   -- 1
SELECT duration_sign('-P1Y'::temporal.duration);   -- -1
SELECT duration_sign('PT0S'::temporal.duration);   -- 0
```

### `duration_is_zero(d duration) → boolean`

Returns `true` if all components are zero.

```sql
SELECT duration_is_zero('PT0S'::temporal.duration);  -- true
SELECT duration_is_zero('PT1S'::temporal.duration);  -- false
```

## Arithmetic

`duration_add` and `duration_subtract` only accept **time-only** durations (hours, minutes, seconds, milliseconds, microseconds, nanoseconds). If either argument contains calendar components (years, months, weeks, days), an error is raised—use `plain_datetime_add` / `zoned_datetime_add` instead.

### `duration_add(a duration, b duration) → duration`

Adds two time-only durations component-wise.

```sql
SELECT duration_add(
  'PT1H'::temporal.duration,
  'PT30M'::temporal.duration
)::text;  -- PT1H30M
```

### `duration_subtract(a duration, b duration) → duration`

Subtracts one time-only duration from another component-wise.

```sql
SELECT duration_subtract(
  'PT2H'::temporal.duration,
  'PT30M'::temporal.duration
)::text;  -- PT1H30M
```

## Rounding

Durations can be rounded to a given `smallest_unit`. Time-only durations (no years/months/weeks/days) can be rounded without a reference date; calendar-component durations require a `ZonedDateTime` or `PlainDateTime` anchor.

### `duration_round(d duration, smallest_unit text) → duration`

Rounds a time-only duration to the given unit. Valid units: `'hour'`, `'minute'`, `'second'`, `'millisecond'`, `'microsecond'`, `'nanosecond'`.

```sql
SELECT duration_round('PT1H30M'::temporal.duration, 'hour')::text;  -- PT2H
SELECT duration_round('PT1H29M'::temporal.duration, 'hour')::text;  -- PT1H
```

### `duration_round_zoned(d duration, smallest_unit text, relative_to zoneddatetime) → duration`

Rounds a duration relative to a `ZonedDateTime`. Use for durations with calendar components or when DST-aware day length matters.

```sql
SELECT duration_round_zoned(
  'PT36H'::temporal.duration, 'day',
  '2025-01-15T00:00:00+00:00[UTC]'::temporal.zoneddatetime
)::text;  -- P2D
```

### `duration_round_plain(d duration, smallest_unit text, relative_to plaindatetime) → duration`

Rounds a duration relative to a `PlainDateTime`. Use for calendar-component durations when timezone-aware day length is not needed.

```sql
SELECT duration_round_plain(
  'P1Y6M'::temporal.duration, 'year',
  '2025-01-01T00:00:00'::temporal.plaindatetime
)::text;  -- P2Y
```

## Total

Returns the total value of a duration expressed as a single fractional unit.

### `duration_total(d duration, unit text) → float8`

Returns the total for a time-only duration. Valid units: `'hour'`, `'minute'`, `'second'`, etc.

```sql
SELECT duration_total('PT1H30M'::temporal.duration, 'minute');  -- 90.0
SELECT duration_total('PT1H'::temporal.duration, 'second');     -- 3600.0
```

### `duration_total_zoned(d duration, unit text, relative_to zoneddatetime) → float8`

Returns the total anchored to a `ZonedDateTime` for DST-aware day/month/year lengths.

```sql
-- February 2024 is a leap year: P1M = 29 days
SELECT duration_total_zoned(
  'P1M'::temporal.duration, 'day',
  '2024-02-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
);  -- 29.0
```

### `duration_total_plain(d duration, unit text, relative_to plaindatetime) → float8`

Returns the total anchored to a `PlainDateTime` for calendar-aware month/year lengths.

```sql
-- January has 31 days
SELECT duration_total_plain(
  'P1M'::temporal.duration, 'day',
  '2025-01-01T00:00:00'::temporal.plaindatetime
);  -- 31.0
```

## Relative arithmetic

When either duration contains calendar components (years, months, weeks, or days), addition and subtraction must be anchored to a reference datetime. These functions apply the durations sequentially to the reference point and return the total elapsed duration.

### `duration_add_zoned(a duration, b duration, relative_to zoneddatetime) → duration`

### `duration_subtract_zoned(a duration, b duration, relative_to zoneddatetime) → duration`

Add or subtract two durations relative to a `ZonedDateTime`. DST transitions are respected.

```sql
SELECT duration_add_zoned(
  'PT12H'::temporal.duration, 'PT12H'::temporal.duration,
  '2025-01-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
)::text;

SELECT duration_subtract_zoned(
  'PT12H'::temporal.duration, 'PT6H'::temporal.duration,
  '2025-01-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
)::text;  -- PT6H
```

### `duration_add_plain(a duration, b duration, relative_to plaindatetime) → duration`

### `duration_subtract_plain(a duration, b duration, relative_to plaindatetime) → duration`

Add or subtract two durations relative to a `PlainDateTime`.

```sql
SELECT duration_add_plain(
  'P1Y'::temporal.duration, 'P6M'::temporal.duration,
  '2025-01-01T00:00:00'::temporal.plaindatetime
)::text;
```

The result uses `DifferenceSettings::default()` (largest unit: days for `PlainDateTime`, hours for `ZonedDateTime`). Use `duration_round_*` to balance to larger units if needed.

## Ordering

`Duration` has **no total order** and no B-tree operator class. ISO 8601 durations are not totally orderable without a reference date: `P1M` vs `P30D` is context-dependent. Use `plain_datetime_until` or `zoned_datetime_until` to compare spans anchored to a specific date.

## Limitations / planned

- Constructor function from individual components — not yet implemented
