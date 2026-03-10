# Instant

`temporal.instant` is an absolute point on the UTC timeline with nanosecond precision. It is the pg_temporal equivalent of the [TC39 Temporal `Instant`](https://tc39.es/proposal-temporal/#sec-temporal-instant).

An `Instant` has no timezone and no calendar — it is a single unambiguous moment, always expressed in UTC. Use it when you care only about when something happened, not where or in which calendar system.

## Quick start

```sql
-- Store an instant
INSERT INTO events (ts) VALUES
  ('2025-03-01T02:16:10Z'::temporal.instant);

-- Offsets are accepted and normalized to UTC on input
INSERT INTO events (ts) VALUES
  ('2025-03-01T11:16:10+09:00'::temporal.instant);

-- Both rows above store the same instant and round-trip identically
SELECT ts FROM events;
-- 2025-03-01T02:16:10Z
-- 2025-03-01T02:16:10Z

-- Get the raw epoch nanoseconds for arithmetic
SELECT instant_epoch_ns(ts)::numeric FROM events;
```

## Text format

Input accepts any RFC 9557 instant string — ISO 8601 date/time with either a `Z` suffix or a numeric UTC offset:

```
2025-03-01T02:16:10Z
2025-03-01T11:16:10+09:00
2025-03-01T02:16:10.000000001Z   -- nanosecond precision
```

IANA timezone annotations (`[Asia/Tokyo]`) and calendar annotations (`[u-ca=…]`) are not meaningful for instants and will cause a parse error if provided without a numeric offset.

Output is always UTC with a `Z` suffix:

```
2025-03-01T02:16:10Z
```

## SQL functions

### `instant_epoch_ns(inst instant) → text`

Returns the instant as nanoseconds since the Unix epoch (`1970-01-01T00:00:00Z`). The value is returned as `text` because there is no native 128-bit integer SQL type; cast to `numeric` for arithmetic.

```sql
SELECT instant_epoch_ns('2025-03-01T02:16:10Z'::temporal.instant)::numeric;
-- 1740791770000000000

-- Arithmetic: how many seconds ago was this instant?
SELECT (extract(epoch FROM now()) * 1e9 -
        instant_epoch_ns('2025-03-01T02:16:10Z'::temporal.instant)::numeric
       ) / 1e9 AS seconds_ago;
```

## Comparison operators

All six comparison operators (`<`, `<=`, `=`, `<>`, `>=`, `>`) are supported and backed by a B-tree operator class, enabling `ORDER BY`, `GROUP BY`, `DISTINCT`, and B-tree indexes. Two `Instant` values are equal when they represent the same epoch nanosecond.

```sql
SELECT '2025-03-01T00:00:00Z'::temporal.instant
       < '2025-03-01T01:00:00Z'::temporal.instant;  -- true

-- ORDER BY uses the built-in btree operator class
SELECT * FROM events ORDER BY ts;
```

### `instant_compare(a instant, b instant) → integer`

Returns -1, 0, or 1.

## Arithmetic

Calendar components (years, months, weeks, days) cannot be used in `Instant` arithmetic—those require a timezone to resolve. Use `zoned_datetime_add` for calendar-aware arithmetic.

### `instant_add(inst instant, dur duration) → instant`

Advances the instant by the given time-only duration.

```sql
SELECT instant_add(
  '1970-01-01T00:00:00Z'::temporal.instant,
  'PT1H'::temporal.duration
)::text;  -- 1970-01-01T01:00:00Z
```

### `instant_subtract(inst instant, dur duration) → instant`

Moves the instant back by the given time-only duration.

```sql
SELECT instant_subtract(
  '1970-01-01T01:00:00Z'::temporal.instant,
  'PT1H'::temporal.duration
)::text;  -- 1970-01-01T00:00:00Z
```

### `instant_until(inst instant, other instant) → duration`

Returns the duration from `inst` to `other`. The result uses seconds as the largest unit—a 2-hour gap returns `PT7200S`, not `PT2H`.

```sql
SELECT instant_until(
  '2025-03-01T00:00:00Z'::temporal.instant,
  '2025-03-01T02:00:00Z'::temporal.instant
)::text;  -- PT7200S
```

### `instant_since(inst instant, other instant) → duration`

Returns the duration elapsed from `other` to `inst`. Same seconds-first behavior as `instant_until`.

```sql
SELECT instant_since(
  '2025-03-01T02:00:00Z'::temporal.instant,
  '2025-03-01T00:00:00Z'::temporal.instant
)::text;  -- PT7200S
```

## Constructors

### `make_instant(epoch_ns text) → instant`

Constructs an `Instant` from nanoseconds since the Unix epoch, supplied as `text` (because there is no native 128-bit integer SQL type).

```sql
SELECT make_instant('1609459200000000000')::text;
-- 2021-01-01T00:00:00Z

SELECT make_instant('0')::text;
-- 1970-01-01T00:00:00Z
```

## Now functions

### `temporal_now_instant() → instant`

Returns the current instant at transaction start time. Backed by PostgreSQL's `GetCurrentTimestamp()`, which is frozen at the beginning of the current transaction (repeatable-read semantics).

```sql
SELECT temporal_now_instant();
```

## Limitations / planned

- Cast from/to `timestamptz` (explicit casts only) — not yet implemented
