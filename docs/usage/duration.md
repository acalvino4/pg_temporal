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

## Planned

- Arithmetic functions (`add`, `subtract`, `round`, `total`)
- Constructor function from individual components
- Comparison operators (for strictly time-only durations)
